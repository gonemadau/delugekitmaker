use crate::command::{AudioCommand, AudioEvent};
use crate::mixer::Mixer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("no default output device")]
    NoDevice,
    #[error("output config error: {0}")]
    Config(String),
    #[error("build stream error: {0}")]
    Build(String),
    #[error("play error: {0}")]
    Play(String),
    #[error("unsupported sample format: {0:?}")]
    Format(cpal::SampleFormat),
    #[error("engine thread crashed")]
    ThreadDied,
}

/// A handle to the audio engine. The actual cpal `Stream` lives on a dedicated
/// thread (because it's `!Send` on macOS); this handle only carries channels,
/// which are `Send + Sync`, so it's safe to stash inside Tauri state.
pub struct AudioEngine {
    pub cmd_tx: Sender<AudioCommand>,
    pub evt_rx: Receiver<AudioEvent>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioEngine {
    /// Start the audio thread. Returns once the cpal stream is playing.
    pub fn new() -> Result<Self, EngineError> {
        let (cmd_tx, cmd_rx) = bounded::<AudioCommand>(256);
        let (evt_tx, evt_rx) = bounded::<AudioEvent>(1024);

        // One-shot channel for the spawned thread to report readiness.
        let (ready_tx, ready_rx) =
            std::sync::mpsc::sync_channel::<Result<(u32, u16), EngineError>>(1);

        std::thread::Builder::new()
            .name("dkm-audio".into())
            .spawn(move || run_audio_thread(cmd_rx, evt_tx, ready_tx))
            .map_err(|e| EngineError::Build(e.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok((sr, ch))) => Ok(AudioEngine {
                cmd_tx,
                evt_rx,
                sample_rate: sr,
                channels: ch,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(EngineError::ThreadDied),
        }
    }

    pub fn send(&self, cmd: AudioCommand) {
        if let Err(e) = self.cmd_tx.try_send(cmd) {
            tracing::warn!("audio command queue: {}", e);
        }
    }
}

fn run_audio_thread(
    cmd_rx: Receiver<AudioCommand>,
    evt_tx: Sender<AudioEvent>,
    ready_tx: std::sync::mpsc::SyncSender<Result<(u32, u16), EngineError>>,
) {
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            let _ = ready_tx.send(Err(EngineError::NoDevice));
            return;
        }
    };
    let supported = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = ready_tx.send(Err(EngineError::Config(e.to_string())));
            return;
        }
    };
    let sample_format = supported.sample_format();
    let stream_config: cpal::StreamConfig = supported.clone().into();
    let sr = stream_config.sample_rate.0;
    let ch = stream_config.channels;

    let mut mixer = Mixer::new(sr, ch, cmd_rx, evt_tx.clone());

    let err_evt_tx = evt_tx.clone();
    let err_fn = move |err: cpal::StreamError| {
        tracing::error!("cpal stream error: {}", err);
        let _ = err_evt_tx.try_send(AudioEvent::Underrun);
    };

    let stream_result = match sample_format {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                mixer.drain_commands();
                let out_frames = data.len() / mixer.channels as usize;
                mixer.render(data, out_frames);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => {
            let mut scratch = vec![0f32; 8192];
            device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    mixer.drain_commands();
                    let out_frames = data.len() / mixer.channels as usize;
                    if scratch.len() < data.len() {
                        scratch.resize(data.len(), 0.0);
                    }
                    let f = &mut scratch[..data.len()];
                    mixer.render(f, out_frames);
                    for (d, s) in data.iter_mut().zip(f.iter()) {
                        *d = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    }
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let mut scratch = vec![0f32; 8192];
            device.build_output_stream(
                &stream_config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    mixer.drain_commands();
                    let out_frames = data.len() / mixer.channels as usize;
                    if scratch.len() < data.len() {
                        scratch.resize(data.len(), 0.0);
                    }
                    let f = &mut scratch[..data.len()];
                    mixer.render(f, out_frames);
                    for (d, s) in data.iter_mut().zip(f.iter()) {
                        *d = ((s.clamp(-1.0, 1.0) * 32767.0) as i32 + 32768) as u16;
                    }
                },
                err_fn,
                None,
            )
        }
        f => {
            let _ = ready_tx.send(Err(EngineError::Format(f)));
            return;
        }
    };

    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            let _ = ready_tx.send(Err(EngineError::Build(e.to_string())));
            return;
        }
    };

    if let Err(e) = stream.play() {
        let _ = ready_tx.send(Err(EngineError::Play(e.to_string())));
        return;
    }

    let _ = ready_tx.send(Ok((sr, ch)));

    // Keep the thread alive (and thus the Stream) for the lifetime of the process.
    // The stream's audio callback runs on its own driver-managed thread.
    loop {
        std::thread::park();
    }
}

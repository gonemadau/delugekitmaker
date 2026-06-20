use crate::command::{AudioCommand, AudioEvent, PadParams, SampleId};
use crate::pattern::Pattern;
use crate::sample::DecodedSample;
use crate::voice::Voice;
use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::sync::Arc;

pub const MAX_VOICES: usize = 32;

pub struct Mixer {
    pub voices: [Voice; MAX_VOICES],
    pub samples: HashMap<SampleId, Arc<DecodedSample>>,
    pub device_sample_rate: u32,
    pub channels: u16,
    pub cmd_rx: Receiver<AudioCommand>,
    pub evt_tx: Sender<AudioEvent>,
    pub pattern: ArcSwap<Pattern>,
    pub frame_counter: u64,
    pub next_step_frame: f64,
    pub current_step: u8,
    pub was_playing: bool,
}

impl Mixer {
    pub fn new(
        device_sample_rate: u32,
        channels: u16,
        cmd_rx: Receiver<AudioCommand>,
        evt_tx: Sender<AudioEvent>,
    ) -> Self {
        let voices: [Voice; MAX_VOICES] = std::array::from_fn(|_| Voice::empty());
        Mixer {
            voices,
            samples: HashMap::with_capacity(128),
            device_sample_rate,
            channels,
            cmd_rx,
            evt_tx,
            pattern: ArcSwap::from_pointee(Pattern::default()),
            frame_counter: 0,
            next_step_frame: 0.0,
            current_step: 0,
            was_playing: false,
        }
    }

    pub fn drain_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                AudioCommand::LoadSample(id, s) => {
                    self.samples.insert(id, s);
                }
                AudioCommand::UnloadSample(id) => {
                    self.samples.remove(&id);
                }
                AudioCommand::AuditionPad { pad, sample, params } => {
                    if let Some(s) = self.samples.get(&sample).cloned() {
                        self.start_voice(pad, s, &params, 0);
                    }
                }
                AudioCommand::StopPad(pad) => {
                    for v in self.voices.iter_mut() {
                        if v.active && v.pad == pad {
                            v.stop();
                        }
                    }
                }
                AudioCommand::StopAll => {
                    for v in self.voices.iter_mut() {
                        v.stop();
                    }
                }
                AudioCommand::SetPattern(p) => {
                    // Preserve transport state across pattern edits so toggling
                    // steps during playback does not stop the sequencer.
                    let prev = self.pattern.load_full();
                    let mut next = (*p).clone();
                    next.playing = prev.playing;
                    self.pattern.store(Arc::new(next));
                }
                AudioCommand::SetBpm(bpm) => {
                    let cur = self.pattern.load_full();
                    let mut next = (*cur).clone();
                    next.bpm = bpm;
                    self.pattern.store(Arc::new(next));
                }
                AudioCommand::TransportPlay => {
                    let cur = self.pattern.load_full();
                    let mut next = (*cur).clone();
                    next.playing = true;
                    self.pattern.store(Arc::new(next));
                }
                AudioCommand::TransportStop => {
                    let cur = self.pattern.load_full();
                    let mut next = (*cur).clone();
                    next.playing = false;
                    self.pattern.store(Arc::new(next));
                    for v in self.voices.iter_mut() {
                        v.stop();
                    }
                }
            }
        }
    }

    fn start_voice(&mut self, pad: u8, sample: Arc<DecodedSample>, params: &PadParams, delay: u32) {
        let idx = self
            .voices
            .iter()
            .enumerate()
            .find_map(|(i, v)| if !v.active { Some(i) } else { None })
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, v)| v.age)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });
        self.voices[idx].start(pad, sample, params, self.device_sample_rate, delay);
        let _ = self.evt_tx.try_send(AudioEvent::VoiceStarted { pad });
    }

    fn schedule_steps(&mut self, out_frames: usize) {
        let pat = self.pattern.load();
        if !pat.playing {
            if self.was_playing {
                self.was_playing = false;
                self.current_step = 0;
                self.next_step_frame = self.frame_counter as f64;
            }
            return;
        }
        if !self.was_playing {
            self.was_playing = true;
            self.current_step = 0;
            self.next_step_frame = self.frame_counter as f64;
        }
        let bpm = pat.bpm.clamp(20.0, 300.0);
        let frames_per_step = (self.device_sample_rate as f64 * 60.0) / (bpm as f64 * 4.0);
        let swing = (pat.swing as f64).clamp(-0.5, 0.5);
        let buffer_start = self.frame_counter as f64;
        let buffer_end = buffer_start + out_frames as f64;

        while self.next_step_frame < buffer_end {
            let step_offset = (self.next_step_frame - buffer_start).max(0.0).min(out_frames as f64) as u32;
            let step = self.current_step;
            for (pi, pad_pat) in pat.pads.iter().enumerate() {
                if pad_pat.step_mask & (1u16 << step) != 0 {
                    if let Some(sid) = pad_pat.sample {
                        if let Some(s) = self.samples.get(&sid).cloned() {
                            // Per-step velocity scales the pad's base volume.
                            let vel = pad_pat.velocities[step as usize] as f32 / 127.0;
                            let mut p = pad_pat.params;
                            p.volume *= vel;
                            self.start_voice(pi as u8, s, &p, step_offset);
                        }
                    }
                }
            }
            let _ = self.evt_tx.try_send(AudioEvent::Step { step });
            self.current_step = (self.current_step + 1) % 16;
            // Swing: odd-index 16ths (1, 3, 5...) are pushed forward by half their step.
            // After we've just emitted step N, the next step's gap from "straight" is:
            //   if N is even (we just fired an on-beat 16th) → next step (odd) gets extra delay
            //   if N is odd (we just fired the swung 16th) → next step (even) gets less delay
            // Net average remains frames_per_step.
            let delta = if step % 2 == 0 {
                frames_per_step * (1.0 + swing)
            } else {
                frames_per_step * (1.0 - swing)
            };
            self.next_step_frame += delta;
        }
    }

    pub fn render(&mut self, out: &mut [f32], out_frames: usize) {
        for x in out.iter_mut() {
            *x = 0.0;
        }
        self.schedule_steps(out_frames);

        if self.channels == 2 {
            for v in self.voices.iter_mut() {
                if v.active {
                    v.render_into(out, out_frames);
                    if !v.active {
                        let _ = self.evt_tx.try_send(AudioEvent::VoiceFinished { pad: v.pad });
                    }
                }
            }
        } else {
            let mut scratch = vec![0.0f32; out_frames * 2];
            for v in self.voices.iter_mut() {
                if v.active {
                    v.render_into(&mut scratch, out_frames);
                    if !v.active {
                        let _ = self.evt_tx.try_send(AudioEvent::VoiceFinished { pad: v.pad });
                    }
                }
            }
            for f in 0..out_frames {
                let l = scratch[f * 2];
                let r = scratch[f * 2 + 1];
                for c in 0..self.channels as usize {
                    let idx = f * self.channels as usize + c;
                    if idx < out.len() {
                        out[idx] = if c % 2 == 0 { l } else { r };
                    }
                }
            }
        }

        self.frame_counter += out_frames as u64;
    }
}

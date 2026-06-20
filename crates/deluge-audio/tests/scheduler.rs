//! Offline scheduler tests — exercise the mixer's step scheduling without cpal.

use crossbeam_channel::{bounded, Receiver};
use deluge_audio::mixer::Mixer;
use deluge_audio::{AudioCommand, AudioEvent, DecodedSample, PadPattern, Pattern, SampleId};
use std::sync::Arc;

fn drain_steps(rx: &Receiver<AudioEvent>) -> Vec<u8> {
    let mut steps = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let AudioEvent::Step { step } = e {
            steps.push(step);
        }
    }
    steps
}

#[test]
fn scheduler_emits_steps_at_expected_rate() {
    let (cmd_tx, cmd_rx) = bounded::<AudioCommand>(64);
    let (evt_tx, evt_rx) = bounded::<AudioEvent>(256);
    let sr = 48000u32;
    let mut mixer = Mixer::new(sr, 2, cmd_rx, evt_tx);

    // Load a small sample and set up a pattern where pad 0 plays every step.
    let sample = Arc::new(DecodedSample {
        data: vec![0.5; 200],
        channels: 1,
        sample_rate: sr,
        frames: 200,
    });
    let sid = SampleId::new();
    cmd_tx.send(AudioCommand::LoadSample(sid, sample)).unwrap();
    let mut pat = Pattern::default();
    pat.bpm = 120.0; // 16th steps = 48000/8 = 6000 frames per step
    pat.playing = true;
    pat.pads[0] = PadPattern {
        step_mask: 0xFFFF, // all 16 steps
        sample: Some(sid),
        ..Default::default()
    };
    cmd_tx.send(AudioCommand::SetPattern(Arc::new(pat))).unwrap();
    cmd_tx.send(AudioCommand::TransportPlay).unwrap();

    // Render exactly 1 bar = 16 steps * 6000 frames = 96000 frames.
    // Process in 512-frame chunks like a real cpal callback (drain + render).
    let mut buf = vec![0f32; 512 * 2];
    let mut frames_rendered = 0u64;
    while frames_rendered < 96_000 {
        mixer.drain_commands();
        mixer.render(&mut buf, 512);
        frames_rendered += 512;
    }
    let steps = drain_steps(&evt_rx);
    // We should have emitted exactly one full bar's worth: 16 steps, in order.
    assert!(
        steps.len() >= 16,
        "expected at least 16 step events, got {}: {:?}",
        steps.len(),
        steps
    );
    let first_bar: Vec<u8> = steps.into_iter().take(16).collect();
    let expected: Vec<u8> = (0..16).collect();
    assert_eq!(first_bar, expected, "step sequence wrong");
}

#[test]
fn scheduler_silent_when_stopped() {
    let (_cmd_tx, cmd_rx) = bounded::<AudioCommand>(16);
    let (evt_tx, evt_rx) = bounded::<AudioEvent>(64);
    let mut mixer = Mixer::new(48000, 2, cmd_rx, evt_tx);
    let mut buf = vec![0f32; 512 * 2];
    for _ in 0..40 {
        mixer.drain_commands();
        mixer.render(&mut buf, 512);
    }
    let steps = drain_steps(&evt_rx);
    assert_eq!(steps.len(), 0, "should not emit steps when stopped");
}

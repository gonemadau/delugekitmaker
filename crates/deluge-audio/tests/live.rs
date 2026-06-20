//! Live audio tests. Marked `#[ignore]` because they require a real output device
//! and make sound. Run explicitly with:
//!
//!     cargo test -p deluge-audio --test live -- --ignored --nocapture

use deluge_audio::{AudioCommand, AudioEngine, DecodedSample, PadParams, SampleId};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
#[ignore]
fn live_play_kick_through_speakers() {
    let wav = workspace_root().join("test-sd-root/SAMPLES/Drums/Kicks/909_KICK.WAV");
    assert!(wav.exists(), "fixture missing: {}", wav.display());
    let sample = Arc::new(DecodedSample::decode_wav(&wav).expect("decode"));
    println!(
        "loaded kick: {} frames @ {} Hz, {} ch",
        sample.frames, sample.sample_rate, sample.channels
    );

    let engine = AudioEngine::new().expect("start engine");
    println!("engine: {} Hz, {} ch", engine.sample_rate, engine.channels);

    let id = SampleId::new();
    engine.send(AudioCommand::LoadSample(id, sample));
    std::thread::sleep(Duration::from_millis(100));

    println!("triggering kick on pad 0");
    engine.send(AudioCommand::AuditionPad {
        pad: 0,
        sample: id,
        params: PadParams::default(),
    });

    std::thread::sleep(Duration::from_millis(800));
    println!("done.");
}

#[test]
#[ignore]
fn live_play_sequenced_pattern() {
    // 1 bar of 4-on-the-floor kick + snares on 5/13 + 16ths of hat, via the
    // engine scheduler (not manual scheduling).
    use deluge_audio::{Pattern, PadPattern};
    let root = workspace_root().join("test-sd-root/SAMPLES/Drums");
    let kick = root.join("Kicks/909_KICK.WAV");
    let snare = root.join("Snares/909_SNARE.WAV");
    let hat = root.join("Hats/909_HAT.WAV");
    let engine = AudioEngine::new().expect("engine");
    let kick_id = SampleId::new();
    let snare_id = SampleId::new();
    let hat_id = SampleId::new();
    engine.send(AudioCommand::LoadSample(kick_id, Arc::new(DecodedSample::decode_wav(&kick).unwrap())));
    engine.send(AudioCommand::LoadSample(snare_id, Arc::new(DecodedSample::decode_wav(&snare).unwrap())));
    engine.send(AudioCommand::LoadSample(hat_id, Arc::new(DecodedSample::decode_wav(&hat).unwrap())));
    std::thread::sleep(Duration::from_millis(150));

    let mut pat = Pattern::default();
    pat.bpm = 120.0;
    pat.playing = true;
    // Kick on steps 0, 4, 8, 12
    pat.pads[0] = PadPattern {
        step_mask: 0b0001_0001_0001_0001,
        sample: Some(kick_id),
        ..Default::default()
    };
    // Snare on 4 and 12
    pat.pads[1] = PadPattern {
        step_mask: 0b0001_0000_0001_0000,
        sample: Some(snare_id),
        ..Default::default()
    };
    // Hat on every step
    pat.pads[2] = PadPattern {
        step_mask: 0xFFFF,
        sample: Some(hat_id),
        params: PadParams { volume: 0.4, ..Default::default() },
        ..Default::default()
    };
    engine.send(AudioCommand::SetPattern(Arc::new(pat)));
    engine.send(AudioCommand::TransportPlay);
    println!("sequencer playing 1 bar @120 BPM");
    std::thread::sleep(Duration::from_millis(2200));
    engine.send(AudioCommand::TransportStop);
    std::thread::sleep(Duration::from_millis(300));
    println!("done.");
}

#[test]
#[ignore]
fn live_play_full_kit_sequence() {
    let root = workspace_root().join("test-sd-root/SAMPLES/Drums");
    let kick = root.join("Kicks/909_KICK.WAV");
    let snare = root.join("Snares/909_SNARE.WAV");
    let hat = root.join("Hats/909_HAT.WAV");

    let engine = AudioEngine::new().expect("start engine");
    let kick_id = SampleId::new();
    let snare_id = SampleId::new();
    let hat_id = SampleId::new();
    engine.send(AudioCommand::LoadSample(
        kick_id,
        Arc::new(DecodedSample::decode_wav(&kick).unwrap()),
    ));
    engine.send(AudioCommand::LoadSample(
        snare_id,
        Arc::new(DecodedSample::decode_wav(&snare).unwrap()),
    ));
    engine.send(AudioCommand::LoadSample(
        hat_id,
        Arc::new(DecodedSample::decode_wav(&hat).unwrap()),
    ));
    std::thread::sleep(Duration::from_millis(100));

    let step_ms = 125; // 16ths at 120 BPM
    let pattern: &[(usize, SampleId, u8)] = &[
        (0, kick_id, 12),
        (2, hat_id, 0),
        (4, snare_id, 14),
        (6, hat_id, 0),
        (8, kick_id, 12),
        (10, hat_id, 0),
        (12, snare_id, 14),
        (14, hat_id, 0),
    ];
    println!("playing 1 bar of kick/snare/hat at 120 BPM");
    let start = std::time::Instant::now();
    let mut next_step = 0usize;
    while next_step < 16 {
        let target = Duration::from_millis(next_step as u64 * step_ms);
        while start.elapsed() < target {
            std::thread::sleep(Duration::from_millis(2));
        }
        for (s, id, pad) in pattern {
            if *s == next_step {
                engine.send(AudioCommand::AuditionPad {
                    pad: *pad,
                    sample: *id,
                    params: PadParams::default(),
                });
            }
        }
        next_step += 1;
    }
    std::thread::sleep(Duration::from_millis(500));
    println!("done.");
}

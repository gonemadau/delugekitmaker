use crossbeam_channel::bounded;
use deluge_audio::{AudioCommand, DecodedSample, PadParams, SampleId};
use std::path::PathBuf;
use std::sync::Arc;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn decode_real_test_wav() {
    let wav = workspace_root().join("test-sd-root/SAMPLES/Drums/Kicks/909_KICK.WAV");
    if !wav.exists() {
        eprintln!("skip: {} not present", wav.display());
        return;
    }
    let s = DecodedSample::decode_wav(&wav).expect("decode");
    assert_eq!(s.channels, 1);
    assert_eq!(s.sample_rate, 44100);
    assert!(s.frames > 1000);
    // Some non-zero content
    let energy: f32 = s.data.iter().map(|x| x.abs()).sum();
    assert!(energy > 1.0, "expected non-silent kick sample, got energy {}", energy);
}

#[test]
fn mixer_renders_audition() {
    use deluge_audio::mixer::Mixer;
    let (tx, rx) = bounded::<AudioCommand>(16);
    let (etx, _erx) = bounded(16);
    let mut mixer = Mixer::new(44100, 2, rx, etx);

    // Build a tiny "sample": 1000 frames of 0.5 amplitude mono.
    let sample = Arc::new(DecodedSample {
        data: vec![0.5; 1000],
        channels: 1,
        sample_rate: 44100,
        frames: 1000,
    });
    let id = SampleId::new();
    tx.send(AudioCommand::LoadSample(id, sample)).unwrap();
    tx.send(AudioCommand::AuditionPad {
        pad: 0,
        sample: id,
        params: PadParams::default(),
    })
    .unwrap();

    let mut buf = vec![0.0f32; 512 * 2]; // 512 stereo frames
    mixer.drain_commands();
    mixer.render(&mut buf, 512);

    // Should have produced non-zero output on both channels.
    let energy: f32 = buf.iter().map(|x| x.abs()).sum();
    assert!(energy > 10.0, "expected mixed audio, energy = {}", energy);
}

#[test]
fn mixer_polyphony() {
    use deluge_audio::mixer::Mixer;
    let (tx, rx) = bounded::<AudioCommand>(64);
    let (etx, _erx) = bounded(64);
    let mut mixer = Mixer::new(44100, 2, rx, etx);

    let s = Arc::new(DecodedSample {
        data: vec![0.3; 2000],
        channels: 1,
        sample_rate: 44100,
        frames: 2000,
    });
    let id = SampleId::new();
    tx.send(AudioCommand::LoadSample(id, s.clone())).unwrap();
    for pad in 0..4 {
        tx.send(AudioCommand::AuditionPad {
            pad,
            sample: id,
            params: PadParams::default(),
        })
        .unwrap();
    }

    let mut buf = vec![0.0f32; 256 * 2];
    mixer.drain_commands();
    mixer.render(&mut buf, 256);

    // 4 voices at 0.3 * 0.85 (volume) * pan_gain ≈ summed louder than one.
    let energy_4: f32 = buf.iter().map(|x| x.abs()).sum();
    // Run another mixer with 1 voice for comparison.
    let (tx2, rx2) = bounded::<AudioCommand>(16);
    let (etx2, _erx2) = bounded(16);
    let mut mixer1 = Mixer::new(44100, 2, rx2, etx2);
    tx2.send(AudioCommand::LoadSample(id, s)).unwrap();
    tx2.send(AudioCommand::AuditionPad {
        pad: 0,
        sample: id,
        params: PadParams::default(),
    })
    .unwrap();
    let mut buf2 = vec![0.0f32; 256 * 2];
    mixer1.drain_commands();
    mixer1.render(&mut buf2, 256);
    let energy_1: f32 = buf2.iter().map(|x| x.abs()).sum();
    assert!(
        energy_4 > energy_1 * 3.0,
        "expected 4-voice mix louder than single (got {} vs {})",
        energy_4,
        energy_1
    );
}

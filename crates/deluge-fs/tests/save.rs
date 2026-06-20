use deluge_fs::{save_kit, BundleMode, SDRoot, SaveOptions};
use deluge_xml::{parse_kit, Drum, Kit, OscSample};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn tmp_sd_root(label: &str) -> PathBuf {
    let p = workspace_root()
        .join("target/test-tmp")
        .join(format!("sd-{}-{}", label, ts_nanos()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn ts_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[test]
fn save_kit_bundles_into_kit_subfolder() {
    let src_root = workspace_root().join("test-sd-root");
    let kick = src_root.join("SAMPLES/Drums/Kicks/909_KICK.WAV");
    let snare = src_root.join("SAMPLES/Drums/Snares/909_SNARE.WAV");
    if !kick.exists() || !snare.exists() {
        eprintln!("skip: source samples missing");
        return;
    }
    let sd = tmp_sd_root("bundle");
    let root = SDRoot::validate(&sd).unwrap();

    let mut kit = Kit::with_default_drums("DropTest", 4);
    kit.drums[0] = Drum {
        name: "Kick".into(),
        osc1: Some(OscSample {
            file_name: kick.to_string_lossy().into_owned(), // absolute → will be copied
            start_samples: 0,
            end_samples: 22050,
            ..Default::default()
        }),
        ..Default::default()
    };
    kit.drums[1] = Drum {
        name: "Snare".into(),
        osc1: Some(OscSample {
            file_name: snare.to_string_lossy().into_owned(),
            start_samples: 0,
            end_samples: 33075,
            ..Default::default()
        }),
        ..Default::default()
    };

    let report = save_kit(
        &root,
        &mut kit,
        &SaveOptions {
            bundle_mode: BundleMode::KitSubfolder,
            flavor: "OfficialV4".into(),
        },
    )
    .unwrap();
    // XML written?
    let xml_path = std::path::PathBuf::from(&report.xml_path);
    assert!(xml_path.exists(), "kit XML not written: {}", report.xml_path);
    assert!(xml_path.file_name().unwrap().to_string_lossy().ends_with(".XML"));

    // Samples bundled into KITS/<name>/?
    let bundle = sd.join("KITS/DropTest");
    assert!(bundle.join("909_KICK.WAV").exists(), "kick not bundled");
    assert!(bundle.join("909_SNARE.WAV").exists(), "snare not bundled");

    // Kit XML refs use SD-relative forward-slash paths.
    let xml_text = std::fs::read_to_string(&xml_path).unwrap();
    assert!(xml_text.contains("KITS/DropTest/909_KICK.WAV"));
    assert!(xml_text.contains("KITS/DropTest/909_SNARE.WAV"));

    // Round-trip parse the saved XML.
    let reparsed = parse_kit(&xml_text).expect("reparse saved");
    assert_eq!(reparsed.drums.len(), 4);
    assert_eq!(reparsed.drums[0].name, "Kick");
    assert_eq!(reparsed.drums[0].osc1.as_ref().unwrap().file_name, "KITS/DropTest/909_KICK.WAV");
    assert_eq!(reparsed.drums[1].osc1.as_ref().unwrap().file_name, "KITS/DropTest/909_SNARE.WAV");
}

#[test]
fn save_kit_shared_samples_mode() {
    let src_root = workspace_root().join("test-sd-root");
    let kick = src_root.join("SAMPLES/Drums/Kicks/909_KICK.WAV");
    if !kick.exists() {
        return;
    }
    let sd = tmp_sd_root("shared");
    let root = SDRoot::validate(&sd).unwrap();

    let mut kit = Kit::with_default_drums("Shared", 1);
    kit.drums[0] = Drum {
        name: "Kick".into(),
        osc1: Some(OscSample {
            file_name: kick.to_string_lossy().into_owned(),
            start_samples: 0,
            end_samples: 22050,
            ..Default::default()
        }),
        ..Default::default()
    };
    let _ = save_kit(
        &root,
        &mut kit,
        &SaveOptions {
            bundle_mode: BundleMode::SharedSamples,
            flavor: "OfficialV4".into(),
        },
    )
    .unwrap();
    assert!(sd.join("SAMPLES/KIT MAKER/Shared/909_KICK.WAV").exists());
    assert_eq!(
        kit.drums[0].osc1.as_ref().unwrap().file_name,
        "SAMPLES/KIT MAKER/Shared/909_KICK.WAV"
    );
}

#[test]
fn save_kit_dedup_same_file() {
    let src_root = workspace_root().join("test-sd-root");
    let kick = src_root.join("SAMPLES/Drums/Kicks/909_KICK.WAV");
    if !kick.exists() {
        return;
    }
    let sd = tmp_sd_root("dedup");
    let root = SDRoot::validate(&sd).unwrap();

    // Save twice — second save should dedup the copy.
    for _ in 0..2 {
        let mut kit = Kit::with_default_drums("Dedup", 1);
        kit.drums[0] = Drum {
            name: "Kick".into(),
            osc1: Some(OscSample {
                file_name: kick.to_string_lossy().into_owned(),
                ..Default::default()
            }),
            ..Default::default()
        };
        save_kit(
            &root,
            &mut kit,
            &SaveOptions::default(),
        )
        .unwrap();
    }
    // Only one bundled file should exist (no _1 suffix).
    let bundle = sd.join("KITS/Dedup");
    let entries: Vec<_> = std::fs::read_dir(&bundle)
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec!["909_KICK.WAV".to_string()], "expected single bundled file, got {:?}", entries);
}

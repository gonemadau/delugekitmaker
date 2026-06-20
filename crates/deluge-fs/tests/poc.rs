//! POC end-to-end test: simulate dropping a folder of WAVs, auto-classify them
//! into a kit, save the kit, list/parse it back, and verify integrity.

use deluge_classify::auto_layout;
use deluge_fs::{list_kits, save_kit, BundleMode, SDRoot, SaveOptions};
use deluge_xml::{parse_kit, Drum, Kit, OscSample};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn ts_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn make_drop_folder(label: &str) -> PathBuf {
    let src = workspace_root().join("test-sd-root/SAMPLES/Drums");
    let kick = src.join("Kicks/909_KICK.WAV");
    let snare = src.join("Snares/909_SNARE.WAV");
    let hat = src.join("Hats/909_HAT.WAV");
    let dir = workspace_root()
        .join("target/test-tmp")
        .join(format!("drop-{}-{}", label, ts_nanos()));
    fs::create_dir_all(&dir).unwrap();
    // Copy with friendly names for the classifier.
    fs::copy(&kick, dir.join("kick_909.wav")).unwrap();
    fs::copy(&snare, dir.join("snare_tight.wav")).unwrap();
    fs::copy(&hat, dir.join("hh_closed.wav")).unwrap();
    fs::copy(&hat, dir.join("ohh_open.wav")).unwrap();
    fs::copy(&snare, dir.join("clap.wav")).unwrap();
    dir
}

#[test]
fn poc_end_to_end_drop_save_reopen() {
    let src = workspace_root().join("test-sd-root/SAMPLES/Drums/Kicks/909_KICK.WAV");
    if !src.exists() {
        eprintln!("skip: source samples missing");
        return;
    }
    let drop = make_drop_folder("poc");
    let sd = workspace_root()
        .join("target/test-tmp")
        .join(format!("sd-poc-{}", ts_nanos()));
    fs::create_dir_all(&sd).unwrap();
    let root = SDRoot::validate(&sd).unwrap();

    // 1. Discover WAVs in the drop folder.
    let mut files: Vec<PathBuf> = fs::read_dir(&drop)
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()).map(|s| s.eq_ignore_ascii_case("wav")).unwrap_or(false))
        .collect();
    files.sort();
    let names: Vec<String> = files.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();

    // 2. Auto-classify into a 16-pad layout.
    let layout = auto_layout(&names);
    // Verify expected categories landed in canonical slots (Deluge layout).
    assert_eq!(layout[12].as_deref(), Some("kick_909.wav"), "kick→pad 12 (bottom-left)");
    assert_eq!(layout[8].as_deref(), Some("snare_tight.wav"), "snare→pad 8");
    assert_eq!(layout[10].as_deref(), Some("clap.wav"), "clap→pad 10");
    assert_eq!(layout[4].as_deref(), Some("hh_closed.wav"), "closed hat→pad 4");
    assert_eq!(layout[5].as_deref(), Some("ohh_open.wav"), "open hat→pad 5");

    // 3. Build a Kit using the layout and the absolute file paths from the drop dir.
    let mut kit = Kit::with_default_drums("DropPOC", 16);
    for (pad_idx, slot) in layout.iter().enumerate() {
        if let Some(name) = slot {
            let abs = drop.join(name);
            kit.drums[pad_idx] = Drum {
                name: name.trim_end_matches(".wav").to_string(),
                osc1: Some(OscSample {
                    file_name: abs.to_string_lossy().into_owned(),
                    ..Default::default()
                }),
                ..Default::default()
            };
        }
    }

    // 4. Save.
    let report = save_kit(
        &root,
        &mut kit,
        &SaveOptions {
            bundle_mode: BundleMode::KitSubfolder,
            flavor: "OfficialV4".into(),
        },
    )
    .unwrap();
    assert!(Path::new(&report.xml_path).exists());

    // 5. The KITS dir should now list our kit.
    let kits = list_kits(&root).unwrap();
    assert!(kits.iter().any(|k| k.name == "DROPPOC"), "expected DROPPOC kit; got {:?}", kits);

    // 6. Re-parse the saved XML and verify the pad layout is intact.
    //    The writer omits empty pads, so we expect exactly the 5 sample-bearing
    //    drums back, regardless of which slot they came from.
    let xml = fs::read_to_string(&report.xml_path).unwrap();
    let reopened = parse_kit(&xml).unwrap();
    assert_eq!(reopened.drums.len(), 16);
    let names: Vec<&str> = reopened.drums.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"kick_909"));
    assert!(names.contains(&"snare_tight"));
    assert!(names.contains(&"hh_closed"));
    let kick = reopened.drums.iter().find(|d| d.name == "kick_909").unwrap();
    assert_eq!(kick.osc1.as_ref().unwrap().file_name, "KITS/DropPOC/kick_909.wav");
    assert!(kick.osc1.as_ref().unwrap().end_ms > 0, "end_ms must be filled from WAV duration");

    // 7. The bundled sample files actually exist on disk.
    let bundle = sd.join("KITS/DropPOC");
    assert!(bundle.join("kick_909.wav").exists());
    assert!(bundle.join("snare_tight.wav").exists());
    assert!(bundle.join("hh_closed.wav").exists());
    assert!(bundle.join("ohh_open.wav").exists());
    assert!(bundle.join("clap.wav").exists());
}

#[test]
fn poc_dedupes_when_redropped() {
    let src = workspace_root().join("test-sd-root/SAMPLES/Drums/Kicks/909_KICK.WAV");
    if !src.exists() {
        return;
    }
    let drop = make_drop_folder("dedup");
    let sd = workspace_root()
        .join("target/test-tmp")
        .join(format!("sd-redrop-{}", ts_nanos()));
    fs::create_dir_all(&sd).unwrap();
    let root = SDRoot::validate(&sd).unwrap();

    // Save twice (simulating user re-dropping the same folder).
    for _ in 0..2 {
        let mut files: Vec<PathBuf> = fs::read_dir(&drop)
            .unwrap()
            .filter_map(|r| r.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()).map(|s| s.eq_ignore_ascii_case("wav")).unwrap_or(false))
            .collect();
        files.sort();
        let names: Vec<String> = files.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        let layout = auto_layout(&names);
        let mut kit = Kit::with_default_drums("RedropPOC", 16);
        for (pad_idx, slot) in layout.iter().enumerate() {
            if let Some(name) = slot {
                kit.drums[pad_idx].osc1 = Some(OscSample {
                    file_name: drop.join(name).to_string_lossy().into_owned(),
                    ..Default::default()
                });
            }
        }
        save_kit(&root, &mut kit, &SaveOptions::default()).unwrap();
    }
    let bundle = sd.join("KITS/RedropPOC");
    let entries: Vec<String> = fs::read_dir(&bundle)
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let dups: Vec<&String> = entries.iter().filter(|n| n.contains("_1.")).collect();
    assert!(dups.is_empty(), "expected no duplicate files; found {:?}", dups);
    assert_eq!(entries.len(), 5, "expected 5 unique bundled samples");
}

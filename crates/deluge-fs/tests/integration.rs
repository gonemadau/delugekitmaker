use deluge_fs::{list_kits, SDRoot};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let cargo_manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(cargo_manifest).parent().unwrap().parent().unwrap().to_path_buf()
}

#[test]
fn validate_creates_subdirs_in_empty_root() {
    let tmp = tempdir_in_target();
    let _ = SDRoot::validate(&tmp).expect("validate");
    assert!(tmp.join("KITS").is_dir());
    assert!(tmp.join("SAMPLES").is_dir());
    assert!(tmp.join("SONGS").is_dir());
    assert!(tmp.join("SYNTHS").is_dir());
}

#[test]
fn lists_kits_from_test_sd_root() {
    // The fake SD root created during dev: workspace_root/test-sd-root with KIT_BASIC.XML
    let sd_root = workspace_root().join("test-sd-root");
    if !sd_root.exists() {
        eprintln!("skipping: {} not present", sd_root.display());
        return;
    }
    let root = SDRoot::validate(&sd_root).expect("validate");
    let kits = list_kits(&root).expect("list");
    assert!(!kits.is_empty(), "expected at least one kit");
    let basic = kits.iter().find(|k| k.name == "KIT_BASIC");
    assert!(basic.is_some(), "expected KIT_BASIC fixture; got: {:?}", kits);
    let basic = basic.unwrap();
    assert_eq!(basic.rel_path, "KITS/KIT_BASIC.XML");
    assert!(basic.size_bytes > 0);
}

#[test]
fn end_to_end_open_kit_via_layer() {
    let sd_root = workspace_root().join("test-sd-root");
    if !sd_root.exists() {
        return;
    }
    let root = SDRoot::validate(&sd_root).expect("validate");
    let kits = list_kits(&root).expect("list");
    let kit_summary = kits
        .iter()
        .find(|k| k.name == "KIT_BASIC")
        .expect("fixture present");
    let xml = std::fs::read_to_string(
        root.root().join(kit_summary.rel_path.replace('/', std::path::MAIN_SEPARATOR_STR.as_ref())),
    )
    .expect("read");
    let kit = deluge_xml::parse_kit(&xml).expect("parse");
    // Parser always builds a 16-slot array; XML order maps to UI pads
    // bottom-up so the first XML sound lands at pad 12 (UI bottom-left).
    assert_eq!(kit.drums.len(), 16);
    assert_eq!(kit.drums[12].name, "Kick");
    assert_eq!(kit.drums[13].name, "Snare");
    assert_eq!(kit.drums[14].name, "Hat");
}

fn tempdir_in_target() -> PathBuf {
    let target = workspace_root().join("target").join("test-tmp").join(format!(
        "sdtest-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&target).unwrap();
    target
}

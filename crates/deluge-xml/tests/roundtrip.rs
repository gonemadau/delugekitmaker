use deluge_xml::{parse_kit, write_kit, Flavor};

const KIT_BASIC: &str = include_str!("fixtures/KIT_BASIC.XML");

#[test]
fn parses_basic_kit_fields() {
    let kit = parse_kit(KIT_BASIC).expect("parse");
    // Older fixture still carries firmware attrs; parser keeps them for fidelity.
    assert_eq!(kit.firmware_version, "4.1.4");
    assert_eq!(kit.earliest_compatible_firmware, "4.1.0-pre1");
    // Drums Vec is now always 16 slots (one per UI pad). XML order maps to
    // pads bottom-up: drum 0 → pad 12, drum 1 → pad 13, drum 2 → pad 14, ...
    assert_eq!(kit.drums.len(), 16);

    let kick = &kit.drums[12]; // first XML drum → bottom-left
    assert_eq!(kick.name, "Kick");
    let osc = kick.osc1.as_ref().expect("kick osc1");
    assert_eq!(osc.file_name, "SAMPLES/Drums/Kicks/909_KICK.WAV");
    assert_eq!(osc.start_samples, 0);
    assert_eq!(osc.end_samples, 22050);
    assert!(!osc.reversed);
    assert_eq!(kick.volume_hex.as_deref(), Some("0x4CCCCCA8"));

    let snare = &kit.drums[13]; // second XML drum
    assert_eq!(snare.name, "Snare");
    assert_eq!(snare.osc1.as_ref().unwrap().start_samples, 100);
    assert_eq!(snare.osc1.as_ref().unwrap().end_samples, 33075);

    let hat = &kit.drums[14]; // third XML drum
    assert_eq!(hat.name, "Hat");
    assert!(hat.osc1.as_ref().unwrap().reversed, "hat should be reversed");
}

#[test]
fn round_trip_preserves_modeled_fields() {
    // Round-trip must preserve which UI pad each drum sits on, by name+path.
    let kit = parse_kit(KIT_BASIC).expect("parse");
    let written = write_kit(&kit, Flavor::OfficialV4).expect("write");
    let kit2 = parse_kit(&written).expect("reparse written");
    assert_eq!(kit.drums.len(), kit2.drums.len());
    for (a, b) in kit.drums.iter().zip(kit2.drums.iter()) {
        assert_eq!(a.name, b.name, "pad name preserved");
        match (&a.osc1, &b.osc1) {
            (Some(oa), Some(ob)) => {
                assert_eq!(oa.file_name, ob.file_name);
                assert_eq!(oa.reversed, ob.reversed);
                assert_eq!(oa.loop_mode, ob.loop_mode);
            }
            (None, None) => {}
            _ => panic!("osc1 presence mismatch for drum {}", a.name),
        }
    }
}

#[test]
fn writer_produces_loadable_format() {
    let kit = parse_kit(KIT_BASIC).expect("parse");
    let written = write_kit(&kit, Flavor::OfficialV4).expect("write");
    assert!(written.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    // No firmware gate — the Deluge would refuse the kit on older firmware
    // if these attributes were present.
    assert!(!written.contains("firmwareVersion=\""));
    assert!(!written.contains("earliestCompatibleFirmware"));
    assert!(written.contains("<soundSources>"));
    assert!(written.contains("<name>Kick</name>"));
    assert!(written.contains("<fileName>SAMPLES/Drums/Kicks/909_KICK.WAV</fileName>"));
    // Zone in milliseconds, not sample positions.
    assert!(written.contains("<startMilliseconds>"));
    assert!(!written.contains("<startSamplePos>"));
}

#[test]
fn round_trip_a_freshly_built_kit() {
    // Place samples at UI bottom-row pads so they survive round-trip in known slots.
    let mut kit = deluge_xml::Kit::with_default_drums("TestKit", 16);
    kit.drums[12].osc1 = Some(deluge_xml::OscSample {
        file_name: "SAMPLES/KIT MAKER/TestKit/KICK.WAV".into(),
        end_ms: 1000,
        ..Default::default()
    });
    kit.drums[13].osc1 = Some(deluge_xml::OscSample {
        file_name: "SAMPLES/KIT MAKER/TestKit/SNARE.WAV".into(),
        start_ms: 50,
        end_ms: 500,
        reversed: true,
        ..Default::default()
    });
    let xml = write_kit(&kit, Flavor::OfficialV4).expect("write");
    let back = parse_kit(&xml).expect("reparse");
    // Always 16 pads; the two populated ones round-trip to their original UI pads.
    assert_eq!(back.drums.len(), 16);
    assert_eq!(back.drums[12].osc1.as_ref().unwrap().file_name, "SAMPLES/KIT MAKER/TestKit/KICK.WAV");
    assert_eq!(back.drums[12].osc1.as_ref().unwrap().end_ms, 1000);
    assert!(back.drums[13].osc1.as_ref().unwrap().reversed);
    assert_eq!(back.drums[13].osc1.as_ref().unwrap().start_ms, 50);
    assert_eq!(back.drums[13].osc1.as_ref().unwrap().end_ms, 500);
}

#[test]
fn xml_emission_order_is_bottom_up() {
    // Place distinct samples on pads 0 (top-left) and 12 (bottom-left).
    // The bottom-left one must appear FIRST in the XML so the Deluge renders
    // it on its bottom row, matching the UI.
    let mut kit = deluge_xml::Kit::with_default_drums("Order", 16);
    kit.drums[0].osc1 = Some(deluge_xml::OscSample {
        file_name: "TOP.WAV".into(), end_ms: 100, ..Default::default()
    });
    kit.drums[0].name = "TOP_LEFT".into();
    kit.drums[12].osc1 = Some(deluge_xml::OscSample {
        file_name: "BOTTOM.WAV".into(), end_ms: 100, ..Default::default()
    });
    kit.drums[12].name = "BOTTOM_LEFT".into();
    let xml = write_kit(&kit, Flavor::OfficialV4).expect("write");
    let bottom_idx = xml.find("BOTTOM_LEFT").expect("bottom in xml");
    let top_idx = xml.find("TOP_LEFT").expect("top in xml");
    assert!(bottom_idx < top_idx, "bottom-row pad must serialise before top-row pad");
}

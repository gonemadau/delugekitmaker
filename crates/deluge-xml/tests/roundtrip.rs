use deluge_xml::{parse_kit, write_kit, Flavor, XmlError};

const KIT_BASIC: &str = include_str!("fixtures/KIT_BASIC.XML");
const KIT_LEGACY_V2: &str = include_str!("fixtures/KIT_LEGACY_V2.XML");

#[test]
fn parses_basic_kit_fields() {
    let kit = parse_kit(KIT_BASIC).expect("parse");
    assert_eq!(kit.firmware_version, "4.1.4");
    assert_eq!(kit.earliest_compatible_firmware, "4.1.0-pre1");
    assert_eq!(kit.drums.len(), 3);

    let kick = &kit.drums[0];
    assert_eq!(kick.name, "Kick");
    let osc = kick.osc1.as_ref().expect("kick osc1");
    assert_eq!(osc.file_name, "SAMPLES/Drums/Kicks/909_KICK.WAV");
    assert_eq!(osc.start_samples, 0);
    assert_eq!(osc.end_samples, 22050);
    assert!(!osc.reversed);
    assert_eq!(kick.volume_hex.as_deref(), Some("0x4CCCCCA8"));

    let snare = &kit.drums[1];
    assert_eq!(snare.name, "Snare");
    assert_eq!(snare.osc1.as_ref().unwrap().start_samples, 100);
    assert_eq!(snare.osc1.as_ref().unwrap().end_samples, 33075);

    let hat = &kit.drums[2];
    assert_eq!(hat.name, "Hat");
    assert!(hat.osc1.as_ref().unwrap().reversed, "hat should be reversed");
}

#[test]
fn round_trip_preserves_modeled_fields() {
    let kit = parse_kit(KIT_BASIC).expect("parse");
    let written = write_kit(&kit, Flavor::OfficialV4).expect("write");
    let kit2 = parse_kit(&written).expect("reparse written");
    assert_eq!(kit.drums.len(), kit2.drums.len());
    for (a, b) in kit.drums.iter().zip(kit2.drums.iter()) {
        assert_eq!(a.name, b.name);
        match (&a.osc1, &b.osc1) {
            (Some(oa), Some(ob)) => {
                assert_eq!(oa.file_name, ob.file_name);
                assert_eq!(oa.start_samples, ob.start_samples);
                assert_eq!(oa.end_samples, ob.end_samples);
                assert_eq!(oa.reversed, ob.reversed);
                assert_eq!(oa.loop_mode, ob.loop_mode);
            }
            (None, None) => {}
            _ => panic!("osc1 presence mismatch for drum {}", a.name),
        }
        assert_eq!(a.volume_hex, b.volume_hex, "volume mismatch on {}", a.name);
        assert_eq!(a.pan_hex, b.pan_hex, "pan mismatch on {}", a.name);
    }
}

#[test]
fn write_uses_v4_header() {
    let kit = parse_kit(KIT_BASIC).expect("parse");
    let written = write_kit(&kit, Flavor::OfficialV4).expect("write");
    assert!(written.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(written.contains("firmwareVersion=\"4.1.4\""));
    assert!(written.contains("earliestCompatibleFirmware=\"4.1.0-pre1\""));
    assert!(written.contains("<soundSources>"));
    assert!(written.contains("<name>Kick</name>"));
    assert!(written.contains("<fileName>SAMPLES/Drums/Kicks/909_KICK.WAV</fileName>"));
}

#[test]
fn rejects_legacy_v2_multi_root() {
    match parse_kit(KIT_LEGACY_V2) {
        Err(XmlError::LegacyV2) => {}
        other => panic!("expected LegacyV2, got {:?}", other.map(|_| "ok")),
    }
}

#[test]
fn round_trip_a_freshly_built_kit() {
    let mut kit = deluge_xml::Kit::with_default_drums("TestKit", 4);
    kit.drums[0].osc1 = Some(deluge_xml::OscSample {
        file_name: "SAMPLES/KIT MAKER/TestKit/KICK.WAV".into(),
        start_samples: 0,
        end_samples: 44100,
        ..Default::default()
    });
    kit.drums[1].osc1 = Some(deluge_xml::OscSample {
        file_name: "SAMPLES/KIT MAKER/TestKit/SNARE.WAV".into(),
        start_samples: 50,
        end_samples: 22050,
        reversed: true,
        ..Default::default()
    });
    let xml = write_kit(&kit, Flavor::OfficialV4).expect("write");
    let back = parse_kit(&xml).expect("reparse");
    assert_eq!(back.drums.len(), 4);
    assert_eq!(back.drums[0].osc1.as_ref().unwrap().file_name, "SAMPLES/KIT MAKER/TestKit/KICK.WAV");
    assert!(back.drums[1].osc1.as_ref().unwrap().reversed);
    assert_eq!(back.drums[1].osc1.as_ref().unwrap().start_samples, 50);
}

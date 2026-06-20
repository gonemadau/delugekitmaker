//! Lock the writer's output structure against a real Deluge-saved kit.
//!
//! We compare element NAMES (not values, which vary per drum) so that any
//! future regression that drops a required block — modKnobs, patchCables,
//! envelopes, etc — fails this test instead of silently producing kits the
//! Deluge refuses to play.

use deluge_xml::{parse_kit, write_kit, Drum, Flavor, Kit, OscSample};

const WORKING_KIT: &str = include_str!("fixtures/KIT_WORKING_REFERENCE.XML");

/// Collect every element name in the document in encounter order. Whitespace
/// and attributes are ignored.
fn element_names(xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut names = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                names.push(String::from_utf8_lossy(e.name().as_ref()).into_owned());
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        buf.clear();
    }
    names
}

/// Sub-tree of element names emitted *within* the first `<sound>` of the
/// document. Used to compare per-sound structure between our output and the
/// reference.
fn first_sound_subtree(xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut names = Vec::new();
    let mut buf = Vec::new();
    let mut depth: i32 = -1;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let n = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if depth >= 0 {
                    names.push(n.clone());
                    depth += 1;
                } else if n == "sound" {
                    depth = 0;
                }
            }
            Ok(Event::End(e)) => {
                let n = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if depth > 0 {
                    depth -= 1;
                } else if depth == 0 && n == "sound" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        buf.clear();
    }
    names
}

fn build_one_drum_kit(name: &str, sample_path: &str, end_ms: u32) -> Kit {
    // Place the test drum at UI pad 12 (bottom-left) so it's emitted as XML
    // sound 0, mirroring the working reference where the first sound is what
    // the Deluge shows at the bottom of the kit.
    let mut kit = Kit::with_default_drums(name, 16);
    kit.drums[12] = Drum {
        name: "TEST".into(),
        osc1: Some(OscSample {
            file_name: sample_path.into(),
            end_ms,
            ..Default::default()
        }),
        ..Default::default()
    };
    kit
}

#[test]
fn reference_kit_loads_without_firmware_gate() {
    let kit = parse_kit(WORKING_KIT).expect("reference kit must parse");
    // 16-pad layout with sounds placed at their XML-order pads.
    assert_eq!(kit.drums.len(), 16);
    // First XML sound is "CHOR" — lands at pad 12 (UI bottom-left).
    let chor = &kit.drums[12];
    assert_eq!(chor.name, "CHOR");
    let osc = chor.osc1.as_ref().expect("CHOR has osc1");
    assert_eq!(osc.start_ms, 120);
    assert_eq!(osc.end_ms, 7428);
    assert_eq!(osc.file_name, "SAMPLES/ARTISTS/Phil Elverum/Chord Clearmoon.wav");
}

#[test]
fn writer_emits_no_firmware_attributes_on_kit() {
    let kit = build_one_drum_kit("MyKit", "SAMPLES/x.wav", 1000);
    let xml = write_kit(&kit, Flavor::OfficialV4).unwrap();
    assert!(
        !xml.contains("firmwareVersion"),
        "writer must not gate kits with a firmwareVersion attribute"
    );
    assert!(
        !xml.contains("earliestCompatibleFirmware"),
        "writer must not include earliestCompatibleFirmware"
    );
}

#[test]
fn writer_emits_zone_in_milliseconds() {
    let kit = build_one_drum_kit("MyKit", "SAMPLES/x.wav", 7428);
    let xml = write_kit(&kit, Flavor::OfficialV4).unwrap();
    assert!(xml.contains("<startMilliseconds>0</startMilliseconds>"));
    assert!(xml.contains("<endMilliseconds>7428</endMilliseconds>"));
    assert!(!xml.contains("startSamplePos"), "must not emit sample-pos zones");
}

#[test]
fn writer_emits_full_sound_block_structure() {
    let kit = build_one_drum_kit("MyKit", "SAMPLES/x.wav", 1000);
    let xml = write_kit(&kit, Flavor::OfficialV4).unwrap();

    // Every block the Deluge expects on a sound must appear.
    let required = [
        "<osc1>", "<osc2>", "<lfo1>", "<lfo2>",
        "<unison>", "<compressor>", "<defaultParams>",
        "<envelope1>", "<envelope2>",
        "<patchCables>", "<patchCable>",
        "<stutterRate>", "<sampleRateReduction>", "<bitCrush>",
        "<equalizer>", "<modFXOffset>", "<modFXFeedback>",
        "<midiKnobs>", "<modKnobs>", "<modKnob>",
    ];
    for needle in required {
        assert!(xml.contains(needle), "writer output missing {}", needle);
    }
}

#[test]
fn writer_sound_subtree_matches_reference_structure() {
    // Per-sound element sequence should mirror the working reference exactly.
    // (We don't compare the kit-level fields because the reference has 8
    // drums and we're building a 1-drum kit.)
    let kit = build_one_drum_kit("MyKit", "SAMPLES/x.wav", 1000);
    let our_xml = write_kit(&kit, Flavor::OfficialV4).unwrap();
    let our_sub = first_sound_subtree(&our_xml);
    let ref_sub = first_sound_subtree(WORKING_KIT);
    assert_eq!(
        our_sub, ref_sub,
        "per-sound element sequence must match the working reference.\n\nOurs:\n  {}\n\nReference:\n  {}",
        our_sub.join(" "),
        ref_sub.join(" "),
    );
}

#[test]
fn writer_top_level_kit_skeleton() {
    let kit = build_one_drum_kit("MyKit", "SAMPLES/x.wav", 1000);
    let xml = write_kit(&kit, Flavor::OfficialV4).unwrap();
    let names = element_names(&xml);
    // First few elements should mirror the reference's outer shape:
    // kit, lpfMode, modFXType, modFXCurrentParam, currentFilterType, defaultParams, ...
    assert_eq!(names[0], "kit");
    assert_eq!(names[1], "lpfMode");
    assert_eq!(names[2], "modFXType");
    assert_eq!(names[3], "modFXCurrentParam");
    assert_eq!(names[4], "currentFilterType");
    assert_eq!(names[5], "defaultParams");
}

#[test]
fn round_trip_preserves_zone_milliseconds() {
    let kit = build_one_drum_kit("MyKit", "SAMPLES/x.wav", 12345);
    let xml = write_kit(&kit, Flavor::OfficialV4).unwrap();
    let reparsed = parse_kit(&xml).unwrap();
    assert_eq!(reparsed.drums.len(), 16);
    // The drum was placed at pad 12 (bottom-left), survives round-trip there.
    let osc = reparsed.drums[12].osc1.as_ref().unwrap();
    assert_eq!(osc.end_ms, 12345);
}

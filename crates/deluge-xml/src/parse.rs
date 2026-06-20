use crate::{
    error::{XmlError, XmlResult},
    kit::{Drum, Kit, OscSample, XML_TO_PAD},
};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Parse a Deluge kit XML string into a [`Kit`].
///
/// Only the fields the POC models are extracted; everything else is ignored.
/// Rejects legacy v2.x multi-root kits with a clear error.
pub fn parse_kit(xml: &str) -> XmlResult<Kit> {
    // strip BOM
    let xml = xml.trim_start_matches('\u{feff}');

    // Quick sanity: must contain a <kit> root element. Accept it whether it's
    // a direct root or wrapped (v2 has <firmwareVersion> as a sibling). We
    // explicitly reject the v2 multi-root format.
    detect_legacy_multi_root(xml)?;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut kit = Kit::default();
    // Pre-allocate the 16 UI pad slots so XML sounds can land at their mapped
    // pad index (bottom row first in XML → pads 12..15 in the UI).
    kit.drums = (0..16).map(|_| Drum::default()).collect();
    let mut current_drum: Option<DrumBuilder> = None;
    let mut sound_count: usize = 0;
    let mut in_kit = false;
    let mut in_sound_sources = false;
    let mut sound_depth: u32 = 0;
    // path stack tracks element nesting within a sound — purely for context
    let mut path: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())?.to_string();
                match name.as_str() {
                    "kit" => {
                        in_kit = true;
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref())?.to_string();
                            let val = attr.decode_and_unescape_value(reader.decoder())?.into_owned();
                            match key.as_str() {
                                "firmwareVersion" => kit.firmware_version = val,
                                "earliestCompatibleFirmware" => kit.earliest_compatible_firmware = val,
                                _ => {}
                            }
                        }
                    }
                    "soundSources" if in_kit => in_sound_sources = true,
                    "sound" if in_sound_sources => {
                        sound_depth = 1;
                        current_drum = Some(DrumBuilder::default());
                    }
                    _ => {
                        if sound_depth > 0 {
                            sound_depth += 1;
                            path.push(name.clone());
                        }
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                // self-closing tag like <foo/> — treat as start+end with empty text
                let _name = std::str::from_utf8(e.name().as_ref())?.to_string();
                // nothing in our model uses self-closing values, ignore
            }
            Ok(Event::Text(t)) => {
                if sound_depth > 0 {
                    let text = t.unescape()?.into_owned();
                    if let Some(b) = current_drum.as_mut() {
                        apply_text(&path, &text, b);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = std::str::from_utf8(e.name().as_ref())?.to_string();
                if name == "sound" && sound_depth > 0 {
                    if let Some(b) = current_drum.take() {
                        // Place each parsed sound at the UI pad it corresponds to,
                        // not sequentially. Extra sounds past 16 are dropped.
                        let pad_idx = XML_TO_PAD.get(sound_count).copied().unwrap_or(usize::MAX);
                        if pad_idx < kit.drums.len() {
                            kit.drums[pad_idx] = b.into_drum();
                        }
                        sound_count += 1;
                    }
                    sound_depth = 0;
                    path.clear();
                } else if name == "soundSources" {
                    in_sound_sources = false;
                } else if name == "kit" {
                    in_kit = false;
                } else if sound_depth > 0 {
                    sound_depth -= 1;
                    path.pop();
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(XmlError::Event {
                    pos: reader.buffer_position() as usize,
                    msg: e.to_string(),
                })
            }
        }
        buf.clear();
    }

    // We always start with 16 empty pads now; success means we either saw a
    // <kit> root (firmware attr or sound) or at least one drum landed.
    let any_sound = kit.drums.iter().any(|d| d.osc1.as_ref().map(|o| !o.file_name.is_empty()).unwrap_or(false));
    if !kit.firmware_version.is_empty() || any_sound || sound_count > 0 {
        Ok(kit)
    } else {
        Err(XmlError::NotAKit("(no <kit> root encountered)".into()))
    }
}

fn detect_legacy_multi_root(xml: &str) -> XmlResult<()> {
    // Heuristic: v2.x kit XML has a <firmwareVersion>...</firmwareVersion> element
    // *before* the <kit> element (two roots). v3+ encodes firmwareVersion as an
    // attribute on <kit>.
    let lower = xml.to_lowercase();
    if let (Some(fv_idx), Some(kit_idx)) = (lower.find("<firmwareversion>"), lower.find("<kit")) {
        if fv_idx < kit_idx {
            return Err(XmlError::LegacyV2);
        }
    }
    Ok(())
}

#[derive(Default, Debug)]
struct DrumBuilder {
    name: String,
    osc1: Option<OscSample>,
    in_osc1: bool,
    osc1_buf: OscSample,
    volume_hex: Option<String>,
    pan_hex: Option<String>,
}

impl DrumBuilder {
    fn into_drum(self) -> Drum {
        let osc1 = if self.in_osc1 { Some(self.osc1_buf) } else { self.osc1 };
        Drum {
            name: self.name,
            osc1,
            volume_hex: self.volume_hex,
            pan_hex: self.pan_hex,
        }
    }
}

fn apply_text(path: &[String], text: &str, b: &mut DrumBuilder) {
    // path looks like ["name"] or ["osc1","fileName"] or ["osc1","zone","startSamplePos"]
    let p: Vec<&str> = path.iter().map(String::as_ref).collect();
    match p.len() {
        1 => {
            if p[0] == "name" {
                b.name = text.to_string();
            }
        }
        2 => match (p[0], p[1]) {
            ("osc1", "fileName") => {
                ensure_osc(b);
                b.osc1_buf.file_name = text.to_string();
            }
            ("osc1", "transpose") => {
                ensure_osc(b);
                b.osc1_buf.transpose = parse_i32(text);
            }
            ("osc1", "cents") => {
                ensure_osc(b);
                b.osc1_buf.cents = parse_i32(text);
            }
            ("osc1", "loopMode") => {
                ensure_osc(b);
                b.osc1_buf.loop_mode = parse_i32(text);
            }
            ("osc1", "reversed") => {
                ensure_osc(b);
                b.osc1_buf.reversed = text.trim() == "1";
            }
            ("defaultParams", "volume") => b.volume_hex = Some(text.to_string()),
            ("defaultParams", "pan") => b.pan_hex = Some(text.to_string()),
            _ => {}
        },
        3 => {
            if p[0] == "osc1" && p[1] == "zone" {
                ensure_osc(b);
                match p[2] {
                    "startSamplePos" => b.osc1_buf.start_samples = parse_u64(text),
                    "endSamplePos" => b.osc1_buf.end_samples = parse_u64(text),
                    "startMilliseconds" => b.osc1_buf.start_ms = parse_u32(text),
                    "endMilliseconds" => b.osc1_buf.end_ms = parse_u32(text),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn ensure_osc(b: &mut DrumBuilder) {
    b.in_osc1 = true;
    // osc1_buf is committed to drum.osc1 in DrumBuilder::into_drum() at sound-end.
}

fn parse_i32(s: &str) -> i32 {
    s.trim().parse().unwrap_or(0)
}
fn parse_u32(s: &str) -> u32 {
    s.trim().parse().unwrap_or(0)
}
fn parse_u64(s: &str) -> u64 {
    s.trim().parse().unwrap_or(0)
}

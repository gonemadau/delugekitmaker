use serde::{Deserialize, Serialize};

/// Output flavor — controls a few schema differences between official and community firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flavor {
    /// Official Synthstrom firmware v4.x
    OfficialV4,
    /// Community firmware Chopin / v1.2.x
    CommunityChopin,
}

impl Default for Flavor {
    fn default() -> Self {
        Flavor::OfficialV4
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    OneShot,    // 0
    Loop,       // 1
    Stretch,    // 2
    CutByPad,   // 3
}

impl LoopMode {
    pub fn from_xml(s: &str) -> Self {
        match s.trim() {
            "1" => LoopMode::Loop,
            "2" => LoopMode::Stretch,
            "3" => LoopMode::CutByPad,
            _ => LoopMode::OneShot,
        }
    }
    pub fn to_xml(self) -> &'static str {
        match self {
            LoopMode::OneShot => "0",
            LoopMode::Loop => "1",
            LoopMode::Stretch => "2",
            LoopMode::CutByPad => "3",
        }
    }
}

/// Sample reference for an oscillator. Paths are SD-root-relative, forward-slash, e.g.
/// `SAMPLES/KIT MAKER/MyKit/KICK.WAV`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OscSample {
    pub file_name: String,
    pub start_samples: u64,
    pub end_samples: u64,
    pub transpose: i32,
    pub cents: i32,
    pub reversed: bool,
    pub loop_mode: i32, // raw, 0..3
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Drum {
    pub name: String,
    pub osc1: Option<OscSample>,
    /// volume as Deluge fixed-point hex string, e.g. `0x4CCCCCA8`
    pub volume_hex: Option<String>,
    pub pan_hex: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kit {
    pub name: String,
    pub firmware_version: String,
    pub earliest_compatible_firmware: String,
    pub drums: Vec<Drum>,
}

/// Runtime per-pad params (for the audio engine, not the on-disk kit format).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PadParams {
    pub volume: f32,           // 0..1
    pub pan: f32,              // -1..1
    pub pitch_semitones: f32,  // -24..24
    pub reverse: bool,
}

impl Default for PadParams {
    fn default() -> Self {
        PadParams {
            volume: 0.8,
            pan: 0.0,
            pitch_semitones: 0.0,
            reverse: false,
        }
    }
}

impl Kit {
    pub fn new(name: impl Into<String>) -> Self {
        Kit {
            name: name.into(),
            firmware_version: "4.1.4".into(),
            earliest_compatible_firmware: "4.1.0-pre1".into(),
            drums: Vec::new(),
        }
    }

    pub fn with_default_drums(name: impl Into<String>, count: usize) -> Self {
        let mut k = Self::new(name);
        for i in 0..count {
            k.drums.push(Drum {
                name: format!("Drum {}", i + 1),
                ..Default::default()
            });
        }
        k
    }
}

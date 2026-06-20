use crate::sample::DecodedSample;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Opaque sample identifier. The engine holds samples in a cache; commands reference
/// them by id rather than path so the audio thread never touches the filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SampleId(pub Uuid);

impl SampleId {
    pub fn new() -> Self {
        SampleId(Uuid::new_v4())
    }
}

impl Default for SampleId {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime per-pad parameters used by the audio engine. Distinct from the
/// on-disk `deluge-xml::PadParams` — that one is for serialization, this one
/// is what the mixer actually consumes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PadParams {
    pub volume: f32,           // 0..1
    pub pan: f32,              // -1..1
    pub pitch_semitones: f32,  // -24..24
    pub reverse: bool,
    pub start_frames: u32,
    pub end_frames: u32,       // 0 = end of sample
}

impl Default for PadParams {
    fn default() -> Self {
        PadParams {
            volume: 0.85,
            pan: 0.0,
            pitch_semitones: 0.0,
            reverse: false,
            start_frames: 0,
            end_frames: 0,
        }
    }
}

#[derive(Debug)]
pub enum AudioCommand {
    LoadSample(SampleId, Arc<DecodedSample>),
    UnloadSample(SampleId),
    AuditionPad {
        pad: u8,
        sample: SampleId,
        params: PadParams,
    },
    StopPad(u8),
    StopAll,
    SetPattern(Arc<crate::pattern::Pattern>),
    SetBpm(f32),
    TransportPlay,
    TransportStop,
}

#[derive(Debug, Clone, Copy)]
pub enum AudioEvent {
    Underrun,
    VoiceStarted { pad: u8 },
    VoiceFinished { pad: u8 },
    Step { step: u8 },
}

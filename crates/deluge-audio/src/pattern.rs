//! Sequencer pattern model.
//!
//! 16 pads × 16 steps. Each pad has a per-step bitmask, an optional `SampleId`
//! that it triggers, and a `PadParams` snapshot used at trigger time.
//!
//! Each active step also carries a velocity (0..127, 100 = normal, 127 = accent).

use crate::command::{PadParams, SampleId};
use serde::{Deserialize, Serialize};

pub const STEPS_PER_PATTERN: usize = 16;
pub const PADS_PER_PATTERN: usize = 16;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PadPattern {
    /// Bit i set = step i active.
    pub step_mask: u16,
    /// Per-step velocity 0..127. Ignored unless the corresponding step bit is set.
    pub velocities: [u8; STEPS_PER_PATTERN],
    pub sample: Option<SampleId>,
    pub params: PadParams,
}

impl Default for PadPattern {
    fn default() -> Self {
        PadPattern {
            step_mask: 0,
            velocities: [100; STEPS_PER_PATTERN],
            sample: None,
            params: PadParams::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub bpm: f32,
    pub playing: bool,
    /// Swing amount in [-0.5, 0.5]. 0 = straight 16ths. Positive delays the
    /// even-index 16ths (the "and" of each beat) for a shuffled feel; negative
    /// rushes them (rarely musical).
    pub swing: f32,
    pub pads: [PadPattern; PADS_PER_PATTERN],
}

impl Default for Pattern {
    fn default() -> Self {
        Pattern {
            bpm: 120.0,
            playing: false,
            swing: 0.0,
            pads: [PadPattern::default(); PADS_PER_PATTERN],
        }
    }
}

impl Pattern {
    pub fn set_step(&mut self, pad: usize, step: usize, on: bool) {
        if pad >= PADS_PER_PATTERN || step >= STEPS_PER_PATTERN {
            return;
        }
        if on {
            self.pads[pad].step_mask |= 1 << step;
        } else {
            self.pads[pad].step_mask &= !(1 << step);
        }
    }
}

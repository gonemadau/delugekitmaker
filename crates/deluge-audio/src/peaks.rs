//! Compute min/max waveform peaks for visualization.

use crate::sample::DecodedSample;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peaks {
    /// Pairs of (min, max) per output column, normalized to [-1, 1].
    pub min: Vec<f32>,
    pub max: Vec<f32>,
    pub source_frames: u32,
    pub source_channels: u16,
    pub source_sample_rate: u32,
}

/// Down-sample a decoded sample to `target_columns` columns, each column
/// producing the min and max amplitude across the frames it covers.
/// Stereo samples are mixed to mono for peaks (sum / 2).
pub fn compute_peaks(sample: &DecodedSample, target_columns: usize) -> Peaks {
    let cols = target_columns.max(1);
    let mut min = vec![0.0f32; cols];
    let mut max = vec![0.0f32; cols];
    if sample.frames == 0 {
        return Peaks {
            min,
            max,
            source_frames: 0,
            source_channels: sample.channels,
            source_sample_rate: sample.sample_rate,
        };
    }
    let frames_per_col = (sample.frames as f64 / cols as f64).max(1.0);
    for c in 0..cols {
        let f_start = (c as f64 * frames_per_col).floor() as u32;
        let f_end =
            (((c + 1) as f64 * frames_per_col).floor() as u32).min(sample.frames);
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for f in f_start..f_end {
            let (l, r) = sample.frame(f);
            let v = (l + r) * 0.5;
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
        if lo.is_finite() {
            min[c] = lo.max(-1.0);
        }
        if hi.is_finite() {
            max[c] = hi.min(1.0);
        }
    }
    Peaks {
        min,
        max,
        source_frames: sample.frames,
        source_channels: sample.channels,
        source_sample_rate: sample.sample_rate,
    }
}

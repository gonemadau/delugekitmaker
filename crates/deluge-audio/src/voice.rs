use crate::command::PadParams;
use crate::sample::DecodedSample;
use std::sync::Arc;

/// One playing voice. Voices are pre-allocated in a fixed pool to avoid
/// any allocation in the audio callback.
pub struct Voice {
    pub active: bool,
    pub pad: u8,
    pub sample: Option<Arc<DecodedSample>>,
    /// Read position in source frames (fractional for pitch shifting).
    pub pos: f64,
    /// Playback rate ratio: source_rate / device_rate * 2^(semitones/12)
    pub rate: f64,
    pub start: u32,
    pub end: u32, // 0 = sample end
    pub reverse: bool,
    pub gain_l: f32,
    pub gain_r: f32,
    /// Buffer-relative offset to wait before starting playback (for sequencer-
    /// scheduled mid-buffer starts). Decremented per frame.
    pub delay_frames: u32,
    /// Age in callbacks since start, used by the stealing policy.
    pub age: u64,
}

impl Voice {
    pub const fn empty() -> Self {
        Voice {
            active: false,
            pad: 0,
            sample: None,
            pos: 0.0,
            rate: 1.0,
            start: 0,
            end: 0,
            reverse: false,
            gain_l: 0.0,
            gain_r: 0.0,
            delay_frames: 0,
            age: 0,
        }
    }

    pub fn start(
        &mut self,
        pad: u8,
        sample: Arc<DecodedSample>,
        params: &PadParams,
        device_sample_rate: u32,
        delay_frames: u32,
    ) {
        let rate = (sample.sample_rate as f64 / device_sample_rate as f64)
            * 2f64.powf(params.pitch_semitones as f64 / 12.0);
        let frames = sample.frames;
        let start = params.start_frames.min(frames);
        let end = if params.end_frames == 0 || params.end_frames > frames {
            frames
        } else {
            params.end_frames
        };
        let initial_pos = if params.reverse {
            (end.saturating_sub(1)) as f64
        } else {
            start as f64
        };
        // Equal-power pan: pan in [-1,1] -> gains via cos/sin
        let pan = params.pan.clamp(-1.0, 1.0);
        let theta = (pan + 1.0) * 0.25 * std::f32::consts::PI; // 0..PI/2
        let gain_l = theta.cos() * params.volume;
        let gain_r = theta.sin() * params.volume;
        self.active = true;
        self.pad = pad;
        self.sample = Some(sample);
        self.pos = initial_pos;
        self.rate = rate;
        self.start = start;
        self.end = end;
        self.reverse = params.reverse;
        self.gain_l = gain_l;
        self.gain_r = gain_r;
        self.delay_frames = delay_frames;
        self.age = 0;
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.sample = None;
    }

    /// Render `out` frames into the interleaved stereo `buf` (length = out * 2).
    /// Returns true if the voice should remain active after this call.
    pub fn render_into(&mut self, buf: &mut [f32], out_frames: usize) {
        if !self.active {
            return;
        }
        let sample = match &self.sample {
            Some(s) => s.clone(),
            None => {
                self.active = false;
                return;
            }
        };
        let start = self.start;
        let end = self.end;
        let mut pos = self.pos;
        let rate = self.rate;
        let gl = self.gain_l;
        let gr = self.gain_r;

        for frame_i in 0..out_frames {
            if self.delay_frames > 0 {
                self.delay_frames -= 1;
                continue;
            }
            // Linear interpolation between floor(pos) and floor(pos)+1.
            let p0 = pos as u32;
            if !self.reverse && p0 >= end {
                self.active = false;
                break;
            }
            if self.reverse && pos < start as f64 {
                self.active = false;
                break;
            }
            let frac = (pos - p0 as f64) as f32;
            let (l0, r0) = sample.frame(p0);
            let (l1, r1) = sample.frame(p0.saturating_add(1).min(end.saturating_sub(1).max(0)));
            let l = l0 + (l1 - l0) * frac;
            let r = r0 + (r1 - r0) * frac;
            let i = frame_i * 2;
            buf[i] += l * gl;
            buf[i + 1] += r * gr;
            if self.reverse {
                pos -= rate;
            } else {
                pos += rate;
            }
        }
        self.pos = pos;
        self.age = self.age.wrapping_add(1);
    }
}

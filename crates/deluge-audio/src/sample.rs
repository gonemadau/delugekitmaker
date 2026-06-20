use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SampleError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("hound: {0}")]
    Hound(#[from] hound::Error),
    #[error("unsupported format: {0}")]
    Unsupported(String),
}

/// Decoded audio sample, ready for the mixer. Samples are stored as interleaved f32.
/// Mono samples are stored mono (one f32 per frame); the mixer duplicates to L/R
/// on the fly.
#[derive(Debug)]
pub struct DecodedSample {
    pub data: Vec<f32>,
    pub channels: u16,    // 1 (mono) or 2 (stereo)
    pub sample_rate: u32,
    pub frames: u32,      // length in frames (data.len() / channels)
}

impl DecodedSample {
    /// Decode a WAV file from disk via `hound`. Supports PCM 8/16/24/32 and
    /// float 32-bit, mono or stereo. Resamples are NOT applied — the mixer's
    /// playback rate is adjusted instead.
    pub fn decode_wav(path: &Path) -> Result<Self, SampleError> {
        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        if spec.channels > 2 {
            return Err(SampleError::Unsupported(format!(
                "channels > 2 not supported (got {})",
                spec.channels
            )));
        }
        let data: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max = match spec.bits_per_sample {
                    8 => 128.0,
                    16 => 32_768.0,
                    24 => 8_388_608.0,
                    32 => 2_147_483_648.0,
                    n => return Err(SampleError::Unsupported(format!("int bits {}", n))),
                };
                reader
                    .samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max))
                    .collect::<Result<Vec<_>, _>>()?
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()?,
        };
        let frames = (data.len() / spec.channels as usize) as u32;
        Ok(DecodedSample {
            data,
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            frames,
        })
    }

    /// Read a single frame as (L, R). Pos is in frames (not interleaved index).
    #[inline]
    pub fn frame(&self, pos: u32) -> (f32, f32) {
        if pos >= self.frames {
            return (0.0, 0.0);
        }
        match self.channels {
            1 => {
                let s = self.data[pos as usize];
                (s, s)
            }
            2 => {
                let i = pos as usize * 2;
                (self.data[i], self.data[i + 1])
            }
            _ => (0.0, 0.0),
        }
    }
}

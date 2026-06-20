//! Audio engine for the kit maker.
//!
//! cpal output stream + fixed voice pool. Commands flow in via a lock-free
//! `crossbeam_channel`; the audio callback drains them on each buffer.
//! Samples are decoded into `Arc<DecodedSample>` so handing them across is
//! a refcount bump.

pub mod command;
pub mod engine;
pub mod mixer;
pub mod pattern;
pub mod peaks;
pub mod sample;
pub mod voice;

pub use command::{AudioCommand, AudioEvent, PadParams, SampleId};
pub use engine::{AudioEngine, EngineError};
pub use pattern::{PadPattern, Pattern, PADS_PER_PATTERN, STEPS_PER_PATTERN};
pub use peaks::{compute_peaks, Peaks};
pub use sample::DecodedSample;

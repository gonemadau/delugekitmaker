//! Synthstrom Deluge kit XML reader/writer.
//!
//! POC scope: model the high-level kit + drum fields needed for the kit-maker app
//! (name, sample paths, trim points, gain/pan, loop mode, reverse) and emit a
//! canonical Deluge kit XML that's compatible with official v4.x and community
//! Chopin firmware. Lower-level synth params (envelopes, LFOs, mod matrix) are
//! filled with safe defaults on write.

mod error;
mod kit;
mod parse;
mod write;

pub use error::{XmlError, XmlResult};
pub use kit::{Drum, Flavor, Kit, LoopMode, OscSample, PadParams};
pub use parse::parse_kit;
pub use write::write_kit;

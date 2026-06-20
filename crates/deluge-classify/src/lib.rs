//! Filename keyword classifier: maps a filename like `kick_909.wav` to a drum category.
//!
//! Ordered, case-insensitive, first-match-wins. Word boundaries on tokens to avoid
//! "snake.wav" matching "sn".
//!
//! Auto-layout fills the 16-pad grid using a priority order:
//!   kicks → snare/claps → rim → hats → toms → cymbals → perc → fx → uncategorized.
//! Within each category, files are placed in natural-numeric order (kick_1, kick_2, kick_10).

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrumCategory {
    Kick,
    Snare,
    Clap,
    Rim,
    ClHat,
    OpHat,
    TomLow,
    TomMid,
    TomHi,
    Tom,
    Ride,
    Crash,
    Perc,
    Fx,
}

impl DrumCategory {
    /// Lower number = higher priority during auto-arrange.
    pub fn priority(self) -> u8 {
        match self {
            DrumCategory::Kick => 0,
            DrumCategory::Snare => 1,
            DrumCategory::Clap => 2,
            DrumCategory::Rim => 3,
            DrumCategory::ClHat => 4,
            DrumCategory::OpHat => 5,
            DrumCategory::TomLow => 6,
            DrumCategory::TomMid => 7,
            DrumCategory::TomHi => 8,
            DrumCategory::Tom => 9,
            DrumCategory::Ride => 10,
            DrumCategory::Crash => 11,
            DrumCategory::Perc => 12,
            DrumCategory::Fx => 13,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DrumCategory::Kick => "kick",
            DrumCategory::Snare => "snare",
            DrumCategory::Clap => "clap",
            DrumCategory::Rim => "rim",
            DrumCategory::ClHat => "hh-cl",
            DrumCategory::OpHat => "hh-op",
            DrumCategory::TomLow => "tom-lo",
            DrumCategory::TomMid => "tom-mid",
            DrumCategory::TomHi => "tom-hi",
            DrumCategory::Tom => "tom",
            DrumCategory::Ride => "ride",
            DrumCategory::Crash => "crash",
            DrumCategory::Perc => "perc",
            DrumCategory::Fx => "fx",
        }
    }
}

struct Rule {
    cat: DrumCategory,
    pattern: Regex,
}

static RULES: Lazy<Vec<Rule>> = Lazy::new(|| {
    fn rx(s: &str) -> Regex {
        Regex::new(&format!(r"(?i)(^|[\s_\-.])({})(\d*)([\s_\-.]|$)", s)).unwrap()
    }
    // Order matters: more specific keywords first.
    vec![
        // Rim must come before Snare so "rim" doesn't get swallowed by snare's old "rim" alias.
        Rule { cat: DrumCategory::Rim,    pattern: rx(r"rim|rimshot|sidestick") },
        Rule { cat: DrumCategory::Kick,   pattern: rx(r"kick|kik|kck|bd|bdrum|bassdrum") },
        Rule { cat: DrumCategory::Snare,  pattern: rx(r"snare|snr|sd") },
        Rule { cat: DrumCategory::Clap,   pattern: rx(r"clap|clp|hc") },
        Rule { cat: DrumCategory::OpHat,  pattern: rx(r"ohh|oh|openhat|open[ _\-]?hat") },
        Rule { cat: DrumCategory::ClHat,  pattern: rx(r"chh|hh|hat|closedhat|closed[ _\-]?hat|ch") },
        Rule { cat: DrumCategory::TomLow, pattern: rx(r"lowtom|lo[ _\-]?tom|tom[ _\-]?lo|floor[ _\-]?tom|lt") },
        Rule { cat: DrumCategory::TomMid, pattern: rx(r"midtom|mid[ _\-]?tom|tom[ _\-]?mid|mt") },
        Rule { cat: DrumCategory::TomHi,  pattern: rx(r"hightom|hi[ _\-]?tom|tom[ _\-]?hi|ht") },
        Rule { cat: DrumCategory::Tom,    pattern: rx(r"tom") },
        Rule { cat: DrumCategory::Ride,   pattern: rx(r"ride|rd|rc") },
        Rule { cat: DrumCategory::Crash,  pattern: rx(r"crash|cy|cymbal|cr") },
        Rule { cat: DrumCategory::Perc,   pattern: rx(r"perc|prc|conga|bongo|cowbell|cb|shaker|tamb|tambourine|clave|wood|block") },
        Rule { cat: DrumCategory::Fx,     pattern: rx(r"fx|riser|swoosh|impact|hit|stab") },
    ]
});

pub fn classify(filename: &str) -> Option<DrumCategory> {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    for rule in RULES.iter() {
        if rule.pattern.is_match(stem) {
            return Some(rule.cat);
        }
    }
    None
}

/// Slot preference per category, ordered from most-to-least preferred.
///
/// Layout matches Deluge convention — kicks anchor the bottom row, instruments
/// climb upward to cymbals/FX at the top. Pad 0 = top-left, pad 15 = bottom-right.
///
///   00 Crash   01 Ride    02 Tom-Hi  03 FX
///   04 HH-Cl   05 HH-Op   06 Tom-Lo  07 Tom-Mid
///   08 Clap    09 Rim     10 Perc    11 Perc
///   12 Kick    13 Kick2   14 Snare   15 Snare2
pub fn slots_for(cat: DrumCategory) -> &'static [u8] {
    match cat {
        DrumCategory::Kick => &[12, 13],
        DrumCategory::Snare => &[14, 15],
        DrumCategory::Clap => &[8, 9, 15],
        DrumCategory::Rim => &[9, 8],
        DrumCategory::ClHat => &[4],
        DrumCategory::OpHat => &[5],
        DrumCategory::TomLow => &[6],
        DrumCategory::TomMid => &[7],
        DrumCategory::TomHi => &[2],
        DrumCategory::Tom => &[6, 7, 2],
        DrumCategory::Ride => &[1],
        DrumCategory::Crash => &[0],
        DrumCategory::Perc => &[10, 11, 3],
        DrumCategory::Fx => &[3, 11, 10],
    }
}

/// Natural-sort key: split a filename into text and number runs so
/// `kick_2.wav` sorts before `kick_10.wav`. Returns a Vec of (is_num, value) tuples.
fn natural_key(name: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut buf = String::new();
    let mut buf_is_num = false;
    for c in name.chars() {
        let c_is_num = c.is_ascii_digit();
        if buf.is_empty() {
            buf_is_num = c_is_num;
            buf.push(c);
        } else if c_is_num == buf_is_num {
            buf.push(c);
        } else {
            out.push((buf_is_num, std::mem::take(&mut buf)));
            buf_is_num = c_is_num;
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        out.push((buf_is_num, buf));
    }
    out
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let ka = natural_key(&a.to_lowercase());
    let kb = natural_key(&b.to_lowercase());
    for ((an, av), (bn, bv)) in ka.iter().zip(kb.iter()) {
        if *an && *bn {
            let ai: u64 = av.parse().unwrap_or(0);
            let bi: u64 = bv.parse().unwrap_or(0);
            match ai.cmp(&bi) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        } else {
            match av.cmp(bv) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
    }
    ka.len().cmp(&kb.len())
}

/// Auto-map filenames into a 16-pad layout, preferring named slots and
/// processing categories in priority order. Within each category, files
/// are placed in natural-numeric order (kick_1, kick_2, kick_10).
///
/// Uncategorized files fill remaining empty slots in natural-sort order.
pub fn auto_layout(filenames: &[String]) -> [Option<String>; 16] {
    let mut grid: [Option<String>; 16] = Default::default();

    // Bucket by category.
    let mut buckets: Vec<(DrumCategory, Vec<String>)> = Vec::new();
    let mut uncategorized: Vec<String> = Vec::new();
    for name in filenames {
        match classify(name) {
            Some(c) => {
                if let Some((_, v)) = buckets.iter_mut().find(|(k, _)| *k == c) {
                    v.push(name.clone());
                } else {
                    buckets.push((c, vec![name.clone()]));
                }
            }
            None => uncategorized.push(name.clone()),
        }
    }

    // Sort buckets by priority, contents by natural order.
    buckets.sort_by_key(|(c, _)| c.priority());
    for (_, v) in buckets.iter_mut() {
        v.sort_by(|a, b| natural_cmp(a, b));
    }
    uncategorized.sort_by(|a, b| natural_cmp(a, b));

    // Place each bucket into its preferred slots.
    let mut overflow: Vec<String> = Vec::new();
    for (cat, names) in &buckets {
        let prefs = slots_for(*cat);
        for name in names {
            let mut placed = false;
            for &slot in prefs {
                if grid[slot as usize].is_none() {
                    grid[slot as usize] = Some(name.clone());
                    placed = true;
                    break;
                }
            }
            if !placed {
                overflow.push(name.clone());
            }
        }
    }

    // Overflow (more samples in a category than reserved slots) + uncategorized
    // fill remaining slots in scan order.
    let leftovers: Vec<String> = overflow.into_iter().chain(uncategorized).collect();
    let mut it = leftovers.into_iter();
    for slot in grid.iter_mut() {
        if slot.is_none() {
            if let Some(n) = it.next() {
                *slot = Some(n);
            } else {
                break;
            }
        }
    }
    grid
}

/// Re-classify and re-layout an *existing* pad assignment. Returns a new 16-pad
/// layout. Pads that were empty are dropped from the output (they remain empty).
pub fn auto_arrange(existing: &[Option<String>; 16]) -> [Option<String>; 16] {
    let filenames: Vec<String> = existing.iter().flatten().cloned().collect();
    auto_layout(&filenames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_names() {
        assert_eq!(classify("kick_909.wav"), Some(DrumCategory::Kick));
        assert_eq!(classify("snare_tight.wav"), Some(DrumCategory::Snare));
        assert_eq!(classify("hh_closed_01.wav"), Some(DrumCategory::ClHat));
        assert_eq!(classify("ohh.wav"), Some(DrumCategory::OpHat));
        assert_eq!(classify("clap.wav"), Some(DrumCategory::Clap));
        assert_eq!(classify("rim.wav"), Some(DrumCategory::Rim));
        assert_eq!(classify("rimshot_01.wav"), Some(DrumCategory::Rim));
        assert_eq!(classify("tom_lo.wav"), Some(DrumCategory::TomLow));
        assert_eq!(classify("ride.wav"), Some(DrumCategory::Ride));
        assert_eq!(classify("perc_01.wav"), Some(DrumCategory::Perc));
        assert_eq!(classify("fx_riser.wav"), Some(DrumCategory::Fx));
        assert_eq!(classify("snake.wav"), None);
        assert_eq!(classify("randomname.wav"), None);
    }

    #[test]
    fn auto_layout_places_into_canonical_slots() {
        let names: Vec<String> = ["kick.wav", "snare.wav", "hh.wav", "ohh.wav", "clap.wav", "rim.wav"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let grid = auto_layout(&names);
        assert_eq!(grid[12], Some("kick.wav".to_string()));
        assert_eq!(grid[14], Some("snare.wav".to_string()));
        assert_eq!(grid[8], Some("clap.wav".to_string()));
        assert_eq!(grid[9], Some("rim.wav".to_string()));
        assert_eq!(grid[4], Some("hh.wav".to_string()));
        assert_eq!(grid[5], Some("ohh.wav".to_string()));
    }

    #[test]
    fn natural_sort_keeps_numbers_in_order() {
        let names: Vec<String> = ["kick_10.wav", "kick_2.wav", "kick_1.wav"]
            .iter().map(|s| s.to_string()).collect();
        let grid = auto_layout(&names);
        assert_eq!(grid[12], Some("kick_1.wav".to_string()));
        assert_eq!(grid[13], Some("kick_2.wav".to_string()));
        let kick10_pos = grid.iter().position(|s| s.as_deref() == Some("kick_10.wav"));
        assert!(kick10_pos.is_some(), "kick_10 must land somewhere");
    }

    #[test]
    fn priority_kicks_take_kick_slots() {
        let names: Vec<String> = ["kick.wav", "kick_2.wav"].iter().map(|s| s.to_string()).collect();
        let grid = auto_layout(&names);
        assert_eq!(grid[12], Some("kick.wav".to_string()));
        assert_eq!(grid[13], Some("kick_2.wav".to_string()));
    }

    #[test]
    fn auto_arrange_collapses_sparse_layout() {
        let mut existing: [Option<String>; 16] = Default::default();
        existing[7] = Some("kick.wav".into());
        existing[3] = Some("snare.wav".into());
        let grid = auto_arrange(&existing);
        assert_eq!(grid[12], Some("kick.wav".to_string()));
        assert_eq!(grid[14], Some("snare.wav".to_string()));
    }
}

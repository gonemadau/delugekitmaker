//! Filesystem browsing for the sample picker sidebar.

use crate::layout::FsError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub abs_path: String,
    /// `dir` or `wav` — anything else is filtered out by `list_dir`.
    pub kind: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirListing {
    pub abs_path: String,
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
}

/// List the immediate children of `dir`: subdirectories + WAV files only.
/// Sorted folders-first, then files, both alphabetically (case-insensitive).
pub fn list_dir(dir: &Path) -> Result<DirListing, FsError> {
    if !dir.exists() {
        return Err(FsError::NotFound(dir.to_path_buf()));
    }
    if !dir.is_dir() {
        return Err(FsError::NotADir(dir.to_path_buf()));
    }
    let mut entries: Vec<DirEntry> = Vec::new();
    for raw in std::fs::read_dir(dir)? {
        let raw = raw?;
        let path = raw.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        // Skip hidden / system entries.
        if name.starts_with('.') {
            continue;
        }
        let md = match raw.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if md.is_dir() {
            entries.push(DirEntry {
                name,
                abs_path: path.to_string_lossy().into_owned(),
                kind: "dir".into(),
                size_bytes: 0,
            });
        } else if md.is_file() && is_wav(&path) {
            entries.push(DirEntry {
                name,
                abs_path: path.to_string_lossy().into_owned(),
                kind: "wav".into(),
                size_bytes: md.len(),
            });
        }
    }
    entries.sort_by(|a, b| match (a.kind.as_str(), b.kind.as_str()) {
        ("dir", "wav") => std::cmp::Ordering::Less,
        ("wav", "dir") => std::cmp::Ordering::Greater,
        _ => a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()),
    });
    let parent = dir
        .parent()
        .filter(|p| *p != Path::new(""))
        .map(|p| p.to_string_lossy().into_owned());
    Ok(DirListing {
        abs_path: dir.to_string_lossy().into_owned(),
        parent,
        entries,
    })
}

fn is_wav(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav") || e.eq_ignore_ascii_case("wave"))
        .unwrap_or(false)
}

/// Pick a starting directory: env override > given path > user home > C:\
pub fn default_browse_root(prefer: Option<&str>) -> PathBuf {
    if let Some(p) = prefer {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return pb;
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let pb = PathBuf::from(home);
        if pb.is_dir() {
            return pb;
        }
    }
    PathBuf::from("C:/")
}

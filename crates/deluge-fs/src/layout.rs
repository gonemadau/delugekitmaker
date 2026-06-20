use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("path does not exist: {0}")]
    NotFound(PathBuf),
    #[error("not a directory: {0}")]
    NotADir(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDRoot(pub PathBuf);

impl SDRoot {
    pub fn validate(p: &Path) -> Result<Self, FsError> {
        if !p.exists() {
            return Err(FsError::NotFound(p.to_path_buf()));
        }
        if !p.is_dir() {
            return Err(FsError::NotADir(p.to_path_buf()));
        }
        // Create canonical subdirs if missing — non-destructive.
        for sub in ["KITS", "SAMPLES", "SONGS", "SYNTHS"] {
            let s = p.join(sub);
            if !s.exists() {
                std::fs::create_dir_all(&s)?;
            }
        }
        Ok(SDRoot(p.to_path_buf()))
    }

    pub fn root(&self) -> &Path {
        &self.0
    }
    pub fn kits_dir(&self) -> PathBuf {
        self.0.join("KITS")
    }
    pub fn samples_dir(&self) -> PathBuf {
        self.0.join("SAMPLES")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitSummary {
    pub name: String,
    pub file_name: String,
    /// Relative path from SD root, forward-slash, e.g. "KITS/KIT000.XML"
    pub rel_path: String,
    pub size_bytes: u64,
}

pub fn list_kits(root: &SDRoot) -> Result<Vec<KitSummary>, FsError> {
    let dir = root.kits_dir();
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != "xml" {
            continue;
        }
        let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let meta = entry.metadata()?;
        out.push(KitSummary {
            name,
            file_name: file_name.clone(),
            rel_path: format!("KITS/{}", file_name),
            size_bytes: meta.len(),
        });
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    Ok(out)
}

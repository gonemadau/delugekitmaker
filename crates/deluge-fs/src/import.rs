//! Save a kit to the SD card folder layout: write XML, copy referenced samples.

use crate::layout::{FsError, SDRoot};
use crate::path::{kit_xml_filename, sanitize_filename};
use deluge_xml::{write_kit, Flavor, Kit};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BundleMode {
    /// Bundle samples into `KITS/<kit-name>/<sample>.WAV` — portable.
    KitSubfolder,
    /// Shared into `SAMPLES/KIT MAKER/<kit-name>/<sample>.WAV` — dedup-friendly.
    SharedSamples,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveOptions {
    pub bundle_mode: BundleMode,
    pub flavor: String, // "OfficialV4" | "CommunityChopin"
}

impl Default for SaveOptions {
    fn default() -> Self {
        SaveOptions {
            bundle_mode: BundleMode::KitSubfolder,
            flavor: "OfficialV4".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveReport {
    pub xml_path: String,
    pub copied_samples: Vec<String>,
    pub reused_samples: Vec<String>,
}

/// Write a kit to the SD root. Returns the absolute xml path written.
///
/// `kit.drums[i].osc1.file_name` is interpreted as:
/// - an absolute path on disk → copy into the bundle dir, rewrite to SD-relative
/// - or an existing SD-relative path → leave as-is (sample already in place)
pub fn save_kit(
    root: &SDRoot,
    kit: &mut Kit,
    options: &SaveOptions,
) -> Result<SaveReport, FsError> {
    let kit_stem = sanitize_filename(&kit.name);
    let xml_name = kit_xml_filename(&kit_stem);

    let bundle_rel = match options.bundle_mode {
        BundleMode::KitSubfolder => format!("KITS/{}", kit_stem),
        BundleMode::SharedSamples => format!("SAMPLES/KIT MAKER/{}", kit_stem),
    };
    let bundle_abs = root.root().join(bundle_rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    fs::create_dir_all(&bundle_abs)?;

    let mut copied = Vec::new();
    let mut reused = Vec::new();

    for drum in kit.drums.iter_mut() {
        let Some(osc) = drum.osc1.as_mut() else { continue };
        if osc.file_name.is_empty() {
            continue;
        }
        // Decide if path is absolute (needs copy) or SD-relative (already in place).
        let p = Path::new(&osc.file_name);
        if p.is_absolute() {
            let src = p.to_path_buf();
            let stem = src
                .file_name()
                .and_then(|s| s.to_str())
                .map(sanitize_filename)
                .unwrap_or_else(|| "sample.wav".into());
            let dest = bundle_abs.join(&stem);
            let final_dest = atomic_copy_with_dedup(&src, &dest)?;
            let final_rel = format!(
                "{}/{}",
                bundle_rel,
                final_dest.file_name().unwrap().to_string_lossy()
            );
            if final_dest.metadata()?.modified()? == src.metadata()?.modified()? {
                reused.push(final_rel.clone());
            } else {
                copied.push(final_rel.clone());
            }
            osc.file_name = final_rel;
        }
        // else: assume it's already SD-relative; leave alone
    }

    let flavor = match options.flavor.as_str() {
        "CommunityChopin" => Flavor::CommunityChopin,
        _ => Flavor::OfficialV4,
    };
    let xml = write_kit(kit, flavor).map_err(|e| FsError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))?;

    let xml_path = root.kits_dir().join(xml_name);
    let tmp = xml_path.with_extension("XML.tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(xml.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &xml_path)?;

    Ok(SaveReport {
        xml_path: xml_path.to_string_lossy().into_owned(),
        copied_samples: copied,
        reused_samples: reused,
    })
}

/// Copy `src` to `dest` atomically. If `dest` exists and has identical SHA-1,
/// skip the copy and return the existing path. Otherwise, if there's a
/// collision, append a numeric suffix until unique.
fn atomic_copy_with_dedup(src: &Path, dest: &Path) -> std::io::Result<PathBuf> {
    let src_hash = sha1_of_file(src)?;

    let mut try_dest = dest.to_path_buf();
    let mut counter = 1u32;
    loop {
        if !try_dest.exists() {
            break;
        }
        let existing_hash = sha1_of_file(&try_dest)?;
        if existing_hash == src_hash {
            return Ok(try_dest); // dedupe hit
        }
        let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("sample");
        let ext = dest.extension().and_then(|s| s.to_str()).unwrap_or("wav");
        try_dest = dest.with_file_name(format!("{}_{}.{}", stem, counter, ext));
        counter += 1;
    }

    let tmp = try_dest.with_extension("partial");
    fs::copy(src, &tmp)?;
    fs::rename(&tmp, &try_dest)?;
    Ok(try_dest)
}

fn sha1_of_file(path: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

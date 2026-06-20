//! Filesystem operations for working with a Deluge SD card folder layout.
//! POC scope: validate root, list kits, copy samples into kit-bundle subfolder.

pub mod browse;
pub mod import;
pub mod layout;
pub mod path;

pub use browse::{default_browse_root, list_dir, DirEntry, DirListing};
pub use import::{save_kit, BundleMode, SaveOptions, SaveReport};
pub use layout::{list_kits, FsError, KitSummary, SDRoot};

//! Windows bloatware removal subsystem.
//!
//! This module is deliberately separate from `modules/` (file/cache/registry cleaning) and
//! `uninstaller/` (general application removal). It targets preinstalled Appx packages,
//! telemetry/advertising policies, and optional capabilities whose risk profile and
//! reversibility are different from ordinary file deletion.

pub mod discovery;
pub mod executor;
pub mod model;

pub use discovery::scan_bloatware;
pub use executor::{create_restore_point, is_elevated, remove_bloat_with_progress};
pub use model::{
    BloatCategory, BloatItem, BloatProgress, BloatReport, BloatRisk, BloatTarget, PolicyScope,
};

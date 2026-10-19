//! Windows bloatware domain model.
//!
//! Bloat removal is intentionally isolated from the file/cache cleaner and from the
//! general-purpose uninstaller. It deals with package de-provisioning, policy writes, and
//! system servicing operations whose risk profile and reversibility differ fundamentally
//! from ordinary file deletion.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BloatRisk {
    /// Removable & re-installable; no system function is affected.
    Safe,
    /// Removes a first-party feature; reversible from the Microsoft Store.
    Careful,
    /// Modifies system servicing or policy; may affect Windows Update and is not guaranteed reversible.
    Expert,
}

impl BloatRisk {
    pub fn label(self) -> &'static str {
        match self {
            BloatRisk::Safe => "SAFE",
            BloatRisk::Careful => "CAREFUL",
            BloatRisk::Expert => "EXPERT",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BloatRisk::Safe => "Removable & re-installable; no system function is affected.",
            BloatRisk::Careful => {
                "Removes a first-party feature; reversible from the Microsoft Store."
            }
            BloatRisk::Expert => {
                "Modifies system servicing or policy; may affect Windows Update and is not guaranteed reversible."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BloatCategory {
    StoreApp,
    Policy,
    Capability,
}

impl BloatCategory {
    pub fn label(self) -> &'static str {
        match self {
            BloatCategory::StoreApp => "PREINSTALLED APP",
            BloatCategory::Policy => "PRIVACY POLICY",
            BloatCategory::Capability => "OPTIONAL FEATURE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyScope {
    /// HKCU only — applies to the current user without elevation.
    UserOnly,
    /// HKLM only — machine-wide policy; requires elevation.
    MachineOnly,
    /// HKCU (primary) plus HKLM when elevated.
    Both,
}

#[derive(Debug, Clone)]
pub enum BloatTarget {
    StoreApp {
        installed_full_name: Option<String>,
        provisioned_full_name: Option<String>,
    },
    Policy {
        scope: PolicyScope,
        key_path: &'static str,
        value_name: &'static str,
        disable_value: u32,
    },
    Capability {
        capability_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct BloatItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk: BloatRisk,
    pub category: BloatCategory,
    pub target: BloatTarget,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct BloatProgress {
    pub item_name: String,
    pub fraction: f32,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct BloatReport {
    pub items_processed: usize,
    pub items_removed: usize,
    pub items_failed: usize,
    pub restore_point_created: bool,
    pub details: Vec<String>,
    pub error_message: Option<String>,
}

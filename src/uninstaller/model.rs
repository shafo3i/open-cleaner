use std::path::PathBuf;

use crate::core::CleanMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSource {
    Win32Machine64,
    Win32Machine32,
    Win32User,
    WindowsStoreAppX,
}

impl AppSource {
    pub fn badge_label(self) -> &'static str {
        match self {
            AppSource::Win32Machine64 => "WIN32 X64",
            AppSource::Win32Machine32 => "WIN32 X86",
            AppSource::Win32User => "USER APP",
            AppSource::WindowsStoreAppX => "MS STORE",
        }
    }

    pub fn scope_label(self) -> &'static str {
        match self {
            AppSource::Win32Machine64 => "HKLM\\...\\Uninstall (x64)",
            AppSource::Win32Machine32 => "HKLM\\WOW6432Node\\...\\Uninstall (x86)",
            AppSource::Win32User => "HKCU\\...\\Uninstall (Current User)",
            AppSource::WindowsStoreAppX => "HKCU\\...\\AppModel\\Repository\\Packages",
        }
    }

    pub fn is_store_app(self) -> bool {
        matches!(self, AppSource::WindowsStoreAppX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerKind {
    Msi,
    InnoSetup,
    Nsis,
    AppX,
    Generic,
}

impl InstallerKind {
    pub fn label(self) -> &'static str {
        match self {
            InstallerKind::Msi => "MSI PACKAGE",
            InstallerKind::InnoSetup => "INNO SETUP",
            InstallerKind::Nsis => "NSIS INSTALLER",
            InstallerKind::AppX => "APPX / MSIX",
            InstallerKind::Generic => "WIN32 UNINSTALLER",
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub id: String,
    pub registry_subkey: String,
    pub display_name: String,
    pub display_version: String,
    pub publisher: String,
    pub install_date: String,
    pub install_location: Option<PathBuf>,
    pub uninstall_string: String,
    pub quiet_uninstall_string: Option<String>,
    pub display_icon: Option<String>,
    pub estimated_size_bytes: u64,
    pub source: AppSource,
    pub installer_kind: InstallerKind,
    pub icon_image: Option<std::sync::Arc<egui::ColorImage>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemnantKind {
    InstallDirectory,
    AppDataLocal,
    AppDataRoaming,
    ProgramData,
    Shortcut,
    RegistryUninstallKey,
    RegistrySoftwareKey,
}

impl RemnantKind {
    pub fn label(self) -> &'static str {
        match self {
            RemnantKind::InstallDirectory => "INSTALL DIR",
            RemnantKind::AppDataLocal => "LOCAL APPDATA",
            RemnantKind::AppDataRoaming => "ROAMING APPDATA",
            RemnantKind::ProgramData => "PROGRAMDATA",
            RemnantKind::Shortcut => "SHORTCUT (.LNK)",
            RemnantKind::RegistryUninstallKey => "REGISTRY UNINSTALL",
            RemnantKind::RegistrySoftwareKey => "REGISTRY CONFIG",
        }
    }

    pub fn is_registry(self) -> bool {
        matches!(
            self,
            RemnantKind::RegistryUninstallKey | RemnantKind::RegistrySoftwareKey
        )
    }
}

#[derive(Debug, Clone)]
pub struct AppRemnant {
    pub kind: RemnantKind,
    pub location: String,
    pub path: Option<PathBuf>,
    pub registry_hive: Option<String>,
    pub registry_subkey: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct UninstallOptions {
    pub run_native_uninstaller: bool,
    pub prefer_silent: bool,
    pub purge_remnants: bool,
    pub clean_mode: CleanMode,
}

impl Default for UninstallOptions {
    fn default() -> Self {
        Self {
            run_native_uninstaller: true,
            prefer_silent: false,
            purge_remnants: true,
            clean_mode: CleanMode::Standard,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UninstallReport {
    pub app_name: String,
    pub uninstaller_ran: bool,
    pub uninstaller_success: bool,
    pub uninstaller_exit_code: Option<i32>,
    pub uninstaller_command: String,
    pub remnants_found: usize,
    pub remnants_removed: usize,
    pub remnants_failed: usize,
    pub bytes_reclaimed: u64,
    pub removed_locations: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UninstallPhase {
    Preparing,
    RunningNativeUninstaller,
    ScanningRemnants,
    PurgingRemnant { current: usize, total: usize },
    RefreshingCatalog,
    Completed,
}

impl UninstallPhase {
    pub fn step_label(&self) -> String {
        match self {
            UninstallPhase::Preparing => "STEP 1/4 · INITIALIZING UNINSTALLER".to_string(),
            UninstallPhase::RunningNativeUninstaller => {
                "STEP 2/4 · EXECUTING NATIVE UNINSTALLER".to_string()
            }
            UninstallPhase::ScanningRemnants => {
                "STEP 3/4 · DEEP-SCANNING POST-UNINSTALL REMNANTS".to_string()
            }
            UninstallPhase::PurgingRemnant { current, total } => {
                format!("STEP 3/4 · PURGING REMNANT ({current}/{total})")
            }
            UninstallPhase::RefreshingCatalog => {
                "STEP 4/4 · VERIFYING & REFRESHING CATALOG".to_string()
            }
            UninstallPhase::Completed => "COMPLETE · APPLICATION & REMNANTS EXPUNGED".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UninstallProgress {
    pub app_id: String,
    pub app_name: String,
    pub phase: UninstallPhase,
    pub fraction: f32,
    pub detail: String,
}

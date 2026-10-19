use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

use super::executor::detect_installer_kind;
use super::model::{AppSource, InstalledApp, InstallerKind};

const UNINSTALL_REL_X64: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
const UNINSTALL_REL_X86: &str =
    "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
const APPX_PACKAGES_REL: &str = "Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppModel\\Repository\\Packages";

/// Enumerates all user-facing installed applications on the host system
/// across 64-bit HKLM, 32-bit WOW6432Node HKLM, Current User HKCU, and UWP/MSIX AppX repositories.
pub fn scan_installed_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    scan_uninstall_hive(
        &hklm,
        UNINSTALL_REL_X64,
        AppSource::Win32Machine64,
        &mut apps,
        &mut seen,
    );
    scan_uninstall_hive(
        &hklm,
        UNINSTALL_REL_X86,
        AppSource::Win32Machine32,
        &mut apps,
        &mut seen,
    );
    scan_uninstall_hive(
        &hkcu,
        UNINSTALL_REL_X64,
        AppSource::Win32User,
        &mut apps,
        &mut seen,
    );
    scan_appx_repository(&hkcu, APPX_PACKAGES_REL, &mut apps, &mut seen);

    apps.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    apps
}

/// Sandboxed application discovery scoped under `HKCU\<prefix>\...` for deterministic testing.
pub fn scan_installed_apps_with_prefix(prefix: &str) -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let trimmed = prefix.trim_matches('\\');
    let user_uninstall = if trimmed.is_empty() {
        UNINSTALL_REL_X64.to_string()
    } else {
        format!("{trimmed}\\{UNINSTALL_REL_X64}")
    };
    let appx_rel = if trimmed.is_empty() {
        APPX_PACKAGES_REL.to_string()
    } else {
        format!("{trimmed}\\{APPX_PACKAGES_REL}")
    };

    scan_uninstall_hive(
        &hkcu,
        &user_uninstall,
        AppSource::Win32User,
        &mut apps,
        &mut seen,
    );
    scan_appx_repository(&hkcu, &appx_rel, &mut apps, &mut seen);

    apps.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    apps
}

fn scan_uninstall_hive(
    root: &RegKey,
    subkey_path: &str,
    source: AppSource,
    out: &mut Vec<InstalledApp>,
    seen: &mut HashSet<String>,
) {
    let Ok(uninstall_key) = root.open_subkey_with_flags(subkey_path, KEY_READ) else {
        return;
    };

    for sub_name in uninstall_key.enum_keys().flatten() {
        let Ok(app_key) = uninstall_key.open_subkey_with_flags(&sub_name, KEY_READ) else {
            continue;
        };

        // Filter 1: Skip SystemComponent == 1
        let system_component: u32 = app_key.get_value("SystemComponent").unwrap_or(0);
        if system_component == 1 {
            continue;
        }

        // Filter 2: Skip updates, hotfixes, and sub-packages with a ParentKeyName
        let release_type: String = app_key.get_value("ReleaseType").unwrap_or_default();
        let release_lower = release_type.trim().to_ascii_lowercase();
        if matches!(
            release_lower.as_str(),
            "security update" | "update" | "hotfix" | "Update Rollup"
        ) {
            continue;
        }

        let parent_key: String = app_key.get_value("ParentKeyName").unwrap_or_default();
        if !parent_key.trim().is_empty() {
            continue;
        }

        // Required: DisplayName
        let display_name: String = app_key.get_value("DisplayName").unwrap_or_default();
        let display_name = display_name.trim().to_string();
        if display_name.is_empty() {
            continue;
        }

        // Required: UninstallString or QuietUninstallString
        let quiet_uninstall: Option<String> = app_key
            .get_value::<String, _>("QuietUninstallString")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let uninstall_string: String = app_key
            .get_value::<String, _>("UninstallString")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| quiet_uninstall.clone())
            .unwrap_or_default();

        if uninstall_string.is_empty() {
            continue;
        }

        let display_version: String = app_key
            .get_value::<String, _>("DisplayVersion")
            .unwrap_or_default()
            .trim()
            .to_string();

        let dedup_key = format!(
            "{}::{}",
            display_name.to_lowercase(),
            display_version.to_lowercase()
        );
        if !seen.insert(dedup_key) {
            continue;
        }

        let publisher: String = app_key
            .get_value::<String, _>("Publisher")
            .unwrap_or_else(|_| "Unknown Publisher".to_string())
            .trim()
            .to_string();

        let install_location: Option<PathBuf> = app_key
            .get_value::<String, _>("InstallLocation")
            .ok()
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        let raw_date: String = app_key.get_value("InstallDate").unwrap_or_default();
        let install_date = normalize_install_date(&raw_date, install_location.as_deref());

        let display_icon: Option<String> = app_key
            .get_value::<String, _>("DisplayIcon")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let estimated_kb: u32 = app_key.get_value("EstimatedSize").unwrap_or(0);
        let estimated_size_bytes = if estimated_kb > 0 {
            (estimated_kb as u64) * 1024
        } else if let Some(loc) = &install_location {
            bounded_directory_size(loc, 3)
        } else {
            0
        };

        let windows_installer_flag: u32 = app_key.get_value("WindowsInstaller").unwrap_or(0);
        let installer_kind = detect_installer_kind(
            &uninstall_string,
            quiet_uninstall.as_deref(),
            windows_installer_flag,
            source,
        );

        let full_subkey = format!("{subkey_path}\\{sub_name}");
        let id = format!("{:?}::{}", source, sub_name);

        out.push(InstalledApp {
            id,
            registry_subkey: full_subkey,
            display_name,
            display_version: if display_version.is_empty() {
                "—".to_string()
            } else {
                display_version
            },
            publisher: if publisher.is_empty() {
                "Unknown Publisher".to_string()
            } else {
                publisher
            },
            install_date,
            install_location,
            uninstall_string,
            quiet_uninstall_string: quiet_uninstall,
            display_icon,
            estimated_size_bytes,
            source,
            installer_kind,
        });
    }
}

fn scan_appx_repository(
    root: &RegKey,
    subkey_path: &str,
    out: &mut Vec<InstalledApp>,
    seen: &mut HashSet<String>,
) {
    let Ok(packages_key) = root.open_subkey_with_flags(subkey_path, KEY_READ) else {
        return;
    };

    for pkg_full_name in packages_key.enum_keys().flatten() {
        if is_system_or_framework_appx(&pkg_full_name) {
            continue;
        }

        let Ok(pkg_key) = packages_key.open_subkey_with_flags(&pkg_full_name, KEY_READ) else {
            continue;
        };

        let (parsed_name, parsed_version, parsed_publisher) = parse_appx_full_name(&pkg_full_name);

        let raw_display_name: String = pkg_key.get_value("DisplayName").unwrap_or_default();
        let display_name = if !raw_display_name.trim().is_empty()
            && !raw_display_name.starts_with("@{")
            && !raw_display_name.starts_with("ms-resource:")
        {
            raw_display_name.trim().to_string()
        } else {
            parsed_name
        };

        if display_name.is_empty() {
            continue;
        }

        let dedup_key = format!(
            "appx::{}::{}",
            display_name.to_lowercase(),
            parsed_version.to_lowercase()
        );
        if !seen.insert(dedup_key) {
            continue;
        }

        let root_folder: Option<PathBuf> = pkg_key
            .get_value::<String, _>("PackageRootFolder")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        let estimated_size_bytes = root_folder
            .as_deref()
            .map(|p| bounded_directory_size(p, 2))
            .unwrap_or(0);

        let install_date = normalize_install_date("", root_folder.as_deref());

        let uninstall_cmd = format!(
            "powershell.exe -NoProfile -WindowStyle Hidden -Command \"Remove-AppxPackage -Package '{}'\"",
            pkg_full_name
        );

        out.push(InstalledApp {
            id: format!("AppX::{pkg_full_name}"),
            registry_subkey: format!("{subkey_path}\\{pkg_full_name}"),
            display_name,
            display_version: parsed_version,
            publisher: parsed_publisher,
            install_date,
            install_location: root_folder,
            uninstall_string: uninstall_cmd.clone(),
            quiet_uninstall_string: Some(uninstall_cmd),
            display_icon: None,
            estimated_size_bytes,
            source: AppSource::WindowsStoreAppX,
            installer_kind: InstallerKind::AppX,
        });
    }
}

fn is_system_or_framework_appx(pkg_full_name: &str) -> bool {
    let lower = pkg_full_name.to_ascii_lowercase();
    const SKIP_PREFIXES: &[&str] = &[
        "microsoft.vclibs",
        "microsoft.net.native",
        "microsoft.ui.xaml",
        "microsoft.windows.",
        "microsoftwindows.client",
        "microsoftwindows.undockeddevkit",
        "windows.cbspreview",
        "windows.immersivecontrolpanel",
        "windows.printdialog",
        "microsoft.services.store",
        "microsoft.win32webviewhost",
        "microsoft.aad.brokerplugin",
        "microsoft.accountscontrol",
        "microsoft.asynctextservice",
        "microsoft.bioenrollment",
        "microsoft.creddialoghost",
        "microsoft.ecapp",
        "microsoft.lockapp",
        "microsoft.microsoftedgedevtoolsclient",
        "microsoft.directxruntime",
        "ncsiuwpapp",
        "1527c705-839a-4832-9118-54d4Bd6a0c89",
        "c5e2524a-ea46-4f67-841f-6a9465d9d515",
        "e2a4f912-2574-4a75-9bb0-0d023378592b",
        "f46d4000-fd22-4db4-ac8e-4e1ddde828fe",
    ];
    SKIP_PREFIXES.iter().any(|p| lower.starts_with(p))
}

fn parse_appx_full_name(pkg_full_name: &str) -> (String, String, String) {
    // Format: PackageName_Version_Arch_ResourceId_PublisherId
    let parts: Vec<&str> = pkg_full_name.split('_').collect();
    let raw_pkg = parts.first().copied().unwrap_or(pkg_full_name);
    let version = parts.get(1).copied().unwrap_or("—").to_string();

    // Split Vendor.AppName if present
    let (publisher, friendly) = if let Some((vendor, rest)) = raw_pkg.split_once('.') {
        (vendor.to_string(), rest.replace('.', " "))
    } else {
        ("Microsoft Store".to_string(), raw_pkg.to_string())
    };

    (friendly, version, publisher)
}

pub fn normalize_install_date(raw: &str, fallback_dir: Option<&Path>) -> String {
    let trimmed = raw.trim();
    if trimmed.len() == 8 && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return format!("{}-{}-{}", &trimmed[0..4], &trimmed[4..6], &trimmed[6..8]);
    }

    if let Some(dir) = fallback_dir
        && let Ok(meta) = fs::metadata(dir)
        && let Ok(created) = meta.created().or_else(|_| meta.modified())
        && let Ok(elapsed) = created.duration_since(std::time::UNIX_EPOCH)
    {
        let days = elapsed.as_secs() / 86_400;
        let (y, m, d) = civil_from_days(days as i64);
        return format!("{y:04}-{m:02}-{d:02}");
    }

    "—".to_string()
}

/// Converts days since 1970-01-01 to `(year, month, day)` (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

pub fn bounded_directory_size(path: &Path, max_depth: usize) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    let mut total = 0u64;
    for entry in entries.flatten() {
        let p = entry.path();
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            } else if meta.is_dir() && max_depth > 0 {
                total = total.saturating_add(bounded_directory_size(&p, max_depth - 1));
            }
        }
    }
    total
}

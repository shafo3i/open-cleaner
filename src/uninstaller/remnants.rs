use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};

use crate::core::cleaner::{CleanResult, clean_directory_contents, secure_delete_file};
use crate::core::{CleanMode, ModuleReport};

use super::discovery::bounded_directory_size;
use super::model::{AppRemnant, AppSource, InstalledApp, RemnantKind};

const SHARED_VENDORS: &[&str] = &[
    "microsoft",
    "microsoft corporation",
    "windows",
    "google",
    "apple",
    "adobe",
    "intel",
    "nvidia",
    "amd",
    "common files",
    "packages",
    "programs",
    "temp",
    "unknown publisher",
];

/// Scans the filesystem and registry for leftover traces of an application.
pub fn scan_app_remnants(app: &InstalledApp) -> Vec<AppRemnant> {
    scan_app_remnants_with_prefix(app, "")
}

/// Scans for application remnants with an optional `HKCU\<reg_prefix>` scope for isolated sandbox testing.
pub fn scan_app_remnants_with_prefix(app: &InstalledApp, reg_prefix: &str) -> Vec<AppRemnant> {
    let mut remnants = Vec::new();
    let mut seen_paths = HashSet::new();

    let app_tokens = candidate_tokens(&app.display_name);
    let pub_tokens = candidate_tokens(&app.publisher);

    // 1. Residual InstallLocation
    if let Some(install_dir) = &app.install_location
        && install_dir.exists()
        && is_safe_install_location(install_dir)
    {
        let canonical_key = install_dir.to_string_lossy().to_lowercase();
        if seen_paths.insert(canonical_key) {
            let size = bounded_directory_size(install_dir, 4);
            remnants.push(AppRemnant {
                kind: RemnantKind::InstallDirectory,
                location: install_dir.display().to_string(),
                path: Some(install_dir.clone()),
                registry_hive: None,
                registry_subkey: None,
                size_bytes: size,
            });
        }
    }

    // 2. %LOCALAPPDATA%
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let base = PathBuf::from(local_appdata);
        collect_appdata_matches(
            &base,
            &app_tokens,
            &pub_tokens,
            RemnantKind::AppDataLocal,
            &mut remnants,
            &mut seen_paths,
        );
        let programs_dir = base.join("Programs");
        collect_appdata_matches(
            &programs_dir,
            &app_tokens,
            &pub_tokens,
            RemnantKind::AppDataLocal,
            &mut remnants,
            &mut seen_paths,
        );
    }

    // 3. %APPDATA% (Roaming)
    if let Ok(appdata) = env::var("APPDATA") {
        let base = PathBuf::from(&appdata);
        collect_appdata_matches(
            &base,
            &app_tokens,
            &pub_tokens,
            RemnantKind::AppDataRoaming,
            &mut remnants,
            &mut seen_paths,
        );

        // Start Menu Programs under %APPDATA%
        let start_menu = base.join("Microsoft\\Windows\\Start Menu\\Programs");
        collect_shortcut_matches(&start_menu, &app_tokens, &mut remnants, &mut seen_paths);
    }

    // 4. %PROGRAMDATA%
    if let Ok(program_data) = env::var("PROGRAMDATA") {
        let base = PathBuf::from(&program_data);
        collect_appdata_matches(
            &base,
            &app_tokens,
            &pub_tokens,
            RemnantKind::ProgramData,
            &mut remnants,
            &mut seen_paths,
        );

        let common_start_menu = base.join("Microsoft\\Windows\\Start Menu\\Programs");
        collect_shortcut_matches(
            &common_start_menu,
            &app_tokens,
            &mut remnants,
            &mut seen_paths,
        );
    }

    // 5. %USERPROFILE%\Desktop shortcuts
    if let Ok(user_profile) = env::var("USERPROFILE") {
        let desktop = PathBuf::from(user_profile).join("Desktop");
        collect_shortcut_matches(&desktop, &app_tokens, &mut remnants, &mut seen_paths);
    }

    // 6. Registry Remnants (Orphaned Uninstall Key + Software\<Publisher>\<AppName>)
    collect_registry_remnants(app, &app_tokens, &pub_tokens, reg_prefix, &mut remnants);

    remnants
}

fn collect_appdata_matches(
    base: &Path,
    app_tokens: &[String],
    pub_tokens: &[String],
    kind: RemnantKind,
    out: &mut Vec<AppRemnant>,
    seen: &mut HashSet<String>,
) {
    if !base.exists() {
        return;
    }

    // Direct <Base>\<AppName>
    for app_tok in app_tokens {
        if !is_safe_folder_token(app_tok) {
            continue;
        }
        let candidate = base.join(app_tok);
        if candidate.exists() && is_safe_install_location(&candidate) {
            let key = candidate.to_string_lossy().to_lowercase();
            if seen.insert(key) {
                let size = bounded_directory_size(&candidate, 4);
                out.push(AppRemnant {
                    kind,
                    location: candidate.display().to_string(),
                    path: Some(candidate),
                    registry_hive: None,
                    registry_subkey: None,
                    size_bytes: size,
                });
            }
        }
    }

    // Nested <Base>\<Publisher>\<AppName>
    for pub_tok in pub_tokens {
        if pub_tok.is_empty() {
            continue;
        }
        let pub_dir = base.join(pub_tok);
        if !pub_dir.is_dir() {
            continue;
        }
        for app_tok in app_tokens {
            if !is_safe_folder_token(app_tok) {
                continue;
            }
            let candidate = pub_dir.join(app_tok);
            if candidate.exists() && is_safe_install_location(&candidate) {
                let key = candidate.to_string_lossy().to_lowercase();
                if seen.insert(key) {
                    let size = bounded_directory_size(&candidate, 4);
                    out.push(AppRemnant {
                        kind,
                        location: candidate.display().to_string(),
                        path: Some(candidate),
                        registry_hive: None,
                        registry_subkey: None,
                        size_bytes: size,
                    });
                }
            }
        }
    }
}

fn collect_shortcut_matches(
    dir: &Path,
    app_tokens: &[String],
    out: &mut Vec<AppRemnant>,
    seen: &mut HashSet<String>,
) {
    if !dir.is_dir() {
        return;
    }

    for tok in app_tokens {
        if !is_safe_folder_token(tok) {
            continue;
        }
        // Check <Dir>\<AppName>.lnk, <Dir>\<AppName>.url, and <Dir>\<AppName> folder
        for candidate in [
            dir.join(format!("{tok}.lnk")),
            dir.join(format!("{tok}.url")),
            dir.join(tok),
        ] {
            if candidate.exists() {
                let key = candidate.to_string_lossy().to_lowercase();
                if seen.insert(key) {
                    let size = bounded_directory_size(&candidate, 2);
                    out.push(AppRemnant {
                        kind: RemnantKind::Shortcut,
                        location: candidate.display().to_string(),
                        path: Some(candidate),
                        registry_hive: None,
                        registry_subkey: None,
                        size_bytes: size,
                    });
                }
            }
        }
    }
}

fn collect_registry_remnants(
    app: &InstalledApp,
    app_tokens: &[String],
    pub_tokens: &[String],
    reg_prefix: &str,
    out: &mut Vec<AppRemnant>,
) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut seen_reg = HashSet::new();

    // 1. Check if the Uninstall registration key itself still exists
    if !app.registry_subkey.is_empty() {
        let (root, hive_name) = match app.source {
            AppSource::Win32Machine64 | AppSource::Win32Machine32 => (&hklm, "HKLM"),
            AppSource::Win32User | AppSource::WindowsStoreAppX => (&hkcu, "HKCU"),
        };
        if root
            .open_subkey_with_flags(&app.registry_subkey, KEY_READ)
            .is_ok()
        {
            let full = format!("{hive_name}\\{}", app.registry_subkey);
            if seen_reg.insert(full.to_lowercase()) {
                out.push(AppRemnant {
                    kind: RemnantKind::RegistryUninstallKey,
                    location: full,
                    path: None,
                    registry_hive: Some(hive_name.to_string()),
                    registry_subkey: Some(app.registry_subkey.clone()),
                    size_bytes: 0,
                });
            }
        }
    }

    let trimmed_prefix = reg_prefix.trim_matches('\\');
    let software_base = if trimmed_prefix.is_empty() {
        "Software".to_string()
    } else {
        format!("{trimmed_prefix}\\Software")
    };

    // 2. Check HKCU\<prefix>\Software\<AppName> and HKCU\<prefix>\Software\<Publisher>\<AppName>
    for app_tok in app_tokens {
        if !is_safe_folder_token(app_tok) {
            continue;
        }
        let direct_key = format!("{software_base}\\{app_tok}");
        if hkcu.open_subkey_with_flags(&direct_key, KEY_READ).is_ok() {
            let full = format!("HKCU\\{direct_key}");
            if seen_reg.insert(full.to_lowercase()) {
                out.push(AppRemnant {
                    kind: RemnantKind::RegistrySoftwareKey,
                    location: full,
                    path: None,
                    registry_hive: Some("HKCU".to_string()),
                    registry_subkey: Some(direct_key),
                    size_bytes: 0,
                });
            }
        }

        for pub_tok in pub_tokens {
            if pub_tok.is_empty() {
                continue;
            }
            let nested_key = format!("{software_base}\\{pub_tok}\\{app_tok}");
            if hkcu.open_subkey_with_flags(&nested_key, KEY_READ).is_ok() {
                let full = format!("HKCU\\{nested_key}");
                if seen_reg.insert(full.to_lowercase()) {
                    out.push(AppRemnant {
                        kind: RemnantKind::RegistrySoftwareKey,
                        location: full,
                        path: None,
                        registry_hive: Some("HKCU".to_string()),
                        registry_subkey: Some(nested_key),
                        size_bytes: 0,
                    });
                }
            }
        }
    }

    // 3. When running on the live system (no sandbox prefix), also check HKLM\Software\<Publisher>\<AppName>
    if trimmed_prefix.is_empty() {
        for app_tok in app_tokens {
            if !is_safe_folder_token(app_tok) {
                continue;
            }
            for pub_tok in pub_tokens {
                if pub_tok.is_empty() {
                    continue;
                }
                let nested_key = format!("Software\\{pub_tok}\\{app_tok}");
                if hklm.open_subkey_with_flags(&nested_key, KEY_READ).is_ok() {
                    let full = format!("HKLM\\{nested_key}");
                    if seen_reg.insert(full.to_lowercase()) {
                        out.push(AppRemnant {
                            kind: RemnantKind::RegistrySoftwareKey,
                            location: full,
                            path: None,
                            registry_hive: Some("HKLM".to_string()),
                            registry_subkey: Some(nested_key),
                            size_bytes: 0,
                        });
                    }
                }
            }
        }
    }
}

/// Purges the discovered remnants using `open-cleaner`'s core deletion engine (`Standard` or `SecureZero`).
pub fn purge_app_remnants(
    remnants: &[AppRemnant],
    mode: CleanMode,
) -> (usize, usize, u64, Vec<String>) {
    let mut removed = 0usize;
    let mut failed = 0usize;
    let mut bytes_reclaimed = 0u64;
    let mut removed_locations = Vec::new();

    for rem in remnants {
        if rem.kind.is_registry() {
            if mode == CleanMode::DryRun {
                removed += 1;
                removed_locations.push(rem.location.clone());
                continue;
            }
            let Some(hive) = &rem.registry_hive else {
                failed += 1;
                continue;
            };
            let Some(subkey) = &rem.registry_subkey else {
                failed += 1;
                continue;
            };
            let root = if hive == "HKLM" {
                RegKey::predef(HKEY_LOCAL_MACHINE)
            } else {
                RegKey::predef(HKEY_CURRENT_USER)
            };

            if delete_registry_subkey_tree(&root, subkey) {
                removed += 1;
                removed_locations.push(rem.location.clone());
            } else {
                failed += 1;
            }
        } else if let Some(path) = &rem.path {
            if !path.exists() {
                continue;
            }
            if !is_safe_install_location(path) {
                failed += 1;
                continue;
            }

            if mode == CleanMode::DryRun {
                removed += 1;
                bytes_reclaimed = bytes_reclaimed.saturating_add(rem.size_bytes);
                removed_locations.push(rem.location.clone());
                continue;
            }

            if path.is_dir() {
                let mut report = ModuleReport::new("UninstallerRemnants");
                let allow_all = |_: &Path| true;
                clean_directory_contents(
                    path,
                    true,
                    false,
                    &allow_all,
                    mode,
                    &mut report,
                    "App Remnant",
                );
                let _ = fs::remove_dir_all(path);
                if !path.exists() {
                    removed += 1;
                    bytes_reclaimed =
                        bytes_reclaimed.saturating_add(report.bytes_reclaimed.max(rem.size_bytes));
                    removed_locations.push(rem.location.clone());
                } else {
                    failed += 1;
                }
            } else {
                match secure_delete_file(path, mode) {
                    CleanResult::Cleaned(sz) | CleanResult::DryRunFound(sz) => {
                        removed += 1;
                        bytes_reclaimed = bytes_reclaimed.saturating_add(sz.max(rem.size_bytes));
                        removed_locations.push(rem.location.clone());
                    }
                    _ if !path.exists() => {
                        removed += 1;
                        bytes_reclaimed = bytes_reclaimed.saturating_add(rem.size_bytes);
                        removed_locations.push(rem.location.clone());
                    }
                    _ => {
                        failed += 1;
                    }
                }
            }
        }
    }

    (removed, failed, bytes_reclaimed, removed_locations)
}

fn delete_registry_subkey_tree(root: &RegKey, subkey_path: &str) -> bool {
    let trimmed = subkey_path.trim_matches('\\');
    if let Some((parent_path, leaf_name)) = trimmed.rsplit_once('\\')
        && let Ok(parent_key) = root.open_subkey_with_flags(parent_path, KEY_READ | KEY_WRITE)
    {
        return parent_key.delete_subkey_all(leaf_name).is_ok();
    }
    root.delete_subkey_all(trimmed).is_ok()
}

/// Generates clean folder/registry candidate names from a display name or publisher string.
pub fn candidate_tokens(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return Vec::new();
    }

    let mut out = Vec::new();
    // Strip trailing version numbers or architectures like "(64-bit)" or "v1.2.3"
    let base = trimmed
        .split('(')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_string();

    if !base.is_empty() {
        out.push(base.clone());
    }

    // Also include compact no-space variant if multi-word
    let compact: String = base
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if !compact.is_empty()
        && !out
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&compact))
    {
        out.push(compact);
    }

    out
}

pub fn is_safe_folder_token(token: &str) -> bool {
    let clean = token.trim();
    if clean.len() < 3 {
        return false;
    }
    if clean.contains("..") || clean.contains('\\') || clean.contains('/') || clean.contains(':') {
        return false;
    }
    let lower = clean.to_ascii_lowercase();
    !SHARED_VENDORS.contains(&lower.as_str())
}

pub fn is_safe_install_location(path: &Path) -> bool {
    // Must have at least 3 path components (e.g. C:\Program Files\VendorApp)
    let components_count = path.components().count();
    if components_count < 3 {
        return false;
    }

    let lower = path
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();

    const PROTECTED_EXACT: &[&str] = &[
        "c:",
        "c:\\windows",
        "c:\\windows\\system32",
        "c:\\windows\\syswow64",
        "c:\\program files",
        "c:\\program files (x86)",
        "c:\\programdata",
        "c:\\users",
    ];
    if PROTECTED_EXACT.contains(&lower.as_str()) {
        return false;
    }

    // Never allow deleting top-level environment roots directly
    for env_key in [
        "LOCALAPPDATA",
        "APPDATA",
        "PROGRAMDATA",
        "USERPROFILE",
        "WINDIR",
    ] {
        if let Ok(val) = env::var(env_key) {
            let env_clean = val.trim_end_matches(['\\', '/']).to_ascii_lowercase();
            if !env_clean.is_empty() && lower == env_clean {
                return false;
            }
        }
    }

    if let Some(leaf) = path.file_name().and_then(|s| s.to_str()) {
        let leaf_lower = leaf.to_ascii_lowercase();
        if SHARED_VENDORS.contains(&leaf_lower.as_str()) {
            return false;
        }
    }

    true
}

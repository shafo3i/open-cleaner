//! Execution of bloatware removal actions.
//!
//! Removal requires elevation for de-provisioning, policy writes, and system packages.
//! Failures are captured per-item into the report rather than aborting the whole pass.

use std::process::Command;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_SET_VALUE};

use super::model::{BloatItem, BloatProgress, BloatReport, BloatTarget, PolicyScope};

fn run_powershell(script: &str) -> Result<(), String> {
    let out = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| format!("failed to launch PowerShell: {e}"))?;

    if out.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let msg = if stderr.is_empty() { stdout } else { stderr };
    Err(if msg.is_empty() {
        format!("PowerShell exited with status {}", out.status)
    } else {
        msg
    })
}

/// Creates a Windows system restore point. Requires elevation and System Protection enabled.
/// Returns a human-readable reason when it cannot be created.
pub fn create_restore_point() -> Result<(), String> {
    if !is_elevated() {
        return Err("requires administrator privileges — run Open Cleaner elevated".to_string());
    }

    match run_powershell(
        "Checkpoint-Computer -Description 'open-cleaner — pre-bloatware-removal' -RestorePointType MODIFY_SETTINGS",
    ) {
        Ok(()) => Ok(()),
        Err(err) => {
            let lower = err.to_ascii_lowercase();
            if lower.contains("system protection")
                || lower.contains("system restore")
                || lower.contains("restore point")
                || lower.contains("not enabled")
            {
                Err("System Protection is disabled — enable it under System Properties → System Protection".to_string())
            } else {
                Err(err)
            }
        }
    }
}

/// Returns whether the process is running with administrator rights.
/// Uses a non-invasive access-right probe against an HKLM key.
pub fn is_elevated() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hklm.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion",
        KEY_SET_VALUE,
    )
    .is_ok()
}

fn remove_capability(capability_id: &str) -> Result<(), String> {
    let out = Command::new("dism.exe")
        .args([
            "/Online",
            "/Remove-Capability",
            &format!("/CapabilityName:{capability_id}"),
            "/NoRestart",
        ])
        .output()
        .map_err(|e| format!("failed to launch DISM: {e}"))?;

    if out.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let msg = if stderr.is_empty() { stdout } else { stderr };
    Err(if msg.is_empty() {
        format!("DISM exited with status {}", out.status)
    } else {
        msg
    })
}

fn remove_appx_package(full_name: &str) -> Result<(), String> {
    run_powershell(&format!(
        "Remove-AppxPackage -Package '{full_name}' -ErrorAction Stop"
    ))
}

fn remove_appx_provisioned(full_name: &str) -> Result<(), String> {
    run_powershell(&format!(
        "Remove-AppxProvisionedPackage -Online -PackageName '{full_name}' -ErrorAction Stop"
    ))
}

fn set_policy_dword(
    root: &RegKey,
    key_path: &str,
    value_name: &str,
    value: u32,
) -> Result<(), String> {
    let denied = || "access denied — run as administrator to apply this policy".to_string();
    let (key, _) = root.create_subkey(key_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            denied()
        } else {
            format!("open/create '{key_path}': {e}")
        }
    })?;
    key.set_value(value_name, &value).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            denied()
        } else {
            format!("set '{value_name}' = {value}: {e}")
        }
    })
}

fn apply_policy(
    scope: PolicyScope,
    key_path: &str,
    value_name: &str,
    value: u32,
) -> Result<Vec<String>, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    match scope {
        PolicyScope::UserOnly => {
            set_policy_dword(&hkcu, key_path, value_name, value)?;
            Ok(vec![format!(
                "Applied user policy {key_path}\\{value_name} = {value} (HKCU)"
            )])
        }
        PolicyScope::MachineOnly => {
            set_policy_dword(&hklm, key_path, value_name, value)?;
            Ok(vec![format!(
                "Applied machine policy {key_path}\\{value_name} = {value} (HKLM)"
            )])
        }
        PolicyScope::Both => {
            set_policy_dword(&hkcu, key_path, value_name, value)?;
            let mut lines = vec![format!(
                "Applied user policy {key_path}\\{value_name} = {value} (HKCU)"
            )];
            match set_policy_dword(&hklm, key_path, value_name, value) {
                Ok(()) => lines.push(format!(
                    "Applied machine policy {key_path}\\{value_name} = {value} (HKLM)"
                )),
                Err(err) => lines.push(format!("Machine policy skipped: {err}")),
            }
            Ok(lines)
        }
    }
}

fn execute_item(item: &BloatItem) -> Result<Vec<String>, String> {
    match &item.target {
        BloatTarget::StoreApp {
            installed_full_name,
            provisioned_full_name,
        } => {
            let mut lines = Vec::new();

            if let Some(full) = installed_full_name {
                remove_appx_package(full)?;
                lines.push(format!("Removed Appx package: {full}"));
            }
            if let Some(full) = provisioned_full_name {
                remove_appx_provisioned(full)?;
                lines.push(format!("De-provisioned Appx package: {full}"));
            }

            if lines.is_empty() {
                return Err("no installed or provisioned package to act on".to_string());
            }
            Ok(lines)
        }
        BloatTarget::Policy {
            scope,
            key_path,
            value_name,
            disable_value,
        } => apply_policy(*scope, key_path, value_name, *disable_value),
        BloatTarget::Capability { capability_id } => {
            remove_capability(capability_id)?;
            Ok(vec![format!("Removed capability: {capability_id}")])
        }
    }
}

/// Runs the full debloat pass over the selected items, streaming `BloatProgress` updates.
pub fn remove_bloat_with_progress<F>(
    items: &[BloatItem],
    create_restore: bool,
    mut on_progress: F,
) -> BloatReport
where
    F: FnMut(BloatProgress),
{
    let mut report = BloatReport {
        items_processed: items.len(),
        ..Default::default()
    };

    if create_restore {
        on_progress(BloatProgress {
            item_name: "SYSTEM RESTORE POINT".to_string(),
            fraction: 0.01,
            detail: "Creating Windows restore point...".to_string(),
        });

        match create_restore_point() {
            Ok(()) => {
                report.restore_point_created = true;
                report.details.push("Restore point created.".to_string());
                on_progress(BloatProgress {
                    item_name: "SYSTEM RESTORE POINT".to_string(),
                    fraction: 0.02,
                    detail: "Restore point created.".to_string(),
                });
            }
            Err(err) => {
                report.details.push(format!("Restore point skipped: {err}"));
                on_progress(BloatProgress {
                    item_name: "SYSTEM RESTORE POINT".to_string(),
                    fraction: 0.02,
                    detail: format!("Restore point skipped: {err}"),
                });
            }
        }
    }

    let total = items.len().max(1);
    for (idx, item) in items.iter().enumerate() {
        let fraction = (idx as f32 + 1.0) / (total as f32 + 1.0);
        on_progress(BloatProgress {
            item_name: item.name.clone(),
            fraction,
            detail: format!("Processing '{}'...", item.name),
        });

        match execute_item(item) {
            Ok(lines) => {
                report.items_removed += 1;
                report.details.extend(lines);
            }
            Err(err) => {
                report.items_failed += 1;
                report
                    .details
                    .push(format!("FAILED '{}': {err}", item.name));
            }
        }
    }

    on_progress(BloatProgress {
        item_name: "COMPLETE".to_string(),
        fraction: 1.0,
        detail: "Debloat pass finished.".to_string(),
    });

    report
}

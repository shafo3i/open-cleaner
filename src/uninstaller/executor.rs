use std::process::Command;

use super::model::{AppSource, InstalledApp, InstallerKind};

/// Fingerprints the installer technology based on CCleaner 7's `AVGenericUninstallerModule` heuristics.
pub fn detect_installer_kind(
    uninstall_string: &str,
    quiet_uninstall: Option<&str>,
    windows_installer_flag: u32,
    source: AppSource,
) -> InstallerKind {
    if source == AppSource::WindowsStoreAppX {
        return InstallerKind::AppX;
    }

    let lower = uninstall_string.to_lowercase();
    let quiet_lower = quiet_uninstall.unwrap_or_default().to_lowercase();

    if windows_installer_flag == 1 || lower.contains("msiexec") {
        InstallerKind::Msi
    } else if lower.contains("unins00")
        || quiet_lower.contains("/verysilent")
        || lower.contains("inno")
    {
        InstallerKind::InnoSetup
    } else if lower.ends_with("uninstall.exe")
        || lower.contains("\\uninstall.exe")
        || lower.contains("\\uninst.exe")
        || quiet_lower.ends_with(" /s")
    {
        InstallerKind::Nsis
    } else {
        InstallerKind::Generic
    }
}

/// Normalizes an application's uninstall command line.
/// Converts `MsiExec.exe /I{GUID}` to `/X{GUID}` and applies silent flags when requested.
pub fn normalize_uninstall_command(app: &InstalledApp, prefer_silent: bool) -> String {
    if prefer_silent && let Some(quiet) = &app.quiet_uninstall_string {
        let trimmed = quiet.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let mut cmd = app.uninstall_string.trim().to_string();
    if cmd.is_empty() {
        return cmd;
    }

    match app.installer_kind {
        InstallerKind::Msi => {
            // Replace /I{GUID} or /i{GUID} with /X{GUID} so MSI uninstalls rather than modifying
            if let Some(pos) = cmd.to_ascii_lowercase().find("/i{") {
                cmd.replace_range(pos..pos + 2, "/X");
            } else if let Some(pos) = cmd.to_ascii_lowercase().find("/i ") {
                cmd.replace_range(pos..pos + 2, "/X");
            }
            if prefer_silent && !cmd.to_ascii_lowercase().contains(" /q") {
                cmd.push_str(" /qn /norestart");
            }
        }
        InstallerKind::InnoSetup => {
            if prefer_silent && !cmd.to_ascii_lowercase().contains("silent") {
                cmd.push_str(" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART");
            }
        }
        InstallerKind::Nsis => {
            if prefer_silent && !cmd.ends_with(" /S") && !cmd.contains(" /S ") {
                cmd.push_str(" /S");
            }
        }
        InstallerKind::AppX | InstallerKind::Generic => {}
    }

    cmd
}

/// Splits a raw Windows command line string into `(executable, arguments)`.
/// Handles quoted paths, unquoted paths with spaces ending in `.exe`, and standard arguments.
pub fn split_command_line(raw: &str) -> Option<(String, Vec<String>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (program, rest) = if let Some(after_quote) = trimmed.strip_prefix('"') {
        let end_quote = after_quote.find('"')?;
        let prog = after_quote[..end_quote].to_string();
        let remainder = after_quote[end_quote + 1..].trim();
        (prog, remainder)
    } else {
        // Check if an unquoted path contains spaces before `.exe`
        let lower = trimmed.to_ascii_lowercase();
        if let Some(exe_pos) = lower.find(".exe") {
            let split_idx = exe_pos + 4;
            let prog = trimmed[..split_idx].trim().to_string();
            let remainder = trimmed[split_idx..].trim();
            (prog, remainder)
        } else {
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let prog = parts.next()?.to_string();
            let remainder = parts.next().unwrap_or("").trim();
            (prog, remainder)
        }
    };

    let args = parse_arguments(rest);
    Some((program, args))
}

fn parse_arguments(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// Executes the native uninstaller command and waits for the process to exit.
/// Returns the process exit code (`0`, `3010`, or `1641` indicate success).
pub fn execute_native_uninstaller(
    app: &InstalledApp,
    prefer_silent: bool,
) -> Result<(String, i32), String> {
    let normalized = normalize_uninstall_command(app, prefer_silent);
    let (program, args) = split_command_line(&normalized).ok_or_else(|| {
        format!(
            "Invalid or empty uninstall command for '{}'",
            app.display_name
        )
    })?;

    let status = Command::new(&program)
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to launch uninstaller '{}': {}", program, e))?;

    let code = status.code().unwrap_or(-1);
    Ok((normalized, code))
}

pub fn is_success_exit_code(code: i32) -> bool {
    // 0 = OK, 3010 = ERROR_SUCCESS_REBOOT_REQUIRED, 1641 = ERROR_SUCCESS_REBOOT_INITIATED
    code == 0 || code == 3010 || code == 1641
}

pub mod discovery;
pub mod executor;
pub mod model;
pub mod remnants;

pub use discovery::{scan_installed_apps, scan_installed_apps_with_prefix};
pub use executor::{
    detect_installer_kind, execute_native_uninstaller, is_success_exit_code,
    normalize_uninstall_command, split_command_line,
};
pub use model::{
    AppRemnant, AppSource, InstalledApp, InstallerKind, RemnantKind, UninstallOptions,
    UninstallPhase, UninstallProgress, UninstallReport,
};
pub use remnants::{purge_app_remnants, scan_app_remnants, scan_app_remnants_with_prefix};

/// Runs a complete application removal cycle:
/// 1. Optionally launches the native uninstaller (`UninstallString` / `QuietUninstallString`) and waits for exit.
/// 2. Scans for residual filesystem folders, shortcuts, and registry keys left behind.
/// 3. Purges all verified remnants using the requested `CleanMode` (`Standard`, `SecureZero`, or `DryRun`).
pub fn uninstall_application_completely(
    app: &InstalledApp,
    options: UninstallOptions,
) -> UninstallReport {
    uninstall_application_with_prefix(app, options, "")
}

/// Complete application removal with an optional `HKCU\<reg_prefix>` registry scope for sandboxed testing.
pub fn uninstall_application_with_prefix(
    app: &InstalledApp,
    options: UninstallOptions,
    reg_prefix: &str,
) -> UninstallReport {
    uninstall_application_with_progress(app, options, reg_prefix, |_| {})
}

/// Complete application removal that streams real-time `UninstallProgress` updates at each stage.
pub fn uninstall_application_with_progress<F>(
    app: &InstalledApp,
    options: UninstallOptions,
    reg_prefix: &str,
    mut on_progress: F,
) -> UninstallReport
where
    F: FnMut(UninstallProgress),
{
    let mut report = UninstallReport {
        app_name: app.display_name.clone(),
        ..Default::default()
    };

    on_progress(UninstallProgress {
        app_id: app.id.clone(),
        app_name: app.display_name.clone(),
        phase: UninstallPhase::Preparing,
        fraction: 0.10,
        detail: format!(
            "Preparing uninstallation pipeline for '{}'...",
            app.display_name
        ),
    });

    if options.run_native_uninstaller {
        let normalized_cmd = normalize_uninstall_command(app, options.prefer_silent);
        on_progress(UninstallProgress {
            app_id: app.id.clone(),
            app_name: app.display_name.clone(),
            phase: UninstallPhase::RunningNativeUninstaller,
            fraction: 0.28,
            detail: format!("Running native uninstaller: {normalized_cmd}"),
        });

        report.uninstaller_ran = true;
        match execute_native_uninstaller(app, options.prefer_silent) {
            Ok((cmd, code)) => {
                report.uninstaller_command = cmd;
                report.uninstaller_exit_code = Some(code);
                report.uninstaller_success = is_success_exit_code(code);
                if !report.uninstaller_success {
                    report.error_message =
                        Some(format!("Native uninstaller exited with status code {code}"));
                }
            }
            Err(err) => {
                report.uninstaller_success = false;
                report.error_message = Some(err);
            }
        }
    } else {
        report.uninstaller_success = true;
    }

    if options.purge_remnants {
        on_progress(UninstallProgress {
            app_id: app.id.clone(),
            app_name: app.display_name.clone(),
            phase: UninstallPhase::ScanningRemnants,
            fraction: 0.55,
            detail: "Scanning AppData, ProgramData, Shortcuts & Registry for leftovers..."
                .to_string(),
        });

        let remnants = scan_app_remnants_with_prefix(app, reg_prefix);
        report.remnants_found = remnants.len();
        let total = remnants.len();

        for (idx, rem) in remnants.iter().enumerate() {
            let current = idx + 1;
            let step_frac = 0.60 + (0.32 * (current as f32 / total.max(1) as f32));
            on_progress(UninstallProgress {
                app_id: app.id.clone(),
                app_name: app.display_name.clone(),
                phase: UninstallPhase::PurgingRemnant { current, total },
                fraction: step_frac,
                detail: format!("[{}] {}", rem.kind.label(), rem.location),
            });

            let (removed, failed, bytes, mut locations) =
                purge_app_remnants(std::slice::from_ref(rem), options.clean_mode);
            report.remnants_removed += removed;
            report.remnants_failed += failed;
            report.bytes_reclaimed = report.bytes_reclaimed.saturating_add(bytes);
            report.removed_locations.append(&mut locations);
        }
    }

    on_progress(UninstallProgress {
        app_id: app.id.clone(),
        app_name: app.display_name.clone(),
        phase: UninstallPhase::Completed,
        fraction: 1.0,
        detail: format!(
            "Completed removal of '{}' ({} remnants purged)",
            app.display_name, report.remnants_removed
        ),
    });

    report
}

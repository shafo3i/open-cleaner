//! Background worker conductor: thread dispatching and asynchronous event routing.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

use crate::bloat::{BloatItem, BloatProgress, BloatReport};
use crate::core::CleanMode;
use crate::core::ui::format_bytes;
use crate::ui::app::SystemCleanerApp;
use crate::ui::logs_view;
use crate::ui::model::TargetStatus;
use crate::uninstaller::{
    InstalledApp, UninstallOptions, UninstallPhase, UninstallProgress, UninstallReport,
    scan_installed_apps, uninstall_application_with_progress,
};
use crate::updater::UpdateInfo;

/// Telemetry messages communicated from background workers to the UI event loop.
pub(crate) enum WorkerResult {
    ScanFinished {
        scan_gen: u64,
        total_reclaimable: u64,
        total_scanned: usize,
        total_locked: usize,
        duration: f64,
        target_reports: Vec<TargetReport>,
    },
    CleanFinished {
        total_reclaimed: u64,
        total_cleaned: usize,
        total_locked: usize,
        duration: f64,
        target_reports: Vec<TargetReport>,
    },
    AppsScanFinished {
        apps: Vec<InstalledApp>,
    },
    AppUninstallProgress {
        progress: UninstallProgress,
    },
    AppUninstallFinished {
        report: UninstallReport,
        updated_apps: Vec<InstalledApp>,
    },
    BloatScanFinished {
        items: Vec<BloatItem>,
    },
    BloatProgress {
        progress: BloatProgress,
    },
    BloatFinished {
        report: BloatReport,
    },
    UpdateCheck {
        update: Option<UpdateInfo>,
    },
    UpdateFinished {
        updated: bool,
        message: String,
    },
}

/// Metrics and access status for a single cleanup target reported by worker threads.
pub(crate) struct TargetReport {
    pub(crate) id: String,
    pub(crate) bytes: u64,
    pub(crate) items: usize,
    pub(crate) remaining_bytes: u64,
    pub(crate) remaining_items: usize,
    pub(crate) locked: usize,
    pub(crate) permission_denied: usize,
    pub(crate) failed: usize,
}

impl SystemCleanerApp {
    /// Dispatches a background scan of installed applications across Registry and AppX catalogs.
    pub(crate) fn run_apps_scan(&mut self) {
        if self.uninstaller_state.is_scanning || self.uninstaller_state.is_uninstalling {
            return;
        }
        self.uninstaller_state.is_scanning = true;
        let tx = self.tx.clone();
        thread::spawn(move || {
            let apps = scan_installed_apps();
            let _ = tx.send(WorkerResult::AppsScanFinished { apps });
        });
    }

    /// Dispatches a background audit for pre-installed bloatware packages.
    pub(crate) fn run_bloat_scan(&mut self) {
        if self.bloat_state.is_scanning || self.bloat_state.is_removing {
            return;
        }
        self.bloat_state.is_scanning = true;
        let tx = self.tx.clone();
        thread::spawn(move || {
            let items = crate::bloat::scan_bloatware();
            let _ = tx.send(WorkerResult::BloatScanFinished { items });
        });
    }

    /// Dispatches removal of selected bloatware components with real-time progress callbacks.
    pub(crate) fn run_bloat_removal(&mut self, create_restore: bool) {
        if self.bloat_state.is_removing {
            return;
        }
        let selected: Vec<BloatItem> = self
            .bloat_state
            .items
            .iter()
            .filter(|item| item.selected)
            .cloned()
            .collect();
        if selected.is_empty() {
            return;
        }

        self.bloat_state.is_removing = true;
        self.bloat_state.progress_log.clear();
        self.bloat_state.last_report = None;
        self.metrics.status_text = format!("DEBLOATING {} SYSTEM COMPONENT(S)...", selected.len());

        let tx = self.tx.clone();
        thread::spawn(move || {
            let tx_prog = tx.clone();
            let report =
                crate::bloat::remove_bloat_with_progress(&selected, create_restore, move |p| {
                    let _ = tx_prog.send(WorkerResult::BloatProgress { progress: p });
                });
            let _ = tx.send(WorkerResult::BloatFinished { report });
        });
    }

    /// Dispatches the end-to-end uninstallation pipeline for a target desktop application.
    pub(crate) fn run_app_uninstall(&mut self, app: InstalledApp, options: UninstallOptions) {
        if self.uninstaller_state.is_uninstalling {
            return;
        }
        self.uninstaller_state.is_uninstalling = true;
        self.uninstaller_state.show_completion_modal = false;
        self.uninstaller_state.progress_log.clear();
        self.uninstaller_state.progress_log.push(format!(
            "[10%] Initializing uninstallation for '{}'...",
            app.display_name
        ));
        self.uninstaller_state.active_progress = Some(UninstallProgress {
            app_id: app.id.clone(),
            app_name: app.display_name.clone(),
            phase: UninstallPhase::Preparing,
            fraction: 0.10,
            detail: format!(
                "Preparing uninstallation pipeline for '{}'...",
                app.display_name
            ),
        });
        self.metrics.status_text = format!(
            "UNINSTALLING {} & EXPUNGING REMNANTS...",
            app.display_name.to_uppercase()
        );
        let tx = self.tx.clone();
        thread::spawn(move || {
            let tx_prog = tx.clone();
            let report = uninstall_application_with_progress(&app, options, "", move |progress| {
                let _ = tx_prog.send(WorkerResult::AppUninstallProgress { progress });
            });
            let _ = tx.send(WorkerResult::AppUninstallProgress {
                progress: UninstallProgress {
                    app_id: app.id.clone(),
                    app_name: app.display_name.clone(),
                    phase: UninstallPhase::RefreshingCatalog,
                    fraction: 0.95,
                    detail: "Refreshing installed applications catalog...".to_string(),
                },
            });
            let updated_apps = scan_installed_apps();
            let _ = tx.send(WorkerResult::AppUninstallFinished {
                report,
                updated_apps,
            });
        });
    }

    /// Spawns parallel worker threads to scan all configured cleanup targets in dry-run mode.
    pub(crate) fn run_scan(&mut self) {
        if self.is_purging {
            return;
        }
        self.worker_gen += 1;
        let scan_gen = self.worker_gen;
        self.metrics.is_busy = true;
        self.metrics.status_text = "AUDITING TARGET REPOSITORIES...".to_string();

        let tx = self.tx.clone();
        let target_ids: Vec<String> = self
            .targets
            .iter()
            .map(|target| target.id.to_string())
            .collect();
        thread::spawn(move || {
            let start = Instant::now();
            let next_idx = AtomicUsize::new(0);
            let results: Mutex<Vec<TargetReport>> =
                Mutex::new(Vec::with_capacity(target_ids.len()));
            let worker_count = target_ids.len().clamp(1, 8);

            thread::scope(|s| {
                for _ in 0..worker_count {
                    s.spawn(|| {
                        loop {
                            let idx = next_idx.fetch_add(1, Ordering::Relaxed);
                            if idx >= target_ids.len() {
                                break;
                            }
                            let id = &target_ids[idx];
                            let report = crate::modules::execute_target(id, CleanMode::DryRun);
                            let tr = TargetReport {
                                id: id.clone(),
                                bytes: report.bytes_reclaimed,
                                items: report.items_scanned,
                                remaining_bytes: report.bytes_reclaimed,
                                remaining_items: report.items_scanned,
                                locked: report.items_skipped_locked,
                                permission_denied: report.items_skipped_permission,
                                failed: report.items_failed,
                            };
                            if let Ok(mut guard) = results.lock() {
                                guard.push(tr);
                            }
                        }
                    });
                }
            });

            let target_reports = results.into_inner().unwrap_or_default();
            let total_reclaimable = target_reports.iter().map(|report| report.bytes).sum();
            let total_scanned = target_reports.iter().map(|report| report.items).sum();
            let total_locked = target_reports
                .iter()
                .map(|report| report.locked + report.permission_denied)
                .sum();
            let duration = start.elapsed().as_secs_f64();

            let _ = tx.send(WorkerResult::ScanFinished {
                scan_gen,
                total_reclaimable,
                total_scanned,
                total_locked,
                duration,
                target_reports,
            });
        });
    }

    /// Spawns worker threads to execute file unlink or DoD zero-fill purge across queued targets.
    pub(crate) fn run_purge(&mut self, target_ids: Vec<String>) {
        if self.is_purging || target_ids.is_empty() {
            return;
        }
        // Invalidate any in-flight background scan so purge executes immediately
        self.worker_gen += 1;
        self.is_purging = true;
        self.metrics.is_busy = true;

        if target_ids.len() == 1 {
            let name = self
                .targets
                .iter()
                .find(|t| t.id == target_ids[0])
                .map(|t| format!("{} {}", t.name_prefix, t.name_suffix))
                .unwrap_or_else(|| "TARGET".to_string());
            self.metrics.status_text = format!("PURGING {}...", name.to_uppercase());
        } else if self.secure_wipe {
            self.metrics.status_text = format!(
                "EXECUTING ZERO-FILL PURGE ON {} TARGETS...",
                target_ids.len()
            );
        } else {
            self.metrics.status_text = format!("PURGING {} SELECTED TARGETS...", target_ids.len());
        }

        let mode = if self.secure_wipe {
            CleanMode::SecureZero
        } else {
            CleanMode::Standard
        };

        let tx = self.tx.clone();
        let targets_to_clean = target_ids;

        thread::spawn(move || {
            let start = Instant::now();
            let next_idx = AtomicUsize::new(0);
            let results: Mutex<Vec<TargetReport>> =
                Mutex::new(Vec::with_capacity(targets_to_clean.len()));
            let worker_count = targets_to_clean.len().clamp(1, 8);

            thread::scope(|s| {
                for _ in 0..worker_count {
                    s.spawn(|| {
                        loop {
                            let idx = next_idx.fetch_add(1, Ordering::Relaxed);
                            if idx >= targets_to_clean.len() {
                                break;
                            }
                            let tid = &targets_to_clean[idx];
                            let report = crate::modules::execute_target(tid, mode);
                            let post_scan = crate::modules::execute_target(tid, CleanMode::DryRun);
                            let tr = TargetReport {
                                id: tid.clone(),
                                bytes: report.bytes_reclaimed,
                                items: report.items_cleaned,
                                remaining_bytes: post_scan.bytes_reclaimed,
                                remaining_items: post_scan.items_scanned,
                                locked: report.items_skipped_locked,
                                permission_denied: report.items_skipped_permission,
                                failed: report.items_failed,
                            };
                            if let Ok(mut guard) = results.lock() {
                                guard.push(tr);
                            }
                        }
                    });
                }
            });

            let target_reports = results.into_inner().unwrap_or_default();
            let total_reclaimed = target_reports.iter().map(|r| r.bytes).sum();
            let total_cleaned = target_reports.iter().map(|r| r.items).sum();
            let total_locked = target_reports
                .iter()
                .map(|r| r.locked + r.permission_denied)
                .sum();

            let duration = start.elapsed().as_secs_f64();
            let _ = tx.send(WorkerResult::CleanFinished {
                total_reclaimed,
                total_cleaned,
                total_locked,
                duration,
                target_reports,
            });
        });
    }

    /// Polls and processes all pending messages from the background worker channel.
    pub(crate) fn check_worker_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                WorkerResult::ScanFinished {
                    scan_gen,
                    total_reclaimable,
                    total_scanned,
                    total_locked,
                    duration,
                    target_reports,
                } => {
                    if scan_gen != self.worker_gen {
                        continue;
                    }
                    self.metrics.is_busy = false;
                    self.metrics.reclaimable_formatted = format_bytes(total_reclaimable);
                    self.metrics.scanned_count = total_scanned;
                    self.metrics.locked_count = total_locked;
                    self.metrics.duration_secs = duration;
                    let target_failures: usize =
                        target_reports.iter().map(|report| report.failed).sum();
                    self.metrics.status_text = if target_failures == 0 {
                        "AUDIT COMPLETE".to_string()
                    } else {
                        format!("AUDIT COMPLETE: {target_failures} TARGET ACCESS ERROR(S)")
                    };

                    // Update targets with real disk metrics
                    for item in &mut self.targets {
                        if let Some(report) =
                            target_reports.iter().find(|report| report.id == item.id)
                        {
                            let skipped = report.locked + report.permission_denied;
                            item.raw_bytes = report.bytes;
                            item.size = if item.category == "registry" {
                                format!("{} keys", report.items)
                            } else {
                                format_bytes(report.bytes)
                            };
                            item.count = if item.category == "registry" {
                                format!("{} keys", report.items)
                            } else if skipped > 0 && report.items == 0 {
                                format!("0 files ({skipped} in use)")
                            } else {
                                format!("{} files", report.items)
                            };
                            item.status = if report.items == 0 && skipped == 0 {
                                TargetStatus::Cleaned
                            } else if item.category == "registry" {
                                TargetStatus::Guarded
                            } else {
                                TargetStatus::Available
                            };
                        }
                    }
                }
                WorkerResult::CleanFinished {
                    total_reclaimed,
                    total_cleaned,
                    total_locked,
                    duration,
                    target_reports,
                } => {
                    self.is_purging = false;
                    self.metrics.is_busy = false;
                    self.metrics.reclaimable_formatted = format_bytes(total_reclaimed);
                    self.metrics.scanned_count = total_cleaned;
                    self.metrics.locked_count = total_locked;
                    self.metrics.duration_secs = duration;
                    let reclaimed_targets = target_reports
                        .iter()
                        .filter(|report| report.items > 0 || report.remaining_items == 0)
                        .count();
                    let all_registry = target_reports.iter().all(|r| r.id.starts_with("reg_"));
                    if all_registry {
                        self.metrics.status_text = format!(
                            "PURGE COMPLETE: {} REGISTRY TARGET(S) PURIFIED ({} KEYS REMOVED)",
                            reclaimed_targets, total_cleaned
                        );
                    } else {
                        self.metrics.status_text = format!(
                            "PURGE COMPLETE: {} TARGET(S) RECLAIMED ({} ITEMS / {} FREED)",
                            reclaimed_targets,
                            total_cleaned,
                            format_bytes(total_reclaimed)
                        );
                    }

                    // Update all purged targets with post-purge state
                    for report in &target_reports {
                        if let Some(target) = self.targets.iter_mut().find(|t| t.id == report.id) {
                            let skipped = report.locked + report.permission_denied;
                            target.raw_bytes = report.remaining_bytes;
                            target.size = if target.category == "registry" {
                                format!("{} keys", report.remaining_items)
                            } else {
                                format_bytes(report.remaining_bytes)
                            };

                            if report.items > 0 || report.remaining_items == 0 {
                                target.status = TargetStatus::Cleaned;
                                target.count = if target.category == "registry" {
                                    format!("{} keys", report.remaining_items)
                                } else if skipped > 0 {
                                    format!("{} files ({skipped} in use)", report.remaining_items)
                                } else {
                                    format!("{} files", report.remaining_items)
                                };
                            } else {
                                target.status = if target.category == "registry" {
                                    TargetStatus::Guarded
                                } else {
                                    TargetStatus::Available
                                };
                                target.count = if target.category == "registry" {
                                    format!("{} keys", report.remaining_items)
                                } else if skipped > 0 {
                                    format!("{} files ({skipped} in use)", report.remaining_items)
                                } else {
                                    format!("{} files", report.remaining_items)
                                };
                            }
                        }
                    }
                }
                WorkerResult::AppsScanFinished { apps } => {
                    self.uninstaller_state.is_scanning = false;
                    self.uninstaller_state.apps = apps;
                }
                WorkerResult::AppUninstallProgress { progress } => {
                    self.metrics.status_text = format!(
                        "{:.0}% · {} ({})",
                        progress.fraction * 100.0,
                        progress.phase.step_label(),
                        progress.app_name.to_uppercase()
                    );
                    let log_line = format!(
                        "[{:>3.0}%] {} — {}",
                        progress.fraction * 100.0,
                        progress.phase.step_label(),
                        progress.detail
                    );
                    self.uninstaller_state.progress_log.push(log_line);
                    self.uninstaller_state.active_progress = Some(progress);
                }
                WorkerResult::AppUninstallFinished {
                    report,
                    updated_apps,
                } => {
                    self.uninstaller_state.is_uninstalling = false;
                    self.uninstaller_state.show_completion_modal = true;
                    self.uninstaller_state.apps = updated_apps;
                    self.metrics.status_text = format!(
                        "UNINSTALL COMPLETE: {} ({} REMNANTS PURGED / {} FREED)",
                        report.app_name.to_uppercase(),
                        report.remnants_removed,
                        format_bytes(report.bytes_reclaimed)
                    );
                    self.uninstaller_state.progress_log.push(format!(
                        "[100%] FINISHED — Purged {} remnants ({} reclaimed)",
                        report.remnants_removed,
                        format_bytes(report.bytes_reclaimed)
                    ));
                    self.uninstaller_state.last_report = Some(report);
                }
                WorkerResult::BloatScanFinished { items } => {
                    self.bloat_state.is_scanning = false;
                    self.bloat_state.items = items;
                }
                WorkerResult::BloatProgress { progress } => {
                    self.metrics.status_text = format!(
                        "{:.0}% · DEBLOAT — {}",
                        progress.fraction * 100.0,
                        progress.item_name.to_uppercase()
                    );
                    let log_line = format!(
                        "[{:>3.0}%] {} — {}",
                        progress.fraction * 100.0,
                        progress.item_name.to_uppercase(),
                        progress.detail
                    );
                    self.bloat_state.progress_log.push(log_line);
                    self.bloat_state.active_progress = Some(progress);
                }
                WorkerResult::BloatFinished { report } => {
                    self.bloat_state.is_removing = false;
                    let removed = report.items_removed;
                    let failed = report.items_failed;
                    let restore_created = report.restore_point_created;
                    self.bloat_state.last_report = Some(report);
                    self.metrics.status_text =
                        format!("DEBLOAT COMPLETE: {removed} REMOVED · {failed} FAILED");
                    self.audit_log.push(logs_view::AuditLogEntry::new(
                        if failed == 0 { "AUDIT" } else { "SECURITY" },
                        "DEBLOAT",
                        "DEBLOAT PASS COMPLETE",
                        &format!(
                            "{removed} removed · {failed} failed · restore point {}",
                            if restore_created {
                                "created"
                            } else {
                                "skipped"
                            }
                        ),
                    ));
                    if self.audit_log.len() > 200 {
                        let excess = self.audit_log.len() - 200;
                        self.audit_log.drain(0..excess);
                    }
                    self.run_bloat_scan();
                }
                WorkerResult::UpdateCheck { update } => {
                    self.pending_update = update;
                }
                WorkerResult::UpdateFinished { updated, message } => {
                    self.updating = false;
                    if updated {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).spawn();
                        }
                        std::process::exit(0);
                    } else {
                        if message == "already up to date" {
                            self.pending_update = None;
                            self.update_error = None;
                        } else {
                            self.update_error = Some(message.clone());
                        }
                        self.metrics.status_text = format!("UPDATE: {message}");
                    }
                }
            }
        }
    }
}

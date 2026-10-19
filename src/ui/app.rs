use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use egui::{CentralPanel, ScrollArea, SidePanel, TopBottomPanel};

use super::banner::{self, AuditMetrics};
use super::header::{self, ViewMode};
use super::model::{self, TargetItem, TargetStatus};
use super::sidebar::{self, AppTab, SidebarCounts};
use super::theme::{self, Tokens};
use super::toolbar;
use super::uninstaller_view::{self, UninstallerUiState, UninstallerViewAction};
use super::widgets::FilterOption;
use super::{bloat_view, grid_view, logs_view, table_view, tools_view, widgets};

use crate::bloat::{BloatItem, BloatProgress, BloatReport};
use crate::core::CleanMode;
use crate::core::ui::format_bytes;
use crate::uninstaller::{
    InstalledApp, UninstallOptions, UninstallPhase, UninstallProgress, UninstallReport,
    scan_app_remnants, scan_installed_apps, uninstall_application_with_progress,
};
use crate::updater::{self, UpdateInfo};

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

pub struct SystemCleanerApp {
    dark_mode: bool,
    view_mode: ViewMode,
    active_tab: AppTab,
    selected_category: String,
    search_query: String,
    secure_wipe: bool,
    pub(crate) metrics: AuditMetrics,
    pub(crate) targets: Vec<TargetItem>,
    utilities: Vec<tools_view::UtilityItem>,
    pub(crate) uninstaller_state: UninstallerUiState,
    pub(crate) bloat_state: bloat_view::BloatUiState,
    audit_log: Vec<logs_view::AuditLogEntry>,
    pending_update: Option<UpdateInfo>,
    updating: bool,
    pub(crate) tx: Sender<WorkerResult>,
    rx: Receiver<WorkerResult>,
    pending_purge: Option<Vec<String>>,
    rule_details_id: Option<String>,
    pub(crate) worker_gen: u64,
    pub(crate) is_purging: bool,
}

impl Default for SystemCleanerApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            dark_mode: true,
            view_mode: ViewMode::Table,
            active_tab: AppTab::Cleaner,
            selected_category: "all".to_string(),
            search_query: String::new(),
            secure_wipe: false,
            metrics: AuditMetrics::default(),
            targets: model::default_targets(),
            utilities: tools_view::default_utilities(),
            uninstaller_state: UninstallerUiState::default(),
            bloat_state: bloat_view::BloatUiState::default(),
            audit_log: Vec::new(),
            pending_update: None,
            updating: false,
            tx,
            rx,
            pending_purge: None,
            rule_details_id: None,
            worker_gen: 0,
            is_purging: false,
        }
    }
}

impl SystemCleanerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup_custom_fonts(&cc.egui_ctx);
        let mut app = Self::default();
        app.run_scan();
        app.run_apps_scan();
        app.run_bloat_scan();

        let tx = app.tx.clone();
        thread::spawn(move || {
            let update = updater::check_for_update();
            let _ = tx.send(WorkerResult::UpdateCheck { update });
        });

        app
    }

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

    fn run_scan(&mut self) {
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
                    let count = apps.len();
                    self.uninstaller_state.apps = apps;
                    if let Some(u_item) = self
                        .utilities
                        .iter_mut()
                        .find(|u| u.id == "tool_uninstaller")
                    {
                        u_item.meta_text = format!("{count} installed programs detected");
                    }
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
                    let count = updated_apps.len();
                    self.uninstaller_state.apps = updated_apps;
                    if let Some(u_item) = self
                        .utilities
                        .iter_mut()
                        .find(|u| u.id == "tool_uninstaller")
                    {
                        u_item.meta_text = format!("{count} installed programs detected");
                    }
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
                        self.pending_update = None;
                        self.metrics.status_text = format!("UPDATE: {message}");
                    }
                }
            }
        }
    }
}

#[allow(clippy::needless_ifs, clippy::collapsible_match)]
impl eframe::App for SystemCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_worker_messages();
        if self.metrics.is_busy
            || self.uninstaller_state.is_scanning
            || self.uninstaller_state.is_uninstalling
        {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
        theme::apply_visuals(ctx, self.dark_mode);

        let t = Tokens::get(self.dark_mode);
        widgets::window_edges(ctx, self.dark_mode);

        let mut trigger_scan = false;
        let mut trigger_apps_scan = false;
        let mut trigger_bloat_scan = false;
        let mut trigger_purge = false;
        let mut single_purge_id = None;
        let mut rule_details_id = None;

        // Top 48px Frameless Header
        TopBottomPanel::top("header_panel")
            .exact_height(theme::HEADER_H)
            .frame(egui::Frame::new().fill(t.bg))
            .show(ctx, |ui| {
                if header::render_header(ui, &mut self.view_mode, &mut self.dark_mode)
                    == header::HeaderAction::Refresh
                {
                    match self.active_tab {
                        AppTab::Uninstaller => trigger_apps_scan = true,
                        AppTab::Bloatware => trigger_bloat_scan = true,
                        _ => trigger_scan = true,
                    }
                }
            });

        // Bottom Footer (34px)
        TopBottomPanel::bottom("footer_panel")
            .exact_height(theme::FOOTER_H)
            .frame(egui::Frame::new().fill(t.bg).stroke(t.hairline()))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    let footer_str = format!("CHECKED JUST NOW  ·  {}", self.metrics.status_text);
                    widgets::hud(ui, &footer_str, t.faint);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        widgets::hud(ui, "SYSTEM FOR WINDOWS X64", t.faint);
                        ui.add_space(16.0);
                        widgets::rule(ui, theme::FOOTER_H, self.dark_mode);
                        ui.add_space(16.0);
                        if widgets::nav_link(ui, "GITHUB ↗", theme::FOOTER_H, false, self.dark_mode)
                            .clicked()
                        {
                            let _ = std::process::Command::new("explorer")
                                .arg("https://github.com/shafo3i/open-cleaner")
                                .spawn();
                        }
                    });
                });
            });

        // Calculate counts for Sidebar
        let cleaner_count = self
            .targets
            .iter()
            .filter(|t| t.category != "registry")
            .count();
        let registry_count = self
            .targets
            .iter()
            .filter(|t| t.category == "registry")
            .count();
        let sidebar_counts = SidebarCounts {
            cleaner_count,
            registry_count,
            uninstaller_count: self.uninstaller_state.apps.len(),
            tools_count: 3,
            bloat_count: self.bloat_state.items.len(),
            logs_count: 4 + self.audit_log.len(),
        };

        // Left Navigation Rail (240px)
        SidePanel::left("sidebar_panel")
            .exact_width(240.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(t.bg))
            .show(ctx, |ui| {
                let prev_tab = self.active_tab;
                sidebar::render_sidebar(ui, &mut self.active_tab, &sidebar_counts, self.dark_mode);

                // Auto-sync category when switching between cleaner and registry
                if self.active_tab != prev_tab {
                    match self.active_tab {
                        AppTab::Registry => self.selected_category = "registry".to_string(),
                        AppTab::Cleaner => {
                            if self.selected_category == "registry" {
                                self.selected_category = "all".to_string();
                            }
                        }
                        AppTab::Uninstaller => {
                            if self.uninstaller_state.apps.is_empty() {
                                trigger_apps_scan = true;
                            }
                        }
                        _ => {}
                    }
                }
            });

        // Central Workspace Area
        CentralPanel::default()
            .frame(egui::Frame::new().fill(t.bg))
            .show(ctx, |ui| match self.active_tab {
                AppTab::Cleaner => {
                    let sys_count = self
                        .targets
                        .iter()
                        .filter(|t| t.category == "system")
                        .count();
                    let br_count = self
                        .targets
                        .iter()
                        .filter(|t| t.category == "browsers")
                        .count();
                    let apps_count = self.targets.iter().filter(|t| t.category == "apps").count();
                    let dev_count = self.targets.iter().filter(|t| t.category == "dev").count();
                    let gfx_count = self
                        .targets
                        .iter()
                        .filter(|t| t.category == "graphics")
                        .count();
                    let priv_count = self
                        .targets
                        .iter()
                        .filter(|t| t.category == "privacy")
                        .count();

                    let options = [
                        FilterOption {
                            key: "all",
                            label: "ALL",
                            count: cleaner_count,
                        },
                        FilterOption {
                            key: "system",
                            label: "SYSTEM",
                            count: sys_count,
                        },
                        FilterOption {
                            key: "browsers",
                            label: "BROWSERS",
                            count: br_count,
                        },
                        FilterOption {
                            key: "apps",
                            label: "APPS",
                            count: apps_count,
                        },
                        FilterOption {
                            key: "dev",
                            label: "DEV & AI",
                            count: dev_count,
                        },
                        FilterOption {
                            key: "graphics",
                            label: "GRAPHICS",
                            count: gfx_count,
                        },
                        FilterOption {
                            key: "privacy",
                            label: "PRIVACY",
                            count: priv_count,
                        },
                        FilterOption {
                            key: "registry",
                            label: "REGISTRY",
                            count: registry_count,
                        },
                    ];

                    match toolbar::render_toolbar(
                        ui,
                        &mut self.selected_category,
                        &options,
                        &mut self.search_query,
                        self.dark_mode,
                    ) {
                        toolbar::ToolbarAction::Analyze => trigger_scan = true,
                        toolbar::ToolbarAction::Purge => trigger_purge = true,
                        toolbar::ToolbarAction::None => {}
                    }

                    banner::render_banner(ui, &self.metrics, &mut self.secure_wipe, self.dark_mode);

                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add_space(12.0);

                            let (sec_idx, sec_title) = match self.selected_category.as_str() {
                                "system" => ("01", "WINDOWS OPERATING SYSTEM CACHES & LOGS"),
                                "browsers" => ("02", "BROWSER PRIVACY, STORAGE & COOKIE DATABASES"),
                                "apps" => ("03", "DESKTOP & UWP APPLICATION CACHES"),
                                "dev" => ("04", "DEVELOPER TOOLS, COMPILERS & AI FRAMEWORKS"),
                                "graphics" => ("05", "GPU DRIVER SHADERS, 3D & THUMBNAIL CACHES"),
                                "privacy" => ("06", "USER ACTIVITY, RECENT DOCS & SHELL TRACES"),
                                "registry" => ("07", "GUARDED REGISTRY INTEGRITY VERIFICATION"),
                                _ => ("01", "VERIFIED FILE & PRIVACY PURGE TARGETS"),
                            };

                            let q = self.search_query.trim().to_lowercase();
                            let sel_cat = self.selected_category.clone();

                            let mut filtered: Vec<&mut TargetItem> = self
                                .targets
                                .iter_mut()
                                .filter(|item| {
                                    let matches_cat = if sel_cat == "all" {
                                        item.category != "registry"
                                    } else {
                                        item.category == sel_cat
                                    };
                                    let matches_search = q.is_empty()
                                        || item.name_prefix.to_lowercase().contains(&q)
                                        || item.name_suffix.to_lowercase().contains(&q)
                                        || item.tagline.to_lowercase().contains(&q)
                                        || item.scope.to_lowercase().contains(&q);
                                    matches_cat && matches_search
                                })
                                .collect();

                            let visible_count = filtered.len();

                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                widgets::eyebrow(ui, sec_idx, sec_title, self.dark_mode);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(16.0);
                                        let cnt_str = format!("{} TARGETS", visible_count);
                                        widgets::hud(ui, &cnt_str, t.faint);
                                    },
                                );
                            });

                            ui.add_space(8.0);

                            let actions = match self.view_mode {
                                ViewMode::Table => {
                                    table_view::render_table(ui, &mut filtered, self.dark_mode)
                                }
                                ViewMode::Grid => {
                                    grid_view::render_grid(ui, &mut filtered, self.dark_mode)
                                }
                            };
                            single_purge_id = actions.purge_id;
                            rule_details_id = actions.rule_id;

                            ui.add_space(24.0);
                        });
                }

                AppTab::Registry => {
                    let options = [FilterOption {
                        key: "registry",
                        label: "REGISTRY INTEGRITY",
                        count: registry_count,
                    }];

                    match toolbar::render_toolbar(
                        ui,
                        &mut self.selected_category,
                        &options,
                        &mut self.search_query,
                        self.dark_mode,
                    ) {
                        toolbar::ToolbarAction::Analyze => trigger_scan = true,
                        toolbar::ToolbarAction::Purge => trigger_purge = true,
                        toolbar::ToolbarAction::None => {}
                    }

                    banner::render_banner(ui, &self.metrics, &mut self.secure_wipe, self.dark_mode);

                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                widgets::eyebrow(
                                    ui,
                                    "02",
                                    "GUARDED REGISTRY INTEGRITY VERIFICATION",
                                    self.dark_mode,
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.add_space(16.0);
                                        let cnt_str = format!("{} KEYS AUDITED", registry_count);
                                        widgets::hud(ui, &cnt_str, t.faint);
                                    },
                                );
                            });

                            ui.add_space(8.0);

                            let q = self.search_query.trim().to_lowercase();
                            let mut filtered: Vec<&mut TargetItem> = self
                                .targets
                                .iter_mut()
                                .filter(|item| item.category == "registry")
                                .filter(|item| {
                                    q.is_empty()
                                        || item.name_prefix.to_lowercase().contains(&q)
                                        || item.name_suffix.to_lowercase().contains(&q)
                                        || item.tagline.to_lowercase().contains(&q)
                                        || item.scope.to_lowercase().contains(&q)
                                })
                                .collect();

                            let actions = match self.view_mode {
                                ViewMode::Table => {
                                    table_view::render_table(ui, &mut filtered, self.dark_mode)
                                }
                                ViewMode::Grid => {
                                    grid_view::render_grid(ui, &mut filtered, self.dark_mode)
                                }
                            };
                            single_purge_id = actions.purge_id;
                            rule_details_id = actions.rule_id;

                            ui.add_space(24.0);
                        });
                }

                AppTab::Uninstaller => {
                    match uninstaller_view::render_uninstaller_view(
                        ui,
                        &mut self.uninstaller_state,
                        self.view_mode,
                        self.dark_mode,
                    ) {
                        UninstallerViewAction::RescanApps => {
                            trigger_apps_scan = true;
                        }
                        UninstallerViewAction::OpenUninstallModal(app) => {
                            let remnants = scan_app_remnants(&app);
                            self.uninstaller_state.modal_remnants = remnants;
                            self.uninstaller_state.modal_options = UninstallOptions::default();
                            self.uninstaller_state.modal_app = Some(*app);
                        }
                        UninstallerViewAction::None => {}
                    }
                }

                AppTab::Tools => {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            if tools_view::render_tools_view(
                                ui,
                                &mut self.utilities,
                                self.view_mode,
                                self.dark_mode,
                            ) == tools_view::ToolsAction::OpenUninstaller
                            {
                                self.active_tab = AppTab::Uninstaller;
                                if self.uninstaller_state.apps.is_empty() {
                                    trigger_apps_scan = true;
                                }
                            }
                        });
                }

                AppTab::Bloatware => {
                    match bloat_view::render_bloat_view(
                        ui,
                        &mut self.bloat_state,
                        self.view_mode,
                        self.dark_mode,
                    ) {
                        bloat_view::BloatViewAction::Rescan => trigger_bloat_scan = true,
                        bloat_view::BloatViewAction::RequestConsent => {
                            self.bloat_state.consent_requested = true;
                            self.bloat_state.consent_ack = false;
                        }
                        bloat_view::BloatViewAction::None => {}
                    }
                }

                AppTab::AuditLogs => {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            logs_view::render_logs_view(
                                ui,
                                &self.metrics.status_text,
                                &self.audit_log,
                                self.dark_mode,
                            );
                        });
                }
            });

        // Update available notification toast
        if let Some(update) = self.pending_update.clone() {
            let dark = self.dark_mode;
            let mut dismiss = false;
            let mut update_now = false;

            egui::Area::new(egui::Id::new("update_toast"))
                .order(egui::Order::Foreground)
                .anchor(
                    egui::Align2::RIGHT_TOP,
                    egui::vec2(-16.0, theme::HEADER_H + 12.0),
                )
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(t.bg)
                        .stroke(egui::Stroke::new(1.0_f32, t.accent))
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            ui.set_width(360.0);
                            ui.spacing_mut().item_spacing.y = 6.0;

                            widgets::eyebrow(ui, "UPDATE", "NEW VERSION AVAILABLE", dark);
                            ui.label(
                                egui::RichText::new(format!(
                                    "Version {} is ready to install.",
                                    update.version()
                                ))
                                .font(egui::FontId::new(13.0, theme::display()))
                                .color(t.ink_strong),
                            );

                            if self.updating {
                                widgets::hud(ui, "DOWNLOADING & INSTALLING UPDATE...", t.warn);
                            } else {
                                ui.horizontal(|ui| {
                                    if widgets::button(
                                        ui,
                                        "DISMISS",
                                        widgets::ButtonKind::Secondary,
                                        0.0,
                                        dark,
                                    )
                                    .clicked()
                                    {
                                        dismiss = true;
                                    }
                                    if widgets::button(
                                        ui,
                                        "UPDATE NOW",
                                        widgets::ButtonKind::Primary,
                                        0.0,
                                        dark,
                                    )
                                    .clicked()
                                    {
                                        update_now = true;
                                    }
                                });
                            }
                        });
                });

            if dismiss {
                self.pending_update = None;
            }
            if update_now && !self.updating {
                self.updating = true;
                let tx = self.tx.clone();
                thread::spawn(move || {
                    let result = updater::perform_update();
                    let (updated, message) = match result {
                        Ok(true) => (true, "update installed".to_string()),
                        Ok(false) => (false, "already up to date".to_string()),
                        Err(e) => (false, e),
                    };
                    let _ = tx.send(WorkerResult::UpdateFinished { updated, message });
                });
            }
        }

        // Process collected actions outside render passes
        if trigger_scan {
            self.run_scan();
        }
        if trigger_apps_scan {
            self.run_apps_scan();
        }
        if trigger_bloat_scan {
            self.run_bloat_scan();
        }
        if trigger_purge {
            let sel_cat = self.selected_category.clone();
            let checked_ids: Vec<String> = self
                .targets
                .iter()
                .filter(|t| t.checked && (sel_cat == "all" || t.category == sel_cat))
                .map(|t| t.id.to_string())
                .collect();
            if !checked_ids.is_empty() {
                self.pending_purge = Some(checked_ids);
            }
        }
        if let Some(target_id) = single_purge_id {
            self.pending_purge = Some(vec![target_id]);
        }
        if let Some(target_id) = rule_details_id {
            self.rule_details_id = Some(target_id);
        }

        // Complete Application Uninstall Modal
        if let Some((app, opts)) = uninstaller_view::render_uninstall_modal(
            ctx,
            &mut self.uninstaller_state,
            self.secure_wipe,
            self.dark_mode,
        ) {
            self.run_app_uninstall(app, opts);
        }

        // Bloatware removal consent modal
        if let Some(true) =
            bloat_view::render_bloat_consent_modal(ctx, &mut self.bloat_state, self.dark_mode)
        {
            let create_restore = self.bloat_state.create_restore;
            self.run_bloat_removal(create_restore);
        }

        // Architectural Rule Specification Modal
        if let Some(target_id) = self.rule_details_id.clone() {
            let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));

            let screen_rect = ctx.screen_rect();
            egui::Area::new(egui::Id::new("rule_modal_scrim"))
                .order(egui::Order::Middle)
                .fixed_pos(screen_rect.min)
                .show(ctx, |ui| {
                    let (rect, resp) =
                        ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_black_alpha(if self.dark_mode { 165 } else { 95 }),
                    );
                    if resp.clicked() {
                        close = true;
                    }
                });

            egui::Area::new(egui::Id::new("rule_modal_card"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    let modal_w = 460.0;
                    egui::Frame::new()
                        .fill(t.bg)
                        .stroke(egui::Stroke::new(1.0_f32, t.line_strong))
                        .corner_radius(egui::CornerRadius::ZERO)
                        .show(ui, |ui| {
                            ui.set_width(modal_w);

                            if let Some(target) =
                                self.targets.iter().find(|target| target.id == target_id)
                            {
                                // 40px Top Strip Header (matching Grid Cards)
                                let (strip_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(modal_w, 40.0),
                                    egui::Sense::hover(),
                                );
                                ui.allocate_new_ui(
                                    egui::UiBuilder::new().max_rect(strip_rect),
                                    |ui| {
                                        ui.horizontal_centered(|ui| {
                                            ui.add_space(16.0);
                                            widgets::tag(
                                                ui,
                                                target.number,
                                                target.color,
                                                egui::Color32::WHITE,
                                            );
                                            ui.add_space(10.0);
                                            widgets::hud(
                                                ui,
                                                "TARGET RULE SPECIFICATION",
                                                t.muted,
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.add_space(12.0);
                                                    if widgets::nav_link(
                                                        ui,
                                                        "CLOSE ×",
                                                        40.0,
                                                        false,
                                                        self.dark_mode,
                                                    )
                                                    .clicked()
                                                    {
                                                        close = true;
                                                    }
                                                },
                                            );
                                        });
                                    },
                                );
                                ui.painter().hline(
                                    strip_rect.x_range(),
                                    strip_rect.bottom(),
                                    t.hairline(),
                                );

                                // Body Content
                                egui::Frame::new()
                                    .inner_margin(egui::Margin::same(20))
                                    .show(ui, |ui| {
                                        ui.set_width(modal_w - 40.0);

                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(target.name_prefix)
                                                    .font(egui::FontId::new(
                                                        18.0,
                                                        theme::display(),
                                                    ))
                                                    .color(t.ink_strong),
                                            );
                                            ui.label(
                                                egui::RichText::new(target.name_suffix)
                                                    .font(egui::FontId::new(
                                                        18.0,
                                                        theme::display(),
                                                    ))
                                                    .color(target.color),
                                            );
                                        });
                                        ui.add_space(2.0);
                                        ui.label(
                                            egui::RichText::new(target.tagline)
                                                .font(egui::FontId::new(
                                                    12.5,
                                                    egui::FontFamily::Proportional,
                                                ))
                                                .color(t.muted),
                                        );

                                        ui.add_space(16.0);

                                        // Sunken Specification Box
                                        egui::Frame::new()
                                            .fill(t.sunken)
                                            .stroke(t.hairline())
                                            .corner_radius(egui::CornerRadius::ZERO)
                                            .inner_margin(egui::Margin::same(14))
                                            .show(ui, |ui| {
                                                ui.set_width(modal_w - 68.0);
                                                ui.spacing_mut().item_spacing.y = 8.0;

                                                ui.horizontal(|ui| {
                                                    widgets::hud(ui, "RULE ID", t.faint);
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                egui::RichText::new(target.id)
                                                                    .font(egui::FontId::new(
                                                                        11.5,
                                                                        theme::mono_bold(),
                                                                    ))
                                                                    .color(t.ink_strong),
                                                            );
                                                        },
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    widgets::hud(ui, "CATEGORY", t.faint);
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let cat_label = match target.category {
                                                                "system" => "SYSTEM",
                                                                "browsers" => "BROWSERS",
                                                                "apps" => "APPS",
                                                                "dev" => "DEV & AI",
                                                                "graphics" => "GRAPHICS",
                                                                "privacy" => "PRIVACY",
                                                                "registry" => "REGISTRY",
                                                                other => other,
                                                            };
                                                            ui.label(
                                                                egui::RichText::new(cat_label)
                                                                    .font(egui::FontId::new(
                                                                        11.0,
                                                                        theme::mono_bold(),
                                                                    ))
                                                                    .color(target.color),
                                                            );
                                                        },
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    widgets::hud(ui, "CONFIGURED SCOPE", t.faint);
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                egui::RichText::new(target.scope)
                                                                    .font(egui::FontId::new(
                                                                        11.0,
                                                                        theme::mono(),
                                                                    ))
                                                                    .color(t.muted),
                                                            );
                                                        },
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    widgets::hud(ui, "DETECTED FOOTPRINT", t.faint);
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let meta = format!(
                                                                "{} · {}",
                                                                target.count, target.size
                                                            );
                                                            ui.label(
                                                                egui::RichText::new(meta)
                                                                    .font(egui::FontId::new(
                                                                        11.5,
                                                                        theme::mono_bold(),
                                                                    ))
                                                                    .color(t.accent_ink),
                                                            );
                                                        },
                                                    );
                                                });
                                            });

                                        ui.add_space(14.0);

                                        let safety_note = match target.status {
                                            TargetStatus::Guarded => {
                                                "GUARDED: Registry entries are strictly verified against disk paths before removal."
                                            }
                                            _ if target.id.starts_with("rule_") => {
                                                "OPT-IN RULE: Loaded from the embedded safe-rule catalog. Root folders are preserved."
                                            }
                                            _ => {
                                                "VERIFIED TARGET: Active OS and application handles are automatically bypassed."
                                            }
                                        };
                                        ui.label(
                                            egui::RichText::new(safety_note)
                                                .font(egui::FontId::new(11.0, theme::mono()))
                                                .color(t.faint),
                                        );

                                        ui.add_space(18.0);

                                        if widgets::button(
                                            ui,
                                            "CLOSE SPECIFICATION",
                                            widgets::ButtonKind::Secondary,
                                            modal_w - 40.0,
                                            self.dark_mode,
                                        )
                                        .clicked()
                                        {
                                            close = true;
                                        }
                                    });
                            }
                        });
                });

            if close {
                self.rule_details_id = None;
            }
        }

        // Architectural Purge Confirmation Modal
        if let Some(targets) = self.pending_purge.clone() {
            let mut proceed = false;
            let mut cancel = ctx.input(|i| i.key_pressed(egui::Key::Escape));

            let screen_rect = ctx.screen_rect();
            egui::Area::new(egui::Id::new("purge_modal_scrim"))
                .order(egui::Order::Middle)
                .fixed_pos(screen_rect.min)
                .show(ctx, |ui| {
                    let (rect, resp) =
                        ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_black_alpha(if self.dark_mode { 165 } else { 95 }),
                    );
                    if resp.clicked() {
                        cancel = true;
                    }
                });

            let single_target = if targets.len() == 1 {
                self.targets.iter().find(|t| t.id == targets[0])
            } else {
                None
            };
            let selected_bytes: u64 = self
                .targets
                .iter()
                .filter(|t| targets.iter().any(|id| id == t.id) && t.category != "registry")
                .map(|t| t.raw_bytes)
                .sum();

            egui::Area::new(egui::Id::new("purge_modal_card"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    let modal_w = 460.0;
                    egui::Frame::new()
                        .fill(t.bg)
                        .stroke(egui::Stroke::new(1.0_f32, t.line_strong))
                        .corner_radius(egui::CornerRadius::ZERO)
                        .show(ui, |ui| {
                            ui.set_width(modal_w);

                            // 40px Top Strip Header (matching Grid Cards)
                            let (strip_rect, _) = ui.allocate_exact_size(
                                egui::vec2(modal_w, 40.0),
                                egui::Sense::hover(),
                            );
                            let tag_color = single_target.map(|st| st.color).unwrap_or(t.bad);
                            let tag_label = single_target
                                .map(|st| st.number.to_string())
                                .unwrap_or_else(|| format!("{:02}", targets.len()));

                            ui.allocate_new_ui(
                                egui::UiBuilder::new().max_rect(strip_rect),
                                |ui| {
                                    ui.horizontal_centered(|ui| {
                                        ui.add_space(16.0);
                                        widgets::tag(
                                            ui,
                                            &tag_label,
                                            tag_color,
                                            egui::Color32::WHITE,
                                        );
                                        ui.add_space(10.0);
                                        widgets::hud(ui, "CONFIRM PURGE EXECUTION", t.muted);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(12.0);
                                                if widgets::nav_link(
                                                    ui,
                                                    "ESC · CANCEL ×",
                                                    40.0,
                                                    false,
                                                    self.dark_mode,
                                                )
                                                .clicked()
                                                {
                                                    cancel = true;
                                                }
                                            },
                                        );
                                    });
                                },
                            );
                            ui.painter().hline(
                                strip_rect.x_range(),
                                strip_rect.bottom(),
                                t.hairline(),
                            );

                            // Body Content
                            egui::Frame::new()
                                .inner_margin(egui::Margin::same(20))
                                .show(ui, |ui| {
                                    ui.set_width(modal_w - 40.0);

                                    // Two-tone Display Title
                                    ui.horizontal(|ui| {
                                        if let Some(st) = single_target {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Purge {}",
                                                    st.name_prefix
                                                ))
                                                .font(egui::FontId::new(18.0, theme::display()))
                                                .color(t.ink_strong),
                                            );
                                            ui.label(
                                                egui::RichText::new(st.name_suffix)
                                                    .font(egui::FontId::new(18.0, theme::display()))
                                                    .color(st.color),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("Purge")
                                                    .font(egui::FontId::new(18.0, theme::display()))
                                                    .color(t.ink_strong),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} Selected Targets",
                                                    targets.len()
                                                ))
                                                .font(egui::FontId::new(18.0, theme::display()))
                                                .color(t.bad),
                                            );
                                        }
                                    });

                                    ui.add_space(2.0);
                                    let subtitle = single_target
                                        .map(|st| st.tagline)
                                        .unwrap_or("Permanent removal of verified cache, log, and privacy artifacts.");
                                    ui.label(
                                        egui::RichText::new(subtitle)
                                            .font(egui::FontId::new(
                                                12.5,
                                                egui::FontFamily::Proportional,
                                            ))
                                            .color(t.muted),
                                    );

                                    ui.add_space(16.0);

                                    // Sunken Telemetry Summary Box
                                    egui::Frame::new()
                                        .fill(t.sunken)
                                        .stroke(t.hairline())
                                        .corner_radius(egui::CornerRadius::ZERO)
                                        .inner_margin(egui::Margin::same(14))
                                        .show(ui, |ui| {
                                            ui.set_width(modal_w - 68.0);
                                            ui.spacing_mut().item_spacing.y = 8.0;

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "TARGET SCOPE", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let scope_str = single_target
                                                            .map(|st| st.scope.to_string())
                                                            .unwrap_or_else(|| {
                                                                format!(
                                                                    "{} MODULES QUEUED",
                                                                    targets.len()
                                                                )
                                                            });
                                                        ui.label(
                                                            egui::RichText::new(scope_str)
                                                                .font(egui::FontId::new(
                                                                    11.0,
                                                                    theme::mono(),
                                                                ))
                                                                .color(t.ink_strong),
                                                        );
                                                    },
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "ESTIMATED RECLAIM", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let est_str = if let Some(st) = single_target
                                                        {
                                                            st.size.clone()
                                                        } else {
                                                            format_bytes(selected_bytes)
                                                        };
                                                        ui.label(
                                                            egui::RichText::new(est_str)
                                                                .font(egui::FontId::new(
                                                                    11.5,
                                                                    theme::mono_bold(),
                                                                ))
                                                                .color(t.good),
                                                        );
                                                    },
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "SANITIZATION MODE", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let (mode_str, mode_col) =
                                                            if self.secure_wipe {
                                                                ("DOD 0x00 ZERO-FILL", t.warn)
                                                            } else {
                                                                (
                                                                    "STANDARD + POSIX UNLINK",
                                                                    t.accent_ink,
                                                                )
                                                            };
                                                        ui.label(
                                                            egui::RichText::new(mode_str)
                                                                .font(egui::FontId::new(
                                                                    11.0,
                                                                    theme::mono_bold(),
                                                                ))
                                                                .color(mode_col),
                                                        );
                                                    },
                                                );
                                            });
                                        });

                                    ui.add_space(14.0);

                                    ui.label(
                                        egui::RichText::new(
                                            "Open application handles are safely skipped. Unlinked files cannot be recovered.",
                                        )
                                        .font(egui::FontId::new(11.0, theme::mono()))
                                        .color(t.faint),
                                    );
                                    if self.secure_wipe {
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "NOTE: Zero-fill overwrite is best-effort on wear-leveled SSDs and cloud storage.",
                                            )
                                            .font(egui::FontId::new(10.5, theme::mono()))
                                            .color(t.warn),
                                        );
                                    }

                                    ui.add_space(18.0);

                                    // Architectural Action Row
                                    ui.horizontal(|ui| {
                                        if widgets::button(
                                            ui,
                                            "CANCEL",
                                            widgets::ButtonKind::Secondary,
                                            135.0,
                                            self.dark_mode,
                                        )
                                        .clicked()
                                        {
                                            cancel = true;
                                        }
                                        ui.add_space(8.0);
                                        if widgets::button(
                                            ui,
                                            "EXECUTE PURGE",
                                            widgets::ButtonKind::Primary,
                                            modal_w - 40.0 - 135.0 - 8.0,
                                            self.dark_mode,
                                        )
                                        .clicked()
                                        {
                                            proceed = true;
                                        }
                                    });
                                });
                        });
                });

            if cancel {
                self.pending_purge = None;
            } else if proceed {
                self.pending_purge = None;
                self.run_purge(targets);
            }
        }
    }
}

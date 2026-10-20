use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

use egui::{CentralPanel, ScrollArea, SidePanel, TopBottomPanel};

use super::banner::AuditMetrics;
use super::dialogs::Dialog;
use super::header::{self, ViewMode};
use super::model::{self, TargetItem};
use super::sidebar::{self, AppTab, SidebarCounts};
use super::theme::{self, Tokens};
use super::uninstaller_view::{self, UninstallerUiState, UninstallerViewAction};
use super::views::ViewActions;
use super::{bloat_view, logs_view, widgets};

use crate::uninstaller::{UninstallOptions, scan_app_remnants};
use crate::updater::{self, UpdateInfo};

// Re-export worker communication types for seamless subsystem interoperability and unit testing.
#[allow(unused_imports)]
pub(crate) use super::conductor::{TargetReport, WorkerResult};

/// Primary presentation shell for OpenCleaner desktop client.
pub struct SystemCleanerApp {
    pub(crate) dark_mode: bool,
    pub(crate) view_mode: ViewMode,
    pub(crate) active_tab: AppTab,
    pub(crate) selected_category: String,
    pub(crate) search_query: String,
    pub(crate) secure_wipe: bool,
    pub(crate) metrics: AuditMetrics,
    pub(crate) targets: Vec<TargetItem>,
    pub(crate) uninstaller_state: UninstallerUiState,
    pub(crate) bloat_state: bloat_view::BloatUiState,
    pub(crate) audit_log: Vec<logs_view::AuditLogEntry>,
    pub(crate) pending_update: Option<UpdateInfo>,
    pub(crate) update_error: Option<String>,
    pub(crate) updating: bool,
    pub(crate) tx: Sender<WorkerResult>,
    pub(crate) rx: Receiver<WorkerResult>,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) worker_gen: u64,
    pub(crate) is_purging: bool,
}

impl Default for SystemCleanerApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            dark_mode: false,
            view_mode: ViewMode::Table,
            active_tab: AppTab::Cleaner,
            selected_category: "all".to_string(),
            search_query: String::new(),
            secure_wipe: false,
            metrics: AuditMetrics::default(),
            targets: model::default_targets(),
            uninstaller_state: UninstallerUiState::default(),
            bloat_state: bloat_view::BloatUiState::default(),
            audit_log: Vec::new(),
            pending_update: None,
            update_error: None,
            updating: false,
            tx,
            rx,
            dialog: None,
            worker_gen: 0,
            is_purging: false,
        }
    }
}

impl SystemCleanerApp {
    /// Initializes application state, custom typography, and triggers background startup scans.
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
}

#[allow(clippy::needless_ifs, clippy::collapsible_match)]
impl eframe::App for SystemCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain and route all telemetry messages from background worker threads
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

        // 1. Top 48px Frameless Header & Drag Handle
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

        // 2. Bottom 34px Status HUD Footer
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
                        let sys_info = format!("OPEN-CLEANER v{} · WINDOWS X64", env!("CARGO_PKG_VERSION"));
                        widgets::hud(ui, &sys_info, t.faint);
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

        // 3. Left Navigation Rail (240px)
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
            bloat_count: self.bloat_state.items.len(),
            logs_count: 4 + self.audit_log.len(),
        };

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

        // 4. Central Workspace Area
        CentralPanel::default()
            .frame(egui::Frame::new().fill(t.bg))
            .show(ctx, |ui| match self.active_tab {
                AppTab::Cleaner => {
                    let actions: ViewActions = self.render_cleaner_tab(ui);
                    trigger_scan |= actions.trigger_scan;
                    trigger_purge |= actions.trigger_purge;
                    if actions.single_purge_id.is_some() {
                        single_purge_id = actions.single_purge_id;
                    }
                    if actions.rule_details_id.is_some() {
                        rule_details_id = actions.rule_details_id;
                    }
                }
                AppTab::Registry => {
                    let actions: ViewActions = self.render_registry_tab(ui);
                    trigger_scan |= actions.trigger_scan;
                    trigger_purge |= actions.trigger_purge;
                    if actions.single_purge_id.is_some() {
                        single_purge_id = actions.single_purge_id;
                    }
                    if actions.rule_details_id.is_some() {
                        rule_details_id = actions.rule_details_id;
                    }
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

        // 5. Route collected workspace actions
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
                self.dialog = Some(Dialog::PurgeConfirmation(checked_ids));
            }
        }
        if let Some(target_id) = single_purge_id {
            self.dialog = Some(Dialog::PurgeConfirmation(vec![target_id]));
        }
        if let Some(target_id) = rule_details_id {
            self.dialog = Some(Dialog::RuleSpecification(target_id));
        }

        // 6. Unified Modal Dialog Layer
        self.render_dialogs(ctx);
    }
}

//! Primary workspace views: Cleaner and Registry target tables and card grids.

use egui::{ScrollArea, Ui};

use crate::ui::app::SystemCleanerApp;
use crate::ui::header::ViewMode;
use crate::ui::model::TargetItem;
use crate::ui::theme::Tokens;
use crate::ui::widgets::FilterOption;
use crate::ui::{banner, grid_view, table_view, toolbar, widgets};

/// Actions emitted during target view rendering that request background operations or modal open.
#[derive(Default)]
pub(crate) struct ViewActions {
    pub(crate) trigger_scan: bool,
    pub(crate) trigger_purge: bool,
    pub(crate) single_purge_id: Option<String>,
    pub(crate) rule_details_id: Option<String>,
}

impl SystemCleanerApp {
    /// Renders the multi-category System Cleaner workspace tab.
    pub(crate) fn render_cleaner_tab(&mut self, ui: &mut Ui) -> ViewActions {
        let mut actions = ViewActions::default();
        let t = Tokens::get(self.dark_mode);

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
            toolbar::ToolbarAction::Analyze => actions.trigger_scan = true,
            toolbar::ToolbarAction::Purge => actions.trigger_purge = true,
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        let cnt_str = format!("{} TARGETS", visible_count);
                        widgets::hud(ui, &cnt_str, t.faint);
                    });
                });

                ui.add_space(8.0);

                let table_or_grid = match self.view_mode {
                    ViewMode::Table => table_view::render_table(ui, &mut filtered, self.dark_mode),
                    ViewMode::Grid => grid_view::render_grid(ui, &mut filtered, self.dark_mode),
                };
                actions.single_purge_id = table_or_grid.purge_id;
                actions.rule_details_id = table_or_grid.rule_id;

                ui.add_space(24.0);
            });

        actions
    }

    /// Renders the specialized Registry Integrity workspace tab.
    pub(crate) fn render_registry_tab(&mut self, ui: &mut Ui) -> ViewActions {
        let mut actions = ViewActions::default();
        let t = Tokens::get(self.dark_mode);

        let registry_count = self
            .targets
            .iter()
            .filter(|t| t.category == "registry")
            .count();

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
            toolbar::ToolbarAction::Analyze => actions.trigger_scan = true,
            toolbar::ToolbarAction::Purge => actions.trigger_purge = true,
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        let cnt_str = format!("{} KEYS AUDITED", registry_count);
                        widgets::hud(ui, &cnt_str, t.faint);
                    });
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

                let table_or_grid = match self.view_mode {
                    ViewMode::Table => table_view::render_table(ui, &mut filtered, self.dark_mode),
                    ViewMode::Grid => grid_view::render_grid(ui, &mut filtered, self.dark_mode),
                };
                actions.single_purge_id = table_or_grid.purge_id;
                actions.rule_details_id = table_or_grid.rule_id;

                ui.add_space(24.0);
            });

        actions
    }
}


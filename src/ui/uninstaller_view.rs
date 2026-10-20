use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, StrokeKind, Ui, vec2};

use crate::core::ui::format_bytes;
use crate::uninstaller::{
    AppRemnant, AppSource, InstalledApp, UninstallOptions, UninstallProgress, UninstallReport,
};

use super::header::ViewMode;
use super::theme::{self, Tokens};
use super::widgets::{self, FilterOption};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppSortOrder {
    #[default]
    SizeDesc,
    NameAsc,
    DateDesc,
}

impl AppSortOrder {
    pub fn next(self) -> Self {
        match self {
            AppSortOrder::SizeDesc => AppSortOrder::NameAsc,
            AppSortOrder::NameAsc => AppSortOrder::DateDesc,
            AppSortOrder::DateDesc => AppSortOrder::SizeDesc,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AppSortOrder::SizeDesc => "SORT: SIZE ↓",
            AppSortOrder::NameAsc => "SORT: NAME A-Z",
            AppSortOrder::DateDesc => "SORT: DATE ↓",
        }
    }
}

pub struct UninstallerUiState {
    pub apps: Vec<InstalledApp>,
    pub is_scanning: bool,
    pub is_uninstalling: bool,
    pub selected_filter: String,
    pub search_query: String,
    pub sort_order: AppSortOrder,
    pub modal_app: Option<InstalledApp>,
    pub modal_remnants: Vec<AppRemnant>,
    pub modal_options: UninstallOptions,
    pub active_progress: Option<UninstallProgress>,
    pub progress_log: Vec<String>,
    pub show_completion_modal: bool,
    pub last_report: Option<UninstallReport>,
    pub icon_cache: std::collections::HashMap<String, Option<egui::TextureHandle>>,
}

impl Default for UninstallerUiState {
    fn default() -> Self {
        Self {
            apps: Vec::new(),
            is_scanning: false,
            is_uninstalling: false,
            selected_filter: "all".to_string(),
            search_query: String::new(),
            sort_order: AppSortOrder::SizeDesc,
            modal_app: None,
            modal_remnants: Vec::new(),
            modal_options: UninstallOptions::default(),
            active_progress: None,
            progress_log: Vec::new(),
            show_completion_modal: false,
            last_report: None,
            icon_cache: std::collections::HashMap::new(),
        }
    }
}

pub enum UninstallerViewAction {
    None,
    RescanApps,
    OpenUninstallModal(Box<InstalledApp>),
}

pub fn source_color(source: AppSource) -> Color32 {
    match source {
        AppSource::Win32Machine64 => Color32::from_rgb(0x7b, 0x1f, 0xa2), // Purple
        AppSource::Win32Machine32 => Color32::from_rgb(0x5c, 0x6b, 0xc0), // Indigo
        AppSource::Win32User => Color32::from_rgb(0x00, 0x89, 0x7b),      // Teal
        AppSource::WindowsStoreAppX => Color32::from_rgb(0x1e, 0x88, 0xe5), // Blue
    }
}

/// Renders either an extracted Windows application icon or a Swiss monogram tile.
pub fn draw_app_icon_or_monogram(
    ui: &mut Ui,
    icon_cache: &mut std::collections::HashMap<String, Option<egui::TextureHandle>>,
    app: &InstalledApp,
    size: f32,
    dark_mode: bool,
) {
    let t = Tokens::get(dark_mode);
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());

    let maybe_handle = icon_cache.entry(app.id.clone()).or_insert_with(|| {
        app.icon_image.as_ref().map(|img_arc| {
            ui.ctx().load_texture(
                format!("app_icon_{}", app.id),
                (**img_arc).clone(),
                egui::TextureOptions::LINEAR,
            )
        })
    });

    if let Some(handle) = maybe_handle {
        ui.painter().rect_filled(rect, CornerRadius::same(4), t.bg);
        ui.painter().image(
            handle.id(),
            rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(4),
            t.hairline(),
            StrokeKind::Inside,
        );
    } else {
        let monogram = crate::uninstaller::icons::app_monogram(&app.display_name);
        let brand_color = source_color(app.source);
        let bg_color = if dark_mode {
            Color32::from_rgb(0x1e, 0x20, 0x28)
        } else {
            Color32::from_rgb(0xe5, 0xe4, 0xdf)
        };

        ui.painter().rect(
            rect,
            CornerRadius::same(4),
            bg_color,
            egui::Stroke::new(1.0_f32, brand_color),
            StrokeKind::Inside,
        );

        let font_size = (size * 0.40).clamp(9.5, 14.0);
        let g = widgets::caps(
            ui,
            &monogram,
            font_size,
            egui::FontFamily::Proportional,
            0.5,
            brand_color,
        );
        widgets::paint_centred(ui, rect, g, brand_color);
    }
}

/// Draws a crisp Swiss-grid horizontal progress bar with 1px hairline border.
pub fn draw_progress_bar(
    ui: &mut Ui,
    width: f32,
    height: f32,
    fraction: f32,
    fill_color: Color32,
    dark_mode: bool,
) {
    let t = Tokens::get(dark_mode);
    let clamped = fraction.clamp(0.0, 1.0);
    let (track_rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());

    ui.painter()
        .rect_filled(track_rect, CornerRadius::ZERO, t.bg);
    ui.painter().rect_stroke(
        track_rect,
        CornerRadius::ZERO,
        t.hairline(),
        StrokeKind::Inside,
    );

    let inner = track_rect.shrink(1.0);
    let fill_w = inner.width() * clamped;
    if fill_w > 0.5 {
        let fill_rect = Rect::from_min_size(inner.min, vec2(fill_w, inner.height()));
        ui.painter()
            .rect_filled(fill_rect, CornerRadius::ZERO, fill_color);
    }
}

pub fn render_uninstaller_view(
    ui: &mut Ui,
    state: &mut UninstallerUiState,
    view_mode: ViewMode,
    dark_mode: bool,
) -> UninstallerViewAction {
    let t = Tokens::get(dark_mode);
    let mut action = UninstallerViewAction::None;

    let total_count = state.apps.len();
    let win32_count = state
        .apps
        .iter()
        .filter(|a| !a.source.is_store_app())
        .count();
    let store_count = state
        .apps
        .iter()
        .filter(|a| a.source.is_store_app())
        .count();
    let large_count = state
        .apps
        .iter()
        .filter(|a| a.estimated_size_bytes >= 100 * 1024 * 1024)
        .count();
    let total_footprint: u64 = state.apps.iter().map(|a| a.estimated_size_bytes).sum();

    // 1. Filter & Search Toolbar (54px)
    let bar_h = 54.0;
    let full_rect = ui.available_rect_before_wrap();
    let bar_rect = Rect::from_min_size(full_rect.min, vec2(full_rect.width(), bar_h));

    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), bar_h),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.add_space(16.0);
            ui.spacing_mut().item_spacing.x = 6.0;

            let filter_options = [
                FilterOption {
                    key: "all",
                    label: "ALL APPS",
                    count: total_count,
                },
                FilterOption {
                    key: "win32",
                    label: "DESKTOP (WIN32)",
                    count: win32_count,
                },
                FilterOption {
                    key: "store",
                    label: "MS STORE (UWP)",
                    count: store_count,
                },
                FilterOption {
                    key: "large",
                    label: "LARGE (>100 MB)",
                    count: large_count,
                },
            ];

            widgets::filter_cells(ui, &mut state.selected_filter, &filter_options, dark_mode);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(16.0);
                ui.spacing_mut().item_spacing.x = 8.0;

                if widgets::button(
                    ui,
                    "RESCAN APPS",
                    widgets::ButtonKind::Secondary,
                    118.0,
                    dark_mode,
                )
                .clicked()
                {
                    action = UninstallerViewAction::RescanApps;
                }

                if widgets::button(
                    ui,
                    state.sort_order.label(),
                    widgets::ButtonKind::Secondary,
                    118.0,
                    dark_mode,
                )
                .clicked()
                {
                    state.sort_order = state.sort_order.next();
                }

                let text_edit = egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search applications")
                    .desired_width(160.0)
                    .margin(egui::Margin::symmetric(10, 8));
                ui.add(text_edit);
            });
        },
    );

    ui.painter()
        .hline(bar_rect.x_range(), bar_rect.bottom() - 0.5, t.hairline());

    // 2. Metrics Telemetry Strip (with embedded Progress Bar)
    render_uninstaller_metrics_strip(
        ui,
        total_count,
        win32_count,
        store_count,
        total_footprint,
        state.is_scanning,
        state.is_uninstalling,
        state.active_progress.as_ref(),
        state.last_report.as_ref(),
        dark_mode,
    );

    // 3. Filter and Sort Application List
    let q = state.search_query.trim().to_lowercase();
    let sel = state.selected_filter.as_str();

    let mut filtered: Vec<InstalledApp> = state
        .apps
        .iter()
        .filter(|app| {
            let matches_cat = match sel {
                "win32" => !app.source.is_store_app(),
                "store" => app.source.is_store_app(),
                "large" => app.estimated_size_bytes >= 100 * 1024 * 1024,
                _ => true,
            };
            let matches_q = q.is_empty()
                || app.display_name.to_lowercase().contains(&q)
                || app.publisher.to_lowercase().contains(&q)
                || app.display_version.to_lowercase().contains(&q)
                || app.installer_kind.label().to_lowercase().contains(&q);
            matches_cat && matches_q
        })
        .cloned()
        .collect();

    match state.sort_order {
        AppSortOrder::SizeDesc => {
            filtered.sort_by(|a, b| {
                b.estimated_size_bytes
                    .cmp(&a.estimated_size_bytes)
                    .then_with(|| {
                        a.display_name
                            .to_lowercase()
                            .cmp(&b.display_name.to_lowercase())
                    })
            });
        }
        AppSortOrder::NameAsc => {
            filtered.sort_by(|a, b| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            });
        }
        AppSortOrder::DateDesc => {
            filtered.sort_by(|a, b| {
                b.install_date.cmp(&a.install_date).then_with(|| {
                    a.display_name
                        .to_lowercase()
                        .cmp(&b.display_name.to_lowercase())
                })
            });
        }
    }

    let active_app_id = state
        .active_progress
        .as_ref()
        .map(|p| p.app_id.clone())
        .filter(|_| state.is_uninstalling);
    let active_frac = state
        .active_progress
        .as_ref()
        .map(|p| p.fraction)
        .unwrap_or(0.0);

    // 4. Scrollable Apps Table / Grid
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.add_space(16.0);
                widgets::eyebrow(
                    ui,
                    "03",
                    "INSTALLED APPLICATIONS & COMPLETE REMNANT UNINSTALLER",
                    dark_mode,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(16.0);
                    let cnt_str = format!("{} APPLICATIONS", filtered.len());
                    widgets::hud(ui, &cnt_str, t.faint);
                });
            });

            ui.add_space(8.0);

            let picked = match view_mode {
                ViewMode::Table => render_apps_table(
                    ui,
                    &mut state.icon_cache,
                    &filtered,
                    active_app_id.as_deref(),
                    active_frac,
                    dark_mode,
                ),
                ViewMode::Grid => render_apps_grid(
                    ui,
                    &mut state.icon_cache,
                    &filtered,
                    active_app_id.as_deref(),
                    active_frac,
                    dark_mode,
                ),
            };
            if let Some(app) = picked {
                action = UninstallerViewAction::OpenUninstallModal(Box::new(app));
            }

            ui.add_space(24.0);
        });

    action
}

#[allow(clippy::too_many_arguments)]
fn render_uninstaller_metrics_strip(
    ui: &mut Ui,
    total_count: usize,
    win32_count: usize,
    store_count: usize,
    total_footprint: u64,
    is_scanning: bool,
    is_uninstalling: bool,
    active_progress: Option<&UninstallProgress>,
    last_report: Option<&UninstallReport>,
    dark_mode: bool,
) {
    let t = Tokens::get(dark_mode);
    let strip_h = 74.0;
    let full_rect = ui.available_rect_before_wrap();
    let strip_rect = Rect::from_min_size(full_rect.min, vec2(full_rect.width(), strip_h));

    ui.painter()
        .rect_filled(strip_rect, CornerRadius::ZERO, t.sunken);

    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), strip_h),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.add_space(14.0);
                widgets::hud(ui, "INSTALLED FOOTPRINT", t.faint);
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format_bytes(total_footprint))
                        .font(egui::FontId::new(20.0, theme::display()))
                        .color(t.ink_strong),
                );
            });

            ui.add_space(24.0);
            widgets::rule(ui, strip_h - 20.0, dark_mode);
            ui.add_space(20.0);

            ui.vertical(|ui| {
                ui.add_space(14.0);
                widgets::hud(ui, "DETECTED PACKAGES", t.faint);
                ui.add_space(4.0);
                let breakdown =
                    format!("{total_count} TOTAL  ·  {win32_count} WIN32  ·  {store_count} STORE");
                widgets::hud(ui, &breakdown, t.ink);
            });

            ui.add_space(24.0);
            widgets::rule(ui, strip_h - 20.0, dark_mode);
            ui.add_space(20.0);

            let remaining_w = (ui.available_width() - 24.0).max(200.0);
            ui.vertical(|ui| {
                ui.add_space(10.0);
                if is_uninstalling {
                    let (frac, step_str) = if let Some(prog) = active_progress {
                        (
                            prog.fraction,
                            format!(
                                "{:.0}%  ·  {} ({})",
                                prog.fraction * 100.0,
                                prog.phase.step_label(),
                                prog.app_name.to_uppercase()
                            ),
                        )
                    } else {
                        (
                            0.15,
                            "15%  ·  EXECUTING COMPLETE UNINSTALLATION...".to_string(),
                        )
                    };
                    widgets::hud(ui, &step_str, t.warn);
                    ui.add_space(5.0);
                    draw_progress_bar(ui, remaining_w, 8.0, frac, t.warn, dark_mode);
                    if let Some(prog) = active_progress {
                        ui.add_space(3.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&prog.detail)
                                    .font(egui::FontId::new(10.0, theme::mono()))
                                    .color(t.muted),
                            )
                            .truncate(),
                        );
                    }
                } else if is_scanning {
                    widgets::hud(
                        ui,
                        "SCANNING  ·  ENUMERATING HKLM / WOW6432NODE / HKCU / APPX REPOSITORIES...",
                        t.warn,
                    );
                    ui.add_space(6.0);
                    draw_progress_bar(ui, remaining_w, 8.0, 0.45, t.warn, dark_mode);
                } else if let Some(rep) = last_report {
                    let status_str = format!(
                        "100% COMPLETE  ·  REMOVED '{}'  ·  {} REMNANTS PURGED ({} FREED)",
                        rep.app_name.to_uppercase(),
                        rep.remnants_removed,
                        format_bytes(rep.bytes_reclaimed)
                    );
                    widgets::hud(ui, &status_str, t.good);
                    ui.add_space(6.0);
                    draw_progress_bar(ui, remaining_w, 8.0, 1.0, t.good, dark_mode);
                } else {
                    widgets::hud(ui, "SUBSYSTEM TELEMETRY", t.faint);
                    ui.add_space(4.0);
                    widgets::hud(
                        ui,
                        "READY FOR COMPLETE UNINSTALL (NATIVE UNINSTALLER + DEEP REMNANT SCAN)",
                        t.muted,
                    );
                }
            });
        },
    );

    ui.painter().hline(
        strip_rect.x_range(),
        strip_rect.bottom() - 0.5,
        t.hairline(),
    );
}

fn render_apps_table(
    ui: &mut Ui,
    icon_cache: &mut std::collections::HashMap<String, Option<egui::TextureHandle>>,
    apps: &[InstalledApp],
    active_app_id: Option<&str>,
    active_frac: f32,
    dark_mode: bool,
) -> Option<InstalledApp> {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;
    let mut selected_app = None;

    if full_w <= 100.0 || apps.is_empty() {
        return None;
    }

    let pad_left = 16.0;
    let col1_w = (full_w * 0.36).max(260.0);
    let col2_w = (full_w * 0.24).max(170.0);
    let col3_w = 135.0;
    let col4_w = 100.0;

    let x1 = pad_left;
    let x2 = x1 + col1_w;
    let x3 = x2 + col2_w;
    let x4 = x3 + col3_w;

    let header_h = 42.0;
    let row_h = 62.0;
    let total_h = header_h + (apps.len() as f32 * row_h);

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        let (table_rect, _) = ui.allocate_exact_size(vec2(full_w, total_h), Sense::hover());

        ui.painter().rect(
            table_rect,
            CornerRadius::ZERO,
            t.bg,
            t.hairline(),
            StrokeKind::Inside,
        );

        // Header Row
        let header_rect = Rect::from_min_size(table_rect.min, vec2(full_w, header_h));
        let hy = header_rect.center().y;
        ui.painter()
            .hline(table_rect.x_range(), header_rect.bottom(), t.hairline());

        for (x_off, title, color) in [
            (x1, "APPLICATION & PUBLISHER", t.ink_strong),
            (x2, "SOURCE & INSTALLER", t.faint),
            (x3, "INSTALLED", t.faint),
            (x4, "FOOTPRINT", t.faint),
        ] {
            let g = widgets::caps(
                ui,
                title,
                theme::HUD_SIZE,
                theme::mono(),
                theme::HUD_SPACING,
                color,
            );
            ui.painter().galley(
                egui::pos2(table_rect.left() + x_off, hy - g.size().y / 2.0),
                g,
                color,
            );
        }

        let col_act = widgets::caps(
            ui,
            "COMPLETE REMOVAL",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.faint,
        );
        ui.painter().galley(
            egui::pos2(
                table_rect.right() - 16.0 - col_act.size().x,
                hy - col_act.size().y / 2.0,
            ),
            col_act,
            t.faint,
        );

        let total_items = apps.len();
        for (idx, app) in apps.iter().enumerate() {
            let row_top = header_rect.bottom() + (idx as f32 * row_h);
            let row_rect =
                Rect::from_min_size(egui::pos2(table_rect.left(), row_top), vec2(full_w, row_h));
            let row_resp = ui.interact(row_rect, ui.id().with(("approw", &app.id)), Sense::hover());
            let is_this_uninstalling = active_app_id == Some(app.id.as_str());

            if row_resp.hovered() || is_this_uninstalling {
                ui.painter()
                    .rect_filled(row_rect, CornerRadius::ZERO, t.sunken);
            }

            if idx + 1 < total_items {
                ui.painter()
                    .hline(table_rect.x_range(), row_rect.bottom(), t.hairline());
            }

            let ry = row_rect.center().y;
            let badge_color = source_color(app.source);

            // Icon / Monogram square (26x26)
            let icon_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 30.0, ry), vec2(26.0, 26.0));
            let mut icon_ui = ui.new_child(egui::UiBuilder::new().max_rect(icon_rect));
            draw_app_icon_or_monogram(&mut icon_ui, icon_cache, app, 26.0, dark_mode);

            // Col 1: App Name + Version + Publisher (Inter Proportional)
            let name_x = table_rect.left() + 58.0;
            let name_w = (col1_w - 58.0 - 8.0).max(120.0);
            let name_rect = Rect::from_min_size(egui::pos2(name_x, ry - 18.0), vec2(name_w, 38.0));
            let mut name_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(name_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            name_ui.add(
                egui::Label::new(
                    RichText::new(&app.display_name)
                        .font(egui::FontId::new(14.0, egui::FontFamily::Proportional))
                        .color(t.ink_strong),
                )
                .truncate(),
            );
            let sub_meta = format!("{}  ·  v{}", app.publisher, app.display_version);
            name_ui.add(
                egui::Label::new(
                    RichText::new(sub_meta)
                        .font(egui::FontId::new(11.0, egui::FontFamily::Proportional))
                        .color(t.muted),
                )
                .truncate(),
            );

            // Col 2: Source Badge + Installer Kind
            let col2_rect = Rect::from_min_size(
                egui::pos2(table_rect.left() + x2, ry - 18.0),
                vec2(col2_w - 10.0, 36.0),
            );
            let mut col2_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(col2_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            widgets::badge(
                &mut col2_ui,
                app.source.badge_label(),
                badge_color,
                dark_mode,
            );
            col2_ui.add(
                egui::Label::new(
                    RichText::new(app.installer_kind.label())
                        .font(egui::FontId::new(10.0, egui::FontFamily::Proportional))
                        .color(t.faint),
                )
                .truncate(),
            );

            // Col 3: Install Date
            let date_rect = Rect::from_min_size(
                egui::pos2(table_rect.left() + x3, ry - 10.0),
                vec2(col3_w - 10.0, 20.0),
            );
            let mut date_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(date_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            date_ui.label(
                RichText::new(&app.install_date)
                    .font(egui::FontId::new(11.0, egui::FontFamily::Proportional))
                    .color(t.muted),
            );

            // Col 4: Footprint Size
            let size_rect = Rect::from_min_size(
                egui::pos2(table_rect.left() + x4, ry - 10.0),
                vec2(col4_w - 8.0, 20.0),
            );
            let mut size_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(size_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let size_str = if app.estimated_size_bytes > 0 {
                format_bytes(app.estimated_size_bytes)
            } else {
                "—".to_string()
            };
            size_ui.label(
                RichText::new(size_str)
                    .font(egui::FontId::new(12.5, theme::mono_bold()))
                    .color(t.ink_strong),
            );

            // Col 5: Complete Uninstall Button OR Active Row Progress Bar
            let btn_w = 136.0;
            let btn_h = 34.0;
            let btn_rect = Rect::from_min_size(
                egui::pos2(table_rect.right() - 16.0 - btn_w, ry - btn_h / 2.0),
                vec2(btn_w, btn_h),
            );
            let mut btn_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(btn_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            if is_this_uninstalling {
                let pct_str = format!("UNINSTALLING {:.0}%", active_frac * 100.0);
                widgets::hud(&mut btn_ui, &pct_str, t.warn);
                btn_ui.add_space(4.0);
                draw_progress_bar(&mut btn_ui, btn_w, 8.0, active_frac, t.warn, dark_mode);
            } else if widgets::button(
                &mut btn_ui,
                "UNINSTALL",
                widgets::ButtonKind::Primary,
                btn_w,
                dark_mode,
            )
            .clicked()
            {
                selected_app = Some(app.clone());
            }
        }
    });

    selected_app
}

fn render_apps_grid(
    ui: &mut Ui,
    icon_cache: &mut std::collections::HashMap<String, Option<egui::TextureHandle>>,
    apps: &[InstalledApp],
    active_app_id: Option<&str>,
    active_frac: f32,
    dark_mode: bool,
) -> Option<InstalledApp> {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;
    let mut selected_app = None;

    if full_w <= 100.0 || apps.is_empty() {
        return None;
    }

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        let cols = ((full_w / 320.0).floor() as usize).max(1);
        let card_w = (full_w - (cols as f32 - 1.0)) / cols as f32;
        let card_h = 240.0;

        let rows = apps.len().div_ceil(cols);
        let grid_total_h = rows as f32 * (card_h + 1.0);

        let (grid_rect, _) = ui.allocate_exact_size(vec2(full_w, grid_total_h), Sense::hover());
        ui.painter()
            .rect_filled(grid_rect, CornerRadius::ZERO, t.line);

        for (idx, app) in apps.iter().enumerate() {
            let row = idx / cols;
            let col = idx % cols;

            let card_x = grid_rect.min.x + col as f32 * (card_w + 1.0);
            let card_y = grid_rect.min.y + row as f32 * (card_h + 1.0);
            let card_rect = Rect::from_min_size(egui::pos2(card_x, card_y), vec2(card_w, card_h));

            let card_resp = ui.interact(
                card_rect,
                ui.id().with(("appcard", &app.id)),
                Sense::hover(),
            );
            let is_this_uninstalling = active_app_id == Some(app.id.as_str());
            let fill = if card_resp.hovered() || is_this_uninstalling {
                t.sunken
            } else {
                t.bg
            };
            ui.painter()
                .rect_filled(card_rect, CornerRadius::ZERO, fill);

            let mut card_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(card_rect)
                    .layout(Layout::top_down(Align::Min)),
            );

            // 1. Top Architectural Strip (40px)
            let strip_rect = Rect::from_min_size(card_rect.min, vec2(card_w, 40.0));
            card_ui.allocate_new_ui(egui::UiBuilder::new().max_rect(strip_rect), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    let badge_color = source_color(app.source);
                    widgets::badge(ui, app.source.badge_label(), badge_color, dark_mode);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(14.0);
                        let size_str = if app.estimated_size_bytes > 0 {
                            format_bytes(app.estimated_size_bytes)
                        } else {
                            "—".to_string()
                        };
                        ui.label(
                            RichText::new(size_str)
                                .font(egui::FontId::new(12.5, theme::mono_bold()))
                                .color(t.ink_strong),
                        );
                    });
                });
            });

            // 1px Hairline divider separating top strip from card body
            card_ui
                .painter()
                .hline(strip_rect.x_range(), strip_rect.bottom(), t.hairline());

            // 2. Card Body (padding 14px)
            let body_rect = Rect::from_min_max(
                egui::pos2(card_rect.left() + 14.0, strip_rect.bottom() + 12.0),
                card_rect.max - vec2(14.0, 12.0),
            );

            card_ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                ui.vertical(|ui| {
                    // Header row: 36x36 App Icon + Titles
                    ui.horizontal(|ui| {
                        draw_app_icon_or_monogram(ui, icon_cache, app, 36.0, dark_mode);
                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            // Title in Inter Proportional (14.0px SemiBold)
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&app.display_name)
                                        .font(egui::FontId::new(14.0, egui::FontFamily::Proportional))
                                        .color(t.ink_strong),
                                )
                                .truncate(),
                            );
                            ui.add_space(1.0);
                            // Publisher in Inter (11.5px Regular)
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&app.publisher)
                                        .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                                        .color(t.muted),
                                )
                                .truncate(),
                            );
                        });
                    });

                    ui.add_space(8.0);

                    // Scope / Install Location Box
                    let loc_str = if let Some(loc) = &app.install_location {
                        loc.to_string_lossy().to_string()
                    } else {
                        app.source.scope_label().to_string()
                    };

                    egui::Frame::new()
                        .fill(t.sunken)
                        .stroke(t.hairline())
                        .corner_radius(CornerRadius::ZERO)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&loc_str)
                                        .font(egui::FontId::new(10.5, theme::mono()))
                                        .color(t.muted),
                                )
                                .truncate(),
                            );
                        });

                    ui.add_space(6.0);

                    // Metadata row: version + installer kind + install date
                    let ver_line = format!(
                        "v{}  ·  {}  ·  {}",
                        app.display_version,
                        app.installer_kind.label(),
                        app.install_date
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(ver_line)
                                .font(egui::FontId::new(10.5, egui::FontFamily::Proportional))
                                .color(t.faint),
                        )
                        .truncate(),
                    );

                    // Pinned bottom action row
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        if is_this_uninstalling {
                            let w = ui.available_width();
                            draw_progress_bar(ui, w, 8.0, active_frac, t.warn, dark_mode);
                            ui.add_space(4.0);
                            let pct = format!("UNINSTALLING {:.0}%...", active_frac * 100.0);
                            widgets::hud(ui, &pct, t.warn);
                        } else {
                            let btn_w = ui.available_width();
                            if widgets::button(
                                ui,
                                "COMPLETE UNINSTALL",
                                widgets::ButtonKind::Primary,
                                btn_w,
                                dark_mode,
                            )
                            .clicked()
                            {
                                selected_app = Some(app.clone());
                            }
                        }
                    });
                });
            });
        }
    });

    selected_app
}

/// Renders the Complete Application Uninstallation & Remnant Expunger modal dialog.
/// Supports 3 live states:
/// 1. Pre-Uninstall Configuration & Remnant Preview
/// 2. Active Uninstallation Progress Bar & Real-Time Step Log
/// 3. Post-Uninstall Completion Summary Report
pub fn render_uninstall_modal(
    ctx: &egui::Context,
    state: &mut UninstallerUiState,
    secure_wipe: bool,
    dark_mode: bool,
) -> Option<(InstalledApp, UninstallOptions)> {
    let app = state.modal_app.clone()?;
    let t = Tokens::get(dark_mode);

    let can_dismiss = !state.is_uninstalling;
    let mut close_modal = can_dismiss && ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut confirmed_uninstall = None;

    let screen_rect = ctx.screen_rect();
    egui::Area::new(egui::Id::new("uninstall_modal_scrim"))
        .order(egui::Order::Middle)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen_rect.size(), Sense::click());
            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                Color32::from_black_alpha(if dark_mode { 175 } else { 105 }),
            );
            if resp.clicked() && can_dismiss {
                close_modal = true;
            }
        });

    egui::Area::new(egui::Id::new("uninstall_modal_card"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            let modal_w = 560.0;
            egui::Frame::new()
                .fill(t.bg)
                .stroke(egui::Stroke::new(1.0_f32, t.line_strong))
                .inner_margin(egui::Margin::same(24))
                .show(ui, |ui| {
                    ui.set_width(modal_w);
                    ui.spacing_mut().item_spacing.y = 8.0;

                    widgets::eyebrow(
                        ui,
                        "COMPLETE UNINSTALLATION",
                        "NATIVE UNINSTALLER + DEEP REMNANT EXPUNGER",
                        dark_mode,
                    );

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(&app.display_name)
                            .font(egui::FontId::new(18.0, theme::display()))
                            .color(t.ink_strong),
                    );
                    let meta = format!(
                        "{}  ·  v{}  ·  {}  ·  {}",
                        app.publisher,
                        app.display_version,
                        app.source.badge_label(),
                        app.installer_kind.label()
                    );
                    ui.label(
                        RichText::new(meta)
                            .font(egui::FontId::new(11.0, theme::mono()))
                            .color(t.muted),
                    );

                    if state.is_uninstalling || state.show_completion_modal {
                        ui.add_space(10.0);
                        let (frac, phase_title, detail_text, bar_color) =
                            if state.is_uninstalling {
                                if let Some(prog) = &state.active_progress {
                                    (
                                        prog.fraction,
                                        prog.phase.step_label(),
                                        prog.detail.clone(),
                                        t.warn,
                                    )
                                } else {
                                    (
                                        0.10,
                                        "STEP 1/4 · INITIALIZING UNINSTALLER".to_string(),
                                        "Launching uninstallation worker...".to_string(),
                                        t.warn,
                                    )
                                }
                            } else if let Some(rep) = &state.last_report {
                                let ok = rep.error_message.is_none();
                                let summary = if let Some(err) = &rep.error_message {
                                    format!("Warning: {err}")
                                } else {
                                    format!(
                                        "Purged {} of {} remnants  ·  Reclaimed {}",
                                        rep.remnants_removed,
                                        rep.remnants_found,
                                        format_bytes(rep.bytes_reclaimed)
                                    )
                                };
                                (
                                    1.0,
                                    "COMPLETE · APPLICATION & REMNANTS EXPUNGED".to_string(),
                                    summary,
                                    if ok { t.good } else { t.warn },
                                )
                            } else {
                                (
                                    1.0,
                                    "COMPLETE".to_string(),
                                    "Uninstallation finished.".to_string(),
                                    t.good,
                                )
                            };

                        // Progress Header Row (Stage + Percentage)
                        ui.horizontal(|ui| {
                            widgets::badge(ui, &phase_title, bar_color, dark_mode);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let pct_str = format!("{:.0}%", frac * 100.0);
                                ui.label(
                                    RichText::new(pct_str)
                                        .font(egui::FontId::new(14.0, theme::mono_bold()))
                                        .color(bar_color),
                                );
                            });
                        });

                        ui.add_space(4.0);
                        draw_progress_bar(ui, modal_w, 12.0, frac, bar_color, dark_mode);

                        ui.add_space(4.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(detail_text)
                                    .font(egui::FontId::new(11.0, theme::mono()))
                                    .color(t.ink),
                            )
                            .truncate(),
                        );

                        ui.add_space(8.0);
                        widgets::hud(ui, "LIVE UNINSTALLATION TELEMETRY LOG", t.faint);
                        egui::Frame::new()
                            .fill(t.sunken)
                            .stroke(t.hairline())
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(modal_w - 20.0);
                                egui::ScrollArea::vertical()
                                    .max_height(150.0)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        for line in &state.progress_log {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(line)
                                                        .font(egui::FontId::new(
                                                            10.5,
                                                            theme::mono(),
                                                        ))
                                                        .color(t.ink),
                                                )
                                                .truncate(),
                                            );
                                        }
                                    });
                            });

                        ui.add_space(12.0);
                        if state.is_uninstalling {
                            widgets::hud(
                                ui,
                                "PLEASE WAIT — UNINSTALLER & REMNANT SCANNER IN PROGRESS...",
                                t.warn,
                            );
                        } else if widgets::button(
                            ui,
                            "DONE / CLOSE",
                            widgets::ButtonKind::Primary,
                            modal_w,
                            dark_mode,
                        )
                        .clicked()
                        {
                            close_modal = true;
                        }
                    } else {
                        ui.add_space(4.0);
                        egui::Frame::new()
                            .fill(t.sunken)
                            .stroke(t.hairline())
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(modal_w - 20.0);
                                widgets::hud(ui, "UNINSTALL COMMAND STRING", t.faint);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&app.uninstall_string)
                                            .font(egui::FontId::new(10.5, theme::mono()))
                                            .color(t.ink),
                                    )
                                    .truncate(),
                                );
                            });

                        ui.add_space(6.0);
                        widgets::hud(ui, "REMOVAL PIPELINE OPTIONS", t.faint);
                        ui.checkbox(
                            &mut state.modal_options.run_native_uninstaller,
                            RichText::new("1. Execute official application uninstaller first")
                                .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                                .color(t.ink_strong),
                        );
                        ui.checkbox(
                            &mut state.modal_options.prefer_silent,
                            RichText::new(
                                "   Prefer silent/unattended flags (/qn, /VERYSILENT, /S) when supported",
                            )
                            .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                            .color(t.muted),
                        );
                        ui.checkbox(
                            &mut state.modal_options.purge_remnants,
                            RichText::new(
                                "2. Deep-scan & purge leftover AppData folders, shortcuts, and registry keys",
                            )
                            .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                            .color(t.ink_strong),
                        );

                        ui.add_space(6.0);
                        let rem_header = format!(
                            "DETECTED APPLICATION FOOTPRINT & REMNANT TARGETS ({})",
                            state.modal_remnants.len()
                        );
                        widgets::hud(ui, &rem_header, t.faint);

                        egui::Frame::new()
                            .fill(t.sunken)
                            .stroke(t.hairline())
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(modal_w - 20.0);
                                egui::ScrollArea::vertical()
                                    .max_height(140.0)
                                    .show(ui, |ui| {
                                        if state.modal_remnants.is_empty() {
                                            ui.label(
                                                RichText::new(
                                                    "No pre-existing remnant paths detected outside native uninstaller.",
                                                )
                                                .font(egui::FontId::new(11.0, theme::mono()))
                                                .color(t.muted),
                                            );
                                        } else {
                                            for rem in &state.modal_remnants {
                                                ui.horizontal(|ui| {
                                                    widgets::badge(
                                                        ui,
                                                        rem.kind.label(),
                                                        t.accent_ink,
                                                        dark_mode,
                                                    );
                                                    ui.add(
                                                        egui::Label::new(
                                                            RichText::new(&rem.location)
                                                                .font(egui::FontId::new(
                                                                    10.5,
                                                                    theme::mono(),
                                                                ))
                                                                .color(t.ink),
                                                        )
                                                        .truncate(),
                                                    );
                                                });
                                            }
                                        }
                                    });
                            });

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            let btn_w = (modal_w - 12.0) / 2.0;
                            if widgets::button(
                                ui,
                                "CANCEL",
                                widgets::ButtonKind::Secondary,
                                btn_w,
                                dark_mode,
                            )
                            .clicked()
                            {
                                close_modal = true;
                            }
                            if widgets::button(
                                ui,
                                "COMPLETE UNINSTALL",
                                widgets::ButtonKind::Primary,
                                btn_w,
                                dark_mode,
                            )
                            .clicked()
                            {
                                let mut opts = state.modal_options;
                                opts.clean_mode = if secure_wipe {
                                    crate::core::CleanMode::SecureZero
                                } else {
                                    crate::core::CleanMode::Standard
                                };
                                confirmed_uninstall = Some((app.clone(), opts));
                            }
                        });
                    }
                });
        });

    if close_modal {
        state.modal_app = None;
        state.modal_remnants.clear();
        state.active_progress = None;
        state.progress_log.clear();
        state.show_completion_modal = false;
    }

    confirmed_uninstall
}

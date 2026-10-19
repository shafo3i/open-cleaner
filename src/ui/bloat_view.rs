use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, StrokeKind, Ui, vec2};

use super::header::ViewMode;
use super::theme::{self, Tokens};
use super::uninstaller_view::draw_progress_bar;
use super::widgets;

use crate::bloat::{BloatItem, BloatProgress, BloatReport, BloatRisk};

pub struct BloatUiState {
    pub items: Vec<BloatItem>,
    pub is_scanning: bool,
    pub is_removing: bool,
    pub consent_requested: bool,
    pub consent_ack: bool,
    pub create_restore: bool,
    pub progress_log: Vec<String>,
    pub active_progress: Option<BloatProgress>,
    pub last_report: Option<BloatReport>,
}

impl Default for BloatUiState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            is_scanning: false,
            is_removing: false,
            consent_requested: false,
            consent_ack: false,
            create_restore: true,
            progress_log: Vec::new(),
            active_progress: None,
            last_report: None,
        }
    }
}

pub enum BloatViewAction {
    None,
    Rescan,
    RequestConsent,
}

fn risk_color(risk: BloatRisk, t: &Tokens) -> Color32 {
    match risk {
        BloatRisk::Safe => t.good,
        BloatRisk::Careful => t.warn,
        BloatRisk::Expert => t.bad,
    }
}

pub fn render_bloat_view(
    ui: &mut Ui,
    state: &mut BloatUiState,
    view_mode: ViewMode,
    dark_mode: bool,
) -> BloatViewAction {
    let t = Tokens::get(dark_mode);
    let mut action = BloatViewAction::None;

    ui.add_space(12.0);

    // Section header
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        widgets::eyebrow(ui, "05", "SYSTEM DEBLOAT & PRIVACY POLICY", dark_mode);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            let cnt = format!("{} ITEMS", state.items.len());
            widgets::hud(ui, &cnt, t.faint);
        });
    });

    ui.add_space(8.0);

    // Risk tier legend
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        widgets::hud(ui, "RISK TIER", t.faint);
        ui.add_space(8.0);
        for risk in [BloatRisk::Safe, BloatRisk::Careful, BloatRisk::Expert] {
            widgets::badge(ui, risk.label(), risk_color(risk, &t), dark_mode)
                .on_hover_text(risk.description());
            ui.add_space(6.0);
        }
    });

    ui.add_space(8.0);

    // Persistent risk warning
    egui::Frame::new()
        .fill(t.sunken)
        .stroke(t.hairline())
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 32.0);
            ui.label(
                RichText::new(
                    "Removing system components can break Windows Update or features and may not be reversible. Create a restore point before continuing. Use at your own risk.",
                )
                .font(egui::FontId::new(11.5, theme::mono()))
                .color(t.warn),
            );
        });

    // Privilege status
    let elevated = crate::bloat::is_elevated();
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        if elevated {
            widgets::hud(ui, "PRIVILEGE: ELEVATED — FULL REMOVAL ENABLED", t.good);
        } else {
            widgets::hud(
                ui,
                "PRIVILEGE: STANDARD — RUN AS ADMINISTRATOR FOR FULL REMOVAL & RESTORE POINT",
                t.warn,
            );
        }
    });

    ui.add_space(8.0);

    // Selection + execution controls (toolbar-style)
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let selected = state.items.iter().filter(|i| i.selected).count();
        widgets::hud(ui, &format!("SELECTED {selected}"), t.faint);
        ui.add_space(10.0);

        if widgets::button(
            ui,
            "SELECT SAFE",
            widgets::ButtonKind::Secondary,
            0.0,
            dark_mode,
        )
        .clicked()
        {
            for item in &mut state.items {
                item.selected = item.risk == BloatRisk::Safe;
            }
        }
        ui.add_space(6.0);
        if widgets::button(ui, "CLEAR", widgets::ButtonKind::Secondary, 0.0, dark_mode).clicked() {
            for item in &mut state.items {
                item.selected = false;
            }
        }
        ui.add_space(6.0);
        if widgets::button(ui, "RESCAN", widgets::ButtonKind::Secondary, 0.0, dark_mode).clicked() {
            action = BloatViewAction::Rescan;
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            let enabled = selected > 0 && !state.is_scanning && !state.is_removing;
            ui.add_enabled_ui(enabled, |ui| {
                if widgets::button(
                    ui,
                    &format!("EXECUTE SELECTED ({selected})"),
                    widgets::ButtonKind::Primary,
                    0.0,
                    dark_mode,
                )
                .clicked()
                {
                    action = BloatViewAction::RequestConsent;
                }
            });
        });
    });

    ui.add_space(8.0);

    if state.is_scanning {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            widgets::hud(ui, "ENUMERATING BLOATWARE...", t.faint);
        });
        ui.add_space(8.0);
    }

    // Progress block (during removal or after completion)
    if state.is_removing || state.last_report.is_some() {
        ui.add_space(6.0);
        egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width() - 32.0);

                let (frac, title, detail, color) = if state.is_removing {
                    let p = state.active_progress.as_ref();
                    (
                        p.map(|p| p.fraction).unwrap_or(0.0),
                        p.map(|p| p.item_name.clone())
                            .unwrap_or_else(|| "WORKING".to_string()),
                        p.map(|p| p.detail.clone())
                            .unwrap_or_else(|| "Starting...".to_string()),
                        t.warn,
                    )
                } else if let Some(r) = &state.last_report {
                    (
                        1.0,
                        "DEBLOAT COMPLETE".to_string(),
                        format!(
                            "{} processed · {} removed · {} failed · restore point {}",
                            r.items_processed,
                            r.items_removed,
                            r.items_failed,
                            if r.restore_point_created {
                                "created"
                            } else {
                                "skipped"
                            }
                        ),
                        if r.items_failed == 0 { t.good } else { t.warn },
                    )
                } else {
                    (0.0, String::new(), String::new(), t.good)
                };

                ui.horizontal(|ui| {
                    widgets::badge(ui, &title, color, dark_mode);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let pct = format!("{:.0}%", frac * 100.0);
                        ui.label(
                            RichText::new(pct)
                                .font(egui::FontId::new(13.0, theme::mono_bold()))
                                .color(color),
                        );
                    });
                });
                ui.add_space(4.0);
                draw_progress_bar(
                    ui,
                    ui.available_width() - 28.0,
                    12.0,
                    frac,
                    color,
                    dark_mode,
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(detail)
                        .font(egui::FontId::new(11.0, theme::mono()))
                        .color(t.ink),
                );
                ui.add_space(8.0);
                widgets::hud(ui, "EXECUTION LOG", t.faint);
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &state.progress_log {
                            ui.label(
                                RichText::new(line)
                                    .font(egui::FontId::new(10.5, theme::mono()))
                                    .color(t.muted),
                            );
                        }
                    });
            });
        ui.add_space(8.0);
    }

    // Items (grid or table, matching the purifier views)
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| match view_mode {
            ViewMode::Grid => render_bloat_grid(ui, state, dark_mode),
            ViewMode::Table => render_bloat_table(ui, state, dark_mode),
        });

    ui.add_space(24.0);
    action
}

/// Matches the Purifier Grid (grid_view.rs) pixel-for-pixel.
fn render_bloat_grid(ui: &mut Ui, state: &mut BloatUiState, dark_mode: bool) {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;

    if state.items.is_empty() {
        if !state.is_scanning {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    RichText::new(
                        "No removable bloatware or unapplied privacy policies detected on this system.",
                    )
                    .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                    .color(t.muted),
                );
            });
        }
        return;
    }

    let items = &mut state.items;
    if full_w <= 100.0 {
        return;
    }

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        let cols = ((full_w / 320.0).floor() as usize).max(1);
        let card_w = (full_w - (cols as f32 - 1.0)) / cols as f32;
        let card_h = 280.0;

        let rows = items.len().div_ceil(cols);
        let grid_total_h = rows as f32 * (card_h + 1.0);

        let (grid_rect, _) = ui.allocate_exact_size(vec2(full_w, grid_total_h), Sense::hover());
        ui.painter()
            .rect_filled(grid_rect, CornerRadius::ZERO, t.line);

        for (idx, item) in items.iter_mut().enumerate() {
            let row = idx / cols;
            let col = idx % cols;

            let card_x = grid_rect.min.x + col as f32 * (card_w + 1.0);
            let card_y = grid_rect.min.y + row as f32 * (card_h + 1.0);
            let card_rect = Rect::from_min_size(egui::pos2(card_x, card_y), vec2(card_w, card_h));

            let (_allocated, card_resp) =
                ui.allocate_exact_size(vec2(card_w, card_h), Sense::hover());
            let is_hovered = card_resp.hovered();

            let fill = if is_hovered { t.sunken } else { t.bg };
            ui.painter()
                .rect_filled(card_rect, CornerRadius::ZERO, fill);

            let accent = risk_color(item.risk, &t);
            let number = format!("{:02}", idx + 1);

            let mut card_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(card_rect)
                    .layout(Layout::top_down(Align::Min)),
            );

            // Top Strip (40px)
            let strip_rect = Rect::from_min_size(card_rect.min, vec2(card_w, 40.0));
            card_ui.allocate_new_ui(egui::UiBuilder::new().max_rect(strip_rect), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);

                    let num_g = widgets::caps(
                        ui,
                        &number,
                        theme::HUD_SIZE,
                        theme::mono_bold(),
                        theme::HUD_SPACING,
                        Color32::WHITE,
                    );
                    let tag_r = Rect::from_center_size(
                        egui::pos2(ui.cursor().left() + 16.0, strip_rect.center().y),
                        vec2(32.0, 24.0),
                    );
                    ui.painter().rect_filled(tag_r, CornerRadius::ZERO, accent);
                    ui.painter()
                        .galley(tag_r.center() - num_g.size() / 2.0, num_g, Color32::WHITE);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(14.0);
                        widgets::badge(ui, item.risk.label(), accent, dark_mode);
                    });
                });
            });

            // Strip divider
            card_ui
                .painter()
                .hline(strip_rect.x_range(), strip_rect.bottom(), t.hairline());

            // Card Body (padding 16px)
            let body_rect = Rect::from_min_max(
                egui::pos2(card_rect.left() + 16.0, strip_rect.bottom() + 16.0),
                card_rect.max - vec2(16.0, 16.0),
            );

            card_ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                ui.vertical(|ui| {
                    // Header Block: Icon + Titles
                    ui.horizontal(|ui| {
                        let icon_r = Rect::from_min_size(ui.cursor().min, vec2(52.0, 52.0));
                        ui.painter().rect_filled(icon_r, CornerRadius::ZERO, accent);
                        let sym_g = widgets::caps(
                            ui,
                            &number,
                            16.0,
                            theme::mono_bold(),
                            1.0,
                            Color32::WHITE,
                        );
                        ui.painter().galley(
                            icon_r.center() - sym_g.size() / 2.0,
                            sym_g,
                            Color32::WHITE,
                        );

                        ui.add_space(60.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new(&item.name)
                                    .font(egui::FontId::new(17.0, theme::display()))
                                    .color(t.ink_strong),
                            );
                            ui.label(
                                RichText::new(item.category.label())
                                    .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                                    .color(accent),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&item.description)
                                        .font(egui::FontId::new(10.5, theme::mono()))
                                        .color(t.faint),
                                )
                                .truncate(),
                            );
                        });
                    });

                    ui.add_space(14.0);

                    // Selection Badge
                    let (sel_color, sel_text) = if item.selected {
                        (t.accent_ink, "[ INCLUDED ]")
                    } else {
                        (t.faint, "[ NOT INCLUDED ]")
                    };
                    widgets::badge(ui, sel_text, sel_color, dark_mode);

                    ui.add_space(8.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(item.risk.description())
                                .font(egui::FontId::new(11.5, theme::mono()))
                                .color(t.muted),
                        )
                        .truncate(),
                    );

                    ui.add_space(16.0);

                    // Actions row at bottom
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            let is_selected = item.selected;
                            let (label, kind) = if is_selected {
                                ("INCLUDED", widgets::ButtonKind::Primary)
                            } else {
                                ("INCLUDE IN PASS", widgets::ButtonKind::Secondary)
                            };
                            if widgets::button(
                                ui,
                                label,
                                kind,
                                ui.available_width() - 8.0,
                                dark_mode,
                            )
                            .clicked()
                            {
                                item.selected = !item.selected;
                            }
                        });
                    });
                });
            });
        }
    });
}

/// Matches the Purifier Table (table_view.rs) pixel-for-pixel.
fn render_bloat_table(ui: &mut Ui, state: &mut BloatUiState, dark_mode: bool) {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;

    if state.items.is_empty() {
        if !state.is_scanning {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    RichText::new(
                        "No removable bloatware or unapplied privacy policies detected on this system.",
                    )
                    .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                    .color(t.muted),
                );
            });
        }
        return;
    }

    let items = &mut state.items;
    if full_w <= 100.0 {
        return;
    }

    let pad_left = 16.0;
    let col1_w = (full_w * 0.35).max(300.0);
    let col2_w = (full_w * 0.26).max(200.0);
    let col3_w = 140.0;

    let x1 = pad_left;
    let x2 = x1 + col1_w;
    let x3 = x2 + col2_w;

    let header_h = 42.0;
    let row_h = 62.0;
    let total_h = header_h + (items.len() as f32 * row_h);

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

        // Header
        let header_rect = Rect::from_min_size(table_rect.min, vec2(full_w, header_h));
        let hy = header_rect.center().y;
        ui.painter()
            .hline(table_rect.x_range(), header_rect.bottom(), t.hairline());

        let col_comp = widgets::caps(
            ui,
            "BLOAT COMPONENT",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.ink_strong,
        );
        ui.painter().galley(
            egui::pos2(table_rect.left() + x1, hy - col_comp.size().y / 2.0),
            col_comp,
            t.ink_strong,
        );

        let col_scope = widgets::caps(
            ui,
            "SCOPE & IMPACT",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.faint,
        );
        ui.painter().galley(
            egui::pos2(table_rect.left() + x2, hy - col_scope.size().y / 2.0),
            col_scope,
            t.faint,
        );

        let col_risk = widgets::caps(
            ui,
            "RISK TIER",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.faint,
        );
        ui.painter().galley(
            egui::pos2(table_rect.left() + x3, hy - col_risk.size().y / 2.0),
            col_risk,
            t.faint,
        );

        let col_stat = widgets::caps(
            ui,
            "OPT-IN",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.faint,
        );
        ui.painter().galley(
            egui::pos2(
                table_rect.right() - 16.0 - col_stat.size().x,
                hy - col_stat.size().y / 2.0,
            ),
            col_stat,
            t.faint,
        );

        // Data Rows
        let total_items = items.len();
        for (idx, item) in items.iter_mut().enumerate() {
            let row_top = header_rect.bottom() + (idx as f32 * row_h);
            let row_rect =
                Rect::from_min_size(egui::pos2(table_rect.left(), row_top), vec2(full_w, row_h));
            let row_resp = ui.interact(row_rect, ui.id().with(("bloatrow", idx)), Sense::hover());
            let hovered = row_resp.hovered();

            if hovered {
                ui.painter()
                    .rect_filled(row_rect, CornerRadius::ZERO, t.sunken);
            }

            if idx + 1 < total_items {
                ui.painter()
                    .hline(table_rect.x_range(), row_rect.bottom(), t.hairline());
            }

            let ry = row_rect.center().y;
            let accent = risk_color(item.risk, &t);
            let number = format!("{:02}", idx + 1);

            // Col 1: Checkbox + Colored Number Tab + Wordmark & Category
            let cb_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 26.0, ry), vec2(20.0, 20.0));
            let mut cb_ui = ui.new_child(egui::UiBuilder::new().max_rect(cb_rect));
            cb_ui.checkbox(&mut item.selected, "");

            let num_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 58.0, ry), vec2(34.0, 34.0));
            ui.painter()
                .rect_filled(num_rect, CornerRadius::ZERO, accent);
            let num_galley = widgets::caps(
                ui,
                &number,
                theme::HUD_SIZE,
                theme::mono_bold(),
                theme::HUD_SPACING,
                Color32::WHITE,
            );
            ui.painter().galley(
                num_rect.center() - num_galley.size() / 2.0,
                num_galley,
                Color32::WHITE,
            );

            let name_x = table_rect.left() + 86.0;
            let name_w = (col1_w - 86.0 - 12.0).max(100.0);
            let name_rect = Rect::from_min_size(egui::pos2(name_x, ry - 18.0), vec2(name_w, 36.0));
            let mut name_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(name_rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            name_ui.label(
                RichText::new(&item.name)
                    .font(egui::FontId::new(15.0, theme::display()))
                    .color(t.ink_strong),
            );
            name_ui.add(
                egui::Label::new(
                    RichText::new(item.category.label())
                        .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                        .color(accent),
                )
                .truncate(),
            );

            // Col 2: Scope & Impact
            let scope_rect = Rect::from_min_size(
                egui::pos2(table_rect.left() + x2, ry - 10.0),
                vec2(col2_w - 12.0, 20.0),
            );
            let mut scope_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(scope_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            scope_ui.add(
                egui::Label::new(
                    RichText::new(&item.description)
                        .font(egui::FontId::new(11.0, theme::mono()))
                        .color(t.muted),
                )
                .truncate(),
            );

            // Col 3: Risk Tier Badge
            let risk_rect = Rect::from_min_size(
                egui::pos2(table_rect.left() + x3, ry - 11.0),
                vec2(col3_w - 10.0, 22.0),
            );
            let mut risk_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(risk_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            widgets::badge(&mut risk_ui, item.risk.label(), accent, dark_mode);

            // Col 4: Opt-in status
            let status_w = 120.0;
            let status_rect = Rect::from_min_size(
                egui::pos2(table_rect.right() - 16.0 - status_w, ry - 11.0),
                vec2(status_w, 22.0),
            );
            let mut status_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(status_rect)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            let (sel_color, sel_text) = if item.selected {
                (t.accent_ink, "[ INCLUDED ]")
            } else {
                (t.faint, "[ NOT INCLUDED ]")
            };
            widgets::badge(&mut status_ui, sel_text, sel_color, dark_mode);
        }
    });
}

/// Renders the high-risk consent modal. Returns `Some(true)` when the user confirms.
pub fn render_bloat_consent_modal(
    ctx: &egui::Context,
    state: &mut BloatUiState,
    dark_mode: bool,
) -> Option<bool> {
    if !state.consent_requested {
        return None;
    }

    let t = Tokens::get(dark_mode);
    let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut confirmed = false;

    let selected: Vec<&BloatItem> = state.items.iter().filter(|i| i.selected).collect();
    let safe_count = selected
        .iter()
        .filter(|i| i.risk == BloatRisk::Safe)
        .count();
    let careful_count = selected
        .iter()
        .filter(|i| i.risk == BloatRisk::Careful)
        .count();
    let expert_count = selected
        .iter()
        .filter(|i| i.risk == BloatRisk::Expert)
        .count();
    let expert_present = expert_count > 0;

    let screen_rect = ctx.screen_rect();
    egui::Area::new(egui::Id::new("bloat_consent_scrim"))
        .order(egui::Order::Middle)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen_rect.size(), Sense::click());
            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                Color32::from_black_alpha(if dark_mode { 175 } else { 105 }),
            );
            if resp.clicked() {
                close = true;
            }
        });

    egui::Area::new(egui::Id::new("bloat_consent_card"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
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
                        "SYSTEM DEBLOAT",
                        "HIGH-RISK OPERATION CONFIRMATION",
                        dark_mode,
                    );

                    ui.add_space(2.0);
                    ui.label(
                        RichText::new("Remove Windows bloatware & policies?")
                            .font(egui::FontId::new(18.0, theme::display()))
                            .color(t.ink_strong),
                    );

                    egui::Frame::new()
                        .fill(t.sunken)
                        .stroke(t.hairline())
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(modal_w - 24.0);
                            ui.label(
                                RichText::new(
                                    "Removing system components can break Windows Update or features and may not be reversible. Create a restore point before continuing. Use at your own risk.",
                                )
                                .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                                .color(t.warn),
                            );
                        });

                    ui.add_space(2.0);
                    widgets::hud(
                        ui,
                        &format!("SELECTED {} ITEM(S) BY RISK TIER", selected.len()),
                        t.faint,
                    );
                    egui::Frame::new()
                        .fill(t.sunken)
                        .stroke(t.hairline())
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(modal_w - 24.0);
                            ui.spacing_mut().item_spacing.y = 6.0;
                            risk_summary_row(ui, "SAFE", safe_count, t.good, dark_mode);
                            risk_summary_row(ui, "CAREFUL", careful_count, t.warn, dark_mode);
                            risk_summary_row(ui, "EXPERT", expert_count, t.bad, dark_mode);
                        });

                    if !crate::bloat::is_elevated() {
                        egui::Frame::new()
                            .fill(t.sunken)
                            .stroke(t.hairline())
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(modal_w - 24.0);
                                ui.label(
                                    RichText::new(
                                        "NOT ELEVATED — restore point creation, app de-provisioning, optional features, and machine-wide policies require administrator privileges. Run Open Cleaner as administrator to enable them.",
                                    )
                                    .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                                    .color(t.warn),
                                );
                            });
                    }

                    ui.checkbox(
                        &mut state.create_restore,
                        RichText::new(
                            "Create a Windows restore point before removing (recommended)",
                        )
                        .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                        .color(t.ink_strong),
                    );

                    let ack_text = if expert_present {
                        "I understand EXPERT items can affect Windows servicing and may not be reversible."
                    } else {
                        "I understand this action modifies system components and I accept the risk."
                    };
                    ui.checkbox(
                        &mut state.consent_ack,
                        RichText::new(ack_text)
                            .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                            .color(t.warn),
                    );

                    ui.add_space(8.0);
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
                            close = true;
                        }
                        ui.add_enabled_ui(state.consent_ack, |ui| {
                            if widgets::button(
                                ui,
                                "CONTINUE — I UNDERSTAND",
                                widgets::ButtonKind::Primary,
                                btn_w,
                                dark_mode,
                            )
                            .clicked()
                            {
                                confirmed = true;
                            }
                        });
                    });
                });
        });

    if close || confirmed {
        state.consent_requested = false;
        state.consent_ack = false;
    }

    if confirmed { Some(true) } else { None }
}

fn risk_summary_row(ui: &mut Ui, label: &str, count: usize, color: Color32, dark_mode: bool) {
    let t = Tokens::get(dark_mode);
    ui.horizontal(|ui| {
        widgets::badge(ui, label, color, dark_mode);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(count.to_string())
                    .font(egui::FontId::new(12.0, theme::mono_bold()))
                    .color(if count > 0 { t.ink_strong } else { t.faint }),
            );
        });
    });
}

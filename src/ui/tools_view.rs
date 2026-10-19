use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Ui, vec2};

use super::header::ViewMode;
use super::theme::{self, Tokens};
use super::widgets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolsAction {
    None,
    OpenUninstaller,
}

#[derive(Clone, Debug)]
pub struct UtilityItem {
    pub id: &'static str,
    pub number: &'static str,
    pub color: Color32,
    pub name_prefix: &'static str,
    pub name_suffix: &'static str,
    pub tagline: &'static str,
    pub sub_tagline: &'static str,
    pub scope: &'static str,
    pub status: &'static str,
    pub meta_text: String,
    pub btn_label: &'static str,
    pub checked: bool,
}

pub fn default_utilities() -> Vec<UtilityItem> {
    vec![
        UtilityItem {
            id: "tool_uninstaller",
            number: "01",
            color: Color32::from_rgb(0x7b, 0x1f, 0xa2), // Purple
            name_prefix: "Software",
            name_suffix: "Uninstaller",
            tagline: "Program removal & remnant expunger",
            sub_tagline: "HKLM/HKCU Uninstall & AppX",
            scope: "Registry Uninstall & UWP AppX",
            status: "[ ACTIVE ]",
            meta_text: "Enumerating installed programs...".to_string(),
            btn_label: "OPEN UNINSTALLER",
            checked: true,
        },
        UtilityItem {
            id: "tool_startup",
            number: "02",
            color: Color32::from_rgb(0x00, 0x89, 0x7b), // Teal
            name_prefix: "Startup",
            name_suffix: "Optimizer",
            tagline: "Boot acceleration & service manager",
            sub_tagline: "Run, RunOnce & Task Scheduler",
            scope: "Registry Autostart & Tasks",
            status: "[ AVAILABLE ]",
            meta_text: "Registry Run / RunOnce & Task Scheduler".to_string(),
            btn_label: "ANALYZE BOOT",
            checked: true,
        },
        UtilityItem {
            id: "tool_updater",
            number: "03",
            color: Color32::from_rgb(0xfb, 0x8c, 0x00), // Amber
            name_prefix: "Software",
            name_suffix: "Updater",
            tagline: "Desktop package patch verification",
            sub_tagline: "Windows Package Manager (winget)",
            scope: "winget repository feed",
            status: "[ AVAILABLE ]",
            meta_text: "Windows Package Manager feed".to_string(),
            btn_label: "CHECK UPDATES",
            checked: true,
        },
    ]
}

pub fn render_tools_view(
    ui: &mut Ui,
    tools: &mut [UtilityItem],
    view_mode: ViewMode,
    dark_mode: bool,
) -> ToolsAction {
    let t = Tokens::get(dark_mode);

    ui.add_space(12.0);

    // Section Eyebrow Header (matching purifier section header)
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        widgets::eyebrow(
            ui,
            "04",
            "WINDOWS UTILITY & MAINTENANCE SUBSYSTEMS",
            dark_mode,
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            let cnt_str = format!("{} UTILITIES", tools.len());
            widgets::hud(ui, &cnt_str, t.faint);
        });
    });

    ui.add_space(8.0);

    let action = match view_mode {
        ViewMode::Grid => render_tools_grid(ui, tools, dark_mode),
        ViewMode::Table => render_tools_table(ui, tools, dark_mode),
    };

    ui.add_space(24.0);
    action
}

/// Matches the Purifier Grid (grid_view.rs) pixel-for-pixel
fn render_tools_grid(ui: &mut Ui, tools: &mut [UtilityItem], dark_mode: bool) -> ToolsAction {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;
    let mut action = ToolsAction::None;

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        let cols = ((full_w / 320.0).floor() as usize).max(1);
        let card_w = (full_w - (cols as f32 - 1.0)) / cols as f32;
        let card_h = 280.0;

        let rows = tools.len().div_ceil(cols);
        let grid_total_h = rows as f32 * (card_h + 1.0);

        let (grid_rect, _) = ui.allocate_exact_size(vec2(full_w, grid_total_h), Sense::hover());
        ui.painter()
            .rect_filled(grid_rect, CornerRadius::ZERO, t.line);

        for (idx, item) in tools.iter_mut().enumerate() {
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

                    // Number Tag in brand color
                    let num_g = widgets::caps(
                        ui,
                        item.number,
                        theme::HUD_SIZE,
                        theme::mono_bold(),
                        theme::HUD_SPACING,
                        Color32::WHITE,
                    );
                    let tag_r = Rect::from_center_size(
                        egui::pos2(ui.cursor().left() + 16.0, strip_rect.center().y),
                        vec2(32.0, 24.0),
                    );
                    ui.painter()
                        .rect_filled(tag_r, CornerRadius::ZERO, item.color);
                    ui.painter()
                        .galley(tag_r.center() - num_g.size() / 2.0, num_g, Color32::WHITE);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(14.0);
                        if widgets::nav_link(ui, "CONFIG ↗", 40.0, false, dark_mode).clicked() {
                            // configure tool
                        }
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
                        ui.painter()
                            .rect_filled(icon_r, CornerRadius::ZERO, item.color);
                        let sym_g = widgets::caps(
                            ui,
                            item.number,
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
                            ui.horizontal(|ui| {
                                let pre = RichText::new(item.name_prefix)
                                    .font(egui::FontId::new(17.0, theme::display()))
                                    .color(t.ink_strong);
                                ui.label(pre);
                                let sfx = RichText::new(item.name_suffix)
                                    .font(egui::FontId::new(17.0, theme::display()))
                                    .color(item.color);
                                ui.label(sfx);
                            });
                            let tag = RichText::new(item.tagline)
                                .font(egui::FontId::new(12.0, egui::FontFamily::Proportional))
                                .color(t.muted);
                            ui.label(tag);
                            let sub = RichText::new(item.sub_tagline)
                                .font(egui::FontId::new(10.5, theme::mono()))
                                .color(t.faint);
                            ui.label(sub);
                        });
                    });

                    ui.add_space(14.0);

                    // Status Badge
                    widgets::badge(ui, item.status, t.accent_ink, dark_mode);

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&item.meta_text)
                            .font(egui::FontId::new(11.5, theme::mono()))
                            .color(t.muted),
                    );

                    ui.add_space(16.0);

                    // Actions row at bottom
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            if widgets::icon_button(ui, dark_mode).clicked()
                                && item.id == "tool_uninstaller"
                            {
                                action = ToolsAction::OpenUninstaller;
                            }
                            ui.add_space(6.0);
                            let action_btn = widgets::button(
                                ui,
                                item.btn_label,
                                widgets::ButtonKind::Primary,
                                ui.available_width() - 8.0,
                                dark_mode,
                            );
                            if action_btn.clicked() && item.id == "tool_uninstaller" {
                                action = ToolsAction::OpenUninstaller;
                            }
                        });
                    });
                });
            });
        }
    });

    action
}

/// Matches the Purifier Table (table_view.rs) pixel-for-pixel
fn render_tools_table(ui: &mut Ui, tools: &mut [UtilityItem], dark_mode: bool) -> ToolsAction {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;
    let mut action = ToolsAction::None;

    if full_w <= 100.0 || tools.is_empty() {
        return action;
    }

    // Column offsets (shared identically between header and rows)
    let pad_left = 16.0;
    let col1_w = (full_w * 0.35).max(300.0);
    let col2_w = (full_w * 0.26).max(200.0);
    let col3_w = 140.0;

    let x1 = pad_left;
    let x2 = x1 + col1_w;
    let x3 = x2 + col2_w;
    let _x4 = x3 + col3_w;

    let header_h = 42.0;
    let row_h = 62.0;
    let total_h = header_h + (tools.len() as f32 * row_h);

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        // Allocate the entire table bounds
        let (table_rect, _) = ui.allocate_exact_size(vec2(full_w, total_h), Sense::hover());

        // Outer 1px hairline border around the table
        ui.painter().rect(
            table_rect,
            CornerRadius::ZERO,
            t.bg,
            t.hairline(),
            egui::StrokeKind::Inside,
        );

        // Header
        let header_rect = Rect::from_min_size(table_rect.min, vec2(full_w, header_h));
        let hy = header_rect.center().y;
        ui.painter()
            .hline(table_rect.x_range(), header_rect.bottom(), t.hairline());

        let col_comp = widgets::caps(
            ui,
            "UTILITY SUBSYSTEM",
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
            "SCOPE & ARCHITECTURE",
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

        let col_stat = widgets::caps(
            ui,
            "STATUS",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.faint,
        );
        ui.painter().galley(
            egui::pos2(table_rect.left() + x3, hy - col_stat.size().y / 2.0),
            col_stat,
            t.faint,
        );

        let col_act = widgets::caps(
            ui,
            "ACTION",
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

        // Data Rows
        let total_items = tools.len();
        for (idx, item) in tools.iter_mut().enumerate() {
            let row_top = header_rect.bottom() + (idx as f32 * row_h);
            let row_rect =
                Rect::from_min_size(egui::pos2(table_rect.left(), row_top), vec2(full_w, row_h));
            let row_resp = ui.interact(row_rect, ui.id().with(("utrow", idx)), Sense::hover());
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

            // Col 1: Checkbox + Colored Number Tab + Wordmark & Tagline
            let cb_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 26.0, ry), vec2(20.0, 20.0));
            let mut cb_ui = ui.new_child(egui::UiBuilder::new().max_rect(cb_rect));
            cb_ui.checkbox(&mut item.checked, "");

            let num_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 58.0, ry), vec2(34.0, 34.0));
            ui.painter()
                .rect_filled(num_rect, CornerRadius::ZERO, item.color);
            let num_galley = widgets::caps(
                ui,
                item.number,
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
            name_ui.horizontal(|ui| {
                ui.label(
                    RichText::new(item.name_prefix)
                        .font(egui::FontId::new(15.0, theme::display()))
                        .color(t.ink_strong),
                );
                ui.label(
                    RichText::new(item.name_suffix)
                        .font(egui::FontId::new(15.0, theme::display()))
                        .color(item.color),
                );
            });
            name_ui.add(
                egui::Label::new(
                    RichText::new(&item.meta_text)
                        .font(egui::FontId::new(11.5, egui::FontFamily::Proportional))
                        .color(t.muted),
                )
                .truncate(),
            );

            // Col 2: Scope & Path
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
                    RichText::new(item.scope)
                        .font(egui::FontId::new(11.0, theme::mono()))
                        .color(t.muted),
                )
                .truncate(),
            );

            // Col 3: Status Badge
            let status_rect = Rect::from_min_size(
                egui::pos2(table_rect.left() + x3, ry - 11.0),
                vec2(col3_w - 10.0, 22.0),
            );
            let mut stat_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(status_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            widgets::badge(&mut stat_ui, item.status, t.accent_ink, dark_mode);

            // Col 4: Action Button
            let btn_w = 132.0;
            let btn_h = 30.0;
            let btn_rect = Rect::from_min_size(
                egui::pos2(table_rect.right() - 16.0 - btn_w, ry - btn_h / 2.0),
                vec2(btn_w, btn_h),
            );
            let mut btn_ui = ui.new_child(egui::UiBuilder::new().max_rect(btn_rect));
            if widgets::button(
                &mut btn_ui,
                item.btn_label,
                widgets::ButtonKind::Primary,
                btn_w,
                dark_mode,
            )
            .clicked()
                && item.id == "tool_uninstaller"
            {
                action = ToolsAction::OpenUninstaller;
            }
        }
    });

    action
}

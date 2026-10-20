use super::grid_view::GridActions;
use super::model::{TargetItem, TargetStatus};
use super::theme::{self, Tokens};
use super::widgets;
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Ui, vec2};

pub fn render_table(ui: &mut Ui, items: &mut [&mut TargetItem], dark_mode: bool) -> GridActions {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;
    let mut actions = GridActions::default();

    if full_w <= 100.0 || items.is_empty() {
        return actions;
    }

    // Column offsets (shared identically between header and rows)
    let pad_left = 16.0;
    let col1_w = (full_w * 0.35).max(300.0);
    let col2_w = (full_w * 0.26).max(200.0);
    let col3_w = 140.0;
    let _col4_w = 110.0;

    let x1 = pad_left;
    let x2 = x1 + col1_w;
    let x3 = x2 + col2_w;
    let x4 = x3 + col3_w;

    let header_h = 42.0;
    let row_h = 62.0;
    let total_h = header_h + (items.len() as f32 * row_h);

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

        // -------------------------------------------------------------------
        // 1. Table Header (42px)
        // -------------------------------------------------------------------
        let header_rect = Rect::from_min_size(table_rect.min, vec2(full_w, header_h));
        let hy = header_rect.center().y;

        // Divider below header
        ui.painter()
            .hline(table_rect.x_range(), header_rect.bottom(), t.hairline());

        // Header Select-All Checkbox
        let all_checked = !items.is_empty() && items.iter().all(|it| it.checked);
        let mut header_checked = all_checked;
        let hcb_rect =
            Rect::from_center_size(egui::pos2(table_rect.left() + 26.0, hy), vec2(20.0, 20.0));
        let mut hcb_ui = ui.new_child(egui::UiBuilder::new().max_rect(hcb_rect));
        if hcb_ui.checkbox(&mut header_checked, "").changed() {
            for it in items.iter_mut() {
                it.checked = header_checked;
            }
        }

        // Header Labels
        let col_comp = widgets::caps(
            ui,
            "TARGET COMPONENT",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.ink_strong,
        );
        ui.painter().galley(
            egui::pos2(table_rect.left() + 76.0, hy - col_comp.size().y / 2.0),
            col_comp,
            t.ink_strong,
        );

        let col_scope = widgets::caps(
            ui,
            "SCOPE & PATH",
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

        let col_space = widgets::caps(
            ui,
            "SPACE",
            theme::HUD_SIZE,
            theme::mono(),
            theme::HUD_SPACING,
            t.faint,
        );
        ui.painter().galley(
            egui::pos2(table_rect.left() + x4, hy - col_space.size().y / 2.0),
            col_space,
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

        // -------------------------------------------------------------------
        // 2. Data Rows (62px each)
        // -------------------------------------------------------------------
        let total_items = items.len();
        for (idx, item) in items.iter_mut().enumerate() {
            let item = &mut **item;
            let row_top = header_rect.bottom() + (idx as f32 * row_h);
            let row_rect =
                Rect::from_min_size(egui::pos2(table_rect.left(), row_top), vec2(full_w, row_h));
            let row_resp = ui.interact(row_rect, ui.id().with(("trow", idx)), Sense::hover());
            let hovered = row_resp.hovered();

            if hovered {
                ui.painter()
                    .rect_filled(row_rect, CornerRadius::ZERO, t.sunken);
            }

            // Divider between rows
            if idx + 1 < total_items {
                ui.painter()
                    .hline(table_rect.x_range(), row_rect.bottom(), t.hairline());
            }

            let ry = row_rect.center().y;

            // Col 1: Target Component (Checkbox + Colored Number Tab + Wordmark & Tagline)
            let cb_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 26.0, ry), vec2(20.0, 20.0));
            let mut cb_ui = ui.new_child(egui::UiBuilder::new().max_rect(cb_rect));
            cb_ui.checkbox(&mut item.checked, "");

            let num_rect =
                Rect::from_center_size(egui::pos2(table_rect.left() + 54.0, ry), vec2(24.0, 24.0));
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

            let name_x = table_rect.left() + 76.0;
            let name_w = (col1_w - 76.0 - 12.0).max(100.0);
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
                    RichText::new(item.tagline)
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
            let (status_color, status_text) = match item.status {
                TargetStatus::Available => (t.accent_ink, "[ AVAILABLE ]"),
                TargetStatus::Guarded => (t.warn, "[ GUARDED AUDIT ]"),
                TargetStatus::Cleaned => (t.good, "[ CLEANED ]"),
            };
            widgets::badge(&mut stat_ui, status_text, status_color, dark_mode);

            // Col 4: Space Reclaimable
            let space_galley =
                widgets::caps(ui, &item.size, 13.0, theme::mono_bold(), 1.0, t.ink_strong);
            ui.painter().galley(
                egui::pos2(table_rect.left() + x4, ry - space_galley.size().y / 2.0),
                space_galley,
                t.ink_strong,
            );

            // Col 5: Actions
            let act_w = 140.0;
            let act_rect = Rect::from_min_size(
                egui::pos2(table_rect.right() - 16.0 - act_w, ry - 17.0),
                vec2(act_w, 34.0),
            );
            let mut act_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(act_rect)
                    .layout(Layout::right_to_left(Align::Center)),
            );
            if widgets::icon_button(&mut act_ui, dark_mode).clicked() {
                actions.rule_id = Some(item.id.to_string());
            }
            act_ui.add_space(6.0);
            if widgets::button(
                &mut act_ui,
                "PURGE",
                widgets::ButtonKind::Secondary,
                0.0,
                dark_mode,
            )
            .clicked()
            {
                actions.purge_id = Some(item.id.to_string());
            }
        }
    });

    actions
}

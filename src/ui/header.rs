use super::theme::{self, Tokens};
use super::widgets;
use egui::{Align, Layout, Rect, RichText, Ui, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Table,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderAction {
    None,
    Refresh,
}

pub fn render_header(ui: &mut Ui, view_mode: &mut ViewMode, dark_mode: &mut bool) -> HeaderAction {
    let t = Tokens::get(*dark_mode);
    let h = theme::HEADER_H;
    let mut action = HeaderAction::None;

    let full_rect = ui.available_rect_before_wrap();
    let header_rect = Rect::from_min_size(full_rect.min, vec2(full_rect.width(), h));

    // Allow dragging window from titlebar
    widgets::handle_title_drag(ui, header_rect);

    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), h),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // Left Branding: OPEN CLEANER
            ui.add_space(16.0);
            let title_text = RichText::new("OPEN CLEANER")
                .font(egui::FontId::new(16.0, theme::display()))
                .color(t.ink_strong);
            ui.label(title_text);

            ui.add_space(14.0);
            widgets::rule(ui, h, *dark_mode);
            ui.add_space(14.0);

            // Tag: FSCLEAR
            widgets::tag(ui, "FSCLEAR", t.invert_bg, t.invert_fg);

            ui.add_space(12.0);
            widgets::hud(ui, "SYSTEM PRIVACY & CLEANING ENGINE", t.muted);

            // Right Action Area
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Window controls at extreme right: Close (Far right), Maximize, Minimize
                widgets::window_controls(ui, h, *dark_mode);

                // Theme toggle: Light / Dark (Crisp vector Sun/Moon, no missing glyphs)
                if widgets::theme_toggle_button(ui, h, *dark_mode).clicked() {
                    *dark_mode = !*dark_mode;
                }

                // Refresh / Audit trigger (Crisp vector reload arrow)
                if widgets::refresh_button(ui, h, *dark_mode).clicked() {
                    action = HeaderAction::Refresh;
                }

                widgets::rule(ui, h, *dark_mode);
                ui.add_space(8.0);

                // Segmented View Switch: [ GRID | TABLE ]
                render_view_switch(ui, view_mode, *dark_mode);
            });
        },
    );

    // Hairline divider along the bottom of the header
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom() - 0.5,
        t.hairline(),
    );
    action
}

fn render_view_switch(ui: &mut Ui, view_mode: &mut ViewMode, dark_mode: bool) {
    let t = Tokens::get(dark_mode);
    let h = 26.0;
    let grid_g = widgets::hud_galley(ui, "GRID", t.ink);
    let table_g = widgets::hud_galley(ui, "TABLE", t.ink);
    let seg_w = (grid_g.size().x.max(table_g.size().x) + 20.0).max(54.0);
    let total_w = seg_w * 2.0;

    let (rect, resp) = ui.allocate_exact_size(vec2(total_w, h), egui::Sense::click());
    let grid_rect = Rect::from_min_size(rect.min, vec2(seg_w, h));
    let table_rect = Rect::from_min_size(egui::pos2(rect.min.x + seg_w, rect.min.y), vec2(seg_w, h));

    let p = ui.painter();
    p.rect(
        rect,
        egui::CornerRadius::ZERO,
        t.bg,
        t.hairline(),
        egui::StrokeKind::Inside,
    );
    p.vline(rect.min.x + seg_w, rect.y_range(), t.hairline());

    let is_grid = *view_mode == ViewMode::Grid;
    let is_table = *view_mode == ViewMode::Table;

    if is_grid {
        p.rect_filled(grid_rect, egui::CornerRadius::ZERO, t.invert_bg);
    }
    if is_table {
        p.rect_filled(table_rect, egui::CornerRadius::ZERO, t.invert_bg);
    }

    let grid_color = if is_grid { t.invert_fg } else { t.muted };
    let table_color = if is_table { t.invert_fg } else { t.muted };

    widgets::paint_centred(ui, grid_rect, grid_g, grid_color);
    widgets::paint_centred(ui, table_rect, table_g, table_color);

    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if grid_rect.contains(pos) {
                *view_mode = ViewMode::Grid;
            } else if table_rect.contains(pos) {
                *view_mode = ViewMode::Table;
            }
        }
    }
}

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
                ui.add_space(6.0);

                // Segmented View Switch: [ GRID | TABLE ]
                // In right-to-left layout: add TABLE first (right side), then GRID second (left side)
                let is_grid = *view_mode == ViewMode::Grid;
                let is_table = *view_mode == ViewMode::Table;

                if widgets::nav_link(ui, "TABLE", 28.0, is_table, *dark_mode).clicked() {
                    *view_mode = ViewMode::Table;
                }
                if widgets::nav_link(ui, "GRID", 28.0, is_grid, *dark_mode).clicked() {
                    *view_mode = ViewMode::Grid;
                }
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

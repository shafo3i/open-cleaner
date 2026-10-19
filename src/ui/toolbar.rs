use super::theme::Tokens;
use super::widgets::{self, ButtonKind, FilterOption};
use egui::{Align, Layout, Ui, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    None,
    Analyze,
    Purge,
}

pub fn render_toolbar(
    ui: &mut Ui,
    selected_category: &mut String,
    options: &[FilterOption<'_>],
    search_query: &mut String,
    dark_mode: bool,
) -> ToolbarAction {
    let t = Tokens::get(dark_mode);
    let bar_rect = ui.available_rect_before_wrap();
    let y_bottom = bar_rect.min.y + 46.0;
    let mut action = ToolbarAction::None;

    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), 46.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.add_space(16.0);

            // Filter cells
            widgets::filter_cells(ui, selected_category, options, dark_mode);

            // Right-aligned search & action buttons
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(16.0);

                // RUN PURGE button (Primary solid invert)
                if widgets::button(ui, "RUN PURGE", ButtonKind::Primary, 0.0, dark_mode).clicked() {
                    action = ToolbarAction::Purge;
                }

                ui.add_space(8.0);

                // ANALYZE button (Secondary outlined)
                if widgets::button(ui, "ANALYZE", ButtonKind::Secondary, 0.0, dark_mode).clicked() {
                    action = ToolbarAction::Analyze;
                }

                ui.add_space(8.0);

                // Search target input box
                let text_edit = egui::TextEdit::singleline(search_query)
                    .hint_text("Search targets")
                    .desired_width(150.0)
                    .margin(egui::Margin::symmetric(10, 8));
                ui.add(text_edit);
            });
        },
    );

    // Hairline divider below toolbar
    ui.painter()
        .hline(bar_rect.x_range(), y_bottom - 0.5, t.hairline());
    action
}

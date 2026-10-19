use super::model::{TargetItem, TargetStatus};
use super::theme::{self, Tokens};
use super::widgets;
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Ui, vec2};

#[derive(Default)]
pub struct GridActions {
    pub purge_id: Option<String>,
    pub rule_id: Option<String>,
}

pub fn render_grid(ui: &mut Ui, items: &mut [&mut TargetItem], dark_mode: bool) -> GridActions {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;
    let mut actions = GridActions::default();

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
            let item = &mut **item;
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
                        if widgets::nav_link(ui, "RULES ↗", 40.0, false, dark_mode).clicked() {
                            actions.rule_id = Some(item.id.to_string());
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
                    let (status_color, status_text) = match item.status {
                        TargetStatus::Available => (t.accent_ink, "[ AVAILABLE ]"),
                        TargetStatus::Guarded => (t.warn, "[ GUARDED AUDIT ]"),
                        TargetStatus::Cleaned => (t.good, "[ CLEANED ]"),
                    };
                    widgets::badge(ui, status_text, status_color, dark_mode);

                    ui.add_space(8.0);
                    let meta_text = format!("{} · {} space", item.count, item.size);
                    ui.label(
                        RichText::new(meta_text)
                            .font(egui::FontId::new(11.5, theme::mono()))
                            .color(t.muted),
                    );

                    ui.add_space(16.0);

                    // Actions row at bottom
                    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            if widgets::icon_button(ui, dark_mode).clicked() {
                                actions.rule_id = Some(item.id.to_string());
                            }
                            ui.add_space(6.0);
                            let purge_btn = widgets::button(
                                ui,
                                "PURGE TARGET",
                                widgets::ButtonKind::Primary,
                                ui.available_width() - 8.0,
                                dark_mode,
                            );
                            if purge_btn.clicked() {
                                actions.purge_id = Some(item.id.to_string());
                            }
                        });
                    });
                });
            });
        }
    });
    actions
}

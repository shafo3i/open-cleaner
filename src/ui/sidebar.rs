use egui::{Align, Color32, CornerRadius, Layout, Rect, Sense, StrokeKind, Ui, vec2};

use super::theme::{self, Tokens};
use super::widgets;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AppTab {
    #[default]
    Cleaner,
    Registry,
    Uninstaller,
    Bloatware,
    AuditLogs,
}

pub struct SidebarCounts {
    pub cleaner_count: usize,
    pub registry_count: usize,
    pub uninstaller_count: usize,
    pub bloat_count: usize,
    pub logs_count: usize,
}

pub fn render_sidebar(
    ui: &mut Ui,
    active_tab: &mut AppTab,
    counts: &SidebarCounts,
    dark_mode: bool,
) {
    let t = Tokens::get(dark_mode);
    let sidebar_rect = ui.available_rect_before_wrap();

    // Bottom System Card pinned to the bottom of the sidebar
    egui::TopBottomPanel::bottom("sidebar_engine_status")
        .exact_height(94.0)
        .frame(egui::Frame::new().fill(t.bg))
        .show_inside(ui, |ui| {
            render_engine_status_card(ui, dark_mode);
        });

    // Navigation area (fills remaining height, scrolls if the window is short)
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(t.bg))
        .show_inside(ui, |ui| {
            ui.add_space(14.0);

            // Eyebrow Header
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                widgets::eyebrow(ui, "01", "SYSTEM MODULES", dark_mode);
            });

            ui.add_space(12.0);

            // Navigation Items
            let items = [
                (
                    AppTab::Cleaner,
                    "01",
                    "CLEANER",
                    "Disk & Privacy Caches",
                    counts.cleaner_count.to_string(),
                ),
                (
                    AppTab::Registry,
                    "02",
                    "REGISTRY",
                    "Integrity Verification",
                    counts.registry_count.to_string(),
                ),
                (
                    AppTab::Uninstaller,
                    "03",
                    "UNINSTALLER",
                    "Complete App Removal",
                    format!("{} APPS", counts.uninstaller_count),
                ),
                (
                    AppTab::Bloatware,
                    "04",
                    "BLOATWARE",
                    "System Debloat & Policy",
                    format!("{} ITEMS", counts.bloat_count),
                ),
                (
                    AppTab::AuditLogs,
                    "05",
                    "AUDIT LOGS",
                    "Forensic Activity",
                    format!("{} LOGS", counts.logs_count),
                ),
            ];

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (tab, num, label, subtext, badge_str) in items {
                        let is_active = *active_tab == tab;
                        let item_w = ui.available_width() - 20.0;
                        let item_h = 58.0;

                        ui.horizontal(|ui| {
                            ui.add_space(10.0);
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(item_w, item_h), Sense::click());
                            let hovered = resp.hovered();

                            // Background & border rendering
                            let fill = if is_active {
                                t.sunken
                            } else if hovered {
                                if dark_mode {
                                    Color32::from_rgb(0x1a, 0x1b, 0x20)
                                } else {
                                    Color32::from_rgb(0xe5, 0xe4, 0xe0)
                                }
                            } else {
                                Color32::TRANSPARENT
                            };

                            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);

                            if is_active {
                                // Left accent indicator bar (3px)
                                let indicator_rect =
                                    Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
                                ui.painter().rect_filled(
                                    indicator_rect,
                                    CornerRadius::ZERO,
                                    t.accent,
                                );
                                ui.painter().rect_stroke(
                                    rect,
                                    CornerRadius::ZERO,
                                    t.hairline(),
                                    StrokeKind::Inside,
                                );
                            } else if hovered {
                                ui.painter().rect_stroke(
                                    rect,
                                    CornerRadius::ZERO,
                                    t.hairline(),
                                    StrokeKind::Inside,
                                );
                            }

                            // Content layout inside item card
                            let content_rect = rect.shrink2(vec2(12.0, 8.0));

                            // Row 1: Number + Label on left, Badge on right
                            let num_g = widgets::hud_galley(
                                ui,
                                num,
                                if is_active { t.accent } else { t.faint },
                            );
                            let label_font = egui::FontId::new(12.0, theme::display());
                            let label_format = egui::TextFormat {
                                font_id: label_font,
                                color: if is_active {
                                    t.ink_strong
                                } else if hovered {
                                    t.ink
                                } else {
                                    t.muted
                                },
                                extra_letter_spacing: 1.0,
                                ..Default::default()
                            };
                            let label_g =
                                ui.painter()
                                    .layout_job(egui::text::LayoutJob::single_section(
                                        label.to_string(),
                                        label_format,
                                    ));

                            // Paint number and label
                            let num_pos =
                                egui::pos2(content_rect.left() + 2.0, content_rect.top() + 2.0);
                            ui.painter().galley_with_override_text_color(
                                num_pos,
                                num_g.clone(),
                                if is_active { t.accent } else { t.faint },
                            );

                            let label_pos = egui::pos2(
                                content_rect.left() + num_g.size().x + 8.0,
                                content_rect.top() + 2.0,
                            );
                            ui.painter().galley_with_override_text_color(
                                label_pos,
                                label_g,
                                if is_active {
                                    t.ink_strong
                                } else if hovered {
                                    t.ink
                                } else {
                                    t.muted
                                },
                            );

                            // Badge on the right
                            let badge_g = widgets::hud_galley(
                                ui,
                                &badge_str,
                                if is_active { t.invert_fg } else { t.faint },
                            );
                            if is_active {
                                let badge_pad = vec2(10.0, 4.0);
                                let badge_rect = Rect::from_min_size(
                                    egui::pos2(
                                        content_rect.right() - badge_g.size().x - badge_pad.x,
                                        content_rect.top() + 1.0,
                                    ),
                                    badge_g.size() + badge_pad,
                                );
                                ui.painter()
                                    .rect_filled(badge_rect, CornerRadius::ZERO, t.invert_bg);
                                let badge_text_pos =
                                    egui::pos2(badge_rect.left() + 5.0, badge_rect.top() + 2.0);
                                ui.painter().galley_with_override_text_color(
                                    badge_text_pos,
                                    badge_g,
                                    t.invert_fg,
                                );
                            } else {
                                let badge_pos = egui::pos2(
                                    content_rect.right() - badge_g.size().x - 4.0,
                                    content_rect.top() + 3.0,
                                );
                                ui.painter().galley_with_override_text_color(
                                    badge_pos,
                                    badge_g,
                                    t.faint,
                                );
                            }

                            // Row 2: Subtitle
                            let sub_g = widgets::hud_galley(
                                ui,
                                subtext,
                                if is_active { t.muted } else { t.faint },
                            );
                            let sub_pos = egui::pos2(
                                content_rect.left() + 2.0,
                                content_rect.bottom() - sub_g.size().y - 2.0,
                            );
                            ui.painter().galley_with_override_text_color(
                                sub_pos,
                                sub_g,
                                if is_active { t.muted } else { t.faint },
                            );

                            if resp.clicked() && !is_active {
                                *active_tab = tab;
                            }
                        });

                        ui.add_space(4.0);
                    }
                });
        });

    // Hairline vertical divider along the right edge of the sidebar
    ui.painter().vline(
        sidebar_rect.right() - 0.5,
        sidebar_rect.y_range(),
        t.hairline(),
    );
}

fn render_engine_status_card(ui: &mut Ui, dark_mode: bool) {
    let t = Tokens::get(dark_mode);

    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let card_w = ui.available_width() - 20.0;
        let card_h = 84.0;
        let (card_rect, _) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::hover());
        ui.painter()
            .rect_filled(card_rect, CornerRadius::ZERO, t.sunken);
        ui.painter().rect_stroke(
            card_rect,
            CornerRadius::ZERO,
            t.hairline(),
            StrokeKind::Inside,
        );

        let c_rect = card_rect.shrink2(vec2(12.0, 10.0));
        let mut card_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(c_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        card_ui.spacing_mut().item_spacing.y = 7.0;

        widgets::hud(&mut card_ui, "ENGINE STATUS", t.faint);

        card_ui.horizontal(|ui| {
            let (dot_rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter().circle_filled(dot_rect.center(), 3.5, t.good);
            ui.add_space(4.0);
            widgets::hud(ui, "PRIVACY ENGINE ACTIVE", t.ink_strong);
        });

        widgets::hud(&mut card_ui, "DOD 0x00 ZERO-FILL READY", t.muted);
    });
}

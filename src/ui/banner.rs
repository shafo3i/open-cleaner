use super::theme::{self, Tokens};
use super::widgets;
use egui::{Align, CornerRadius, Layout, RichText, Ui};

pub struct AuditMetrics {
    pub reclaimable_formatted: String,
    pub scanned_count: usize,
    pub locked_count: usize,
    pub duration_secs: f64,
    pub is_busy: bool,
    pub status_text: String,
}

impl Default for AuditMetrics {
    fn default() -> Self {
        Self {
            reclaimable_formatted: "3.69 GB".to_string(),
            scanned_count: 13300,
            locked_count: 375,
            duration_secs: 0.96,
            is_busy: false,
            status_text: "AUDIT COMPLETE".to_string(),
        }
    }
}

pub fn render_banner(ui: &mut Ui, metrics: &AuditMetrics, secure_wipe: &mut bool, dark_mode: bool) {
    let t = Tokens::get(dark_mode);
    let full_w = ui.available_width() - 32.0;

    ui.horizontal(|ui| {
        ui.add_space(16.0);

        let banner_frame = egui::Frame::new()
            .fill(t.sunken)
            .stroke(t.hairline())
            .corner_radius(CornerRadius::ZERO)
            .inner_margin(egui::Margin::same(16));

        banner_frame.show(ui, |ui| {
            ui.set_width(full_w);
            ui.horizontal(|ui| {
                // Metric 1: Reclaimable Storage (Big Display Heading)
                ui.vertical(|ui| {
                    widgets::hud(ui, "RECLAIMABLE STORAGE", t.faint);
                    ui.add_space(2.0);
                    let big_text = RichText::new(&metrics.reclaimable_formatted)
                        .font(egui::FontId::new(24.0, theme::display()))
                        .color(t.ink_strong);
                    ui.label(big_text);
                });

                ui.add_space(28.0);
                widgets::rule(ui, 42.0, dark_mode);
                ui.add_space(28.0);

                // Metric 2: Scanned Target Objects
                ui.vertical(|ui| {
                    widgets::hud(ui, "SCANNED OBJECTS", t.faint);
                    ui.add_space(2.0);
                    let count_str = format_thousands(metrics.scanned_count);
                    let mid_text = RichText::new(count_str)
                        .font(egui::FontId::new(24.0, theme::display()))
                        .color(t.ink_strong);
                    ui.label(mid_text);
                });

                ui.add_space(28.0);
                widgets::rule(ui, 42.0, dark_mode);
                ui.add_space(28.0);

                // Metric 3: Active Locks / Protected
                ui.vertical(|ui| {
                    widgets::hud(ui, "LOCKED / SKIPPED", t.faint);
                    ui.add_space(2.0);
                    let lock_str = format_thousands(metrics.locked_count);
                    let mid_text = RichText::new(lock_str)
                        .font(egui::FontId::new(24.0, theme::display()))
                        .color(t.warn);
                    ui.label(mid_text);
                });

                ui.add_space(28.0);
                widgets::rule(ui, 42.0, dark_mode);
                ui.add_space(28.0);

                // Metric 4: Audit Duration
                ui.vertical(|ui| {
                    widgets::hud(ui, "AUDIT TIME", t.faint);
                    ui.add_space(2.0);
                    let time_str = format!("{:.2}s", metrics.duration_secs);
                    let mid_text = RichText::new(time_str)
                        .font(egui::FontId::new(24.0, theme::display()))
                        .color(t.ink_strong);
                    ui.label(mid_text);
                });

                // Right side: Secure Wipe / Zero-Fill sanitization toggle
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(12.0);
                    let chk_label = RichText::new("BEST-EFFORT ZERO-OVERWRITE")
                        .font(egui::FontId::new(11.0, theme::mono_bold()))
                        .color(if *secure_wipe { t.accent_ink } else { t.muted });
                    ui.checkbox(secure_wipe, chk_label);
                });
            });
        });
    });
}

fn format_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

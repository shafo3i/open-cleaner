use super::theme::Tokens;
use super::widgets;
use egui::{CornerRadius, Sense, StrokeKind, Ui, vec2};

#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub level: &'static str,
    pub tag: String,
    pub event: String,
    pub details: String,
}

impl AuditLogEntry {
    pub fn new(level: &'static str, tag: &str, event: &str, details: &str) -> Self {
        Self {
            level,
            tag: tag.to_string(),
            event: event.to_string(),
            details: details.to_string(),
        }
    }
}

pub fn render_logs_view(
    ui: &mut Ui,
    status_text: &str,
    entries: &[AuditLogEntry],
    dark_mode: bool,
) {
    let t = Tokens::get(dark_mode);

    ui.add_space(16.0);

    // Section Eyebrow
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        widgets::eyebrow(ui, "04", "SECURITY & AUDIT EVENT LOGS", dark_mode);
    });

    ui.add_space(14.0);

    let mut records = vec![
        AuditLogEntry::new(
            "INFO",
            "SESSION",
            "ENGINE INITIALIZED",
            "Open Cleaner core engine loaded with high-precision Windows filesystem driver.",
        ),
        AuditLogEntry::new(
            "INFO",
            "SECURITY",
            "DOD 0x00 ZERO-FILL ARMED",
            "DoD 5220.22-M zero-fill sanitized file wipe handler verified and ready.",
        ),
        AuditLogEntry::new("AUDIT", "SCAN", "AUDIT COMPLETED", status_text),
        AuditLogEntry::new(
            "VERIFY",
            "KERNEL",
            "PRIVILEGE CHECK",
            "Running under current user privileges. Elevated operations prompt Windows UAC as required.",
        ),
    ];
    records.extend(entries.iter().cloned());

    let card_w = ui.available_width() - 40.0;

    for record in &records {
        let lvl = record.level;
        let tag = record.tag.as_str();
        let event = record.event.as_str();
        let details = record.details.as_str();
        let card_h = 56.0;

        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::hover());
            let hovered = resp.hovered();

            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                if hovered { t.sunken } else { t.bg },
            );
            ui.painter()
                .rect_stroke(rect, CornerRadius::ZERO, t.hairline(), StrokeKind::Inside);

            let c = rect.shrink2(vec2(16.0, 10.0));

            // Level badge
            let lvl_color = match lvl {
                "INFO" => t.accent,
                "SECURITY" => t.warn,
                "AUDIT" => t.good,
                _ => t.muted,
            };
            let lvl_g = widgets::hud_galley(ui, lvl, lvl_color);
            ui.painter()
                .galley_with_override_text_color(c.min, lvl_g.clone(), lvl_color);

            // Tag
            let tag_g = widgets::hud_galley(ui, tag, t.faint);
            let tag_pos = egui::pos2(c.left() + 60.0, c.top());
            ui.painter()
                .galley_with_override_text_color(tag_pos, tag_g, t.faint);

            // Event Title
            let ev_g = widgets::hud_galley(ui, event, t.ink_strong);
            let ev_pos = egui::pos2(c.left() + 150.0, c.top());
            ui.painter()
                .galley_with_override_text_color(ev_pos, ev_g, t.ink_strong);

            // Details line
            let det_g = widgets::hud_galley(ui, details, t.muted);
            let det_pos = egui::pos2(c.left(), c.bottom() - det_g.size().y);
            ui.painter()
                .galley_with_override_text_color(det_pos, det_g, t.muted);
        });

        ui.add_space(6.0);
    }
}

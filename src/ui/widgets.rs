use super::theme::{self, Tokens};
use egui::{Color32, CornerRadius, Galley, Rect, Response, Sense, Stroke, Ui, vec2};
use std::sync::Arc;

pub fn caps(
    ui: &Ui,
    text: &str,
    size: f32,
    family: egui::FontFamily,
    spacing: f32,
    color: Color32,
) -> Arc<Galley> {
    let format = egui::TextFormat {
        font_id: egui::FontId::new(size, family),
        color,
        extra_letter_spacing: spacing,
        ..Default::default()
    };
    ui.painter()
        .layout_job(egui::text::LayoutJob::single_section(
            text.to_uppercase(),
            format,
        ))
}

pub fn hud_galley(ui: &Ui, text: &str, color: Color32) -> Arc<Galley> {
    caps(
        ui,
        text,
        theme::HUD_SIZE,
        theme::mono(),
        theme::HUD_SPACING,
        color,
    )
}

pub fn paint_left(ui: &Ui, rect: Rect, galley: Arc<Galley>, color: Color32) {
    let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter()
        .galley_with_override_text_color(pos, galley, color);
}

pub fn paint_centred(ui: &Ui, rect: Rect, galley: Arc<Galley>, color: Color32) {
    ui.painter().galley_with_override_text_color(
        rect.center() - galley.size() / 2.0,
        galley,
        color,
    );
}

pub fn hud(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let galley = hud_galley(ui, text, color);
    let (rect, resp) = ui.allocate_exact_size(galley.size(), Sense::hover());
    paint_left(ui, rect, galley, color);
    resp
}

pub fn eyebrow(ui: &mut Ui, index: &str, label: &str, dark: bool) {
    let t = Tokens::get(dark);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        hud(ui, index, t.faint);
        hud(ui, " / ", t.faint);
        hud(ui, label, t.muted);
    });
}

pub fn tag(ui: &mut Ui, text: &str, fill: Color32, ink: Color32) -> Response {
    let galley = caps(
        ui,
        text,
        theme::HUD_SIZE,
        theme::mono_bold(),
        theme::HUD_SPACING,
        ink,
    );
    let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(16.0, 8.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
    paint_centred(ui, rect, galley, ink);
    resp
}

pub fn badge(ui: &mut Ui, text: &str, color: Color32, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let galley = caps(ui, text, 10.0, theme::mono(), 1.0, color);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 16.0, 22.0), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        t.sunken,
        t.hairline(),
        egui::StrokeKind::Inside,
    );
    paint_centred(ui, rect, galley, color);
    resp
}

pub fn rule(ui: &mut Ui, height: f32, dark: bool) {
    let t = Tokens::get(dark);
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    ui.painter()
        .vline(rect.center().x, rect.y_range(), t.hairline());
}

pub fn nav_link(ui: &mut Ui, text: &str, height: f32, active: bool, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let galley = hud_galley(ui, text, t.muted);
    let (rect, resp) = ui.allocate_exact_size(vec2(galley.size().x + 20.0, height), Sense::click());
    let on = active || resp.hovered();

    if on {
        let span = Rect::from_center_size(rect.center(), galley.size() + vec2(12.0, 6.0));
        ui.painter()
            .rect_filled(span, CornerRadius::ZERO, t.invert_bg);
    }
    paint_centred(ui, rect, galley, if on { t.invert_fg } else { t.muted });
    resp
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
}

pub fn button(ui: &mut Ui, text: &str, kind: ButtonKind, width: f32, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let enabled = ui.is_enabled();

    let galley = caps(
        ui,
        text,
        theme::HUD_SIZE,
        theme::mono_bold(),
        theme::BUTTON_SPACING,
        t.ink,
    );
    let final_w = (galley.size().x + 28.0).max(width);
    let (rect, resp) = ui.allocate_exact_size(vec2(final_w, theme::CONTROL_H), Sense::click());
    let hovered = resp.hovered() && enabled;

    let (fill, stroke, text_color) = if !enabled {
        // Disabled state: match the app surface, never the inverted (white) primary fill.
        (t.sunken, t.hairline(), t.faint)
    } else {
        match kind {
            ButtonKind::Primary => (
                t.invert_bg.gamma_multiply(if hovered { 0.85 } else { 1.0 }),
                Stroke::NONE,
                t.invert_fg,
            ),
            ButtonKind::Secondary if hovered => (t.invert_bg, Stroke::NONE, t.invert_fg),
            ButtonKind::Secondary => (t.bg, Stroke::new(1.0_f32, t.line_strong), t.ink),
        }
    };

    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    paint_centred(ui, rect, galley, text_color);
    resp
}

pub fn icon_button(ui: &mut Ui, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let (rect, resp) =
        ui.allocate_exact_size(vec2(theme::CONTROL_H, theme::CONTROL_H), Sense::click());
    let hovered = resp.hovered();
    let (fill, stroke, dot_color) = if hovered {
        (t.invert_bg, Stroke::NONE, t.invert_fg)
    } else {
        (t.bg, Stroke::new(1.0_f32, t.line_strong), t.muted)
    };
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::ZERO,
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );

    // Draw 3 crisp dots
    let center = rect.center();
    let dot_r = 1.6;
    p.circle_filled(center + vec2(-5.0, 0.0), dot_r, dot_color);
    p.circle_filled(center, dot_r, dot_color);
    p.circle_filled(center + vec2(5.0, 0.0), dot_r, dot_color);
    resp
}

pub fn theme_toggle_button(ui: &mut Ui, height: f32, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let (rect, resp) = ui.allocate_exact_size(vec2(height, height), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    if hovered {
        p.rect_filled(rect, CornerRadius::ZERO, t.invert_bg);
    }
    p.vline(rect.left() + 0.5, rect.y_range(), t.hairline());
    let icon_color = if hovered { t.invert_fg } else { t.muted };
    let c = rect.center();

    if dark {
        // Draw crisp vector Sun
        let r_disk = 3.5;
        p.circle_filled(c, r_disk, icon_color);
        for i in 0..8 {
            let angle = (i as f32) * std::f32::consts::PI / 4.0;
            let dir = vec2(angle.cos(), angle.sin());
            p.line_segment(
                [c + dir * 5.2, c + dir * 7.5],
                Stroke::new(1.3_f32, icon_color),
            );
        }
    } else {
        // Draw crisp vector Moon crescent
        let r_outer = 6.0;
        let segs = 12;
        let mut outer_pts = Vec::with_capacity(segs + 1);
        for i in 0..=segs {
            let a =
                -std::f32::consts::FRAC_PI_2 + (std::f32::consts::PI) * (i as f32 / segs as f32);
            outer_pts.push(c + vec2(a.cos() * r_outer, a.sin() * r_outer));
        }
        for w in outer_pts.windows(2) {
            p.line_segment([w[0], w[1]], Stroke::new(1.4_f32, icon_color));
        }
        let r_inner = 5.2;
        let inner_c = c + vec2(2.4, 0.0);
        let mut inner_pts = Vec::with_capacity(segs + 1);
        for i in 0..=segs {
            let a =
                -std::f32::consts::FRAC_PI_2 + (std::f32::consts::PI) * (i as f32 / segs as f32);
            inner_pts.push(inner_c + vec2(a.cos() * r_inner, a.sin() * r_inner));
        }
        for w in inner_pts.windows(2) {
            p.line_segment([w[0], w[1]], Stroke::new(1.4_f32, icon_color));
        }
    }

    resp.on_hover_text(if dark {
        "Switch to light mode"
    } else {
        "Switch to dark mode"
    })
}

pub fn refresh_button(ui: &mut Ui, height: f32, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let (rect, resp) = ui.allocate_exact_size(vec2(height, height), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    if hovered {
        p.rect_filled(rect, CornerRadius::ZERO, t.invert_bg);
    }
    p.vline(rect.left() + 0.5, rect.y_range(), t.hairline());
    let icon_color = if hovered { t.invert_fg } else { t.muted };
    let c = rect.center();

    let r = 5.5;
    let segs = 12;
    let start_angle = 0.4 * std::f32::consts::PI;
    let end_angle = 2.1 * std::f32::consts::PI;
    let points: Vec<egui::Pos2> = (0..=segs)
        .map(|i| {
            let a = start_angle + (end_angle - start_angle) * (i as f32 / segs as f32);
            c + vec2(a.cos() * r, a.sin() * r)
        })
        .collect();
    for w in points.windows(2) {
        p.line_segment([w[0], w[1]], Stroke::new(1.3_f32, icon_color));
    }
    if let Some(&last) = points.last() {
        p.line_segment(
            [last, last + vec2(-3.0, 0.5)],
            Stroke::new(1.3_f32, icon_color),
        );
        p.line_segment(
            [last, last + vec2(0.5, -3.0)],
            Stroke::new(1.3_f32, icon_color),
        );
    }

    resp.on_hover_text("Audit / Refresh targets")
}

pub struct FilterOption<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub count: usize,
}

pub fn filter_cells(
    ui: &mut Ui,
    selected: &mut String,
    options: &[FilterOption<'_>],
    dark: bool,
) -> bool {
    let t = Tokens::get(dark);
    let mut changed = false;
    let mut rects = Vec::with_capacity(options.len());

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for opt in options {
            let is_on = *selected == opt.key;
            let count_str = opt.count.to_string();
            let label_g = hud_galley(ui, opt.label, t.ink);
            let count_g = hud_galley(ui, &count_str, t.faint);
            let w = label_g.size().x + count_g.size().x + 22.0;

            let (rect, resp) = ui.allocate_exact_size(vec2(w, theme::CONTROL_H), Sense::click());
            let hovered = resp.hovered();

            let (fill, ink, count_ink) = match (is_on, hovered) {
                (true, _) => (t.invert_bg, t.invert_fg, t.invert_fg),
                (false, true) => (t.sunken, t.ink_strong, t.muted),
                (false, false) => (Color32::TRANSPARENT, t.muted, t.faint),
            };

            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);

            let text_rect = rect.shrink2(vec2(9.0, 0.0));
            paint_left(ui, text_rect, label_g, ink);

            let pos_c = egui::pos2(
                text_rect.right() - count_g.size().x,
                text_rect.center().y - count_g.size().y / 2.0,
            );
            ui.painter()
                .galley_with_override_text_color(pos_c, count_g, count_ink);

            if resp.clicked() && !is_on {
                *selected = opt.key.to_string();
                changed = true;
            }
            rects.push(rect);
        }
    });

    if let (Some(first), Some(last)) = (rects.first(), rects.last()) {
        let p = ui.painter();
        p.rect_stroke(
            first.union(*last),
            CornerRadius::ZERO,
            t.hairline(),
            egui::StrokeKind::Inside,
        );
        let (top, bottom) = (first.top(), first.bottom());
        for r in rects.iter().skip(1) {
            p.vline(r.left(), top..=bottom, t.hairline());
        }
    }

    changed
}

// ---------------------------------------------------------------------------
// Custom Frameless Window Controls (Title Bar)
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowBtnKind {
    Minimize,
    Maximize,
    Restore,
    Close,
}

pub fn window_controls(ui: &mut Ui, height: f32, dark: bool) {
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

    // IN RIGHT-TO-LEFT LAYOUT:
    // 1st button added goes to the EXTREME RIGHT -> Close
    if window_btn(ui, WindowBtnKind::Close, height, dark).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    // 2nd button added goes to the left of Close -> Maximize / Restore
    let max_kind = if maximized {
        WindowBtnKind::Restore
    } else {
        WindowBtnKind::Maximize
    };
    if window_btn(ui, max_kind, height, dark).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }

    // 3rd button added goes to the left of Maximize -> Minimize
    if window_btn(ui, WindowBtnKind::Minimize, height, dark).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    // Hairline divider to the left of Minimize
    rule(ui, height, dark);
}

fn window_btn(ui: &mut Ui, kind: WindowBtnKind, height: f32, dark: bool) -> Response {
    let t = Tokens::get(dark);
    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, height), Sense::click());
    let hovered = resp.hovered();

    let (fill, stroke_color) = match (kind, hovered) {
        (WindowBtnKind::Close, true) => (Color32::from_rgb(0xc4, 0x2b, 0x1c), Color32::WHITE),
        (_, true) => (t.sunken, t.ink_strong),
        (_, false) => (Color32::TRANSPARENT, t.muted),
    };

    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::ZERO, fill);

    let c = rect.center();
    let stroke = Stroke::new(1.1_f32, stroke_color);
    let s = 4.5;

    match kind {
        WindowBtnKind::Minimize => {
            p.hline(c.x - s..=c.x + s, c.y, stroke);
        }
        WindowBtnKind::Maximize => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(2.0 * s, 2.0 * s)),
                CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        WindowBtnKind::Restore => {
            let front =
                Rect::from_min_size(c + vec2(-s, -s + 2.0), vec2(2.0 * s - 2.0, 2.0 * s - 2.0));
            p.rect_stroke(front, CornerRadius::ZERO, stroke, egui::StrokeKind::Inside);
            p.hline(c.x - s + 2.0..=c.x + s, c.y - s, stroke);
            p.vline(c.x + s, c.y - s..=c.y + s - 2.0, stroke);
        }
        WindowBtnKind::Close => {
            p.line_segment([c + vec2(-s, -s), c + vec2(s, s)], stroke);
            p.line_segment([c + vec2(-s, s), c + vec2(s, -s)], stroke);
        }
    }

    let tip = match kind {
        WindowBtnKind::Minimize => "Minimize",
        WindowBtnKind::Maximize => "Maximize",
        WindowBtnKind::Restore => "Restore",
        WindowBtnKind::Close => "Close",
    };
    resp.on_hover_text(tip)
}

const RESIZE_EDGE: f32 = 5.0;

pub fn handle_title_drag(ui: &mut Ui, rect: Rect) {
    let hit = Rect::from_min_max(
        rect.min + vec2(RESIZE_EDGE, RESIZE_EDGE),
        rect.max - vec2(RESIZE_EDGE, 0.0),
    );
    let resp = ui.interact(hit, ui.id().with("title-bar-drag"), Sense::click_and_drag());
    if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if resp.double_clicked() {
        let is_max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
    }
}

pub fn window_edges(ctx: &egui::Context, dark: bool) {
    use egui::CursorIcon as C;
    use egui::viewport::ResizeDirection as D;
    let (maximized, fullscreen) = ctx.input(|i| {
        (
            i.viewport().maximized.unwrap_or(false),
            i.viewport().fullscreen.unwrap_or(false),
        )
    });
    if maximized || fullscreen {
        return;
    }
    let t = Tokens::get(dark);
    let r = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("window-border"),
    ))
    .rect_stroke(
        r,
        CornerRadius::ZERO,
        t.hairline(),
        egui::StrokeKind::Inside,
    );
    let Some(p) = ctx
        .input(|i| i.pointer.hover_pos())
        .filter(|p| r.contains(*p))
    else {
        return;
    };
    let near = |d: f32| d < RESIZE_EDGE;
    let corner = |d: f32| d < 3.0 * RESIZE_EDGE;
    let (left, right, top, bottom) = (
        p.x - r.left(),
        r.right() - p.x,
        p.y - r.top(),
        r.bottom() - p.y,
    );
    let hit = if (near(top) && corner(left)) || (near(left) && corner(top)) {
        Some((D::NorthWest, C::ResizeNorthWest))
    } else if (near(top) && corner(right)) || (near(right) && corner(top)) {
        Some((D::NorthEast, C::ResizeNorthEast))
    } else if (near(bottom) && corner(left)) || (near(left) && corner(bottom)) {
        Some((D::SouthWest, C::ResizeSouthWest))
    } else if (near(bottom) && corner(right)) || (near(right) && corner(bottom)) {
        Some((D::SouthEast, C::ResizeSouthEast))
    } else if near(top) {
        Some((D::North, C::ResizeNorth))
    } else if near(bottom) {
        Some((D::South, C::ResizeSouth))
    } else if near(left) {
        Some((D::West, C::ResizeWest))
    } else if near(right) {
        Some((D::East, C::ResizeEast))
    } else {
        None
    };
    if let Some((direction, cursor)) = hit {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
        }
    }
}

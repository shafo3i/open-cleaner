use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
    Visuals,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub dark: bool,
    pub bg: Color32,
    pub sunken: Color32,
    pub ink: Color32,
    pub ink_strong: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub line: Color32,
    pub line_strong: Color32,
    pub accent: Color32,
    pub accent_ink: Color32,
    pub invert_bg: Color32,
    pub invert_fg: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

const fn ink_alpha(rgb: u32, alpha: u8) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    let r_p = ((r as u32 * alpha as u32) / 255) as u8;
    let g_p = ((g as u32 * alpha as u32) / 255) as u8;
    let b_p = ((b as u32 * alpha as u32) / 255) as u8;
    Color32::from_rgba_premultiplied(r_p, g_p, b_p, alpha)
}

impl Tokens {
    pub const LIGHT: Self = Self {
        dark: false,
        bg: hex(0xf2f1ee),
        sunken: hex(0xe9e8e4),
        ink: hex(0x101014),
        ink_strong: hex(0x000000),
        muted: ink_alpha(0x101014, 158),
        faint: ink_alpha(0x101014, 107),
        line: ink_alpha(0x101014, 41),
        line_strong: ink_alpha(0x101014, 102),
        accent: hex(0x2d81ff),
        accent_ink: hex(0x1659c4),
        invert_bg: hex(0x101014),
        invert_fg: hex(0xf2f1ee),
        good: hex(0x0a7563),
        warn: hex(0x8a5800),
        bad: hex(0xb3261e),
    };

    pub const DARK: Self = Self {
        dark: true,
        bg: hex(0x121316),
        sunken: hex(0x060607),
        ink: hex(0xf2f1ee),
        ink_strong: hex(0xffffff),
        muted: ink_alpha(0xf2f1ee, 153),
        faint: ink_alpha(0xf2f1ee, 97),
        line: ink_alpha(0xf2f1ee, 38),
        line_strong: ink_alpha(0xf2f1ee, 102),
        accent: hex(0x2d81ff),
        accent_ink: hex(0x74aaff),
        invert_bg: hex(0xffffff),
        invert_fg: hex(0x0b0b0c),
        good: hex(0x3fd6b8),
        warn: hex(0xffc14d),
        bad: hex(0xff6259),
    };

    pub fn get(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }

    pub fn hairline(&self) -> Stroke {
        Stroke::new(1.0_f32, self.line)
    }
}

pub const CONTROL_H: f32 = 34.0;
pub const HEADER_H: f32 = 48.0;
pub const FOOTER_H: f32 = 34.0;
pub const HUD_SIZE: f32 = 11.0;
pub const HUD_SPACING: f32 = HUD_SIZE * 0.15;
pub const BUTTON_SPACING: f32 = HUD_SIZE * 0.12;

pub fn display() -> FontFamily {
    FontFamily::Name("display".into())
}

pub fn mono() -> FontFamily {
    FontFamily::Name("mono".into())
}

pub fn mono_bold() -> FontFamily {
    FontFamily::Name("mono-bold".into())
}

pub fn apply_visuals(ctx: &egui::Context, dark: bool) {
    let t = Tokens::get(dark);
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    v.override_text_color = Some(t.ink);
    v.panel_fill = t.bg;
    v.window_fill = t.bg;
    v.faint_bg_color = t.sunken;
    v.extreme_bg_color = t.sunken;

    v.widgets.noninteractive.bg_fill = t.bg;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, t.ink);
    v.widgets.noninteractive.bg_stroke = t.hairline();
    v.widgets.noninteractive.corner_radius = CornerRadius::ZERO;

    v.widgets.inactive.bg_fill = t.bg;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, t.muted);
    v.widgets.inactive.bg_stroke = t.hairline();
    v.widgets.inactive.corner_radius = CornerRadius::ZERO;

    v.widgets.hovered.bg_fill = t.sunken;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, t.ink_strong);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, t.line_strong);
    v.widgets.hovered.corner_radius = CornerRadius::ZERO;

    v.window_corner_radius = CornerRadius::ZERO;
    v.window_stroke = Stroke::new(1.0_f32, t.line_strong);
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;

    v.widgets.active.bg_fill = t.invert_bg;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, t.invert_fg);
    v.widgets.active.bg_stroke = Stroke::NONE;
    v.widgets.active.corner_radius = CornerRadius::ZERO;

    ctx.set_visuals(v);
}

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let display_bytes = include_bytes!("../../assets/fonts/Archivo-Variable.ttf");
    let mono_bytes = include_bytes!("../../assets/fonts/GeistMono-Variable.ttf");
    let body_bytes = include_bytes!("../../assets/fonts/Inter-Variable.ttf");

    fonts.font_data.insert(
        "Archivo".to_owned(),
        Arc::new(FontData::from_static(display_bytes)),
    );
    fonts.font_data.insert(
        "GeistMono".to_owned(),
        Arc::new(FontData::from_static(mono_bytes)),
    );
    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(FontData::from_static(body_bytes)),
    );

    fonts.families.insert(
        FontFamily::Name("display".into()),
        vec!["Archivo".to_owned(), "Inter".to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name("mono".into()),
        vec!["GeistMono".to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name("mono-bold".into()),
        vec!["GeistMono".to_owned()],
    );

    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "Inter".to_owned());
    }
    if let Some(m) = fonts.families.get_mut(&FontFamily::Monospace) {
        m.insert(0, "GeistMono".to_owned());
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(18.0, display())),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, mono())),
        (TextStyle::Button, FontId::new(11.0, mono())),
        (TextStyle::Small, FontId::new(10.5, mono())),
    ]
    .into();

    ctx.set_style(style);
}

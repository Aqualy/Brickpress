//! Design tokens and the egui style built from them.
//!
//! Colours come from docs/ui-design.md (light and dark, both WCAG AA for text
//! and 3:1 for control boundaries). The interface is deliberately quiet: the
//! inks, the paper and the pieces carry the colour.

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Theme,
    Vec2,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub workspace: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub selected: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_disabled: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub on_accent: Color32,
    pub boundary: Color32,
    pub divider: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub success: Color32,
    pub focus: Color32,
    pub shadow: Color32,
    pub grid: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub const LIGHT: Tokens = Tokens {
    workspace: hex(0xE9EBEF),
    panel: hex(0xF8F8FA),
    surface: hex(0xFFFFFF),
    hover: hex(0xEDEEF2),
    pressed: hex(0xE2E4E9),
    selected: hex(0xE4EDFC),
    text: hex(0x202124),
    text_secondary: hex(0x60646C),
    text_disabled: hex(0x9A9EA6),
    accent: hex(0x0057D8),
    accent_text: hex(0x0057D8),
    on_accent: hex(0xFFFFFF),
    boundary: hex(0x858B96),
    divider: hex(0xDFE1E6),
    danger: hex(0xB42318),
    warning: hex(0xA15C07),
    success: hex(0x067647),
    focus: hex(0x0057D8),
    shadow: Color32::from_black_alpha(28),
    grid: hex(0x747E8D),
};

pub const DARK: Tokens = Tokens {
    workspace: hex(0x191B20),
    panel: hex(0x232428),
    surface: hex(0x2C2E34),
    hover: hex(0x33353C),
    pressed: hex(0x3B3E45),
    selected: hex(0x293C5B),
    text: hex(0xF1F2F5),
    text_secondary: hex(0xB5BAC5),
    text_disabled: hex(0x7A7F89),
    accent: hex(0x2365D8),
    accent_text: hex(0x9BBCFF),
    on_accent: hex(0xFFFFFF),
    boundary: hex(0x858B98),
    divider: hex(0x393B42),
    danger: hex(0xF97066),
    warning: hex(0xFDB022),
    success: hex(0x47CD89),
    focus: hex(0x9BBCFF),
    shadow: Color32::from_black_alpha(90),
    grid: hex(0x747E8D),
};

pub fn tokens(ctx: &egui::Context) -> Tokens {
    tokens_for(ctx.theme())
}

pub fn tokens_for(theme: Theme) -> Tokens {
    match theme {
        Theme::Dark => DARK,
        Theme::Light => LIGHT,
    }
}

/// Spacing scale (points).
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

pub const CONTROL_HEIGHT: f32 = 28.0;
pub const TOOLBAR_HEIGHT: f32 = 44.0;
pub const RADIUS: u8 = 6;

pub fn medium() -> FontFamily {
    FontFamily::Name("medium".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}
pub fn icon_family() -> FontFamily {
    FontFamily::Name("icons".into())
}

pub fn heading_text_style() -> TextStyle {
    TextStyle::Name("section".into())
}

pub fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut fonts, "inter-400", include_bytes!("../assets/fonts/Inter-400.ttf"));
    add(&mut fonts, "inter-500", include_bytes!("../assets/fonts/Inter-500.ttf"));
    add(&mut fonts, "inter-600", include_bytes!("../assets/fonts/Inter-600.ttf"));
    add(&mut fonts, "plex-mono", include_bytes!("../assets/fonts/IBMPlexMono-400.ttf"));
    add(&mut fonts, "lucide", include_bytes!("../assets/fonts/lucide.ttf"));
    let fallbacks: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let chain = |first: &str| {
        let mut list = vec![first.to_string(), "lucide".to_string()];
        list.extend(fallbacks.iter().cloned());
        list
    };
    fonts.families.insert(FontFamily::Proportional, chain("inter-400"));
    fonts.families.insert(medium(), chain("inter-500"));
    fonts.families.insert(semibold(), chain("inter-600"));
    fonts.families.insert(FontFamily::Monospace, chain("plex-mono"));
    fonts.families.insert(icon_family(), vec!["lucide".into()]);
    fonts
}

/// Applies tokens to both light and dark styles.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    for theme in [Theme::Light, Theme::Dark] {
        ctx.style_mut_of(theme, |style| apply(style, tokens_for(theme), theme == Theme::Dark));
    }
}

fn apply(style: &mut egui::Style, t: Tokens, dark: bool) {
    style.text_styles = [
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, medium())),
        (TextStyle::Heading, FontId::new(15.0, semibold())),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        (heading_text_style(), FontId::new(12.0, semibold())),
    ]
    .into();
    let s = &mut style.spacing;
    s.item_spacing = Vec2::new(space::SM, 6.0);
    s.button_padding = Vec2::new(10.0, 4.0);
    s.interact_size = Vec2::new(28.0, CONTROL_HEIGHT);
    s.menu_margin = Margin::same(6);
    s.window_margin = Margin::same(16);
    s.slider_width = 140.0;
    s.combo_width = 140.0;
    s.icon_width = 16.0;
    s.icon_width_inner = 10.0;
    s.icon_spacing = 6.0;
    s.tooltip_width = 280.0;
    s.menu_width = 220.0;
    s.combo_height = 320.0;
    s.text_edit_width = 160.0;
    style.animation_time = 0.12;
    style.explanation_tooltips = false;
    style.url_in_tooltip = false;

    let v = &mut style.visuals;
    *v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.dark_mode = dark;
    v.panel_fill = t.panel;
    v.window_fill = t.surface;
    v.window_stroke = Stroke::new(1.0, t.divider);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_shadow = Shadow { offset: [0, 8], blur: 24, spread: 0, color: t.shadow };
    v.popup_shadow = Shadow { offset: [0, 4], blur: 14, spread: 0, color: t.shadow };
    v.extreme_bg_color = t.surface;
    v.text_edit_bg_color = Some(t.surface);
    v.faint_bg_color = t.hover;
    v.code_bg_color = t.hover;
    v.hyperlink_color = t.accent_text;
    v.warn_fg_color = t.warning;
    v.error_fg_color = t.danger;
    v.selection.bg_fill = if dark { t.accent } else { Color32::from_rgb(0xB9, 0xD2, 0xFA) };
    v.selection.stroke = Stroke::new(1.0, t.accent_text);
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.striped = false;
    v.indent_has_left_vline = false;
    v.collapsing_header_frame = false;
    v.disabled_alpha = 0.45;
    v.text_cursor.stroke = Stroke::new(1.5, t.accent_text);

    let radius = CornerRadius::same(RADIUS);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = t.panel;
    w.noninteractive.weak_bg_fill = t.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    w.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = t.surface;
    w.inactive.weak_bg_fill = t.surface;
    w.inactive.bg_stroke = Stroke::new(1.0, t.boundary.gamma_multiply(0.75));
    w.inactive.fg_stroke = Stroke::new(1.0, t.text);
    w.inactive.corner_radius = radius;
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = t.hover;
    w.hovered.weak_bg_fill = t.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, t.boundary);
    w.hovered.fg_stroke = Stroke::new(1.0, t.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = t.pressed;
    w.active.weak_bg_fill = t.pressed;
    w.active.bg_stroke = Stroke::new(1.0, t.accent_text);
    w.active.fg_stroke = Stroke::new(1.0, t.text);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;

    w.open.bg_fill = t.selected;
    w.open.weak_bg_fill = t.selected;
    w.open.bg_stroke = Stroke::new(1.0, t.accent_text);
    w.open.fg_stroke = Stroke::new(1.0, t.text);
    w.open.corner_radius = radius;
}

/// Readable text colour for a swatch.
pub fn contrast_on(color: Color32) -> Color32 {
    let l = 0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b());
    if l > 0.35 {
        Color32::from_rgb(0x20, 0x21, 0x24)
    } else {
        Color32::WHITE
    }
}

fn linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn color32(hex: &str) -> Color32 {
    let [r, g, b] = brickpress_core::parse_color(hex).unwrap_or([0, 0, 0]);
    Color32::from_rgb(r, g, b)
}

pub fn hex_of(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratio(a: Color32, b: Color32) -> f32 {
        let l = |c: Color32| 0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b());
        let (x, y) = (l(a), l(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn tokens_meet_wcag_contrast() {
        for t in [LIGHT, DARK] {
            assert!(ratio(t.text, t.panel) >= 4.5);
            assert!(ratio(t.text_secondary, t.panel) >= 4.5);
            assert!(ratio(t.text_secondary, t.surface) >= 4.5);
            assert!(ratio(t.accent_text, t.selected) >= 4.5);
            assert!(ratio(t.on_accent, t.accent) >= 4.5);
            assert!(ratio(t.boundary, t.surface) >= 3.0);
            assert!(ratio(t.danger, t.panel) >= 4.5);
        }
    }
}

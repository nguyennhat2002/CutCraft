//! CutCraft visual tokens and egui style configuration.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Stroke, Style, TextStyle, Theme, Visuals,
};

/// Shared colors for the CutCraft shell.
///
/// Neutral charcoal surfaces with a cyan accent. The palette is original and
/// does not reuse proprietary editor artwork.
pub struct Palette;

impl Palette {
    pub const APP_BG: Color32 = Color32::from_rgb(0x15, 0x15, 0x15);
    pub const MENU_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x19);
    pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1d);
    pub const PANEL_BG: Color32 = Color32::from_rgb(0x20, 0x20, 0x21);
    pub const PANEL_RAISED: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2c);
    pub const CARD_BG: Color32 = Color32::from_rgb(0x24, 0x24, 0x25);
    pub const CARD_HOVER: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2d);
    pub const SURFACE: Color32 = Color32::from_rgb(0x30, 0x30, 0x33);
    pub const STATUS_BG: Color32 = Color32::from_rgb(0x10, 0x10, 0x11);
    pub const TRACK_BG: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1c);
    pub const TRACK_ALT_BG: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x21);
    pub const TRACK_HEADER_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x28);
    pub const RULER_BG: Color32 = Color32::from_rgb(0x17, 0x17, 0x19);

    pub const BORDER: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2d);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xa4, 0xa4, 0xa8);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x73, 0x73, 0x77);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x55, 0x55, 0x5a);

    pub const ACCENT: Color32 = Color32::from_rgb(0x18, 0xc5, 0xcf);
    pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x5b, 0xe7, 0xef);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x0e, 0x4a, 0x50);
    pub const ACCENT_BG: Color32 = Color32::from_rgb(0x10, 0x38, 0x3c);
    pub const ON_ACCENT: Color32 = Color32::from_rgb(0x04, 0x1a, 0x1c);
    pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0x5a, 0x5f);
    pub const CANVAS: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);

    pub const WIDGET_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2c);
    pub const WIDGET_HOVER: Color32 = Color32::from_rgb(0x34, 0x34, 0x38);
    pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(0x3d, 0x3d, 0x42);

    pub const INFO: Color32 = Self::ACCENT;
    pub const WARNING: Color32 = Color32::from_rgb(0xe5, 0xa8, 0x48);
    pub const ERROR: Color32 = Color32::from_rgb(0xf0, 0x65, 0x65);
    pub const SUCCESS: Color32 = Color32::from_rgb(0x38, 0xd3, 0x9f);
}

/// Force the dark CutCraft theme and install compact desktop spacing.
pub fn configure(context: &Context) {
    context.set_theme(Theme::Dark);
    context.style_mut_of(Theme::Dark, configure_style);
    context.style_mut_of(Theme::Light, configure_style);
}

fn configure_style(style: &mut Style) {
    style.visuals = dark_visuals();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(18.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(10.5, FontFamily::Proportional),
        ),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.interact_size = egui::vec2(26.0, 24.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style.spacing.scroll.bar_width = 8.0;
}

fn dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = Palette::PANEL_BG;
    visuals.window_fill = Palette::PANEL_RAISED;
    visuals.extreme_bg_color = Palette::APP_BG;
    visuals.faint_bg_color = Palette::WIDGET_BG;
    visuals.window_stroke = Stroke::new(1.0, Palette::BORDER);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(5);
    visuals.hyperlink_color = Palette::ACCENT;
    visuals.selection.bg_fill = Palette::ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, Palette::ACCENT);

    visuals.widgets.noninteractive.bg_fill = Palette::PANEL_BG;
    visuals.widgets.noninteractive.weak_bg_fill = Palette::PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Palette::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Palette::TEXT_SECONDARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = Palette::WIDGET_BG;
    visuals.widgets.inactive.weak_bg_fill = Palette::WIDGET_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Palette::BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = Palette::WIDGET_HOVER;
    visuals.widgets.hovered.weak_bg_fill = Palette::WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Palette::BORDER);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = Palette::WIDGET_ACTIVE;
    visuals.widgets.active.weak_bg_fill = Palette::WIDGET_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Palette::ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.widgets.open = visuals.widgets.active;
    visuals.disabled_alpha = 0.52;

    visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_selects_dark_theme_and_shell_colors() {
        let context = Context::default();
        configure(&context);

        assert_eq!(context.theme(), Theme::Dark);
        let style = context.style_of(Theme::Dark);
        assert_eq!(style.visuals.panel_fill, Palette::PANEL_BG);
        assert_eq!(style.visuals.extreme_bg_color, Palette::APP_BG);
        assert_eq!(
            style.text_styles.get(&TextStyle::Monospace),
            Some(&FontId::new(12.0, FontFamily::Monospace))
        );
    }

    #[test]
    fn palette_foregrounds_remain_visibly_separated_from_backgrounds() {
        assert!(Palette::TEXT_PRIMARY.r() > Palette::PANEL_BG.r());
        assert!(Palette::TEXT_SECONDARY.r() > Palette::PANEL_BG.r());
        assert!(Palette::TEXT_DISABLED.r() < Palette::TEXT_MUTED.r());
        assert!(Palette::PLAYHEAD.r() > Palette::TRACK_BG.r());
    }

    #[test]
    fn accent_is_cyan_teal() {
        assert!(Palette::ACCENT.g() > Palette::ACCENT.r());
        assert!(Palette::ACCENT.b() > Palette::ACCENT.r());
        assert!(Palette::ACCENT.b() > 0x80);
    }
}

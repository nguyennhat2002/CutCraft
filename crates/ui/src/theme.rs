//! Dark editor palette and egui style configuration.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Stroke, Style, TextStyle, Theme, Visuals,
};

/// Shared colors for the CutCraft shell.
///
/// The palette is original and uses a dark charcoal base with a restrained
/// teal accent. No proprietary editor artwork or brand assets are used.
pub struct Palette;

impl Palette {
    pub const APP_BG: Color32 = Color32::from_rgb(0x0d, 0x11, 0x14);
    pub const MENU_BG: Color32 = Color32::from_rgb(0x10, 0x15, 0x19);
    pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0x13, 0x1a, 0x1f);
    pub const PANEL_BG: Color32 = Color32::from_rgb(0x14, 0x1a, 0x1f);
    pub const PANEL_RAISED: Color32 = Color32::from_rgb(0x1b, 0x23, 0x29);
    pub const CARD_BG: Color32 = Color32::from_rgb(0x17, 0x1f, 0x24);
    pub const CARD_HOVER: Color32 = Color32::from_rgb(0x1d, 0x27, 0x2d);
    pub const STATUS_BG: Color32 = Color32::from_rgb(0x09, 0x0c, 0x0e);
    pub const TRACK_BG: Color32 = Color32::from_rgb(0x11, 0x17, 0x1b);
    pub const TRACK_ALT_BG: Color32 = Color32::from_rgb(0x14, 0x1b, 0x20);
    pub const TRACK_HEADER_BG: Color32 = Color32::from_rgb(0x17, 0x1f, 0x25);
    pub const RULER_BG: Color32 = Color32::from_rgb(0x0e, 0x14, 0x17);

    pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x32, 0x38);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x1b, 0x25, 0x2b);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xec, 0xf3, 0xf2);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xa8, 0xb8, 0xb8);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x77, 0x88, 0x8a);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x50, 0x60, 0x63);

    pub const ACCENT: Color32 = Color32::from_rgb(0x2d, 0xd4, 0xbf);
    pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x5e, 0xea, 0xd4);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x13, 0x4e, 0x4a);
    pub const ACCENT_BG: Color32 = Color32::from_rgb(0x0e, 0x2e, 0x2a);
    pub const ON_ACCENT: Color32 = Color32::from_rgb(0x06, 0x20, 0x1c);
    pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0x5f, 0x57);
    pub const CANVAS: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);

    pub const WIDGET_BG: Color32 = Color32::from_rgb(0x20, 0x2a, 0x30);
    pub const WIDGET_HOVER: Color32 = Color32::from_rgb(0x29, 0x35, 0x3c);
    pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(0x33, 0x41, 0x49);

    pub const INFO: Color32 = Self::ACCENT;
    pub const WARNING: Color32 = Color32::from_rgb(0xe8, 0xb4, 0x5a);
    pub const ERROR: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);
    pub const SUCCESS: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99);
}

/// Force the dark editor theme and install the shell's spacing and typography.
pub fn configure(context: &Context) {
    context.set_theme(Theme::Dark);
    context.style_mut_of(Theme::Dark, configure_style);
    // Keep both style slots dark so an OS theme change cannot produce a
    // half-light shell while the app is running.
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
            FontId::new(12.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(10.5, FontFamily::Proportional),
        ),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size = egui::vec2(28.0, 26.0);
    style.spacing.window_margin = egui::Margin::same(14);
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
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(7);
    visuals.hyperlink_color = Palette::ACCENT;
    visuals.selection.bg_fill = Palette::ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, Palette::ACCENT);

    visuals.widgets.noninteractive.bg_fill = Palette::PANEL_BG;
    visuals.widgets.noninteractive.weak_bg_fill = Palette::PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Palette::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Palette::TEXT_SECONDARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = Palette::WIDGET_BG;
    visuals.widgets.inactive.weak_bg_fill = Palette::WIDGET_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Palette::BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = Palette::WIDGET_HOVER;
    visuals.widgets.hovered.weak_bg_fill = Palette::WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Palette::ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = Palette::WIDGET_ACTIVE;
    visuals.widgets.active.weak_bg_fill = Palette::WIDGET_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Palette::ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    visuals.widgets.open = visuals.widgets.active;
    visuals.disabled_alpha = 0.45;

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
    fn accent_is_teal_rather_than_blue() {
        assert!(Palette::ACCENT.g() > Palette::ACCENT.r());
        assert!(Palette::ACCENT.g() > Palette::ACCENT.b());
    }
}

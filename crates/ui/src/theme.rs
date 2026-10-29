//! CutCraft visual tokens and egui style configuration.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Stroke, Style, TextStyle, Theme, Visuals,
};

/// Shared colors for the CutCraft shell.
///
/// Charcoal editor chrome, a darker teal-tinted Home surface, and a cyan accent
/// drawn from the reference measurements. The palette is original.
pub struct Palette;

impl Palette {
    // Editor chrome
    pub const APP_BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
    pub const MENU_BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
    pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0x30, 0x30, 0x30);
    pub const PANEL_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
    pub const PANEL_RAISED: Color32 = Color32::from_rgb(0x34, 0x34, 0x37);
    pub const CARD_BG: Color32 = Color32::from_rgb(0x34, 0x34, 0x37);
    pub const CARD_HOVER: Color32 = Color32::from_rgb(0x3d, 0x3d, 0x40);
    pub const SURFACE: Color32 = Color32::from_rgb(0x3b, 0x3b, 0x3e);
    pub const STATUS_BG: Color32 = Color32::from_rgb(0x10, 0x10, 0x10);
    pub const TRACK_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
    pub const TRACK_ALT_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2c);
    pub const TRACK_HEADER_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
    pub const RULER_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);

    // Home and modal surfaces
    pub const HOME_BG: Color32 = Color32::from_rgb(0x07, 0x11, 0x14);
    pub const HOME_SIDEBAR: Color32 = Color32::from_rgb(0x0a, 0x18, 0x1b);
    pub const HOME_CARD: Color32 = Color32::from_rgb(0x13, 0x20, 0x23);
    pub const HOME_CARD_HOVER: Color32 = Color32::from_rgb(0x19, 0x29, 0x2d);
    pub const HOME_BORDER: Color32 = Color32::from_rgb(0x20, 0x30, 0x34);
    pub const MODAL_BG: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);

    pub const BORDER: Color32 = Color32::from_rgb(0x38, 0x38, 0x3b);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x30);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xf0, 0xf0, 0xf0);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xb0, 0xb0, 0xb4);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x7c, 0x7c, 0x80);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x5e);

    pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0xc1, 0xcd);
    pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x66, 0xe5, 0xee);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x00, 0x54, 0x5c);
    pub const ACCENT_BG: Color32 = Color32::from_rgb(0x10, 0x3b, 0x40);
    pub const ON_ACCENT: Color32 = Color32::from_rgb(0x00, 0x27, 0x2b);
    pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0x4f, 0x5b);
    pub const CANVAS: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);

    pub const WIDGET_BG: Color32 = Color32::from_rgb(0x34, 0x34, 0x37);
    pub const WIDGET_HOVER: Color32 = Color32::from_rgb(0x3e, 0x3e, 0x41);
    pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(0x48, 0x48, 0x4b);

    pub const INFO: Color32 = Self::ACCENT;
    pub const WARNING: Color32 = Color32::from_rgb(0xe5, 0xa8, 0x48);
    pub const ERROR: Color32 = Color32::from_rgb(0xf0, 0x65, 0x65);
    pub const SUCCESS: Color32 = Color32::from_rgb(0x38, 0xd3, 0x9f);
}

/// Shared corner radii for the CutCraft desktop shell.
pub struct Radii;

impl Radii {
    /// Small chips, badges, and compact inline elements.
    pub const SMALL: CornerRadius = CornerRadius::same(3);
    /// Standard buttons, fields, dropdowns, and tabs.
    pub const CONTROL: CornerRadius = CornerRadius::same(5);
    /// Sections, tiles, and standard cards.
    pub const CARD: CornerRadius = CornerRadius::same(8);
    /// Large feature and creative-suite cards.
    pub const LARGE_CARD: CornerRadius = CornerRadius::same(10);
    /// Modal and window containers.
    pub const DIALOG: CornerRadius = CornerRadius::same(12);
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
            FontId::new(17.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (
            TextStyle::Monospace,
            FontId::new(11.5, FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(10.0, FontFamily::Proportional),
        ),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(5.0, 4.0);
    style.spacing.button_padding = egui::vec2(7.0, 3.0);
    style.spacing.interact_size = egui::vec2(24.0, 22.0);
    style.spacing.window_margin = egui::Margin::same(10);
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
    visuals.window_corner_radius = Radii::DIALOG;
    visuals.menu_corner_radius = Radii::CONTROL;
    visuals.hyperlink_color = Palette::ACCENT;
    visuals.selection.bg_fill = Palette::ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, Palette::ACCENT);

    visuals.widgets.noninteractive.bg_fill = Palette::PANEL_BG;
    visuals.widgets.noninteractive.weak_bg_fill = Palette::PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Palette::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = Radii::SMALL;

    visuals.widgets.inactive.bg_fill = Palette::WIDGET_BG;
    visuals.widgets.inactive.weak_bg_fill = Palette::WIDGET_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Palette::BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = Radii::CONTROL;

    visuals.widgets.hovered.bg_fill = Palette::WIDGET_HOVER;
    visuals.widgets.hovered.weak_bg_fill = Palette::WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Palette::BORDER);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = Radii::CONTROL;

    visuals.widgets.active.bg_fill = Palette::WIDGET_ACTIVE;
    visuals.widgets.active.weak_bg_fill = Palette::WIDGET_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Palette::ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Palette::TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = Radii::CONTROL;

    visuals.widgets.open = visuals.widgets.active;
    visuals.disabled_alpha = 0.55;

    visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_selects_dark_theme_and_reference_surfaces() {
        let context = Context::default();
        configure(&context);

        assert_eq!(context.theme(), Theme::Dark);
        let style = context.style_of(Theme::Dark);
        assert_eq!(style.visuals.panel_fill, Palette::PANEL_BG);
        assert_eq!(style.visuals.extreme_bg_color, Palette::APP_BG);
        assert_eq!(
            style.text_styles.get(&TextStyle::Monospace),
            Some(&FontId::new(11.5, FontFamily::Monospace))
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
    fn accent_matches_reference_cyan() {
        assert_eq!(Palette::ACCENT, Color32::from_rgb(0x00, 0xc1, 0xcd));
        assert!(Palette::ACCENT.b() > Palette::ACCENT.r());
    }

    #[test]
    fn themed_interactive_widgets_share_the_standard_control_radius() {
        let context = Context::default();
        configure(&context);
        let style = context.style_of(Theme::Dark);

        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            CornerRadius::same(5)
        );
        assert_eq!(
            style.visuals.widgets.hovered.corner_radius,
            CornerRadius::same(5)
        );
        assert_eq!(
            style.visuals.widgets.active.corner_radius,
            CornerRadius::same(5)
        );
        assert_eq!(
            style.visuals.widgets.open.corner_radius,
            CornerRadius::same(5)
        );
    }

    #[test]
    fn themed_windows_and_menus_use_the_dialog_scale_radii() {
        let context = Context::default();
        configure(&context);
        let style = context.style_of(Theme::Dark);

        assert_eq!(style.visuals.window_corner_radius, CornerRadius::same(12));
        assert_eq!(style.visuals.menu_corner_radius, CornerRadius::same(5));
    }

    #[test]
    fn radius_tokens_follow_the_design_scale() {
        assert_eq!(Radii::SMALL, CornerRadius::same(3));
        assert_eq!(Radii::CONTROL, CornerRadius::same(5));
        assert_eq!(Radii::CARD, CornerRadius::same(8));
        assert_eq!(Radii::LARGE_CARD, CornerRadius::same(10));
        assert_eq!(Radii::DIALOG, CornerRadius::same(12));
    }

    #[test]
    fn interactive_states_are_distinct_and_neutral_until_active() {
        let context = Context::default();
        configure(&context);
        let visuals = &context.style_of(Theme::Dark).visuals;

        assert_ne!(
            visuals.widgets.inactive.bg_fill,
            visuals.widgets.hovered.bg_fill
        );
        assert_ne!(
            visuals.widgets.hovered.bg_fill,
            visuals.widgets.active.bg_fill
        );
        assert_eq!(
            visuals.widgets.inactive.bg_stroke,
            Stroke::new(1.0, Palette::BORDER_SUBTLE)
        );
        assert_eq!(
            visuals.widgets.hovered.bg_stroke,
            Stroke::new(1.0, Palette::BORDER)
        );
        assert_eq!(
            visuals.widgets.active.bg_stroke,
            Stroke::new(1.0, Palette::ACCENT)
        );
        assert_eq!(visuals.selection.stroke, Stroke::new(1.0, Palette::ACCENT));
        assert!(visuals.disabled_alpha < 1.0);
    }
}

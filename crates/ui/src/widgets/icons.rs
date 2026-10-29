//! Centralized Lucide icon registry for the CutCraft shell.
//!
//! Icons are bundled from Lucide 1.61.0 through `egui-lucide`. Rendering goes
//! through egui's cached SVG texture loader at the exact device-pixel size, so
//! there is no runtime download and no per-frame SVG decoding.

use egui::{
    load::{SizeHint, TexturePoll},
    pos2, Color32, Painter, Rect, TextureOptions, Vec2,
};
use egui_lucide::Lucide;

use crate::theme::Palette;

/// Lucide's optical stroke width chosen for CutCraft's compact desktop chrome.
pub(crate) const STROKE_WIDTH: f32 = 1.75;

/// Shared optical sizes for icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IconSize {
    /// Compact metadata, chevrons, and dense inline controls.
    Small,
    /// Standard editor controls and toolbar buttons.
    Standard,
    /// Sidebar and top-level navigation.
    Navigation,
    /// Invitation and empty-state illustrations.
    EmptyState,
}

impl IconSize {
    pub(crate) const fn points(self) -> f32 {
        match self {
            Self::Small => 16.0,
            Self::Standard => 18.0,
            Self::Navigation => 20.0,
            Self::EmptyState => 32.0,
        }
    }
}

/// Shared icon color roles.
pub(crate) struct IconColors;

impl IconColors {
    pub(crate) const NORMAL: Color32 = Palette::TEXT_PRIMARY;
    pub(crate) const MUTED: Color32 = Palette::TEXT_MUTED;
    pub(crate) const HOVERED: Color32 = Palette::ACCENT_LIGHT;
    pub(crate) const SELECTED: Color32 = Palette::ACCENT;
    pub(crate) const DISABLED: Color32 = Palette::TEXT_DISABLED;
    pub(crate) const ON_ACCENT: Color32 = Palette::ON_ACCENT;
}

/// Semantic icon identifiers used by the CutCraft UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Home,
    Projects,
    Settings,
    Spaces,
    Media,
    Audio,
    Text,
    Stickers,
    Effects,
    Transitions,
    Captions,
    Filters,
    Folder,
    Export,
    Close,
    Eye,
    Lock,
    Volume,
    Undo,
    Redo,
    Grid,
    List,
    Plus,
    Select,
    Split,
    Delete,
    Snap,
    Link,
    Menu,
    Crop,
    Fullscreen,
    TrimIn,
    TrimOut,
    Freeze,
    Reverse,
    DetachAudio,
    Voiceover,
    Adjust,
    More,
    ZoomIn,
    ZoomOut,
    Fit,
    Play,
    CirclePlay,
    ChevronDown,
    Bold,
    Italic,
    Underline,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    Keyboard,
}

impl Icon {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 53] = [
        Self::Home,
        Self::Projects,
        Self::Settings,
        Self::Spaces,
        Self::Media,
        Self::Audio,
        Self::Text,
        Self::Stickers,
        Self::Effects,
        Self::Transitions,
        Self::Captions,
        Self::Filters,
        Self::Folder,
        Self::Export,
        Self::Close,
        Self::Eye,
        Self::Lock,
        Self::Volume,
        Self::Undo,
        Self::Redo,
        Self::Grid,
        Self::List,
        Self::Plus,
        Self::Select,
        Self::Split,
        Self::Delete,
        Self::Snap,
        Self::Link,
        Self::Menu,
        Self::Crop,
        Self::Fullscreen,
        Self::TrimIn,
        Self::TrimOut,
        Self::Freeze,
        Self::Reverse,
        Self::DetachAudio,
        Self::Voiceover,
        Self::Adjust,
        Self::More,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::Fit,
        Self::Play,
        Self::CirclePlay,
        Self::ChevronDown,
        Self::Bold,
        Self::Italic,
        Self::Underline,
        Self::AlignLeft,
        Self::AlignCenter,
        Self::AlignRight,
        Self::AlignJustify,
        Self::Keyboard,
    ];

    pub(crate) const fn lucide(self) -> Lucide {
        match self {
            Self::Home => Lucide::House,
            Self::Projects | Self::Folder => Lucide::Folder,
            Self::Settings => Lucide::Settings,
            Self::Spaces => Lucide::Users,
            Self::Media => Lucide::Clapperboard,
            Self::Audio => Lucide::AudioLines,
            Self::Text => Lucide::Type,
            Self::Stickers => Lucide::Sticker,
            Self::Effects => Lucide::Sparkles,
            Self::Transitions => Lucide::Repeat2,
            Self::Captions => Lucide::Captions,
            Self::Filters => Lucide::Blend,
            Self::Export => Lucide::Upload,
            Self::Close => Lucide::X,
            Self::Eye => Lucide::Eye,
            Self::Lock => Lucide::Lock,
            Self::Volume => Lucide::Volume2,
            Self::Undo => Lucide::Undo2,
            Self::Redo => Lucide::Redo2,
            Self::Grid => Lucide::Grid2x2,
            Self::List => Lucide::List,
            Self::Plus => Lucide::Plus,
            Self::Select => Lucide::MousePointer2,
            Self::Split => Lucide::Scissors,
            Self::Delete => Lucide::Trash,
            Self::Snap => Lucide::Magnet,
            Self::Link => Lucide::Link2,
            Self::Menu => Lucide::Menu,
            Self::Crop => Lucide::Crop,
            Self::Fullscreen => Lucide::Fullscreen,
            Self::TrimIn => Lucide::PanelLeft,
            Self::TrimOut => Lucide::PanelRight,
            Self::Freeze => Lucide::Snowflake,
            Self::Reverse => Lucide::RotateCcw,
            Self::DetachAudio => Lucide::AudioWaveform,
            Self::Voiceover => Lucide::Mic,
            Self::Adjust => Lucide::SlidersHorizontal,
            Self::More => Lucide::Ellipsis,
            Self::ZoomIn => Lucide::ZoomIn,
            Self::ZoomOut => Lucide::ZoomOut,
            Self::Fit => Lucide::Scan,
            Self::Play => Lucide::Play,
            Self::CirclePlay => Lucide::CirclePlay,
            Self::ChevronDown => Lucide::ChevronDown,
            Self::Bold => Lucide::Bold,
            Self::Italic => Lucide::Italic,
            Self::Underline => Lucide::Underline,
            Self::AlignLeft => Lucide::TextAlignStart,
            Self::AlignCenter => Lucide::TextAlignCenter,
            Self::AlignRight => Lucide::TextAlignEnd,
            Self::AlignJustify => Lucide::TextAlignJustify,
            Self::Keyboard => Lucide::Keyboard,
        }
    }

    /// Paint a square icon centered in `rect` at the requested optical size.
    pub(crate) fn paint_sized(self, painter: &Painter, rect: Rect, size: IconSize, color: Color32) {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        let points = size.points();
        let square = Rect::from_center_size(rect.center(), Vec2::splat(points));
        let pixels = raster_pixels(points, painter.ctx().pixels_per_point());
        let source = self
            .lucide()
            .size(points)
            .stroke_width(STROKE_WIDTH)
            .color(color)
            .source();
        let size_hint = SizeHint::Size {
            width: pixels,
            height: pixels,
            maintain_aspect_ratio: true,
        };
        match source.load(painter.ctx(), TextureOptions::LINEAR, size_hint) {
            Ok(TexturePoll::Ready { texture }) => {
                painter.image(
                    texture.id,
                    square,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Ok(TexturePoll::Pending { .. }) => {
                painter.ctx().request_repaint();
            }
            Err(_) => {
                // The SVG loader is installed by `theme::configure`; failing
                // closed avoids drawing a mismatched fallback icon.
            }
        }
    }
}

/// Paint a semantic icon with an explicit optical size.
pub(crate) fn paint_sized(
    painter: &Painter,
    rect: Rect,
    icon: Icon,
    size: IconSize,
    color: Color32,
) {
    icon.paint_sized(painter, rect, size, color);
}

fn raster_pixels(points: f32, pixels_per_point: f32) -> u32 {
    let pixels_per_point = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    (points * pixels_per_point).round().max(8.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> egui::Context {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        context
    }

    #[test]
    fn every_semantic_icon_maps_to_a_lucide_name() {
        for icon in Icon::ALL {
            assert!(
                !icon.lucide().name().is_empty(),
                "{icon:?} has no Lucide mapping"
            );
        }
    }

    #[test]
    fn icon_sizes_follow_the_shared_scale() {
        assert_eq!(IconSize::Small.points(), 16.0);
        assert_eq!(IconSize::Standard.points(), 18.0);
        assert_eq!(IconSize::Navigation.points(), 20.0);
        assert_eq!(IconSize::EmptyState.points(), 32.0);
        assert_eq!(STROKE_WIDTH, 1.75);
    }

    #[test]
    fn raster_size_scales_with_device_pixel_ratio() {
        assert_eq!(raster_pixels(16.0, 1.0), 16);
        assert_eq!(raster_pixels(18.0, 1.5), 27);
        assert_eq!(raster_pixels(32.0, 2.0), 64);
        assert_eq!(raster_pixels(16.0, f32::NAN), 16);
    }

    #[test]
    fn icon_color_roles_are_distinct() {
        assert_ne!(IconColors::MUTED, IconColors::NORMAL);
        assert_ne!(IconColors::NORMAL, IconColors::HOVERED);
        assert_ne!(IconColors::HOVERED, IconColors::SELECTED);
        assert_ne!(IconColors::SELECTED, IconColors::DISABLED);
        assert_eq!(IconColors::SELECTED, Palette::ACCENT);
    }

    #[test]
    fn every_icon_paints_without_panicking() {
        let context = context();
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            for icon in Icon::ALL {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::splat(IconSize::Standard.points()),
                    egui::Sense::hover(),
                );
                icon.paint_sized(ui.painter(), rect, IconSize::Standard, IconColors::NORMAL);
            }
        });
        output.drop_without_applying_deltas();
    }
}

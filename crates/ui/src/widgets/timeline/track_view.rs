//! Paint supplied track headers. No predefined rows live in the renderer.

use egui::{pos2, vec2, Align2, CornerRadius, FontId, Painter, Rect, Stroke};

use crate::theme::Palette;
use crate::timeline_state::{TimelineTrackView, TrackKind};
use crate::widgets::icons::{self, Icon, IconColors, IconSize};

pub(super) fn paint_header(painter: &Painter, row: Rect, track: &TimelineTrackView) {
    painter.rect_filled(row, CornerRadius::ZERO, Palette::TRACK_HEADER_BG);
    let stripe_color = match track.kind {
        TrackKind::Video => Palette::ACCENT,
        TrackKind::Audio => Palette::WARNING,
    };
    painter.line_segment(
        [pos2(row.left(), row.top()), pos2(row.right(), row.top())],
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
    );
    painter.rect_filled(
        Rect::from_min_size(row.min, vec2(3.0, row.height())),
        CornerRadius::ZERO,
        stripe_color,
    );
    painter.text(
        pos2(row.left() + 13.0, row.center().y - 6.0),
        Align2::LEFT_CENTER,
        &track.label,
        FontId::monospace(10.5),
        Palette::TEXT_PRIMARY,
    );
    let name_painter = painter.with_clip_rect(Rect::from_min_max(
        pos2(row.left() + 42.0, row.top()),
        pos2(row.right() - 48.0, row.bottom()),
    ));
    name_painter.text(
        pos2(row.left() + 42.0, row.center().y - 6.0),
        Align2::LEFT_CENTER,
        &track.name,
        FontId::proportional(10.5),
        Palette::TEXT_SECONDARY,
    );
    name_painter.text(
        pos2(row.left() + 42.0, row.center().y + 8.0),
        Align2::LEFT_CENTER,
        match track.kind {
            TrackKind::Video => "Video track",
            TrackKind::Audio => "Audio track",
        },
        FontId::proportional(8.5),
        Palette::TEXT_DISABLED,
    );
    let primary_icon = match track.kind {
        TrackKind::Video => Icon::Eye,
        TrackKind::Audio => Icon::Volume,
    };
    for (offset, icon) in [(40.0_f32, primary_icon), (18.0, Icon::Lock)] {
        icons::paint_sized(
            painter,
            Rect::from_center_size(pos2(row.right() - offset, row.center().y), vec2(16.0, 16.0)),
            icon,
            IconSize::Small,
            IconColors::DISABLED,
        );
    }
}

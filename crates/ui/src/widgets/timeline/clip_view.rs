//! Item blocks for supplied view data and the explicitly selected QA fixture.
//! No decoded thumbnails or waveforms are fabricated.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind};

use crate::theme::{Palette, Radii};
use crate::timeline_state::{TimelineItemView, TrackKind};

use super::scale::TimelineScale;

pub(super) fn rect(row: Rect, scale: &TimelineScale, item: &TimelineItemView) -> Rect {
    let x0 = row.left() + scale.x_for_seconds(item.start_seconds());
    let x1 = row.left() + scale.x_for_seconds(item.end_seconds());
    Rect::from_min_max(pos2(x0, row.top() + 4.0), pos2(x1, row.bottom() - 4.0))
}

pub(super) fn paint(
    painter: &Painter,
    row: Rect,
    scale: &TimelineScale,
    item: &TimelineItemView,
    kind: TrackKind,
    demo: bool,
    selected: bool,
) {
    let clip_rect = rect(row, scale, item);
    if clip_rect.width() < 2.0 || !painter.clip_rect().intersects(clip_rect) {
        return;
    }
    let (fill, border, text) = match kind {
        TrackKind::Audio => (
            Color32::from_rgb(0x3a, 0x30, 0x1e),
            Color32::from_rgb(0x6a, 0x54, 0x2a),
            Palette::WARNING,
        ),
        TrackKind::Video => (
            Palette::ACCENT_BG,
            Palette::ACCENT_DIM,
            Palette::ACCENT_LIGHT,
        ),
    };
    let border_color = if selected {
        Palette::TEXT_PRIMARY
    } else {
        border
    };
    let painter = painter.with_clip_rect(clip_rect.intersect(painter.clip_rect()));
    painter.rect_filled(clip_rect, Radii::SMALL, fill);
    painter.rect_stroke(
        clip_rect,
        Radii::SMALL,
        Stroke::new(if selected { 1.5 } else { 1.0 }, border_color),
        StrokeKind::Inside,
    );
    painter.rect_filled(
        Rect::from_min_size(clip_rect.min, vec2(clip_rect.width(), 2.0)),
        CornerRadius::ZERO,
        border_color,
    );
    if clip_rect.width() > 42.0 {
        painter.text(
            pos2(clip_rect.left() + 7.0, clip_rect.center().y + 1.0),
            Align2::LEFT_CENTER,
            item.label(),
            FontId::proportional(9.0),
            text,
        );
    }
    if demo && clip_rect.width() > 150.0 {
        let badge = Rect::from_min_size(
            pos2(clip_rect.right() - 42.0, clip_rect.top() + 5.0),
            vec2(37.0, 13.0),
        );
        painter.rect_filled(badge, Radii::SMALL, fill);
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            "DEMO",
            FontId::proportional(7.0),
            text,
        );
    }
}

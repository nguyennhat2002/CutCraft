//! Static time-zero playhead. Seeking and playback are not implemented.

use egui::{pos2, vec2, Painter, Rect, Stroke, StrokeKind};

use crate::theme::{Palette, Radii};

use super::scale::TimelineScale;

/// Horizontal position for a display-only playhead time.
pub fn x_for_millis(scale: &TimelineScale, millis: u64) -> f32 {
    scale.x_for_seconds(millis as f64 / 1_000.0)
}

/// Quiet line and outlined handle matching the empty-canvas reference.
pub fn paint(painter: &Painter, top: f32, bottom: f32, x: f32) {
    if !x.is_finite() || !top.is_finite() || !bottom.is_finite() || bottom <= top {
        return;
    }
    painter.line_segment(
        [pos2(x, top + 12.0), pos2(x, bottom)],
        Stroke::new(1.0, Palette::PLAYHEAD),
    );
    painter.rect_stroke(
        Rect::from_min_size(pos2(x - 3.5, top + 1.0), vec2(7.0, 12.0)),
        Radii::SMALL,
        Stroke::new(1.5, Palette::PLAYHEAD),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playhead_starts_at_the_leading_padding() {
        let scale = TimelineScale::new(80.0, 14.0);
        assert_eq!(x_for_millis(&scale, 0), 14.0);
    }

    #[test]
    fn playhead_position_uses_display_scale() {
        let scale = TimelineScale::new(80.0, 14.0);
        assert!((x_for_millis(&scale, 1_500) - 134.0).abs() < 0.01);
    }
}

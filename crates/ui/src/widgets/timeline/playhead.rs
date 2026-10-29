//! Static timeline playhead rendering.
//!
//! The playhead is always at time zero in M1.1; seeking and playback are not
//! implemented.

use egui::{pos2, Color32, Painter, Shape, Stroke};

use crate::theme::Palette;

use super::scale::TimelineScale;

/// Horizontal position for a display-only playhead time.
pub fn x_for_millis(scale: &TimelineScale, millis: u64) -> f32 {
    scale.x_for_seconds(millis as f64 / 1_000.0)
}

/// Draw a line and top handle for the playhead.
pub fn paint(painter: &Painter, top: f32, bottom: f32, x: f32) {
    if !x.is_finite() || !top.is_finite() || !bottom.is_finite() || bottom <= top {
        return;
    }

    let color: Color32 = Palette::PLAYHEAD;
    let line_top = top + 6.0;
    painter.line_segment(
        [pos2(x, line_top), pos2(x, bottom)],
        Stroke::new(2.0, color),
    );
    painter.add(Shape::convex_polygon(
        vec![pos2(x - 5.0, top), pos2(x + 5.0, top), pos2(x, line_top)],
        color,
        Stroke::NONE,
    ));
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

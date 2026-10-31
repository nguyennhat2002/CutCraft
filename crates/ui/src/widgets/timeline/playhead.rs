//! Authoritative playhead painting. Seeking itself is driven by the session's
//! integer playhead; no decoded frame is implied.

use egui::{pos2, vec2, Painter, Rect, Stroke, StrokeKind};

use crate::theme::{Palette, Radii};

/// Compact ruler handle rectangle, also used for hit testing guidance.
pub fn handle_rect(top: f32, x: f32) -> Rect {
    Rect::from_center_size(pos2(x, top + 7.0), vec2(9.0, 13.0))
}

/// Quiet line and outlined handle matching the empty-canvas reference.
pub fn paint(painter: &Painter, top: f32, bottom: f32, x: f32, active: bool) {
    if !x.is_finite() || !top.is_finite() || !bottom.is_finite() || bottom <= top {
        return;
    }
    let handle = handle_rect(top, x);
    let stroke_width = if active { 1.6 } else { 1.0 };
    painter.line_segment(
        [pos2(x, handle.bottom() - 1.0), pos2(x, bottom)],
        Stroke::new(stroke_width, Palette::PLAYHEAD),
    );
    painter.rect_filled(handle, Radii::SMALL, Palette::PLAYHEAD);
    painter.rect_stroke(
        handle,
        Radii::SMALL,
        Stroke::new(1.0, Palette::ON_ACCENT),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_sits_inside_the_ruler_band() {
        let top = 40.0;
        let handle = handle_rect(top, 123.0);
        assert_eq!(handle.center().x, 123.0);
        assert!(handle.top() >= top);
        assert!(handle.bottom() <= top + 22.0);
    }
}

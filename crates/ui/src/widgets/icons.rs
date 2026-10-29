//! Small original vector icons drawn with egui primitives.
//!
//! These are simple geometric marks for the CutCraft shell. No third-party
//! icon font or artwork is embedded.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Shape, Stroke, StrokeKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Home,
    Projects,
    Settings,
    Media,
    Audio,
    Text,
    Effects,
    Transitions,
    Folder,
    Export,
    Close,
}

pub(crate) fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let area = rect.shrink(2.0);

    match icon {
        Icon::Home => {
            let left = area.left();
            let right = area.right();
            let bottom = area.bottom();
            let mid_y = area.top() + area.height() * 0.42;
            let apex = pos2(area.center().x, area.top());
            painter.line_segment([pos2(left, mid_y), apex], stroke);
            painter.line_segment([apex, pos2(right, mid_y)], stroke);
            painter.line_segment([pos2(left, mid_y), pos2(left, bottom)], stroke);
            painter.line_segment([pos2(right, mid_y), pos2(right, bottom)], stroke);
            painter.line_segment([pos2(left, bottom), pos2(right, bottom)], stroke);
            let door = Rect::from_min_max(
                pos2(
                    area.center().x - area.width() * 0.14,
                    bottom - area.height() * 0.28,
                ),
                pos2(area.center().x + area.width() * 0.14, bottom),
            );
            painter.rect_stroke(
                door,
                CornerRadius::same(1),
                Stroke::new(1.2, color),
                StrokeKind::Inside,
            );
        }
        Icon::Projects | Icon::Folder => {
            let tab = Rect::from_min_max(
                pos2(area.left(), area.top() + area.height() * 0.12),
                pos2(
                    area.left() + area.width() * 0.46,
                    area.top() + area.height() * 0.34,
                ),
            );
            let body = Rect::from_min_max(
                pos2(area.left(), area.top() + area.height() * 0.25),
                area.right_bottom(),
            );
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Inside);
            painter.rect_stroke(
                tab,
                CornerRadius::same(1),
                Stroke::new(1.2, color),
                StrokeKind::Inside,
            );
        }
        Icon::Settings => {
            for (index, fraction) in [0.26_f32, 0.5, 0.74].into_iter().enumerate() {
                let y = area.top() + area.height() * fraction;
                painter.line_segment(
                    [pos2(area.left(), y), pos2(area.right(), y)],
                    Stroke::new(1.3, color),
                );
                let knob_x = area.left() + area.width() * [0.68_f32, 0.34, 0.56][index];
                painter.circle_filled(pos2(knob_x, y), 2.2, color);
            }
        }
        Icon::Media => {
            let body =
                Rect::from_center_size(area.center(), vec2(area.width(), area.height() * 0.72));
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Inside);
            for index in 0..3 {
                let x = body.left() + body.width() * (0.2 + index as f32 * 0.3);
                for y in [body.top() + 3.0, body.bottom() - 3.0] {
                    painter.rect_filled(
                        Rect::from_center_size(pos2(x, y), vec2(3.0, 1.6)),
                        CornerRadius::same(1),
                        color,
                    );
                }
            }
            let center = body.center();
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(center.x - 2.0, center.y - 3.0),
                    pos2(center.x - 2.0, center.y + 3.0),
                    pos2(center.x + 3.5, center.y),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Audio => {
            let bars = [0.38_f32, 0.66, 0.92, 0.58, 0.3];
            for (index, height) in bars.into_iter().enumerate() {
                let x = area.left() + area.width() * (0.16 + index as f32 * 0.17);
                let half = area.height() * height * 0.5;
                painter.line_segment(
                    [
                        pos2(x, area.center().y - half),
                        pos2(x, area.center().y + half),
                    ],
                    Stroke::new(2.0, color),
                );
            }
        }
        Icon::Text => {
            painter.text(
                area.center(),
                Align2::CENTER_CENTER,
                "T",
                FontId::proportional(area.height() * 0.9),
                color,
            );
        }
        Icon::Effects => {
            let center = area.center();
            let long = area.height() * 0.46;
            let short = area.height() * 0.16;
            painter.line_segment(
                [
                    pos2(center.x, center.y - long),
                    pos2(center.x, center.y + long),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(center.x - long, center.y),
                    pos2(center.x + long, center.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(center.x - short, center.y - short),
                    pos2(center.x + short, center.y + short),
                ],
                Stroke::new(1.2, color),
            );
            painter.line_segment(
                [
                    pos2(center.x + short, center.y - short),
                    pos2(center.x - short, center.y + short),
                ],
                Stroke::new(1.2, color),
            );
        }
        Icon::Transitions => {
            let half_width = area.width() * 0.36;
            let left_triangle = vec![
                pos2(area.left(), area.top()),
                pos2(area.left() + half_width, area.center().y),
                pos2(area.left(), area.bottom()),
            ];
            let right_triangle = vec![
                pos2(area.right(), area.top()),
                pos2(area.right() - half_width, area.center().y),
                pos2(area.right(), area.bottom()),
            ];
            painter.add(Shape::convex_polygon(
                left_triangle,
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.add(Shape::convex_polygon(
                right_triangle,
                Color32::TRANSPARENT,
                stroke,
            ));
        }
        Icon::Export => {
            let top = area.top() + area.height() * 0.12;
            let bottom = area.bottom() - area.height() * 0.05;
            let center_x = area.center().x;
            painter.line_segment(
                [
                    pos2(center_x, top),
                    pos2(center_x, bottom - area.height() * 0.28),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(center_x, top),
                    pos2(center_x - area.width() * 0.18, top + area.height() * 0.22),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(center_x, top),
                    pos2(center_x + area.width() * 0.18, top + area.height() * 0.22),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(area.left(), bottom - area.height() * 0.28),
                    pos2(area.left(), bottom),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(area.right(), bottom - area.height() * 0.28),
                    pos2(area.right(), bottom),
                ],
                stroke,
            );
            painter.line_segment(
                [pos2(area.left(), bottom), pos2(area.right(), bottom)],
                stroke,
            );
        }
        Icon::Close => {
            painter.line_segment([area.left_top(), area.right_bottom()], stroke);
            painter.line_segment([area.right_top(), area.left_bottom()], stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_paints_without_panicking() {
        let context = egui::Context::default();
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            let icons = [
                Icon::Home,
                Icon::Projects,
                Icon::Settings,
                Icon::Media,
                Icon::Audio,
                Icon::Text,
                Icon::Effects,
                Icon::Transitions,
                Icon::Folder,
                Icon::Export,
                Icon::Close,
            ];
            for icon in icons {
                let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                paint(ui.painter(), rect, icon, Color32::WHITE);
            }
        });
        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();
    }
}

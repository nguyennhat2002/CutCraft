//! Small original vector icons drawn with egui primitives.
//!
//! These are simple geometric marks for the CutCraft shell. No third-party
//! icon font or artwork is embedded. Corner radii inside this module describe
//! glyph geometry and are deliberately independent of the shell `Radii` scale.

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
}

pub(crate) fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let thin = Stroke::new(1.2, color);
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
            painter.rect_stroke(door, CornerRadius::same(1), thin, StrokeKind::Inside);
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
            painter.rect_stroke(tab, CornerRadius::same(1), thin, StrokeKind::Inside);
        }
        Icon::Settings => {
            for (index, fraction) in [0.26_f32, 0.5, 0.74].into_iter().enumerate() {
                let y = area.top() + area.height() * fraction;
                painter.line_segment([pos2(area.left(), y), pos2(area.right(), y)], thin);
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
        Icon::Stickers => {
            painter.rect_stroke(area, CornerRadius::same(4), stroke, StrokeKind::Inside);
            for x in [0.34_f32, 0.66] {
                painter.circle_filled(
                    pos2(
                        area.left() + area.width() * x,
                        area.top() + area.height() * 0.4,
                    ),
                    1.5,
                    color,
                );
            }
            let smile_y = area.top() + area.height() * 0.58;
            painter.line_segment(
                [
                    pos2(area.left() + area.width() * 0.32, smile_y),
                    pos2(
                        area.left() + area.width() * 0.5,
                        area.bottom() - area.height() * 0.24,
                    ),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(
                        area.left() + area.width() * 0.5,
                        area.bottom() - area.height() * 0.24,
                    ),
                    pos2(area.left() + area.width() * 0.68, smile_y),
                ],
                thin,
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
                thin,
            );
            painter.line_segment(
                [
                    pos2(center.x + short, center.y - short),
                    pos2(center.x - short, center.y + short),
                ],
                thin,
            );
        }
        Icon::Transitions => {
            let half_width = area.width() * 0.36;
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(area.left(), area.top()),
                    pos2(area.left() + half_width, area.center().y),
                    pos2(area.left(), area.bottom()),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(area.right(), area.top()),
                    pos2(area.right() - half_width, area.center().y),
                    pos2(area.right(), area.bottom()),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
        }
        Icon::Captions => {
            painter.rect_stroke(area, CornerRadius::same(2), stroke, StrokeKind::Inside);
            for y in [0.38_f32, 0.58] {
                painter.line_segment(
                    [
                        pos2(
                            area.left() + area.width() * 0.18,
                            area.top() + area.height() * y,
                        ),
                        pos2(
                            area.right() - area.width() * 0.18,
                            area.top() + area.height() * y,
                        ),
                    ],
                    thin,
                );
            }
        }
        Icon::Filters => {
            for x in [0.3_f32, 0.5, 0.7] {
                painter.circle_stroke(
                    pos2(area.left() + area.width() * x, area.center().y),
                    area.height() * 0.24,
                    stroke,
                );
            }
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
        Icon::Eye => {
            painter.line_segment(
                [
                    pos2(area.left(), area.center().y),
                    pos2(area.center().x, area.top() + area.height() * 0.24),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(area.center().x, area.top() + area.height() * 0.24),
                    pos2(area.right(), area.center().y),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(area.right(), area.center().y),
                    pos2(area.center().x, area.bottom() - area.height() * 0.24),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(area.center().x, area.bottom() - area.height() * 0.24),
                    pos2(area.left(), area.center().y),
                ],
                thin,
            );
            painter.circle_filled(area.center(), area.height() * 0.13, color);
        }
        Icon::Lock => {
            let body = Rect::from_min_max(
                pos2(
                    area.left() + area.width() * 0.18,
                    area.top() + area.height() * 0.44,
                ),
                area.right_bottom(),
            );
            painter.rect_stroke(body, CornerRadius::same(2), thin, StrokeKind::Inside);
            let shackle_left = area.left() + area.width() * 0.3;
            let shackle_right = area.right() - area.width() * 0.3;
            let shackle_top = area.top() + area.height() * 0.16;
            painter.line_segment(
                [
                    pos2(shackle_left, body.top()),
                    pos2(shackle_left, shackle_top),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(shackle_left, shackle_top),
                    pos2(shackle_right, shackle_top),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(shackle_right, shackle_top),
                    pos2(shackle_right, body.top()),
                ],
                thin,
            );
        }
        Icon::Volume => {
            let speaker = vec![
                pos2(area.left(), area.center().y - area.height() * 0.16),
                pos2(
                    area.left() + area.width() * 0.28,
                    area.center().y - area.height() * 0.16,
                ),
                pos2(
                    area.left() + area.width() * 0.52,
                    area.center().y - area.height() * 0.36,
                ),
                pos2(
                    area.left() + area.width() * 0.52,
                    area.center().y + area.height() * 0.36,
                ),
                pos2(
                    area.left() + area.width() * 0.28,
                    area.center().y + area.height() * 0.16,
                ),
                pos2(area.left(), area.center().y + area.height() * 0.16),
            ];
            painter.add(Shape::convex_polygon(speaker, Color32::TRANSPARENT, thin));
            painter.line_segment(
                [
                    pos2(
                        area.left() + area.width() * 0.65,
                        area.center().y - area.height() * 0.22,
                    ),
                    pos2(area.left() + area.width() * 0.8, area.center().y),
                ],
                thin,
            );
            painter.line_segment(
                [
                    pos2(area.left() + area.width() * 0.8, area.center().y),
                    pos2(
                        area.left() + area.width() * 0.65,
                        area.center().y + area.height() * 0.22,
                    ),
                ],
                thin,
            );
        }
        Icon::Undo | Icon::Redo => {
            let left = area.left() + area.width() * 0.16;
            let right = area.right() - area.width() * 0.16;
            let y = area.center().y + area.height() * 0.12;
            painter.line_segment([pos2(left, y), pos2(right, y)], stroke);
            let tip_x = if icon == Icon::Undo { left } else { right };
            let direction = if icon == Icon::Undo { 1.0 } else { -1.0 };
            painter.line_segment(
                [
                    pos2(tip_x, y),
                    pos2(
                        tip_x + direction * area.width() * 0.22,
                        y - area.height() * 0.22,
                    ),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(tip_x, y),
                    pos2(
                        tip_x + direction * area.width() * 0.22,
                        y + area.height() * 0.22,
                    ),
                ],
                stroke,
            );
        }
        Icon::Plus => {
            painter.line_segment([area.center_top(), area.center_bottom()], stroke);
            painter.line_segment([area.left_center(), area.right_center()], stroke);
        }
        Icon::Select => {
            painter.add(Shape::closed_line(
                vec![
                    area.left_top(),
                    pos2(area.left() + area.width() * 0.2, area.bottom()),
                    area.center(),
                    pos2(area.right(), area.center().y - area.height() * 0.1),
                ],
                thin,
            ));
        }
        Icon::Split => {
            for x in [area.left(), area.right()] {
                let direction = if x == area.left() { 1.0 } else { -1.0 };
                painter.line_segment([pos2(x, area.top()), pos2(x, area.bottom())], thin);
                for y in [area.top(), area.bottom()] {
                    painter.line_segment([pos2(x, y), pos2(x + direction * 3.0, y)], thin);
                }
            }
            painter.line_segment([area.center_top(), area.center_bottom()], thin);
        }
        Icon::Delete => {
            let bin = Rect::from_min_max(
                pos2(area.left() + 2.0, area.top() + 3.0),
                pos2(area.right() - 2.0, area.bottom()),
            );
            painter.rect_stroke(bin, CornerRadius::same(1), thin, StrokeKind::Inside);
            painter.line_segment(
                [pos2(area.left(), bin.top()), pos2(area.right(), bin.top())],
                thin,
            );
            painter.line_segment(
                [
                    pos2(area.center().x - 2.0, area.top()),
                    pos2(area.center().x + 2.0, area.top()),
                ],
                thin,
            );
        }
        Icon::Snap => {
            let left = pos2(area.left(), area.top());
            let right = pos2(area.right(), area.top());
            painter.add(Shape::line(
                vec![left, area.left_bottom(), area.right_bottom(), right],
                stroke,
            ));
            for x in [area.left(), area.right()] {
                painter.line_segment(
                    [
                        pos2(x - 2.0, area.top() + 3.0),
                        pos2(x + 2.0, area.top() + 3.0),
                    ],
                    thin,
                );
            }
        }
        Icon::Link => {
            for x in [area.left(), area.center().x - 1.0] {
                painter.rect_stroke(
                    Rect::from_min_size(
                        pos2(x, area.center().y - 3.0),
                        vec2(area.width() * 0.55, 6.0),
                    ),
                    CornerRadius::same(2),
                    thin,
                    StrokeKind::Inside,
                );
            }
        }
        Icon::Grid => {
            let gap = area.width() * 0.12;
            let cell = (area.width() - gap) * 0.5;
            for (x, y) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                let min = pos2(
                    area.left() + x * (cell + gap),
                    area.top() + y * (cell + gap),
                );
                painter.rect_stroke(
                    Rect::from_min_size(min, vec2(cell, cell)),
                    CornerRadius::same(1),
                    thin,
                    StrokeKind::Inside,
                );
            }
        }
        Icon::List => {
            for (index, y) in [0.25_f32, 0.5, 0.75].into_iter().enumerate() {
                let py = area.top() + area.height() * y;
                painter.circle_filled(pos2(area.left() + 1.5, py), 1.2, color);
                painter.line_segment(
                    [
                        pos2(area.left() + area.width() * 0.18, py),
                        pos2(area.right(), py),
                    ],
                    thin,
                );
                let _ = index;
            }
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
                Icon::Stickers,
                Icon::Effects,
                Icon::Transitions,
                Icon::Captions,
                Icon::Filters,
                Icon::Folder,
                Icon::Export,
                Icon::Close,
                Icon::Eye,
                Icon::Lock,
                Icon::Volume,
                Icon::Undo,
                Icon::Redo,
                Icon::Grid,
                Icon::List,
                Icon::Plus,
                Icon::Select,
                Icon::Split,
                Icon::Delete,
                Icon::Snap,
                Icon::Link,
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

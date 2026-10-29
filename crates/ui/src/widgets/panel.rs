//! Reusable rounded outer panel surface for the four editor regions.
//!
//! Children are clipped to the panel rectangle, and corner masks are painted
//! last so headers, scroll areas, or custom backgrounds cannot square off the
//! rounded outer corners. The gaps between panels remain the window background.

use egui::{
    pos2, vec2, Align, Color32, CornerRadius, Layout, Mesh, Painter, Rect, Shape, Stroke,
    StrokeKind, Ui, UiBuilder,
};

use crate::theme::{Palette, Radii};

/// Rounded surface reserved for the main editor panels.
pub(crate) fn show(ui: &mut Ui, rect: Rect, fill: Color32, add_contents: impl FnOnce(&mut Ui)) {
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    let radius = Radii::LARGE_CARD;
    ui.painter().rect_filled(rect, radius, fill);
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.set_clip_rect(rect);
            add_contents(ui);
        },
    );
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    paint_corner_masks(ui.painter(), rect, radius, Palette::APP_BG);
}

/// Restore rounded outer corners after arbitrary child painting.
///
/// Each mask covers the area between a rectangle corner and the inscribed
/// quarter-circle, using the parent background color.
pub(crate) fn paint_corner_masks(
    painter: &Painter,
    rect: Rect,
    radius: CornerRadius,
    parent_fill: Color32,
) {
    for mesh in corner_mask_meshes(rect, radius, parent_fill) {
        painter.add(Shape::mesh(mesh));
    }
}

pub(crate) fn corner_mask_meshes(rect: Rect, radius: CornerRadius, color: Color32) -> Vec<Mesh> {
    let radius = f32::from(radius.nw).min(rect.width().min(rect.height()) / 2.0);
    if radius <= 0.0 {
        return Vec::new();
    }

    let corners = [
        (
            rect.left_top(),
            pos2(rect.left() + radius, rect.top() + radius),
            -std::f32::consts::FRAC_PI_2,
            -std::f32::consts::PI,
        ),
        (
            rect.right_top(),
            pos2(rect.right() - radius, rect.top() + radius),
            -std::f32::consts::FRAC_PI_2,
            0.0,
        ),
        (
            rect.right_bottom(),
            pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
            std::f32::consts::FRAC_PI_2,
        ),
        (
            rect.left_bottom(),
            pos2(rect.left() + radius, rect.bottom() - radius),
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
        ),
    ];

    corners
        .into_iter()
        .map(|(corner, center, start_angle, end_angle)| {
            let mut mesh = Mesh::default();
            mesh.colored_vertex(corner, color);
            let steps = 8;
            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let angle = start_angle + (end_angle - start_angle) * t;
                mesh.colored_vertex(center + vec2(angle.cos(), angle.sin()) * radius, color);
            }
            for index in 0..steps {
                mesh.add_triangle(0, (index + 1) as u32, (index + 2) as u32);
            }
            mesh
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(pos2(100.0, 50.0), vec2(400.0, 250.0))
    }

    fn bounding_box(mesh: &Mesh) -> Rect {
        let mut result = Rect::NOTHING;
        for vertex in &mesh.vertices {
            result = result.union(Rect::from_min_size(vertex.pos, egui::Vec2::ZERO));
        }
        result
    }

    #[test]
    fn corner_masks_cover_only_the_four_corner_squares() {
        let masks = corner_mask_meshes(rect(), Radii::LARGE_CARD, Palette::APP_BG);
        assert_eq!(masks.len(), 4);

        let radius = f32::from(Radii::LARGE_CARD.nw);
        let corners = [
            Rect::from_min_size(rect().left_top(), vec2(radius, radius)),
            Rect::from_min_size(
                pos2(rect().right() - radius, rect().top()),
                vec2(radius, radius),
            ),
            Rect::from_min_size(
                pos2(rect().right() - radius, rect().bottom() - radius),
                vec2(radius, radius),
            ),
            Rect::from_min_size(
                pos2(rect().left(), rect().bottom() - radius),
                vec2(radius, radius),
            ),
        ];
        for (mesh, corner) in masks.iter().zip(corners) {
            let bounds = bounding_box(mesh);
            assert!(corner.contains_rect(bounds), "mask escaped its corner");
            assert!(mesh.vertices.iter().all(|v| v.color == Palette::APP_BG));
        }
    }

    #[test]
    fn small_or_degenerate_panels_do_not_emit_masks() {
        assert!(corner_mask_meshes(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(1.0, 1.0)),
            Radii::LARGE_CARD,
            Palette::APP_BG
        )
        .iter()
        .all(|mesh| mesh
            .vertices
            .iter()
            .all(|v| v.pos.x <= 1.0 && v.pos.y <= 1.0)));

        assert!(corner_mask_meshes(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 10.0)),
            Radii::LARGE_CARD,
            Palette::APP_BG
        )
        .is_empty());
    }
}

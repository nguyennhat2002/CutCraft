//! Preview Monitor panel.
//!
//! The canvas is intentionally empty: no decoder, GPU texture upload, or
//! playback exists yet. Aspect selection is display-only and transport controls
//! are painted disabled.

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Layout, Painter, Rect,
    Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui,
};

use crate::config;
use crate::editor_state::{EditorSession, PreviewAspect};
use crate::theme::Palette;
use crate::timecode::format_timecode;
use crate::widgets::common;

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    ui.horizontal(|ui| {
        common::section_heading(ui, "Preview");
        ui.add_space(6.0);
        common::badge(ui, "Placeholder", Palette::WARNING);
        ui.add_space(4.0);

        for aspect in PreviewAspect::ALL {
            let selected = session.preview_aspect == aspect;
            let response = ui.add(
                egui::Button::new(RichText::new(aspect.label()).size(10.0).strong().color(
                    if selected {
                        Palette::ACCENT
                    } else {
                        Palette::TEXT_MUTED
                    },
                ))
                .selected(selected)
                .min_size(vec2(40.0, 22.0)),
            );
            if response.clicked() {
                session.preview_aspect = aspect;
            }
            let _ = response.on_hover_text("Preview aspect ratio (display only).");
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(resolution_label(session.preview_aspect))
                    .size(10.5)
                    .color(Palette::TEXT_MUTED),
            );
        });
    });

    ui.add_space(8.0);

    let footer_height = 38.0;
    let available = ui.available_size();
    let canvas_area_height = (available.y - footer_height).max(1.0);
    let (canvas_area, _) =
        ui.allocate_exact_size(vec2(available.x, canvas_area_height), Sense::hover());

    let padded = canvas_area.shrink2(vec2(6.0, 4.0));
    let canvas = fit_preview_rect(padded, session.preview_aspect.ratio());
    let painter = ui.painter_at(canvas_area);

    if canvas.width() > 0.5 && canvas.height() > 0.5 {
        painter.rect_filled(canvas, CornerRadius::same(2), Palette::CANVAS);
        painter.rect_stroke(
            canvas,
            CornerRadius::same(2),
            Stroke::new(1.0, Palette::BORDER),
            StrokeKind::Inside,
        );

        if canvas.height() > 54.0 {
            painter.text(
                canvas.center() + vec2(0.0, -8.0),
                Align2::CENTER_CENTER,
                "No media loaded",
                FontId::proportional(15.0),
                Palette::TEXT_MUTED,
            );
        }
        if canvas.height() > 84.0 {
            painter.text(
                canvas.center() + vec2(0.0, 12.0),
                Align2::CENTER_CENTER,
                "Preview rendering is not implemented",
                FontId::proportional(10.5),
                Palette::TEXT_DISABLED,
            );
        }

        painter.text(
            canvas.left_top() + vec2(10.0, 9.0),
            Align2::LEFT_TOP,
            session.preview_aspect.label(),
            FontId::monospace(9.0),
            Palette::TEXT_DISABLED,
        );
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.add_space(4.0);
        let _ = transport_button(ui, TransportGlyph::ToStart);
        let _ = transport_button(ui, TransportGlyph::Play);
        let _ = transport_button(ui, TransportGlyph::Pause);
        let _ = transport_button(ui, TransportGlyph::ToEnd);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{} / 00:00:00:00", placeholder_timecode()))
                    .monospace()
                    .size(11.0)
                    .color(Palette::TEXT_DISABLED),
            )
            .on_hover_text("No project duration is available.");
        });
    });
}

fn placeholder_timecode() -> String {
    format_timecode(
        config::PLAYHEAD_MILLIS,
        config::PREVIEW_FPS_NUMERATOR,
        config::PREVIEW_FPS_DENOMINATOR,
    )
    .unwrap_or_else(|| "--:--:--:--".to_owned())
}

fn resolution_label(aspect: PreviewAspect) -> String {
    let (width, height) = aspect.dimensions();
    format!(
        "{width} \u{00d7} {height}  \u{00b7}  {}",
        rate_label(
            config::PREVIEW_FPS_NUMERATOR,
            config::PREVIEW_FPS_DENOMINATOR
        )
    )
}

fn rate_label(numerator: u32, denominator: u32) -> String {
    if denominator == 0 {
        return "unknown fps".to_owned();
    }
    if denominator == 1 {
        format!("{numerator} fps")
    } else {
        format!("{:.2} fps", f64::from(numerator) / f64::from(denominator))
    }
}

/// Fit a rectangle with `aspect` (width / height) inside `area`, centered.
///
/// Returns an empty rectangle at `area.min` for non-positive or non-finite
/// inputs so painting code cannot produce invalid geometry.
pub fn fit_preview_rect(area: Rect, aspect: f32) -> Rect {
    let size = area.size();
    if !aspect.is_finite()
        || aspect <= 0.0
        || !size.x.is_finite()
        || !size.y.is_finite()
        || size.x <= 0.0
        || size.y <= 0.0
    {
        return Rect::from_min_size(area.min, vec2(0.0, 0.0));
    }

    let (width, height) = if size.x / size.y > aspect {
        (size.y * aspect, size.y)
    } else {
        (size.x, size.x / aspect)
    };

    Rect::from_center_size(area.center(), vec2(width, height))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportGlyph {
    ToStart,
    Play,
    Pause,
    ToEnd,
}

impl TransportGlyph {
    fn tooltip(self) -> &'static str {
        match self {
            Self::ToStart | Self::ToEnd => "Seeking and playback are not implemented.",
            Self::Play => "Playback is not implemented in this milestone.",
            Self::Pause => "Pause is unavailable until playback exists.",
        }
    }
}

fn transport_button(ui: &mut Ui, glyph: TransportGlyph) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(30.0, 24.0), Sense::hover());
    let hovered = response.hovered();
    let fill = if hovered {
        Palette::WIDGET_HOVER
    } else {
        Palette::PANEL_RAISED
    };
    let color = if hovered {
        Palette::TEXT_SECONDARY
    } else {
        Palette::TEXT_DISABLED
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    paint_transport_glyph(&painter, glyph, rect.center(), color);

    response
        .on_hover_cursor(CursorIcon::NotAllowed)
        .on_hover_text(glyph.tooltip())
}

fn paint_transport_glyph(
    painter: &Painter,
    glyph: TransportGlyph,
    center: egui::Pos2,
    color: Color32,
) {
    let triangle_right = |cx: f32, cy: f32| {
        vec![
            pos2(cx - 4.0, cy - 5.0),
            pos2(cx - 4.0, cy + 5.0),
            pos2(cx + 5.0, cy),
        ]
    };
    let triangle_left = |cx: f32, cy: f32| {
        vec![
            pos2(cx + 4.0, cy - 5.0),
            pos2(cx + 4.0, cy + 5.0),
            pos2(cx - 5.0, cy),
        ]
    };

    match glyph {
        TransportGlyph::ToStart => {
            painter.rect_filled(
                Rect::from_center_size(pos2(center.x - 6.5, center.y), vec2(2.0, 10.0)),
                CornerRadius::ZERO,
                color,
            );
            painter.add(Shape::convex_polygon(
                triangle_left(center.x, center.y),
                color,
                Stroke::NONE,
            ));
        }
        TransportGlyph::Play => {
            painter.add(Shape::convex_polygon(
                triangle_right(center.x, center.y),
                color,
                Stroke::NONE,
            ));
        }
        TransportGlyph::Pause => {
            for offset in [-3.0_f32, 3.0] {
                painter.rect_filled(
                    Rect::from_center_size(pos2(center.x + offset, center.y), vec2(2.5, 10.0)),
                    CornerRadius::same(1),
                    color,
                );
            }
        }
        TransportGlyph::ToEnd => {
            painter.add(Shape::convex_polygon(
                triangle_right(center.x - 1.0, center.y),
                color,
                Stroke::NONE,
            ));
            painter.rect_filled(
                Rect::from_center_size(pos2(center.x + 6.5, center.y), vec2(2.0, 10.0)),
                CornerRadius::ZERO,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_wide_area_by_height_and_centers() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let fitted = fit_preview_rect(area, PreviewAspect::Landscape.ratio());

        assert!((fitted.width() - 800.0).abs() < 0.01);
        assert!((fitted.height() - 450.0).abs() < 0.01);
        assert!((fitted.center().y - area.center().y).abs() < 0.01);
    }

    #[test]
    fn fits_tall_area_by_width_and_centers() {
        let area = Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 600.0));
        let fitted = fit_preview_rect(area, 16.0 / 9.0);

        assert!((fitted.width() - 400.0).abs() < 0.01);
        assert!((fitted.height() - 225.0).abs() < 0.01);
        assert!((fitted.center().x - area.center().x).abs() < 0.01);
        assert!((fitted.center().y - area.center().y).abs() < 0.01);
    }

    #[test]
    fn portrait_aspect_fits_inside_a_wide_area() {
        let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let fitted = fit_preview_rect(area, PreviewAspect::Portrait.ratio());

        assert!((fitted.width() - 337.5).abs() < 0.01);
        assert!((fitted.height() - 600.0).abs() < 0.01);
        assert!((fitted.center().x - area.center().x).abs() < 0.01);
    }

    #[test]
    fn invalid_geometry_returns_an_empty_rect() {
        let area = Rect::from_min_size(pos2(3.0, 4.0), vec2(0.0, 100.0));
        assert_eq!(fit_preview_rect(area, 16.0 / 9.0).size(), vec2(0.0, 0.0));

        let area = Rect::from_min_size(pos2(3.0, 4.0), vec2(100.0, 100.0));
        assert_eq!(fit_preview_rect(area, f32::NAN).size(), vec2(0.0, 0.0));
        assert_eq!(fit_preview_rect(area, 0.0).size(), vec2(0.0, 0.0));
    }

    #[test]
    fn rate_label_distinguishes_integer_and_rational_rates() {
        assert_eq!(rate_label(30, 1), "30 fps");
        assert_eq!(rate_label(30_000, 1_001), "29.97 fps");
        assert_eq!(rate_label(30, 0), "unknown fps");
    }

    #[test]
    fn preview_renders_headlessly_and_aspect_selection_is_stored() {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 700.0))),
            ..Default::default()
        };
        let mut session = EditorSession {
            preview_aspect: PreviewAspect::Portrait,
            ..EditorSession::default()
        };
        let output = context.run_ui(input, |ui| show(ui, &mut session));

        assert!(!output.shapes.is_empty());
        assert_eq!(session.preview_aspect, PreviewAspect::Portrait);
        output.drop_without_applying_deltas();
    }
}

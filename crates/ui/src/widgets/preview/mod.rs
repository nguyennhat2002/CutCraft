//! Preview monitor with aspect-aware stage and disabled transport controls.

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
        ui.add_space(4.0);
        common::badge(ui, "Placeholder", Palette::WARNING);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(resolution_label(session.preview_aspect))
                    .size(9.5)
                    .color(Palette::TEXT_MUTED),
            );
        });
    });

    ui.add_space(5.0);

    let footer_height = 58.0;
    let available = ui.available_size();
    let monitor_height = (available.y - footer_height).max(80.0);
    let (monitor_area, _) =
        ui.allocate_exact_size(vec2(available.x, monitor_height), Sense::hover());

    let painter = ui.painter_at(monitor_area);
    painter.rect_filled(
        monitor_area,
        CornerRadius::same(4),
        Color32::from_rgb(0x0b, 0x0b, 0x0c),
    );
    painter.rect_stroke(
        monitor_area,
        CornerRadius::same(4),
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    let padded = monitor_area.shrink2(vec2(12.0, 10.0));
    let canvas = fit_preview_rect(padded, session.preview_aspect.ratio());

    if canvas.width() > 1.0 && canvas.height() > 1.0 {
        painter.rect_filled(canvas, CornerRadius::same(2), Palette::CANVAS);
        painter.rect_stroke(
            canvas,
            CornerRadius::same(2),
            Stroke::new(1.0, Palette::BORDER),
            StrokeKind::Inside,
        );

        let (width, height) = session.preview_aspect.dimensions();
        painter.text(
            canvas.left_top() + vec2(10.0, 9.0),
            Align2::LEFT_TOP,
            format!("{width} \u{00d7} {height}"),
            FontId::monospace(8.5),
            Palette::TEXT_DISABLED,
        );

        if canvas.height() > 72.0 {
            let center = canvas.center();
            let icon_center = center - vec2(0.0, 24.0);
            painter.circle_stroke(icon_center, 17.0, Stroke::new(1.2, Palette::TEXT_DISABLED));
            painter.add(Shape::convex_polygon(
                vec![
                    pos2(icon_center.x - 4.0, icon_center.y - 7.0),
                    pos2(icon_center.x - 4.0, icon_center.y + 7.0),
                    pos2(icon_center.x + 7.0, icon_center.y),
                ],
                Palette::TEXT_DISABLED,
                Stroke::NONE,
            ));
            painter.text(
                center + vec2(0.0, 12.0),
                Align2::CENTER_CENTER,
                "No media loaded",
                FontId::proportional(13.0),
                Palette::TEXT_MUTED,
            );
            painter.text(
                center + vec2(0.0, 29.0),
                Align2::CENTER_CENTER,
                "Preview rendering is not implemented",
                FontId::proportional(9.0),
                Palette::TEXT_DISABLED,
            );
        }
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} / 00:00:00:00", placeholder_timecode()))
                .monospace()
                .size(10.5)
                .color(Palette::TEXT_SECONDARY),
        )
        .on_hover_text("Static placeholder timecode; no project is loaded.");

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let _ = transport_button(ui, TransportGlyph::ToEnd);
            let _ = transport_button(ui, TransportGlyph::Pause);
            let _ = transport_button(ui, TransportGlyph::Play);
            let _ = transport_button(ui, TransportGlyph::ToStart);
        });
    });

    ui.add_space(2.0);

    ui.horizontal(|ui| {
        aspect_button(ui, session, PreviewAspect::Landscape);
        aspect_button(ui, session, PreviewAspect::Portrait);
        ui.add_space(4.0);
        let fit = ui.add_enabled(false, egui::Button::new(RichText::new("Fit").size(9.5)));
        let _ = fit.on_disabled_hover_text("Viewport fitting is not implemented.");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let zoom = ui.add_enabled(
                false,
                egui::Button::new(RichText::new("100%").size(9.5).monospace()),
            );
            let _ = zoom.on_disabled_hover_text("Preview zoom is not implemented.");
        });
    });
}

fn aspect_button(ui: &mut Ui, session: &mut EditorSession, aspect: PreviewAspect) {
    let selected = session.preview_aspect == aspect;
    let response = ui.add(
        egui::Button::new(RichText::new(aspect.label()).size(9.5).color(if selected {
            Palette::ACCENT
        } else {
            Palette::TEXT_MUTED
        }))
        .selected(selected)
        .min_size(vec2(38.0, 22.0)),
    );
    if response.clicked() {
        session.preview_aspect = aspect;
    }
    let _ = response.on_hover_text("Preview aspect ratio (display only).");
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
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 22.0), Sense::hover());
    let hovered = response.hovered();
    let fill = if hovered {
        Palette::WIDGET_HOVER
    } else {
        Palette::WIDGET_BG
    };
    let color = if hovered {
        Palette::TEXT_SECONDARY
    } else {
        Palette::TEXT_DISABLED
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(3), fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(3),
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
            pos2(cx - 3.5, cy - 4.5),
            pos2(cx - 3.5, cy + 4.5),
            pos2(cx + 4.5, cy),
        ]
    };
    let triangle_left = |cx: f32, cy: f32| {
        vec![
            pos2(cx + 3.5, cy - 4.5),
            pos2(cx + 3.5, cy + 4.5),
            pos2(cx - 4.5, cy),
        ]
    };

    match glyph {
        TransportGlyph::ToStart => {
            painter.rect_filled(
                Rect::from_center_size(pos2(center.x - 6.0, center.y), vec2(1.8, 9.0)),
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
            for offset in [-2.8_f32, 2.8] {
                painter.rect_filled(
                    Rect::from_center_size(pos2(center.x + offset, center.y), vec2(2.2, 9.0)),
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
                Rect::from_center_size(pos2(center.x + 6.0, center.y), vec2(1.8, 9.0)),
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
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 500.0))),
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

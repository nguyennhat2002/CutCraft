//! Preview monitor with reference-style player header and transport bar.

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin,
    Painter, Rect, Response, RichText, Sense, Shape, Stroke, StrokeKind, Ui,
};

use crate::config;
use crate::editor_state::{EditorSession, PreviewAspect};
use crate::theme::Palette;
use crate::timecode::format_timecode;

const HEADER_HEIGHT: f32 = config::EDITOR_COLUMN_HEADER_HEIGHT;
const FOOTER_HEIGHT: f32 = 40.0;

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    header(ui, session);

    let available = ui.available_size();
    let monitor_height = (available.y - FOOTER_HEIGHT - 10.0).max(80.0);
    let (monitor_area, _) =
        ui.allocate_exact_size(vec2(available.x, monitor_height), Sense::hover());
    paint_monitor(ui, monitor_area, session);

    ui.add_space(5.0);
    footer(ui);
}

fn header(ui: &mut Ui, session: &mut EditorSession) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEADER_HEIGHT), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, Palette::TOOLBAR_BG);
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
    );

    let player = Rect::from_min_size(rect.min, vec2(74.0, rect.height()));
    painter.text(
        player.center() - vec2(0.0, 2.0),
        Align2::CENTER_CENTER,
        "Player",
        FontId::proportional(11.0),
        Palette::TEXT_PRIMARY,
    );
    painter.rect_filled(
        Rect::from_min_size(
            pos2(player.center().x - 16.0, player.bottom() - 2.0),
            vec2(32.0, 2.0),
        ),
        CornerRadius::same(1),
        Palette::ACCENT,
    );

    let timeline = Rect::from_min_size(pos2(player.right(), rect.top()), vec2(96.0, rect.height()));
    let timeline_response = ui.interact(
        timeline,
        ui.scope_id().with("preview_timeline_tab"),
        Sense::hover(),
    );
    ui.painter().text(
        timeline.center() - vec2(0.0, 2.0),
        Align2::CENTER_CENTER,
        "Timeline 01",
        FontId::proportional(10.5),
        Palette::TEXT_MUTED,
    );
    timeline_response.on_hover_text("Timeline-player docking is not implemented.");

    let chip = Rect::from_min_size(
        pos2(rect.right() - 72.0, rect.center().y - 11.0),
        vec2(62.0, 22.0),
    );
    let response = ui.interact(chip, ui.scope_id().with("preview_aspect"), Sense::click());
    let hovered = response.hovered();
    ui.painter().rect_filled(
        chip,
        CornerRadius::same(4),
        if hovered {
            Palette::WIDGET_HOVER
        } else {
            Palette::WIDGET_BG
        },
    );
    ui.painter().rect_stroke(
        chip,
        CornerRadius::same(4),
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );
    ui.painter().text(
        pos2(chip.left() + 9.0, chip.center().y),
        Align2::LEFT_CENTER,
        session.preview_aspect.label(),
        FontId::monospace(9.5),
        Palette::TEXT_SECONDARY,
    );
    let cx = chip.right() - 11.0;
    let cy = chip.center().y - 1.0;
    let chevron = Stroke::new(1.2, Palette::TEXT_MUTED);
    ui.painter()
        .line_segment([pos2(cx - 3.0, cy), pos2(cx, cy + 3.0)], chevron);
    ui.painter()
        .line_segment([pos2(cx, cy + 3.0), pos2(cx + 3.0, cy)], chevron);

    if response.clicked() {
        session.preview_aspect = match session.preview_aspect {
            PreviewAspect::Landscape => PreviewAspect::Portrait,
            PreviewAspect::Portrait => PreviewAspect::Landscape,
        };
    }
    let _ = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{} · click to switch preview aspect (display only)",
            resolution_label(session.preview_aspect)
        ));
}

fn paint_monitor(ui: &Ui, area: Rect, session: &EditorSession) {
    let painter = ui.painter_at(area);
    painter.rect_filled(
        area,
        CornerRadius::ZERO,
        Color32::from_rgb(0x0b, 0x0b, 0x0c),
    );

    let padded = area.shrink2(vec2(14.0, 10.0));
    let canvas = fit_preview_rect(padded, session.preview_aspect.ratio());
    if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
        return;
    }

    painter.rect_filled(canvas, CornerRadius::same(2), Palette::CANVAS);
    painter.rect_stroke(
        canvas,
        CornerRadius::same(2),
        Stroke::new(1.0, Palette::BORDER_SUBTLE),
        StrokeKind::Inside,
    );

    let (width, height) = session.preview_aspect.dimensions();
    if canvas.height() > 260.0 {
        painter.text(
            canvas.left_top() + vec2(8.0, 7.0),
            Align2::LEFT_TOP,
            format!("{width} \u{00d7} {height}"),
            FontId::monospace(8.0),
            Palette::TEXT_DISABLED,
        );
    }

    if canvas.height() < 90.0 {
        return;
    }

    let center = canvas.center();
    let icon_center = center - vec2(0.0, 20.0);
    painter.circle_stroke(icon_center, 15.0, Stroke::new(1.0, Palette::TEXT_DISABLED));
    painter.add(Shape::convex_polygon(
        vec![
            pos2(icon_center.x - 3.5, icon_center.y - 6.0),
            pos2(icon_center.x - 3.5, icon_center.y + 6.0),
            pos2(icon_center.x + 6.0, icon_center.y),
        ],
        Palette::TEXT_DISABLED,
        Stroke::NONE,
    ));
    painter.text(
        center + vec2(0.0, 14.0),
        Align2::CENTER_CENTER,
        "No media loaded",
        FontId::proportional(12.0),
        Palette::TEXT_SECONDARY,
    );
    painter.text(
        center + vec2(0.0, 30.0),
        Align2::CENTER_CENTER,
        "Preview rendering is not implemented",
        FontId::proportional(8.5),
        Palette::TEXT_DISABLED,
    );
}

fn footer(ui: &mut Ui) {
    Frame::new()
        .fill(Palette::PANEL_BG)
        .stroke(Stroke::new(1.0, Palette::BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal_centered(|ui| {
                let left_width = 112.0;
                let right_width = 52.0;
                let center_width = (ui.available_width() - left_width - right_width).max(96.0);

                ui.allocate_ui_with_layout(
                    vec2(left_width, 24.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(placeholder_timecode())
                                .monospace()
                                .size(10.0)
                                .color(Palette::TEXT_SECONDARY),
                        )
                        .on_hover_text("Static placeholder timecode; no project is loaded.");
                    },
                );

                let transport_width = 3.0 * 26.0 + 2.0 * 4.0;
                ui.allocate_ui_with_layout(
                    vec2(center_width, 24.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.add_space(((center_width - transport_width) / 2.0).max(0.0));
                        let _ = transport_button(ui, TransportGlyph::ToStart, false);
                        ui.add_space(4.0);
                        let _ = transport_button(ui, TransportGlyph::Play, true);
                        ui.add_space(4.0);
                        let _ = transport_button(ui, TransportGlyph::ToEnd, false);
                    },
                );

                ui.allocate_ui_with_layout(
                    vec2(right_width, 24.0),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        let zoom = ui.add_enabled(
                            false,
                            egui::Button::new(RichText::new("100%").size(9.5).monospace())
                                .min_size(vec2(44.0, 22.0)),
                        );
                        let _ = zoom.on_disabled_hover_text("Preview zoom is not implemented.");
                    },
                );
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
    ToEnd,
}

impl TransportGlyph {
    fn tooltip(self) -> &'static str {
        match self {
            Self::ToStart | Self::ToEnd => "Seeking and playback are not implemented.",
            Self::Play => "Playback is not implemented in this milestone.",
        }
    }
}

fn transport_button(ui: &mut Ui, glyph: TransportGlyph, primary: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(26.0, 22.0), Sense::hover());
    let hovered = response.hovered();
    let (fill, color) = if primary {
        (
            if hovered {
                Palette::ACCENT_LIGHT
            } else {
                Palette::ACCENT
            },
            Palette::ON_ACCENT,
        )
    } else {
        (
            if hovered {
                Palette::WIDGET_HOVER
            } else {
                Palette::WIDGET_BG
            },
            if hovered {
                Palette::TEXT_SECONDARY
            } else {
                Palette::TEXT_DISABLED
            },
        )
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(3), fill);
    if !primary {
        painter.rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, Palette::BORDER_SUBTLE),
            StrokeKind::Inside,
        );
    }
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
            pos2(cx - 3.0, cy - 4.0),
            pos2(cx - 3.0, cy + 4.0),
            pos2(cx + 4.0, cy),
        ]
    };
    let triangle_left = |cx: f32, cy: f32| {
        vec![
            pos2(cx + 3.0, cy - 4.0),
            pos2(cx + 3.0, cy + 4.0),
            pos2(cx - 4.0, cy),
        ]
    };

    match glyph {
        TransportGlyph::ToStart => {
            painter.rect_filled(
                Rect::from_center_size(pos2(center.x - 5.5, center.y), vec2(1.6, 8.0)),
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
        TransportGlyph::ToEnd => {
            painter.add(Shape::convex_polygon(
                triangle_right(center.x - 1.0, center.y),
                color,
                Stroke::NONE,
            ));
            painter.rect_filled(
                Rect::from_center_size(pos2(center.x + 5.5, center.y), vec2(1.6, 8.0)),
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
        crate::theme::configure(&context);
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

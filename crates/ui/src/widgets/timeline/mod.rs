//! Timeline shell: toolbar, ruler, empty track rows, playhead, and scrolling.
//!
//! Zoom is display-only and changes ruler density. No clips exist and every
//! editing command remains disabled.

pub mod clip_view;
pub mod playhead;
pub mod scale;
pub mod track_view;

use egui::{
    pos2, vec2, Align, Align2, CornerRadius, FontId, Rect, RichText, ScrollArea, Sense, Stroke, Ui,
};

use crate::config;
use crate::editor_state::EditorSession;
use crate::theme::Palette;
use crate::widgets::common;
use crate::widgets::icons::{self, Icon};

use self::scale::{format_ruler_label, TimelineScale};
use self::track_view::TrackKind;

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    ui.horizontal(|ui| {
        common::section_heading(ui, "Timeline");
        ui.add_space(3.0);
        common::badge(ui, "Empty", Palette::TEXT_MUTED);
        ui.add_space(3.0);
        ui.separator();
        ui.add_space(3.0);

        for (label, hint) in [
            (
                "Split",
                "Splitting requires timeline commands and is not implemented.",
            ),
            (
                "Delete",
                "Deleting clips requires selection and is not implemented.",
            ),
            ("Add track", "Track management is not implemented."),
        ] {
            let response = ui.add_enabled(
                false,
                egui::Button::new(RichText::new(label).size(9.5)).min_size(vec2(0.0, 20.0)),
            );
            let _ = response.on_disabled_hover_text(hint);
        }

        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("00:00:00:00 / 00:00:00:00")
                    .monospace()
                    .size(9.5)
                    .color(Palette::TEXT_DISABLED),
            );
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} s \u{00b7} 0 clips",
                    config::TIMELINE_DURATION_SECONDS
                ))
                .size(9.5)
                .color(Palette::TEXT_DISABLED),
            );
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let plus = ui.small_button("+").on_hover_text("Zoom timeline in");
            if plus.clicked() {
                session.zoom_timeline_in();
            }
            ui.label(
                RichText::new(format!(
                    "{:.0}%",
                    session.timeline_pixels_per_second / config::TIMELINE_PIXELS_PER_SECOND * 100.0
                ))
                .monospace()
                .size(9.5)
                .color(Palette::TEXT_SECONDARY),
            );
            let minus = ui
                .small_button("\u{2212}")
                .on_hover_text("Zoom timeline out");
            if minus.clicked() {
                session.zoom_timeline_out();
            }
        });
    });

    ui.add_space(6.0);

    let tracks = track_view::tracks();
    let tracks_height =
        config::TIMELINE_RULER_HEIGHT + config::TIMELINE_TRACK_HEIGHT * tracks.len() as f32;
    let available_body_height = ui.available_height().max(0.0);
    let content_height = (available_body_height - 16.0).max(tracks_height);
    let scale = TimelineScale::new(
        session.timeline_pixels_per_second,
        config::TIMELINE_LEADING_PADDING,
    );

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        let (header_rect, _) = ui.allocate_exact_size(
            vec2(config::TRACK_HEADER_WIDTH, content_height),
            Sense::hover(),
        );
        paint_track_headers(ui, header_rect);

        let _ = ScrollArea::horizontal()
            .id_salt("timeline_horizontal_scroll")
            .auto_shrink([false, true])
            .max_height(available_body_height)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                let (content_rect, _) = ui.allocate_exact_size(
                    vec2(
                        scale.content_width(config::TIMELINE_DURATION_SECONDS),
                        content_height,
                    ),
                    Sense::hover(),
                );
                paint_timeline_content(ui, content_rect, &scale, config::TIMELINE_DURATION_SECONDS);
            });
    });
}

fn paint_track_headers(ui: &Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(3), Palette::TRACK_HEADER_BG);

    let ruler_rect =
        Rect::from_min_size(rect.min, vec2(rect.width(), config::TIMELINE_RULER_HEIGHT));
    painter.rect_filled(ruler_rect, CornerRadius::same(3), Palette::RULER_BG);
    painter.text(
        pos2(rect.left() + 12.0, ruler_rect.center().y),
        Align2::LEFT_CENTER,
        "TRACKS",
        FontId::proportional(8.5),
        Palette::TEXT_DISABLED,
    );
    painter.text(
        pos2(rect.right() - 10.0, ruler_rect.center().y),
        Align2::RIGHT_CENTER,
        format!("{} TRACKS", track_view::tracks().len()),
        FontId::proportional(7.5),
        Palette::TEXT_DISABLED,
    );

    for (index, track) in track_view::tracks().iter().enumerate() {
        let row = track_row_rect(rect, index);
        let stripe_color = match track.kind {
            TrackKind::Video => Palette::ACCENT,
            TrackKind::Audio => Palette::WARNING,
        };

        painter.line_segment(
            [pos2(row.left(), row.top()), pos2(row.right(), row.top())],
            Stroke::new(1.0, Palette::BORDER_SUBTLE),
        );
        painter.rect_filled(
            Rect::from_min_size(row.min, vec2(3.0, row.height())),
            CornerRadius::ZERO,
            stripe_color,
        );
        painter.text(
            pos2(row.left() + 13.0, row.top() + 16.0),
            Align2::LEFT_CENTER,
            track.label,
            FontId::monospace(10.5),
            Palette::TEXT_PRIMARY,
        );
        painter.text(
            pos2(row.left() + 42.0, row.top() + 16.0),
            Align2::LEFT_CENTER,
            track.name,
            FontId::proportional(10.5),
            Palette::TEXT_SECONDARY,
        );
        painter.text(
            pos2(row.left() + 42.0, row.top() + 32.0),
            Align2::LEFT_CENTER,
            match track.kind {
                TrackKind::Video => "Video track",
                TrackKind::Audio => "Audio track",
            },
            FontId::proportional(8.5),
            Palette::TEXT_DISABLED,
        );

        let primary_icon = match track.kind {
            TrackKind::Video => Icon::Eye,
            TrackKind::Audio => Icon::Volume,
        };
        for (offset, icon) in [(40.0_f32, primary_icon), (18.0, Icon::Lock)] {
            let icon_rect = Rect::from_center_size(
                pos2(row.right() - offset, row.center().y),
                vec2(14.0, 14.0),
            );
            icons::paint(&painter, icon_rect, icon, Palette::TEXT_DISABLED);
        }
    }

    painter.line_segment(
        [
            pos2(rect.right() - 1.0, rect.top()),
            pos2(rect.right() - 1.0, rect.bottom()),
        ],
        Stroke::new(1.0, Palette::BORDER),
    );
}

fn paint_timeline_content(ui: &Ui, rect: Rect, scale: &TimelineScale, duration_seconds: u64) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(3), Palette::TRACK_BG);

    let ruler_rect =
        Rect::from_min_size(rect.min, vec2(rect.width(), config::TIMELINE_RULER_HEIGHT));
    painter.rect_filled(ruler_rect, CornerRadius::same(3), Palette::RULER_BG);

    for (index, track) in track_view::tracks().iter().enumerate() {
        let row = track_row_rect(rect, index);
        if index % 2 == 1 {
            painter.rect_filled(row, CornerRadius::ZERO, Palette::TRACK_ALT_BG);
        }
        painter.line_segment(
            [pos2(row.left(), row.top()), pos2(row.right(), row.top())],
            Stroke::new(1.0, Palette::BORDER_SUBTLE),
        );
        painter.text(
            pos2(row.left() + scale.leading_padding() + 12.0, row.center().y),
            Align2::LEFT_CENTER,
            match track.kind {
                TrackKind::Video => "No clips \u{2014} video lane",
                TrackKind::Audio => "No clips \u{2014} audio lane",
            },
            FontId::proportional(9.5),
            Palette::TEXT_DISABLED,
        );
    }

    let content_left = rect.left();
    let intervals = scale.ticks(duration_seconds);
    let interval_seconds = scale.major_tick_interval() as f64;

    for second in &intervals {
        let x = content_left + scale.x_for_seconds(*second as f64);
        if x < rect.left() || x > rect.right() {
            continue;
        }

        painter.line_segment(
            [pos2(x, ruler_rect.bottom()), pos2(x, rect.bottom())],
            Stroke::new(1.0, Palette::BORDER_SUBTLE),
        );
        painter.line_segment(
            [
                pos2(x, ruler_rect.bottom() - 7.0),
                pos2(x, ruler_rect.bottom()),
            ],
            Stroke::new(1.0, Palette::BORDER),
        );

        for step in 1..5 {
            let minor_x = content_left
                + scale.x_for_seconds(*second as f64 + interval_seconds * step as f64 / 5.0);
            if minor_x > rect.left() && minor_x < rect.right() {
                painter.line_segment(
                    [
                        pos2(minor_x, ruler_rect.bottom() - 3.0),
                        pos2(minor_x, ruler_rect.bottom()),
                    ],
                    Stroke::new(1.0, Palette::BORDER_SUBTLE),
                );
            }
        }

        if x + 36.0 < rect.right() {
            painter.text(
                pos2(x + 4.0, ruler_rect.center().y),
                Align2::LEFT_CENTER,
                format_ruler_label(*second),
                FontId::monospace(9.0),
                Palette::TEXT_MUTED,
            );
        }
    }

    painter.line_segment(
        [
            pos2(rect.left(), ruler_rect.bottom()),
            pos2(rect.right(), ruler_rect.bottom()),
        ],
        Stroke::new(1.0, Palette::BORDER),
    );

    let playhead_x = content_left + playhead::x_for_millis(scale, config::PLAYHEAD_MILLIS);
    playhead::paint(&painter, rect.top(), rect.bottom(), playhead_x);
}

fn track_row_rect(rect: Rect, index: usize) -> Rect {
    Rect::from_min_size(
        pos2(
            rect.left(),
            rect.top()
                + config::TIMELINE_RULER_HEIGHT
                + config::TIMELINE_TRACK_HEIGHT * index as f32,
        ),
        vec2(rect.width(), config::TIMELINE_TRACK_HEIGHT),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session() -> EditorSession {
        EditorSession::default()
    }

    #[test]
    fn timeline_renders_headlessly_with_scroll_content() {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 360.0))),
            ..Default::default()
        };
        let mut session = test_session();
        let output = context.run_ui(input, |ui| show(ui, &mut session));

        assert!(!output.shapes.is_empty());
        output.drop_without_applying_deltas();
    }

    #[test]
    fn zero_ruler_label_and_playhead_are_not_hidden_under_track_headers() {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 360.0))),
            ..Default::default()
        };
        let mut session = test_session();
        let output = context.run_ui(input, |ui| show(ui, &mut session));

        let zero_label = output.shapes.iter().find_map(|clipped| {
            if let egui::Shape::Text(text) = &clipped.shape {
                (text.galley.text() == "0:00").then_some((text.pos, clipped.clip_rect))
            } else {
                None
            }
        });
        let (label_pos, label_clip) = zero_label.expect("zero-second label should be painted");
        assert!(
            label_clip.contains(label_pos),
            "0:00 label must be inside the visible timeline clip"
        );

        let playhead = output.shapes.iter().find_map(|clipped| {
            if let egui::Shape::LineSegment { points, stroke } = &clipped.shape {
                (stroke.color == Palette::PLAYHEAD && stroke.width >= 2.0)
                    .then_some((points[0].x, clipped.clip_rect))
            } else {
                None
            }
        });
        let (playhead_x, playhead_clip) = playhead.expect("playhead line should be painted");
        let expected = playhead_clip.left() + config::TIMELINE_LEADING_PADDING;
        assert!(
            (playhead_x - expected).abs() < 0.01,
            "playhead should sit at time zero inside the content area"
        );
        assert!(playhead_clip.contains(pos2(playhead_x, playhead_clip.center().y)));

        output.drop_without_applying_deltas();
    }

    #[test]
    fn track_rows_tile_below_the_ruler() {
        let rect = Rect::from_min_size(
            pos2(10.0, 20.0),
            vec2(
                600.0,
                config::TIMELINE_RULER_HEIGHT
                    + config::TIMELINE_TRACK_HEIGHT * track_view::tracks().len() as f32,
            ),
        );
        let first = track_row_rect(rect, 0);
        let second = track_row_rect(rect, 1);
        let last = track_row_rect(rect, track_view::tracks().len() - 1);

        assert_eq!(first.top(), rect.top() + config::TIMELINE_RULER_HEIGHT);
        assert_eq!(second.top(), first.bottom());
        assert_eq!(last.bottom(), rect.bottom());
    }
}

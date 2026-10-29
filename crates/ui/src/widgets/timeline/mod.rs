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
use crate::theme::{Palette, Radii};
use crate::widgets::common;
use crate::widgets::icons::{self, Icon};

use self::scale::{format_ruler_label, TimelineScale};
use self::track_view::TrackKind;

pub fn show(ui: &mut Ui, session: &mut EditorSession) {
    ui.horizontal(|ui| {
        common::section_heading(ui, "Timeline");
        ui.add_space(3.0);
        common::badge(ui, "Demo", Palette::WARNING);
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
                    "{} s \u{00b7} demo only",
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
    let available_body_height = ui.available_height().max(0.0);
    let track_height = ((available_body_height - 16.0 - config::TIMELINE_RULER_HEIGHT)
        / tracks.len() as f32)
        .clamp(20.0, 58.0);
    let tracks_height = config::TIMELINE_RULER_HEIGHT + track_height * tracks.len() as f32;
    let content_height = tracks_height.max((available_body_height - 14.0).max(0.0));
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
        paint_track_headers(ui, header_rect, track_height);

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
                paint_timeline_content(
                    ui,
                    content_rect,
                    &scale,
                    config::TIMELINE_DURATION_SECONDS,
                    track_height,
                );
            });
    });
}

fn paint_track_headers(ui: &Ui, rect: Rect, track_height: f32) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, Palette::TRACK_HEADER_BG);

    let ruler_rect =
        Rect::from_min_size(rect.min, vec2(rect.width(), config::TIMELINE_RULER_HEIGHT));
    painter.rect_filled(ruler_rect, CornerRadius::ZERO, Palette::RULER_BG);
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
        let row = track_row_rect(rect, index, track_height);
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
            pos2(row.left() + 13.0, row.center().y - 6.0),
            Align2::LEFT_CENTER,
            track.label,
            FontId::monospace(10.5),
            Palette::TEXT_PRIMARY,
        );
        painter.text(
            pos2(row.left() + 42.0, row.center().y - 6.0),
            Align2::LEFT_CENTER,
            track.name,
            FontId::proportional(10.5),
            Palette::TEXT_SECONDARY,
        );
        painter.text(
            pos2(row.left() + 42.0, row.center().y + 8.0),
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

fn paint_timeline_content(
    ui: &Ui,
    rect: Rect,
    scale: &TimelineScale,
    duration_seconds: u64,
    track_height: f32,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, Palette::TRACK_BG);

    let ruler_rect =
        Rect::from_min_size(rect.min, vec2(rect.width(), config::TIMELINE_RULER_HEIGHT));
    painter.rect_filled(ruler_rect, CornerRadius::ZERO, Palette::RULER_BG);

    for (index, track) in track_view::tracks().iter().enumerate() {
        let row = track_row_rect(rect, index, track_height);
        if index % 2 == 1 {
            painter.rect_filled(row, CornerRadius::ZERO, Palette::TRACK_ALT_BG);
        }
        painter.line_segment(
            [pos2(row.left(), row.top()), pos2(row.right(), row.top())],
            Stroke::new(1.0, Palette::BORDER_SUBTLE),
        );
        let has_demo = demo_clip(index).is_some();
        if !has_demo {
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
    }

    paint_demo_clips(&painter, rect, scale, track_height);

    let content_left = rect.left();
    let viewport = painter.clip_rect();
    let intervals = scale.ticks(duration_seconds);
    let interval_seconds = scale.major_tick_interval() as f64;

    for second in &intervals {
        let x = content_left + scale.x_for_seconds(*second as f64);
        if x < viewport.left() || x > viewport.right() {
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

        let subdivisions = if interval_seconds <= 2.0 { 4 } else { 10 };
        for step in 1..subdivisions {
            let minor_seconds =
                *second as f64 + interval_seconds * step as f64 / f64::from(subdivisions);
            let minor_x = content_left + scale.x_for_seconds(minor_seconds);
            if minor_x > viewport.left() && minor_x < viewport.right() {
                painter.line_segment(
                    [
                        pos2(minor_x, ruler_rect.bottom() - 4.0),
                        pos2(minor_x, ruler_rect.bottom()),
                    ],
                    Stroke::new(1.0, Palette::BORDER_SUBTLE),
                );
            }
        }

        if x + 36.0 < viewport.right() {
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

fn track_row_rect(rect: Rect, index: usize, track_height: f32) -> Rect {
    Rect::from_min_size(
        pos2(
            rect.left(),
            rect.top() + config::TIMELINE_RULER_HEIGHT + track_height * index as f32,
        ),
        vec2(rect.width(), track_height),
    )
}

struct DemoClip {
    start: f32,
    end: f32,
    label: &'static str,
    audio: bool,
}

fn demo_clip(track_index: usize) -> Option<DemoClip> {
    match track_index {
        1 => Some(DemoClip {
            start: 2.0,
            end: 8.0,
            label: "Demo title",
            audio: false,
        }),
        2 => Some(DemoClip {
            start: 0.5,
            end: 14.0,
            label: "Demo clip \u{00b7} preview only",
            audio: false,
        }),
        3 => Some(DemoClip {
            start: 0.5,
            end: 14.0,
            label: "Demo audio \u{00b7} preview only",
            audio: true,
        }),
        _ => None,
    }
}

fn paint_demo_clips(painter: &egui::Painter, rect: Rect, scale: &TimelineScale, track_height: f32) {
    for index in 0..track_view::tracks().len() {
        let Some(clip) = demo_clip(index) else {
            continue;
        };
        let row = track_row_rect(rect, index, track_height);
        let x0 = rect.left() + scale.x_for_seconds(f64::from(clip.start));
        let x1 = rect.left() + scale.x_for_seconds(f64::from(clip.end));
        let clip_rect = Rect::from_min_max(pos2(x0, row.top() + 4.0), pos2(x1, row.bottom() - 4.0));
        if clip_rect.width() < 2.0 {
            continue;
        }

        let (fill, border, text) = if clip.audio {
            (
                egui::Color32::from_rgb(0x3a, 0x30, 0x1e),
                egui::Color32::from_rgb(0x6a, 0x54, 0x2a),
                Palette::WARNING,
            )
        } else {
            (
                egui::Color32::from_rgb(0x14, 0x3c, 0x40),
                Palette::ACCENT_DIM,
                Palette::ACCENT_LIGHT,
            )
        };
        painter.rect_filled(clip_rect, Radii::SMALL, fill);
        painter.rect_stroke(
            clip_rect,
            Radii::SMALL,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        painter.rect_filled(
            Rect::from_min_size(clip_rect.min, vec2(clip_rect.width(), 2.0)),
            CornerRadius::ZERO,
            border,
        );
        if clip_rect.width() > 42.0 {
            painter.text(
                pos2(clip_rect.left() + 7.0, clip_rect.center().y + 1.0),
                Align2::LEFT_CENTER,
                clip.label,
                FontId::proportional(if clip_rect.width() < 80.0 { 8.0 } else { 9.0 }),
                text,
            );
        }
        if clip_rect.width() > 150.0 {
            let badge = Rect::from_min_size(
                pos2(clip_rect.right() - 42.0, clip_rect.top() + 5.0),
                vec2(37.0, 13.0),
            );
            painter.rect_filled(badge, Radii::SMALL, egui::Color32::from_black_alpha(70));
            painter.text(
                badge.center(),
                Align2::CENTER_CENTER,
                "DEMO",
                FontId::proportional(7.0),
                text,
            );
        }
    }
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
        let first = track_row_rect(rect, 0, config::TIMELINE_TRACK_HEIGHT);
        let second = track_row_rect(rect, 1, config::TIMELINE_TRACK_HEIGHT);
        let last = track_row_rect(
            rect,
            track_view::tracks().len() - 1,
            config::TIMELINE_TRACK_HEIGHT,
        );

        assert_eq!(first.top(), rect.top() + config::TIMELINE_RULER_HEIGHT);
        assert_eq!(second.top(), first.bottom());
        assert_eq!(last.bottom(), rect.bottom());
    }
}

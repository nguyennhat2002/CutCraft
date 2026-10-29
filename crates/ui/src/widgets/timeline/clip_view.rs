//! Rich Timeline item rendering.
//!
//! Title bands, synthetic demo thumbnails and synthetic demo waveforms are
//! painted here. No project media is decoded: project clips without a media
//! provider fall back to an honest neutral treatment.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Stroke, StrokeKind};

use crate::theme::{Palette, Radii};
use crate::timeline_state::{ThumbnailMode, TimelineItemKind, TimelineItemView, TrackKind};
use crate::widgets::icons::{self, Icon};

use super::clip_visuals::{
    aggregate_peak, thumbnail_tiles, PeakStatus, ThumbnailStatus, TimelineVisualCache, PEAK_BUCKETS,
};
use super::scale::TimelineScale;

const TITLE_HEIGHT: f32 = 16.0;
const CLIP_RADIUS: u8 = 3;

#[derive(Debug, Clone, Copy)]
pub(super) struct ClipPaint<'a> {
    pub scale: &'a TimelineScale,
    pub item: &'a TimelineItemView,
    pub track_kind: TrackKind,
    pub demo: bool,
    pub selected: bool,
    pub hovered: bool,
    pub show_thumbnails: bool,
    pub thumbnail_mode: ThumbnailMode,
    pub show_waveforms: bool,
}

struct ClipStyle {
    base: Color32,
    border: Color32,
    title_bg: Color32,
    title_text: Color32,
    waveform: Color32,
}

fn style(kind: TimelineItemKind) -> ClipStyle {
    match kind {
        TimelineItemKind::Video => ClipStyle {
            base: Palette::ACCENT_BG,
            border: Palette::ACCENT_DIM,
            title_bg: Color32::from_rgb(0x0b, 0x2c, 0x31),
            title_text: Palette::ACCENT_LIGHT,
            waveform: Palette::ACCENT_LIGHT,
        },
        TimelineItemKind::Audio => ClipStyle {
            base: Color32::from_rgb(0x3a, 0x30, 0x1e),
            border: Color32::from_rgb(0x6a, 0x54, 0x2a),
            title_bg: Color32::from_rgb(0x46, 0x39, 0x22),
            title_text: Palette::WARNING,
            waveform: Color32::from_rgb(0xff, 0xd7, 0x86),
        },
        TimelineItemKind::Text => ClipStyle {
            base: Color32::from_rgb(0x3a, 0x25, 0x2a),
            border: Color32::from_rgb(0x83, 0x4a, 0x58),
            title_bg: Color32::from_rgb(0x4a, 0x2c, 0x33),
            title_text: Color32::from_rgb(0xf0, 0xa6, 0xb4),
            waveform: Color32::from_rgb(0xf0, 0xa6, 0xb4),
        },
        TimelineItemKind::Effect => ClipStyle {
            base: Color32::from_rgb(0x2f, 0x26, 0x42),
            border: Color32::from_rgb(0x68, 0x55, 0x95),
            title_bg: Color32::from_rgb(0x3a, 0x2f, 0x52),
            title_text: Color32::from_rgb(0xc9, 0xb0, 0xf5),
            waveform: Color32::from_rgb(0xc9, 0xb0, 0xf5),
        },
    }
}

pub(super) fn rect(row: Rect, scale: &TimelineScale, item: &TimelineItemView) -> Rect {
    let x0 = row.left() + scale.x_for_seconds(item.start_seconds());
    let x1 = row.left() + scale.x_for_seconds(item.end_seconds());
    Rect::from_min_max(pos2(x0, row.top() + 4.0), pos2(x1, row.bottom() - 4.0))
}

pub(super) fn paint(
    painter: &Painter,
    row: Rect,
    paint: ClipPaint<'_>,
    assets: &mut TimelineVisualCache,
) {
    let clip_rect = rect(row, paint.scale, paint.item);
    if clip_rect.width() < 2.0 || !painter.clip_rect().intersects(clip_rect) {
        return;
    }
    let kind = paint.item.resolved_kind(paint.track_kind);
    let style = style(kind);
    let painter = painter.with_clip_rect(clip_rect.intersect(painter.clip_rect()));
    painter.rect_filled(clip_rect, CornerRadius::same(CLIP_RADIUS), style.base);
    painter.rect_stroke(
        clip_rect,
        CornerRadius::same(CLIP_RADIUS),
        Stroke::new(1.0, style.border),
        StrokeKind::Inside,
    );

    let title_height = TITLE_HEIGHT.min(clip_rect.height());
    let title_rect = Rect::from_min_size(clip_rect.min, vec2(clip_rect.width(), title_height));
    painter.rect_filled(
        title_rect,
        CornerRadius {
            nw: CLIP_RADIUS,
            ne: CLIP_RADIUS,
            sw: 0,
            se: 0,
        },
        style.title_bg,
    );

    let body_rect = Rect::from_min_max(
        pos2(clip_rect.left(), title_rect.bottom()),
        pos2(clip_rect.right(), clip_rect.bottom()),
    );
    if body_rect.height() >= 5.0 {
        match kind {
            TimelineItemKind::Video => {
                if paint.demo && paint.show_thumbnails {
                    paint_thumbnails(&painter, body_rect, clip_rect, paint, assets);
                } else {
                    paint_missing_media(&painter, body_rect, style.border);
                }
            }
            TimelineItemKind::Audio => {
                paint_audio(
                    &painter,
                    body_rect,
                    clip_rect,
                    paint,
                    assets,
                    style.waveform,
                );
            }
            TimelineItemKind::Text => paint_text_identity(&painter, body_rect, style.title_text),
            TimelineItemKind::Effect => {
                paint_effect_identity(&painter, body_rect, style.title_text)
            }
        }
    }

    paint_title(
        &painter,
        title_rect,
        paint.item,
        style.title_text,
        paint.demo,
    );
    paint_frame(&painter, clip_rect, paint.selected, paint.hovered);
}

fn paint_title(
    painter: &Painter,
    title_rect: Rect,
    item: &TimelineItemView,
    color: Color32,
    demo: bool,
) {
    if title_rect.width() < 18.0 {
        return;
    }
    let mut label_width = title_rect.width() - 12.0;
    if demo && title_rect.width() > 96.0 {
        label_width -= 34.0;
        let badge = Rect::from_min_size(
            pos2(title_rect.right() - 38.0, title_rect.top() + 3.0),
            vec2(32.0, 10.0),
        );
        painter.rect_filled(badge, Radii::SMALL, Color32::from_black_alpha(90));
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            "DEMO",
            FontId::proportional(6.5),
            color.gamma_multiply(0.85),
        );
    }
    if title_rect.width() < 32.0 {
        return;
    }
    let text = elide(
        painter,
        item.label(),
        FontId::proportional(9.5),
        label_width,
    );
    painter.text(
        pos2(title_rect.left() + 5.0, title_rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(9.5),
        color,
    );
}

fn paint_thumbnails(
    painter: &Painter,
    body: Rect,
    clip: Rect,
    paint: ClipPaint<'_>,
    assets: &mut TimelineVisualCache,
) {
    let tiles = thumbnail_tiles(
        paint.thumbnail_mode,
        clip,
        body,
        paint.item,
        painter.clip_rect(),
        super::clip_visuals::MAX_THUMBNAIL_TILES,
    );
    let body_painter = painter.with_clip_rect(body.intersect(painter.clip_rect()));
    if tiles.is_empty() {
        paint_missing_media(
            &body_painter,
            body,
            style(paint.item.resolved_kind(paint.track_kind)).border,
        );
        return;
    }
    let source_width = (body.height() * super::clip_visuals::THUMBNAIL_ASPECT).max(1.0);
    let media = paint.item.media();
    for tile in tiles {
        let rect = tile.rect.intersect(clip);
        if rect.width() <= 0.0 {
            continue;
        }
        match assets.thumbnail(body_painter.ctx(), media, tile.source_seconds) {
            ThumbnailStatus::Ready(texture) => {
                let u1 = (rect.width() / source_width).clamp(0.0, 1.0);
                body_painter.image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(u1, 1.0)),
                    Color32::WHITE,
                );
            }
            ThumbnailStatus::Pending | ThumbnailStatus::Unavailable => {
                body_painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_black_alpha(28));
            }
        }
    }
}

fn paint_missing_media(painter: &Painter, body: Rect, border: Color32) {
    if body.width() < 12.0 {
        return;
    }
    let painter = painter.with_clip_rect(body);
    let y = body.center().y;
    painter.line_segment(
        [pos2(body.left() + 4.0, y), pos2(body.right() - 4.0, y)],
        Stroke::new(1.0, border.gamma_multiply(0.5)),
    );
    if body.width() > 110.0 && body.height() > 18.0 {
        painter.text(
            pos2(body.left() + 6.0, body.top() + 6.0),
            Align2::LEFT_TOP,
            "Preview unavailable",
            FontId::proportional(8.0),
            Palette::TEXT_DISABLED,
        );
    }
}

fn paint_audio(
    painter: &Painter,
    body: Rect,
    clip: Rect,
    paint: ClipPaint<'_>,
    assets: &mut TimelineVisualCache,
    color: Color32,
) {
    if body.width() < 4.0 || body.height() < 6.0 {
        return;
    }
    let body_painter = painter.with_clip_rect(body.intersect(painter.clip_rect()));
    body_painter.line_segment(
        [
            pos2(body.left(), body.center().y),
            pos2(body.right(), body.center().y),
        ],
        Stroke::new(1.0, color.gamma_multiply(0.35)),
    );
    if !paint.show_waveforms {
        return;
    }
    let peaks = match assets.peaks(paint.item.media()) {
        PeakStatus::Ready(peaks) => peaks,
        PeakStatus::Pending | PeakStatus::Unavailable => {
            if body.width() > 90.0 && body.height() > 16.0 {
                body_painter.text(
                    pos2(body.left() + 6.0, body.top() + 4.0),
                    Align2::LEFT_TOP,
                    "Waveform unavailable",
                    FontId::proportional(8.0),
                    Palette::TEXT_DISABLED,
                );
            }
            return;
        }
    };
    let Some(media) = paint.item.media() else {
        return;
    };
    let source_duration = media.source_duration_seconds().max(0.001);
    let clip_width = clip.width().max(1.0);
    let duration = paint.item.duration_seconds();
    let half_height = (body.height() * 0.46).max(2.0);
    let step = 1.0;
    let step_seconds = f64::from(step) / f64::from(clip_width) * duration;
    let mut mesh = egui::Mesh::default();
    let mut x = body.left().max(body_painter.clip_rect().left());
    let right = body.right().min(body_painter.clip_rect().right());
    while x < right {
        let t0 = paint.item.start_seconds() + f64::from((x - clip.left()) / clip_width) * duration;
        let t1 = t0 + step_seconds;
        let source0 = paint.item.source_time_at(t0).unwrap_or(0.0);
        let source1 = paint.item.source_time_at(t1).unwrap_or(source0);
        let idx0 = source0 / source_duration * PEAK_BUCKETS as f64;
        let idx1 = (source1 / source_duration * PEAK_BUCKETS as f64).max(idx0 + 0.01);
        let peak = aggregate_peak(&peaks, idx0, idx1);
        if peak.max - peak.min > 0.005 {
            let top = body.center().y - peak.max.clamp(0.0, 1.0) * half_height;
            let bottom = body.center().y - peak.min.clamp(-1.0, 0.0) * half_height;
            let rect = Rect::from_min_max(pos2(x, top), pos2((x + step).min(right), bottom));
            mesh.add_colored_rect(rect, color);
        }
        x += step;
    }
    body_painter.add(mesh);
}

fn paint_text_identity(painter: &Painter, body: Rect, color: Color32) {
    if body.width() < 24.0 || body.height() < 14.0 {
        return;
    }
    let painter = painter.with_clip_rect(body);
    let center = body.center();
    painter.text(
        center,
        Align2::CENTER_CENTER,
        "Aa",
        FontId::proportional((body.height() * 0.42).clamp(9.0, 17.0)),
        color.gamma_multiply(0.75),
    );
    if body.width() > 80.0 {
        painter.text(
            pos2(center.x, center.y + body.height() * 0.28),
            Align2::CENTER_CENTER,
            "text",
            FontId::proportional(7.0),
            color.gamma_multiply(0.45),
        );
    }
}

fn paint_effect_identity(painter: &Painter, body: Rect, color: Color32) {
    if body.width() < 22.0 || body.height() < 14.0 {
        return;
    }
    let size = body.height().min(body.width()).clamp(10.0, 18.0);
    icons::paint_sized(
        painter,
        Rect::from_center_size(body.center(), vec2(size, size)),
        Icon::Effects,
        crate::widgets::icons::IconSize::Standard,
        crate::widgets::icons::IconColors::SELECTED,
    );
    if body.width() > 58.0 && body.height() > 22.0 {
        painter.text(
            pos2(body.right() - 7.0, body.bottom() - 7.0),
            Align2::RIGHT_BOTTOM,
            "fx",
            FontId::proportional(7.5),
            color.gamma_multiply(0.75),
        );
    }
}

fn paint_frame(painter: &Painter, clip: Rect, selected: bool, hovered: bool) {
    if selected {
        painter.rect_stroke(
            clip.shrink(0.5),
            CornerRadius::same(CLIP_RADIUS),
            Stroke::new(1.0, Color32::BLACK),
            StrokeKind::Inside,
        );
        painter.rect_stroke(
            clip.shrink(1.0),
            CornerRadius::same(CLIP_RADIUS),
            Stroke::new(2.0, Palette::TEXT_PRIMARY),
            StrokeKind::Inside,
        );
    } else if hovered {
        painter.rect_stroke(
            clip,
            CornerRadius::same(CLIP_RADIUS),
            Stroke::new(1.0, Palette::ACCENT_LIGHT),
            StrokeKind::Inside,
        );
    }
}

fn elide(painter: &Painter, text: &str, font: FontId, max_width: f32) -> String {
    if max_width <= 0.0 {
        return String::new();
    }
    if painter
        .layout_no_wrap(text.to_owned(), font.clone(), Palette::TEXT_PRIMARY)
        .size()
        .x
        <= max_width
    {
        return text.to_owned();
    }
    let ellipsis = "…";
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate = format!("{}{}", chars.iter().collect::<String>(), ellipsis);
        if painter
            .layout_no_wrap(candidate.clone(), font.clone(), Palette::TEXT_PRIMARY)
            .size()
            .x
            <= max_width
        {
            return candidate;
        }
    }
    ellipsis.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elision_keeps_labels_within_the_title_band() {
        let context = egui::Context::default();
        crate::theme::configure(&context);
        let output = context.run_ui(Default::default(), |ui| {
            let text = elide(
                ui.painter(),
                "A very long demo clip title that cannot fit",
                FontId::proportional(9.5),
                54.0,
            );
            assert!(text.ends_with('…'));
            let width = ui
                .painter()
                .layout_no_wrap(text, FontId::proportional(9.5), Palette::TEXT_PRIMARY)
                .size()
                .x;
            assert!(width <= 54.5);
        });
        output.drop_without_applying_deltas();
    }
}

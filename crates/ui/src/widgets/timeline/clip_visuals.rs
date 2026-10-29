//! Presentation-side thumbnail and waveform assets.
//!
//! There is no media decoder in CutCraft yet. This module defines the request
//! seam, cache keys and bounded caches used by the Timeline; deterministic
//! images/peaks are generated only in explicit Visual Demo mode. Project clips
//! return `Unavailable` rather than pretending thumbnails were decoded.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{pos2, vec2, Color32, ColorImage, Context, Rect, TextureHandle, TextureOptions};

use crate::timeline_state::{ThumbnailMode, TimelineItemView, TimelineMediaRef};

pub const THUMBNAIL_ASPECT: f32 = 16.0 / 9.0;
pub const THUMBNAIL_TEX_WIDTH: u16 = 128;
pub const THUMBNAIL_TEX_HEIGHT: u16 = 72;
pub const MAX_THUMBNAIL_TILES: usize = 24;
pub const MAX_TEXTURES: usize = 96;
pub const MAX_REQUESTS_PER_FRAME: usize = 48;
pub const MAX_PEAK_ENTRIES: usize = 24;
pub const PEAK_BUCKETS: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThumbnailKey {
    media_id: u64,
    revision: u64,
    source_millis: u64,
    width: u16,
    height: u16,
}

impl ThumbnailKey {
    pub fn new(
        media: &TimelineMediaRef,
        source_seconds: f64,
        width: u16,
        height: u16,
    ) -> Option<Self> {
        if !source_seconds.is_finite() || width == 0 || height == 0 {
            return None;
        }
        let millis = (source_seconds.max(0.0) * 1000.0).round();
        if millis > u64::MAX as f64 {
            return None;
        }
        Some(Self {
            media_id: hash_text(media.id()),
            revision: media.revision(),
            source_millis: millis as u64,
            width,
            height,
        })
    }

    pub fn source_seconds(self) -> f64 {
        self.source_millis as f64 / 1000.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioPeakKey {
    media_id: u64,
    revision: u64,
    stream: u32,
}

impl AudioPeakKey {
    pub fn new(media: &TimelineMediaRef, stream: u32) -> Self {
        Self {
            media_id: hash_text(media.id()),
            revision: media.revision(),
            stream,
        }
    }
}

#[derive(Clone)]
pub enum ThumbnailStatus {
    Ready(TextureHandle),
    Pending,
    Unavailable,
}

#[derive(Clone)]
pub enum PeakStatus {
    Ready(Arc<Vec<AudioPeak>>),
    Pending,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioPeak {
    pub min: f32,
    pub max: f32,
}

impl AudioPeak {
    pub const SILENT: Self = Self { min: 0.0, max: 0.0 };

    pub fn amplitude(self) -> f32 {
        self.min.abs().max(self.max.abs()).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbnailTile {
    pub rect: Rect,
    pub source_seconds: f64,
}

/// Visible thumbnail tiles for a clip. Tiles are anchored to the clip, so
/// scrolling reveals stable requests instead of re-timing every frame.
pub fn thumbnail_tiles(
    mode: ThumbnailMode,
    clip_rect: Rect,
    body_rect: Rect,
    item: &TimelineItemView,
    visible_rect: Rect,
    max_tiles: usize,
) -> Vec<ThumbnailTile> {
    let width = clip_rect.width();
    let height = body_rect.height();
    if !width.is_finite()
        || !height.is_finite()
        || width < 12.0
        || height < 12.0
        || !clip_rect.is_positive()
        || max_tiles == 0
    {
        return Vec::new();
    }
    let tile_width = (height * THUMBNAIL_ASPECT).max(1.0);
    let last_offset = (width - tile_width).max(0.0);
    let continuous_source_at = |x: f32| {
        let fraction =
            f64::from(((x + tile_width * 0.5 - clip_rect.left()) / width).clamp(0.0, 1.0));
        item.source_time_at(item.start_seconds() + fraction * item.duration_seconds())
    };
    let visible = visible_rect.intersect(body_rect);
    if visible.width() <= 0.0 {
        return Vec::new();
    }
    let mut tiles = Vec::new();
    let push = |tiles: &mut Vec<ThumbnailTile>, x: f32, source: Option<f64>| {
        if tiles.len() >= max_tiles {
            return;
        }
        let rect = Rect::from_min_size(
            pos2(x, body_rect.top()),
            vec2(tile_width.min(clip_rect.right() - x).max(0.0), height),
        );
        if rect.width() <= 0.0 || !rect.intersects(visible) {
            return;
        }
        if let Some(source_seconds) = source {
            tiles.push(ThumbnailTile {
                rect,
                source_seconds,
            });
        }
    };

    match mode {
        ThumbnailMode::First => push(
            &mut tiles,
            clip_rect.left(),
            item.source_time_at(item.start_seconds()),
        ),
        ThumbnailMode::FirstAndLast => {
            push(
                &mut tiles,
                clip_rect.left(),
                item.source_time_at(item.start_seconds()),
            );
            if width >= tile_width * 2.0 {
                push(
                    &mut tiles,
                    clip_rect.left() + last_offset,
                    item.source_time_at(item.end_seconds()),
                );
            }
        }
        ThumbnailMode::Continuous => {
            let first =
                ((visible.left() - clip_rect.left()).max(0.0) / tile_width).floor() as usize;
            let mut index = first;
            loop {
                let x = clip_rect.left() + index as f32 * tile_width;
                if x >= clip_rect.right() || tiles.len() >= max_tiles {
                    break;
                }
                if x + tile_width >= visible.left() {
                    push(&mut tiles, x, continuous_source_at(x));
                }
                index += 1;
            }
        }
    }
    tiles
}

/// Aggregate min/max peaks in a fractional index range for one screen column.
pub fn aggregate_peak(peaks: &[AudioPeak], start: f64, end: f64) -> AudioPeak {
    if peaks.is_empty() || !start.is_finite() || !end.is_finite() || end <= start {
        return AudioPeak::SILENT;
    }
    let first = start.floor().max(0.0) as usize;
    if first >= peaks.len() {
        return AudioPeak::SILENT;
    }
    let last = (end.ceil() as usize).max(first + 1).min(peaks.len());
    peaks[first..last]
        .iter()
        .fold(AudioPeak::SILENT, |acc, peak| AudioPeak {
            min: acc.min.min(peak.min),
            max: acc.max.max(peak.max),
        })
}

#[derive(Default)]
pub struct TimelineVisualCache {
    textures: HashMap<ThumbnailKey, TextureHandle>,
    texture_order: VecDeque<ThumbnailKey>,
    peaks: HashMap<AudioPeakKey, Arc<Vec<AudioPeak>>>,
    peak_order: VecDeque<AudioPeakKey>,
    requests_this_frame: usize,
    texture_hits: usize,
    texture_misses: usize,
    evictions: usize,
    demo: bool,
}

impl TimelineVisualCache {
    /// Select synthetic assets only for the explicitly labeled visual demo.
    pub fn set_demo(&mut self, demo: bool) {
        self.demo = demo;
    }

    pub fn begin_frame(&mut self) {
        self.requests_this_frame = 0;
    }

    pub fn texture_hits(&self) -> usize {
        self.texture_hits
    }

    pub fn texture_misses(&self) -> usize {
        self.texture_misses
    }

    pub fn evictions(&self) -> usize {
        self.evictions
    }

    pub fn thumbnail(
        &mut self,
        context: &Context,
        media: Option<&TimelineMediaRef>,
        source_seconds: f64,
    ) -> ThumbnailStatus {
        let Some(media) = media else {
            return ThumbnailStatus::Unavailable;
        };
        let Some(key) = ThumbnailKey::new(
            media,
            source_seconds,
            THUMBNAIL_TEX_WIDTH,
            THUMBNAIL_TEX_HEIGHT,
        ) else {
            return ThumbnailStatus::Unavailable;
        };
        if !self.demo {
            return ThumbnailStatus::Unavailable;
        }
        if let Some(texture) = self.textures.get(&key) {
            self.texture_hits += 1;
            return ThumbnailStatus::Ready(texture.clone());
        }
        if self.requests_this_frame >= MAX_REQUESTS_PER_FRAME {
            context.request_repaint();
            return ThumbnailStatus::Pending;
        }
        self.requests_this_frame += 1;
        self.texture_misses += 1;
        let image = synthetic_thumbnail(key);
        let texture = context.load_texture(
            format!("cutcraft-demo-thumb-{key:?}"),
            image,
            TextureOptions::LINEAR,
        );
        self.textures.insert(key, texture.clone());
        self.texture_order.push_back(key);
        while self.textures.len() > MAX_TEXTURES {
            if let Some(oldest) = self.texture_order.pop_front() {
                if self.textures.remove(&oldest).is_some() {
                    self.evictions += 1;
                }
            } else {
                break;
            }
        }
        ThumbnailStatus::Ready(texture)
    }

    pub fn peaks(&mut self, media: Option<&TimelineMediaRef>) -> PeakStatus {
        let Some(media) = media else {
            return PeakStatus::Unavailable;
        };
        if !self.demo {
            return PeakStatus::Unavailable;
        }
        let key = AudioPeakKey::new(media, 0);
        if let Some(peaks) = self.peaks.get(&key) {
            return PeakStatus::Ready(peaks.clone());
        }
        let peaks = Arc::new(synthetic_peaks(media));
        self.peaks.insert(key, peaks.clone());
        self.peak_order.push_back(key);
        while self.peaks.len() > MAX_PEAK_ENTRIES {
            if let Some(oldest) = self.peak_order.pop_front() {
                self.peaks.remove(&oldest);
            } else {
                break;
            }
        }
        PeakStatus::Ready(peaks)
    }
}

fn hash_text(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn seed_of(key: ThumbnailKey) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Deterministic, clearly synthetic landscape tile. Only used in Visual Demo.
fn synthetic_thumbnail(key: ThumbnailKey) -> ColorImage {
    let width = key.width as usize;
    let height = key.height as usize;
    let seed = seed_of(key);
    let time_phase = (key.source_seconds() * 0.13).sin() as f32;
    let hue_shift = ((seed >> 8) & 0xff) as f32 / 255.0;
    let sun_x = 0.22 + ((seed >> 16) & 0xff) as f32 / 255.0 * 0.56;
    let sun_y = 0.24 + ((seed >> 24) & 0xff) as f32 / 255.0 * 0.12;
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        let fy = y as f32 / height.max(1) as f32;
        for x in 0..width {
            let fx = x as f32 / width.max(1) as f32;
            let sky = 1.0 - fy;
            let base = Color32::from_rgb(
                (42.0 + 58.0 * hue_shift + 78.0 * sky + 12.0 * time_phase).clamp(0.0, 255.0) as u8,
                (82.0 + 18.0 * hue_shift + 74.0 * sky).clamp(0.0, 255.0) as u8,
                (108.0 + 24.0 * hue_shift + 88.0 * sky).clamp(0.0, 255.0) as u8,
            );
            let sun = ((fx - sun_x).powi(2) + (fy - sun_y).powi(2)).sqrt() < 0.075;
            let ridge =
                0.64 + (fx * 8.0 + hue_shift * 6.0).sin() * 0.05 + (fx * 17.0 + 1.8).cos() * 0.03;
            let ground = fy > ridge;
            let color = if sun {
                Color32::from_rgb(0xf3, 0xd8, 0x9a)
            } else if ground {
                Color32::from_rgb(
                    (20.0 + 16.0 * hue_shift) as u8,
                    (32.0 + 12.0 * hue_shift) as u8,
                    (35.0 + 22.0 * hue_shift) as u8,
                )
            } else {
                base
            };
            pixels.push(color);
        }
    }
    ColorImage::new([width, height], pixels)
}

/// Deterministic synthetic peaks for QA. Never presented as decoded media.
fn synthetic_peaks(media: &TimelineMediaRef) -> Vec<AudioPeak> {
    let seed = hash_text(media.id());
    let seconds = media.source_duration_seconds().max(0.001);
    (0..PEAK_BUCKETS)
        .map(|index| {
            let t = index as f64 / PEAK_BUCKETS as f64 * seconds;
            let quiet = ((t * 0.31).sin() * 0.5 + 0.5) < 0.14;
            if quiet {
                return AudioPeak::SILENT;
            }
            let envelope = (0.52
                + 0.28 * (t * 0.45 + seed as f64 * 0.0001).sin().abs()
                + 0.14 * (t * 1.7).cos().abs())
            .clamp(0.0, 1.0);
            let wobble = (t * 7.3 + index as f64 * 0.17).sin() * 0.09;
            let grain = (((seed.rotate_left((index % 31) as u32) ^ index as u64) % 1000) as f64
                / 1000.0
                - 0.5)
                * 0.22;
            let amp = (envelope + wobble + grain).clamp(0.12, 1.0) as f32;
            AudioPeak {
                min: -amp * (0.62 + 0.38 * (index as f32 * 0.21).cos().abs()),
                max: amp * (0.68 + 0.32 * (index as f32 * 0.13).sin().abs()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline_state::TimelineItemView;

    fn item() -> TimelineItemView {
        let media = TimelineMediaRef::new("media", 1, 30.0).expect("media");
        TimelineItemView::new("Clip", 0.0, 4.0)
            .expect("item")
            .with_source(2.0, 1.5)
            .expect("source")
            .with_media(media)
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    #[test]
    fn cache_keys_distinguish_media_time_size_and_revision() {
        let media = TimelineMediaRef::new("a", 1, 30.0).expect("media");
        let base = ThumbnailKey::new(&media, 1.5, 128, 72).expect("key");
        let other_time = ThumbnailKey::new(&media, 1.6, 128, 72).expect("key");
        let other_size = ThumbnailKey::new(&media, 1.5, 64, 36).expect("key");
        let other_revision = ThumbnailKey::new(
            &TimelineMediaRef::new("a", 2, 30.0).expect("media"),
            1.5,
            128,
            72,
        )
        .expect("key");
        let other_media = ThumbnailKey::new(
            &TimelineMediaRef::new("b", 1, 30.0).expect("media"),
            1.5,
            128,
            72,
        )
        .expect("key");
        assert_ne!(base, other_time);
        assert_ne!(base, other_size);
        assert_ne!(base, other_revision);
        assert_ne!(base, other_media);
    }

    #[test]
    fn continuous_tiles_are_visible_anchored_and_bounded() {
        let clip = rect(100.0, 0.0, 420.0, 60.0);
        let body = rect(100.0, 20.0, 420.0, 40.0);
        let visible = rect(180.0, 20.0, 220.0, 40.0);
        let tiles = thumbnail_tiles(
            ThumbnailMode::Continuous,
            clip,
            body,
            &item(),
            visible,
            MAX_THUMBNAIL_TILES,
        );
        assert!(!tiles.is_empty());
        assert!(tiles.len() <= MAX_THUMBNAIL_TILES);
        assert!(tiles.iter().all(|tile| tile.rect.intersects(visible)));
        assert!(tiles
            .windows(2)
            .all(|pair| pair[0].rect.left() < pair[1].rect.left()));
        let first_source = tiles[0].source_seconds;
        let scrolled = thumbnail_tiles(
            ThumbnailMode::Continuous,
            clip,
            body,
            &item(),
            rect(300.0, 20.0, 200.0, 40.0),
            MAX_THUMBNAIL_TILES,
        );
        assert!(scrolled
            .iter()
            .all(|tile| (tile.source_seconds - first_source).abs() >= 0.0));
    }

    #[test]
    fn first_and_last_modes_respect_width_and_tile_limit() {
        let clip = rect(0.0, 0.0, 400.0, 60.0);
        let body = rect(0.0, 20.0, 400.0, 40.0);
        let first = thumbnail_tiles(
            ThumbnailMode::First,
            clip,
            body,
            &item(),
            body,
            MAX_THUMBNAIL_TILES,
        );
        assert_eq!(first.len(), 1);

        let first_last = thumbnail_tiles(
            ThumbnailMode::FirstAndLast,
            clip,
            body,
            &item(),
            body,
            MAX_THUMBNAIL_TILES,
        );
        assert_eq!(first_last.len(), 2);
        assert!(first_last[1].rect.left() > first_last[0].rect.left());
        assert!((first_last[0].source_seconds - 2.0).abs() < f64::EPSILON);
        assert!((first_last[1].source_seconds - 8.0).abs() < f64::EPSILON);

        let capped = thumbnail_tiles(
            ThumbnailMode::Continuous,
            rect(0.0, 0.0, 4000.0, 60.0),
            rect(0.0, 20.0, 4000.0, 40.0),
            &item(),
            rect(0.0, 20.0, 4000.0, 40.0),
            3,
        );
        assert_eq!(capped.len(), 3);
    }

    #[test]
    fn zoom_levels_change_tile_count_without_losing_visible_culling() {
        let item = item();
        let narrow = thumbnail_tiles(
            ThumbnailMode::Continuous,
            rect(0.0, 0.0, 180.0, 60.0),
            rect(0.0, 20.0, 180.0, 40.0),
            &item,
            rect(0.0, 20.0, 180.0, 40.0),
            MAX_THUMBNAIL_TILES,
        );
        let wide = thumbnail_tiles(
            ThumbnailMode::Continuous,
            rect(0.0, 0.0, 720.0, 60.0),
            rect(0.0, 20.0, 720.0, 40.0),
            &item,
            rect(0.0, 20.0, 720.0, 40.0),
            MAX_THUMBNAIL_TILES,
        );
        assert!(wide.len() > narrow.len());
        assert!(wide.iter().all(|tile| tile.source_seconds >= 2.0));
        assert!(wide
            .iter()
            .all(|tile| tile.source_seconds <= 8.0 + f64::EPSILON));
        let scrolled = thumbnail_tiles(
            ThumbnailMode::Continuous,
            rect(0.0, 0.0, 720.0, 60.0),
            rect(0.0, 20.0, 720.0, 40.0),
            &item,
            rect(500.0, 20.0, 100.0, 40.0),
            MAX_THUMBNAIL_TILES,
        );
        assert!(scrolled.len() < wide.len());
        assert!(scrolled.iter().all(|tile| tile.rect.right() > 500.0));
    }

    #[test]
    fn narrow_or_short_clips_do_not_render_tiles() {
        let narrow = rect(0.0, 0.0, 8.0, 60.0);
        assert!(thumbnail_tiles(
            ThumbnailMode::Continuous,
            narrow,
            rect(0.0, 20.0, 8.0, 40.0),
            &item(),
            narrow,
            MAX_THUMBNAIL_TILES,
        )
        .is_empty());
        let short = rect(0.0, 0.0, 100.0, 25.0);
        assert!(thumbnail_tiles(
            ThumbnailMode::Continuous,
            short,
            rect(0.0, 20.0, 100.0, 5.0),
            &item(),
            short,
            MAX_THUMBNAIL_TILES,
        )
        .is_empty());
    }

    #[test]
    fn project_mode_never_generates_synthetic_assets() {
        let context = Context::default();
        let media = TimelineMediaRef::new("project", 1, 10.0).expect("media");
        let mut cache = TimelineVisualCache::default();
        assert!(matches!(
            cache.thumbnail(&context, Some(&media), 1.0),
            ThumbnailStatus::Unavailable
        ));
        assert!(matches!(cache.peaks(Some(&media)), PeakStatus::Unavailable));
        assert_eq!(cache.texture_misses(), 0);
    }

    #[test]
    fn demo_mode_reuses_textures_and_peaks_within_bounds() {
        let context = Context::default();
        let media = TimelineMediaRef::new("demo", 1, 10.0).expect("media");
        let mut cache = TimelineVisualCache::default();
        cache.set_demo(true);
        cache.begin_frame();
        let first = cache.thumbnail(&context, Some(&media), 1.0);
        assert!(matches!(first, ThumbnailStatus::Ready(_)));
        let second = cache.thumbnail(&context, Some(&media), 1.0);
        assert!(matches!(second, ThumbnailStatus::Ready(_)));
        assert_eq!(cache.texture_misses(), 1);
        assert_eq!(cache.texture_hits(), 1);
        let peaks = cache.peaks(Some(&media));
        assert!(matches!(peaks, PeakStatus::Ready(_)));
        assert!(matches!(cache.peaks(Some(&media)), PeakStatus::Ready(_)));
    }

    #[test]
    fn demo_assets_are_not_reused_in_project_mode() {
        let context = Context::default();
        let media = TimelineMediaRef::new("same-id", 1, 10.0).expect("media");
        let mut cache = TimelineVisualCache::default();
        cache.set_demo(true);
        assert!(matches!(
            cache.thumbnail(&context, Some(&media), 1.0),
            ThumbnailStatus::Ready(_)
        ));
        assert!(matches!(cache.peaks(Some(&media)), PeakStatus::Ready(_)));
        cache.set_demo(false);
        assert!(matches!(
            cache.thumbnail(&context, Some(&media), 1.0),
            ThumbnailStatus::Unavailable
        ));
        assert!(matches!(cache.peaks(Some(&media)), PeakStatus::Unavailable));
    }

    #[test]
    fn per_frame_request_budget_is_bounded() {
        let context = Context::default();
        let mut cache = TimelineVisualCache::default();
        cache.set_demo(true);
        cache.begin_frame();
        for index in 0..(MAX_REQUESTS_PER_FRAME + 8) {
            let media = TimelineMediaRef::new(format!("demo-{index}"), 1, 10.0).expect("media");
            let _ = cache.thumbnail(&context, Some(&media), index as f64);
        }
        assert!(cache.texture_misses() <= MAX_REQUESTS_PER_FRAME);
    }

    #[test]
    fn peak_aggregation_handles_silence_empty_and_downsampling() {
        assert_eq!(aggregate_peak(&[], 0.0, 1.0), AudioPeak::SILENT);
        let peaks = vec![
            AudioPeak {
                min: -0.1,
                max: 0.2,
            },
            AudioPeak {
                min: -0.8,
                max: 0.3,
            },
            AudioPeak { min: 0.0, max: 0.0 },
        ];
        let aggregate = aggregate_peak(&peaks, 0.0, 3.0);
        assert_eq!(aggregate.min, -0.8);
        assert_eq!(aggregate.max, 0.3);
        assert_eq!(aggregate_peak(&peaks, 2.0, 2.0), AudioPeak::SILENT);
    }
}

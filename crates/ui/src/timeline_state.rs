//! Read-only timeline presentation data, independent of egui.
//!
//! This is a projection seam for future engine snapshots, NOT an authoritative
//! project/time model. Normal sessions start with no rows or items. Widgets never
//! insert or delete content; import and edit commands remain unimplemented.

use std::collections::BTreeSet;
use std::sync::LazyLock;

/// Derived from supplied content, never from a separate "is empty" flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineContentState {
    PristineEmpty,
    EmptyTracks,
    Populated,
}

/// An explicit, presentation-only source selector. New sessions use `Project`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TimelineDisplayMode {
    #[default]
    Project,
    VisualDemo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Audio,
}

/// Presentation category for an item. Text and effect clips can live on a video
/// track without pretending to have video frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineItemKind {
    Video,
    Audio,
    Text,
    Effect,
}

/// Timeline thumbnail display preference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThumbnailMode {
    First,
    FirstAndLast,
    #[default]
    Continuous,
}

impl ThumbnailMode {
    pub const ALL: [Self; 3] = [Self::First, Self::FirstAndLast, Self::Continuous];

    pub const fn label(self) -> &'static str {
        match self {
            Self::First => "First Frame",
            Self::FirstAndLast => "First and Last",
            Self::Continuous => "Continuous",
        }
    }
}

/// Presentation-only media identity for cache keys and time mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineMediaRef {
    id: String,
    revision: u64,
    source_duration_seconds_millis: u64,
}

impl TimelineMediaRef {
    /// Media identity must be non-empty and carry a finite positive duration.
    pub fn new(id: impl Into<String>, revision: u64, source_duration_seconds: f64) -> Option<Self> {
        let id = id.into();
        if id.trim().is_empty()
            || !source_duration_seconds.is_finite()
            || source_duration_seconds <= 0.0
        {
            return None;
        }
        let millis = (source_duration_seconds * 1000.0).round();
        if !(1.0..=u64::MAX as f64).contains(&millis) {
            return None;
        }
        Some(Self {
            id,
            revision,
            source_duration_seconds_millis: millis as u64,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn source_duration_seconds(&self) -> f64 {
        self.source_duration_seconds_millis as f64 / 1000.0
    }
}

/// Display geometry and label for an item. Seconds are visualization only;
/// authoritative timing and editing will belong to core/engine.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItemView {
    label: String,
    start_seconds: f64,
    end_seconds: f64,
    kind: Option<TimelineItemKind>,
    source_in_seconds: f64,
    speed: f64,
    media: Option<TimelineMediaRef>,
}

impl TimelineItemView {
    /// Own the label and validate a finite, non-negative, non-empty display range.
    /// Returns `None` for invalid geometry; no media is opened or synthesized.
    pub fn new(label: impl Into<String>, start_seconds: f64, end_seconds: f64) -> Option<Self> {
        if !start_seconds.is_finite()
            || !end_seconds.is_finite()
            || start_seconds < 0.0
            || end_seconds <= start_seconds
        {
            return None;
        }
        Some(Self {
            label: label.into(),
            start_seconds,
            end_seconds,
            kind: None,
            source_in_seconds: 0.0,
            speed: 1.0,
            media: None,
        })
    }

    pub fn with_kind(mut self, kind: TimelineItemKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Configure a presentation-level source mapping. Invalid source-in or
    /// speed leaves the item untouched and returns `None`.
    pub fn with_source(mut self, source_in_seconds: f64, speed: f64) -> Option<Self> {
        if !source_in_seconds.is_finite() || source_in_seconds < 0.0 {
            return None;
        }
        if !speed.is_finite() || speed <= 0.0 {
            return None;
        }
        self.source_in_seconds = source_in_seconds;
        self.speed = speed;
        Some(self)
    }

    pub fn with_media(mut self, media: TimelineMediaRef) -> Self {
        self.media = Some(media);
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn start_seconds(&self) -> f64 {
        self.start_seconds
    }

    pub fn end_seconds(&self) -> f64 {
        self.end_seconds
    }

    pub fn duration_seconds(&self) -> f64 {
        self.end_seconds - self.start_seconds
    }

    pub fn source_in_seconds(&self) -> f64 {
        self.source_in_seconds
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn media(&self) -> Option<&TimelineMediaRef> {
        self.media.as_ref()
    }

    pub fn resolved_kind(&self, track_kind: TrackKind) -> TimelineItemKind {
        self.kind.unwrap_or(match track_kind {
            TrackKind::Video => TimelineItemKind::Video,
            TrackKind::Audio => TimelineItemKind::Audio,
        })
    }

    /// Map a display position to source media time. This is a presentation
    /// projection; authoritative source PTS mapping remains an engine concern.
    pub fn source_time_at(&self, timeline_seconds: f64) -> Option<f64> {
        if !timeline_seconds.is_finite()
            || timeline_seconds < self.start_seconds
            || timeline_seconds > self.end_seconds
        {
            return None;
        }
        let offset = (timeline_seconds - self.start_seconds).clamp(0.0, self.duration_seconds());
        let source = self.source_in_seconds + offset * self.speed;
        Some(match &self.media {
            Some(media) => source.clamp(0.0, media.source_duration_seconds()),
            None => source,
        })
    }
}

/// Stable-within-the-current-view identity for a timeline item.
///
/// This is a presentation projection key for selection. A future engine adapter
/// should replace it with the engine's persistent clip identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimelineItemId {
    pub track: usize,
    pub item: usize,
}

/// Media makeup of the valid selected timeline items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineSelectionKind {
    VideoOnly,
    AudioOnly,
    Mixed,
}

/// Resolved selection counts after dropping stale ids that no longer exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimelineSelectionContext {
    pub selected: usize,
    pub video: usize,
    pub audio: usize,
}

impl TimelineSelectionContext {
    pub fn classify(view: &TimelineView, selection: &BTreeSet<TimelineItemId>) -> Self {
        let mut context = Self::default();
        for id in selection {
            let Some(track) = view.tracks.get(id.track) else {
                continue;
            };
            if track.items.get(id.item).is_none() {
                continue;
            }
            context.selected += 1;
            match track.kind {
                TrackKind::Video => context.video += 1,
                TrackKind::Audio => context.audio += 1,
            }
        }
        context
    }

    pub fn kind(self) -> Option<TimelineSelectionKind> {
        match (self.video, self.audio) {
            (0, 0) => None,
            (_, 0) => Some(TimelineSelectionKind::VideoOnly),
            (0, _) => Some(TimelineSelectionKind::AudioOnly),
            _ => Some(TimelineSelectionKind::Mixed),
        }
    }

    pub fn is_empty(self) -> bool {
        self.selected == 0
    }
}

/// One supplied row. Empty item lists do not remove intentionally created tracks.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTrackView {
    pub label: String,
    pub name: String,
    pub kind: TrackKind,
    pub items: Vec<TimelineItemView>,
}

/// Owned view data retained by the temporary session across Home/Resume.
/// A future engine adapter may replace this projection after successful commands.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TimelineView {
    pub tracks: Vec<TimelineTrackView>,
}

impl TimelineView {
    pub fn content_state(&self) -> TimelineContentState {
        if self.tracks.is_empty() {
            TimelineContentState::PristineEmpty
        } else if self.tracks.iter().all(|track| track.items.is_empty()) {
            TimelineContentState::EmptyTracks
        } else {
            TimelineContentState::Populated
        }
    }

    /// Furthest supplied item boundary, for scroll extent only, not project duration.
    pub fn display_end_seconds(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|track| &track.items)
            .map(TimelineItemView::end_seconds)
            .fold(0.0, f64::max)
    }
}

/// The previous synthetic layout, available only through explicit visual QA mode.
/// Borrowed separately: it can never be appended to or replace session content.
pub(crate) fn visual_demo() -> &'static TimelineView {
    static DEMO: LazyLock<TimelineView> = LazyLock::new(|| {
        let mut tracks = Vec::new();
        for (index, label, name, kind) in [
            (0, "V3", "Video 3", TrackKind::Video),
            (1, "V2", "Video 2", TrackKind::Video),
            (2, "V1", "Video 1", TrackKind::Video),
            (3, "A1", "Audio 1", TrackKind::Audio),
            (4, "A2", "Audio 2", TrackKind::Audio),
            (5, "A3", "Audio 3", TrackKind::Audio),
        ] {
            let items = match index {
                0 => vec![demo_effect("Demo glow", 15.0, 18.0)],
                1 => vec![
                    demo_text("Demo title", 2.0, 5.0),
                    demo_text("Demo lower third", 5.0, 7.0),
                ],
                2 => vec![
                    demo_video("Demo film A", 0.5, 6.5, "demo-video-a", 12.0),
                    demo_video("Demo film B", 6.5, 14.0, "demo-video-b", 4.0),
                ],
                3 => vec![demo_audio(
                    "Demo music · preview only",
                    0.5,
                    14.0,
                    "demo-audio-music",
                    8.0,
                )],
                4 => vec![demo_audio(
                    "Demo voice · preview only",
                    14.0,
                    18.0,
                    "demo-audio-voice",
                    2.0,
                )],
                _ => Vec::new(),
            };
            tracks.push(TimelineTrackView {
                label: label.to_owned(),
                name: name.to_owned(),
                kind,
                items: items.into_iter().flatten().collect(),
            });
        }
        TimelineView { tracks }
    });
    &DEMO
}

fn demo_source(
    label: &str,
    start: f64,
    end: f64,
    kind: TimelineItemKind,
    media_id: &str,
    source_in: f64,
) -> Option<TimelineItemView> {
    let media = TimelineMediaRef::new(media_id, 1, 60.0)?;
    TimelineItemView::new(label, start, end)?
        .with_kind(kind)
        .with_source(source_in, 1.0)
        .map(|item| item.with_media(media))
}

fn demo_video(
    label: &str,
    start: f64,
    end: f64,
    media_id: &str,
    source_in: f64,
) -> Option<TimelineItemView> {
    demo_source(
        label,
        start,
        end,
        TimelineItemKind::Video,
        media_id,
        source_in,
    )
}

fn demo_audio(
    label: &str,
    start: f64,
    end: f64,
    media_id: &str,
    source_in: f64,
) -> Option<TimelineItemView> {
    demo_source(
        label,
        start,
        end,
        TimelineItemKind::Audio,
        media_id,
        source_in,
    )
}

fn demo_text(label: &str, start: f64, end: f64) -> Option<TimelineItemView> {
    TimelineItemView::new(label, start, end).map(|item| item.with_kind(TimelineItemKind::Text))
}

fn demo_effect(label: &str, start: f64, end: f64) -> Option<TimelineItemView> {
    TimelineItemView::new(label, start, end).map(|item| item.with_kind(TimelineItemKind::Effect))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn empty_track() -> TimelineTrackView {
        TimelineTrackView {
            label: "V1".to_owned(),
            name: "User track".to_owned(),
            kind: TrackKind::Video,
            items: Vec::new(),
        }
    }

    #[test]
    fn empty_view_has_no_synthetic_tracks() {
        let view = TimelineView::default();
        assert!(view.tracks.is_empty());
        assert_eq!(view.content_state(), TimelineContentState::PristineEmpty);
        assert_eq!(view.display_end_seconds(), 0.0);
    }

    #[test]
    fn supplied_empty_track_exits_pristine_without_inventing_items() {
        let mut view = TimelineView::default();
        view.tracks.push(empty_track());
        assert_eq!(view.content_state(), TimelineContentState::EmptyTracks);
        assert!(view.tracks[0].items.is_empty());
    }

    #[test]
    fn classifying_empty_tracks_never_removes_them() {
        let view = TimelineView {
            tracks: vec![empty_track(), empty_track()],
        };
        let before = view.clone();
        for _ in 0..3 {
            assert_eq!(view.content_state(), TimelineContentState::EmptyTracks);
        }
        assert_eq!(view, before);
    }

    #[test]
    fn supplied_items_select_populated_even_with_other_empty_tracks() {
        let mut view = TimelineView {
            tracks: vec![empty_track(), empty_track()],
        };
        view.tracks[1]
            .items
            .push(TimelineItemView::new("Supplied item", 1.0, 7.0).expect("valid display range"));
        assert_eq!(view.content_state(), TimelineContentState::Populated);
        assert_eq!(view.tracks.len(), 2);
        assert!(view.tracks[0].items.is_empty());
        assert_eq!(view.display_end_seconds(), 7.0);
    }

    #[test]
    fn selection_context_resolves_video_audio_and_mixed_items() {
        let mut view = TimelineView {
            tracks: vec![
                TimelineTrackView {
                    label: "V1".to_owned(),
                    name: "Video".to_owned(),
                    kind: TrackKind::Video,
                    items: vec![TimelineItemView::new("video", 0.0, 2.0).expect("valid")],
                },
                TimelineTrackView {
                    label: "A1".to_owned(),
                    name: "Audio".to_owned(),
                    kind: TrackKind::Audio,
                    items: vec![TimelineItemView::new("audio", 0.0, 2.0).expect("valid")],
                },
            ],
        };
        let none = TimelineSelectionContext::classify(&view, &BTreeSet::new());
        assert_eq!(none.kind(), None);

        let video_id = TimelineItemId { track: 0, item: 0 };
        let audio_id = TimelineItemId { track: 1, item: 0 };
        let video = TimelineSelectionContext::classify(&view, &BTreeSet::from([video_id]));
        assert_eq!(video.kind(), Some(TimelineSelectionKind::VideoOnly));
        let audio = TimelineSelectionContext::classify(&view, &BTreeSet::from([audio_id]));
        assert_eq!(audio.kind(), Some(TimelineSelectionKind::AudioOnly));
        let mixed =
            TimelineSelectionContext::classify(&view, &BTreeSet::from([video_id, audio_id]));
        assert_eq!(mixed.kind(), Some(TimelineSelectionKind::Mixed));

        view.tracks[0].items.clear();
        let stale = TimelineSelectionContext::classify(&view, &BTreeSet::from([video_id]));
        assert!(stale.is_empty());
        assert_eq!(stale.kind(), None);
    }

    #[test]
    fn source_mapping_honors_source_in_trim_and_speed() {
        let media = TimelineMediaRef::new("clip", 1, 30.0).expect("valid media");
        let item = TimelineItemView::new("Trimmed", 2.0, 6.0)
            .expect("range")
            .with_source(10.0, 2.0)
            .expect("source")
            .with_media(media);
        assert_eq!(item.source_time_at(2.0), Some(10.0));
        assert_eq!(item.source_time_at(4.0), Some(14.0));
        assert_eq!(item.source_time_at(6.0), Some(18.0));
        assert_eq!(item.source_time_at(1.9), None);
        assert_eq!(item.source_time_at(6.1), None);
    }

    #[test]
    fn source_mapping_clamps_to_media_duration_and_rejects_bad_speed() {
        let media = TimelineMediaRef::new("clip", 1, 5.0).expect("valid media");
        let item = TimelineItemView::new("Clamp", 0.0, 10.0)
            .expect("range")
            .with_source(4.0, 2.0)
            .expect("source")
            .with_media(media);
        assert_eq!(item.source_time_at(0.0), Some(4.0));
        assert_eq!(item.source_time_at(10.0), Some(5.0));
        assert!(TimelineItemView::new("Bad", 0.0, 1.0)
            .expect("range")
            .with_source(0.0, 0.0)
            .is_none());
        assert!(TimelineMediaRef::new("", 1, 10.0).is_none());
        assert!(TimelineMediaRef::new("x", 1, f64::NAN).is_none());
    }

    #[test]
    fn visual_demo_contains_all_media_categories_with_varied_widths() {
        let view = visual_demo();
        let mut kinds = BTreeSet::new();
        let mut widths = BTreeSet::new();
        let mut media_count = 0;
        for track in &view.tracks {
            for item in &track.items {
                kinds.insert(format!("{:?}", item.resolved_kind(track.kind)));
                widths.insert((item.duration_seconds() * 10.0).round() as i64);
                if item.media().is_some() {
                    media_count += 1;
                }
            }
        }
        for expected in ["Video", "Audio", "Text", "Effect"] {
            assert!(kinds.contains(expected), "missing {expected} demo item");
        }
        assert!(media_count >= 3);
        assert!(widths.len() >= 4, "demo should show varied clip widths");
        assert_eq!(
            view.tracks[2].items.len(),
            2,
            "adjacent video clips should be demonstrated"
        );
    }

    #[test]
    fn item_display_ranges_reject_invalid_geometry() {
        for (start, end) in [
            (-1.0, 2.0),
            (2.0, 2.0),
            (3.0, 2.0),
            (f64::NAN, 2.0),
            (0.0, f64::INFINITY),
        ] {
            assert!(TimelineItemView::new("Invalid", start, end).is_none());
        }
    }
}

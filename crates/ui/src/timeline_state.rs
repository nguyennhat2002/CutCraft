//! Read-only timeline presentation data, independent of egui.
//!
//! This is a projection seam for future engine snapshots, NOT an authoritative
//! project/time model. Normal sessions start with no rows or items. Widgets never
//! insert or delete content; import and edit commands remain unimplemented.

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

/// Display geometry and label for an item. Seconds are visualization only;
/// authoritative timing and editing will belong to core/engine.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItemView {
    label: String,
    start_seconds: f64,
    end_seconds: f64,
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
        })
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
            let item = match index {
                1 => TimelineItemView::new("Demo title", 2.0, 8.0),
                2 => TimelineItemView::new("Demo clip · preview only", 0.5, 14.0),
                3 => TimelineItemView::new("Demo audio · preview only", 0.5, 14.0),
                _ => None,
            };
            tracks.push(TimelineTrackView {
                label: label.to_owned(),
                name: name.to_owned(),
                kind,
                items: item.into_iter().collect(),
            });
        }
        TimelineView { tracks }
    });
    &DEMO
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

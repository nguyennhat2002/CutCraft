//! Projection from the authoritative native timeline into the read-only
//! `TimelineView` used by existing widgets. Display floats never become
//! authoritative edit state.

use std::collections::BTreeSet;

use capcut_core::time::{
    FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks,
    TIMELINE_TICKS_PER_SECOND,
};
use capcut_engine::command::{Command, CommandError, CommandOutcome, NativeEditor};
use capcut_engine::project::{ClipId, ClipKind, ClipSpec, Project, SourceMapping, TrackKind};

use crate::timeline_state::{
    TimelineItemId, TimelineItemKind, TimelineItemView, TimelineTrackView, TimelineView,
    TrackKind as ViewTrackKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTimeline {
    editor: NativeEditor,
    clip_map: Vec<Vec<ClipId>>,
}

impl NativeTimeline {
    pub fn from_project(project: Project) -> Result<Self, String> {
        let editor = NativeEditor::new(project).map_err(|error| error.to_string())?;
        Self::from_editor(editor)
    }

    pub fn from_editor(editor: NativeEditor) -> Result<Self, String> {
        let mut timeline = Self {
            editor,
            clip_map: Vec::new(),
        };
        timeline.rebuild_map();
        Ok(timeline)
    }

    pub fn qa_project() -> Result<Self, String> {
        let mut project = Project::named(
            FrameRate::new(30, 1).map_err(|e| e.to_string())?,
            "Native QA Project",
        )
        .map_err(|e| e.to_string())?;
        let text_track = project
            .add_track("T1", TrackKind::Video, false)
            .map_err(|e| e.to_string())?;
        let text_range = TimelineRange::new(
            TimelineTicks::new(12 * TIMELINE_TICKS_PER_SECOND),
            TimelineTicks::new(16 * TIMELINE_TICKS_PER_SECOND),
        )
        .map_err(|e| e.to_string())?;
        project
            .add_clip(
                text_track,
                ClipSpec {
                    name: "QA Caption".to_owned(),
                    kind: ClipKind::Subtitle,
                    range: text_range,
                    source: None,
                    media_asset_id: None,
                    linked_clip_ids: BTreeSet::new(),
                    nested: false,
                },
            )
            .map_err(|e| e.to_string())?;
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .map_err(|e| e.to_string())?;
        project
            .assign_main_track(track)
            .map_err(|e| e.to_string())?;
        for (label, start, end) in [
            ("QA Clip A", 0, 5),
            ("QA Clip B", 10, 15),
            ("QA Clip C", 20, 25),
            ("QA Clip D", 30, 35),
        ] {
            let duration = end - start;
            let source = SourceMapping::new(
                SourceRange::new(
                    SourceTicks::ZERO,
                    SourceTicks::new(duration * TIMELINE_TICKS_PER_SECOND),
                )
                .map_err(|e| e.to_string())?,
                SourceRange::new(
                    SourceTicks::ZERO,
                    SourceTicks::new(duration * TIMELINE_TICKS_PER_SECOND),
                )
                .map_err(|e| e.to_string())?,
                Timebase::new(TIMELINE_TICKS_PER_SECOND).map_err(|e| e.to_string())?,
                1,
                1,
            )
            .map_err(|e| e.to_string())?;
            let range = TimelineRange::new(
                TimelineTicks::new(start * TIMELINE_TICKS_PER_SECOND),
                TimelineTicks::new(end * TIMELINE_TICKS_PER_SECOND),
            )
            .map_err(|e| e.to_string())?;
            project
                .add_clip(
                    track,
                    ClipSpec {
                        name: label.to_owned(),
                        kind: ClipKind::Video,
                        range,
                        source: Some(source),
                        media_asset_id: None,
                        linked_clip_ids: BTreeSet::new(),
                        nested: false,
                    },
                )
                .map_err(|e| e.to_string())?;
        }
        Self::from_project(project)
    }

    pub fn project(&self) -> &Project {
        self.editor.project()
    }

    pub fn editor(&self) -> &NativeEditor {
        &self.editor
    }

    pub fn execute(&mut self, command: Command) -> Result<CommandOutcome, String> {
        let outcome = self
            .editor
            .execute(command)
            .map_err(|error| error.to_string())?;
        if outcome.changed {
            self.rebuild_map();
        }
        Ok(outcome)
    }

    pub fn clip_map(&self) -> &[Vec<ClipId>] {
        &self.clip_map
    }

    pub fn item_for_clip(&self, clip: ClipId) -> Option<TimelineItemId> {
        self.clip_map.iter().enumerate().find_map(|(track, clips)| {
            clips
                .iter()
                .position(|entry| *entry == clip)
                .map(|item| TimelineItemId { track, item })
        })
    }

    pub fn clip_at(&self, item: TimelineItemId) -> Option<ClipId> {
        self.clip_map
            .get(item.track)
            .and_then(|clips| clips.get(item.item))
            .copied()
    }

    pub fn selected_clips(&self, selected: &BTreeSet<TimelineItemId>) -> Vec<ClipId> {
        selected
            .iter()
            .filter_map(|item| self.clip_at(*item))
            .collect()
    }

    pub fn selected_items(&self) -> BTreeSet<TimelineItemId> {
        self.editor
            .selection()
            .ids()
            .iter()
            .filter_map(|clip| self.item_for_clip(*clip))
            .collect()
    }

    pub fn sync_selection(&mut self, selected: &BTreeSet<TimelineItemId>) {
        self.editor.clear_selection();
        for clip in self.selected_clips(selected) {
            self.editor.select_clip(clip);
        }
    }

    pub fn magnet_enabled(&self) -> bool {
        self.editor.magnet_enabled()
    }

    /// Compact every main-track gap in one engine transaction.
    pub fn enable_magnet(&mut self) -> Result<(), String> {
        self.editor.enable_magnet().map_err(command_error_text)?;
        self.refresh_after_edit();
        Ok(())
    }

    /// Disable the preference without moving clips.
    pub fn disable_magnet(&mut self) {
        self.editor.disable_magnet();
    }

    pub fn can_delete_selected(&self, selected: &BTreeSet<TimelineItemId>) -> bool {
        let clips = self.selected_clips(selected);
        clips.len() == 1
            && self
                .editor
                .validate_command(Command::DeleteClip { clip: clips[0] })
                .is_ok()
    }

    pub fn delete_selected(&mut self, selected: &BTreeSet<TimelineItemId>) -> Result<(), String> {
        let clips = self.selected_clips(selected);
        if clips.len() != 1 {
            return Err("Delete requires exactly one selected native clip.".to_owned());
        }
        self.sync_selection(selected);
        self.editor
            .execute(Command::DeleteClip { clip: clips[0] })
            .map_err(command_error_text)?;
        self.refresh_after_edit();
        Ok(())
    }

    pub fn can_split_clip(&self, clip: ClipId, at: TimelineTicks) -> bool {
        self.editor
            .validate_command(Command::SplitClip { clip, at })
            .is_ok()
    }

    pub fn split_clip(&mut self, clip: ClipId, at: TimelineTicks) -> Result<(), String> {
        self.editor
            .execute(Command::SplitClip { clip, at })
            .map_err(command_error_text)?;
        self.refresh_after_edit();
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), String> {
        self.editor.undo().map_err(command_error_text)?;
        self.refresh_after_edit();
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        self.editor.redo().map_err(command_error_text)?;
        self.refresh_after_edit();
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.editor.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.editor.can_redo()
    }

    fn refresh_after_edit(&mut self) {
        self.rebuild_map();
    }

    fn rebuild_map(&mut self) {
        self.clip_map = self
            .editor
            .project()
            .tracks()
            .iter()
            .map(|track| track.clips().iter().map(|clip| clip.id()).collect())
            .collect();
    }

    pub fn view(&self) -> TimelineView {
        let mut tracks = Vec::new();
        for track in self.editor.project().tracks() {
            let kind = match track.kind() {
                TrackKind::Video => ViewTrackKind::Video,
                TrackKind::Audio => ViewTrackKind::Audio,
            };
            let items = track
                .clips()
                .iter()
                .filter_map(|clip| {
                    let start =
                        clip.range().start().get() as f64 / TIMELINE_TICKS_PER_SECOND as f64;
                    let end = clip.range().end().get() as f64 / TIMELINE_TICKS_PER_SECOND as f64;
                    TimelineItemView::new(clip.name().to_owned(), start, end)
                        .map(|item| item.with_kind(view_kind(clip.kind())))
                })
                .collect();
            tracks.push(TimelineTrackView {
                label: track.name().to_owned(),
                name: track.name().to_owned(),
                kind,
                items,
            });
        }
        TimelineView { tracks }
    }
}

fn view_kind(kind: ClipKind) -> TimelineItemKind {
    match kind {
        ClipKind::Video | ClipKind::NestedComposition => TimelineItemKind::Video,
        ClipKind::Audio => TimelineItemKind::Audio,
        ClipKind::Subtitle => TimelineItemKind::Text,
        ClipKind::Effect => TimelineItemKind::Effect,
    }
}

fn command_error_text(error: CommandError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn main_starts(native: &NativeTimeline) -> Vec<f64> {
        native.view().tracks[1]
            .items
            .iter()
            .map(TimelineItemView::start_seconds)
            .collect()
    }

    fn main_ranges(native: &NativeTimeline) -> Vec<(i64, i64)> {
        native
            .editor()
            .project()
            .main_track()
            .expect("main")
            .clips()
            .iter()
            .map(|clip| (clip.range().start().get(), clip.range().end().get()))
            .collect()
    }

    #[test]
    fn qa_project_projects_four_main_clips_and_unlinked_text() {
        let native = NativeTimeline::qa_project().expect("qa");
        let view = native.view();
        assert_eq!(view.tracks.len(), 2);
        assert_eq!(view.tracks[0].label, "T1");
        assert_eq!(view.tracks[0].items.len(), 1);
        assert_eq!(
            view.tracks[0].items[0].resolved_kind(ViewTrackKind::Video),
            TimelineItemKind::Text
        );
        assert_eq!(view.tracks[0].items[0].start_seconds(), 12.0);
        assert_eq!(view.tracks[1].label, "V1");
        assert_eq!(view.tracks[1].items.len(), 4);
        assert_eq!(main_starts(&native), vec![0.0, 10.0, 20.0, 30.0]);
        assert_eq!(native.clip_map()[1].len(), 4);
        assert!(!native.magnet_enabled());
        let caption = native
            .editor()
            .project()
            .clip(native.clip_map()[0][0])
            .expect("caption");
        assert!(caption.linked_clip_ids().is_empty());
        assert_eq!(
            caption.range(),
            TimelineRange::new(
                TimelineTicks::new(12 * TIMELINE_TICKS_PER_SECOND),
                TimelineTicks::new(16 * TIMELINE_TICKS_PER_SECOND),
            )
            .expect("range")
        );
    }

    #[test]
    fn magnet_activation_delete_undo_and_redo_stay_coherent() {
        let mut native = NativeTimeline::qa_project().expect("qa");
        native.enable_magnet().expect("enable");
        assert!(native.magnet_enabled());
        assert_eq!(main_starts(&native), vec![0.0, 5.0, 10.0, 15.0]);
        assert_eq!(
            main_ranges(&native),
            vec![
                (0, 600_000),
                (600_000, 1_200_000),
                (1_200_000, 1_800_000),
                (1_800_000, 2_400_000)
            ],
            "qualitative display must match exact authoritative ticks"
        );
        assert_eq!(
            native.view().tracks[0].items[0].start_seconds(),
            12.0,
            "unlinked text must not follow the main track"
        );

        let selected = BTreeSet::from([TimelineItemId { track: 1, item: 1 }]);
        native
            .delete_selected(&selected)
            .expect("delete under Magnet");
        assert_eq!(main_starts(&native), vec![0.0, 5.0, 10.0]);
        assert_eq!(native.view().tracks[0].items[0].start_seconds(), 12.0);

        native.undo().expect("undo delete");
        assert_eq!(main_starts(&native), vec![0.0, 5.0, 10.0, 15.0]);
        assert!(native.magnet_enabled());

        native.undo().expect("undo activation");
        assert_eq!(main_starts(&native), vec![0.0, 10.0, 20.0, 30.0]);
        assert!(!native.magnet_enabled());

        native.redo().expect("redo activation");
        assert_eq!(main_starts(&native), vec![0.0, 5.0, 10.0, 15.0]);
        assert!(native.magnet_enabled());

        native.redo().expect("redo delete");
        assert_eq!(main_starts(&native), vec![0.0, 5.0, 10.0]);
        assert!(native.magnet_enabled());
    }

    #[test]
    fn magnet_off_delete_preserves_gaps_and_selection_mapping() {
        let mut native = NativeTimeline::qa_project().expect("qa");
        let selected = BTreeSet::from([TimelineItemId { track: 1, item: 1 }]);
        assert!(native.can_delete_selected(&selected));
        native.delete_selected(&selected).expect("delete");
        assert!(!native.magnet_enabled());
        assert_eq!(main_starts(&native), vec![0.0, 20.0, 30.0]);
        assert_eq!(native.selected_items(), BTreeSet::new());
        native.undo().expect("undo");
        assert_eq!(main_starts(&native), vec![0.0, 10.0, 20.0, 30.0]);
    }

    #[test]
    fn disabling_magnet_keeps_the_compacted_positions() {
        let mut native = NativeTimeline::qa_project().expect("qa");
        native.enable_magnet().expect("enable");
        native.disable_magnet();
        assert!(!native.magnet_enabled());
        assert_eq!(main_starts(&native), vec![0.0, 5.0, 10.0, 15.0]);
    }
}

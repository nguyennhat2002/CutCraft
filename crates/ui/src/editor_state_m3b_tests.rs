//! M3-B playhead and split session tests.

use super::*;
use crate::native_timeline::NativeTimeline;
use crate::timeline_state::TimelineItemId;
use capcut_core::time::{SourceRange, SourceTicks, Timebase, TimelineRange};
use capcut_engine::project::{
    ClipId, ClipKind, ClipSpec, Project as EngineProject, SourceMapping,
    TrackKind as EngineTrackKind,
};

fn native_frame_session(rate: FrameRate) -> EditorSession {
    let mut project = EngineProject::named(rate, "Frame Navigation").expect("project");
    let track = project
        .add_track("V1", EngineTrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let end = rate.frame_to_ticks(120).expect("frame end");
    let source = SourceMapping::new(
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(end.get())).expect("available"),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(end.get())).expect("used"),
        Timebase::new(120_000).expect("timebase"),
        1,
        1,
    )
    .expect("source");
    project
        .add_clip(
            track,
            ClipSpec {
                name: "A".to_owned(),
                kind: ClipKind::Video,
                range: TimelineRange::new(TimelineTicks::ZERO, end).expect("range"),
                source: Some(source),
                media_asset_id: None,
                linked_clip_ids: Default::default(),
                nested: false,
            },
        )
        .expect("clip");
    let native = NativeTimeline::from_project(project).expect("native");
    let view = native.view();
    EditorSession {
        project_name: "Frame Navigation".to_owned(),
        timeline_view: view,
        native_timeline: Some(native),
        ..EditorSession::default()
    }
}

fn native_project_session(project: EngineProject) -> EditorSession {
    let native = NativeTimeline::from_project(project).expect("native");
    let name = native.project().name().to_owned();
    let view = native.view();
    EditorSession {
        project_name: name,
        timeline_view: view,
        native_timeline: Some(native),
        ..EditorSession::default()
    }
}

fn source_for(duration: i64) -> SourceMapping {
    SourceMapping::new(
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("available"),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("used"),
        Timebase::new(120_000).expect("timebase"),
        1,
        1,
    )
    .expect("source")
}

fn clip_spec(name: &str, start: i64, end: i64) -> ClipSpec {
    ClipSpec {
        name: name.to_owned(),
        kind: ClipKind::Video,
        range: TimelineRange::new(TimelineTicks::new(start), TimelineTicks::new(end))
            .expect("range"),
        source: Some(source_for(end - start)),
        media_asset_id: None,
        linked_clip_ids: Default::default(),
        nested: false,
    }
}

#[test]
fn playhead_navigation_uses_exact_frame_indices_at_all_supported_rates() {
    for rate in [
        FrameRate::new(24, 1).expect("24"),
        FrameRate::new(25, 1).expect("25"),
        FrameRate::new(30, 1).expect("30"),
        FrameRate::new(60, 1).expect("60"),
        FrameRate::new(24_000, 1_001).expect("23.976"),
        FrameRate::new(30_000, 1_001).expect("29.97"),
    ] {
        let mut session = native_frame_session(rate);
        assert_eq!(session.playhead_ticks(), TimelineTicks::ZERO);
        assert!(session.next_frame().expect("next"));
        assert_eq!(
            session.playhead_ticks(),
            rate.frame_to_ticks(1).expect("frame 1")
        );
        assert!(session.previous_frame().expect("previous"));
        assert_eq!(session.playhead_ticks(), TimelineTicks::ZERO);

        let frame_ten = rate.frame_to_ticks(10).expect("frame 10");
        session.set_playhead_ticks(frame_ten);
        assert!(session.previous_frame().expect("previous ten"));
        assert_eq!(
            session.playhead_ticks(),
            rate.frame_to_ticks(9).expect("frame 9")
        );
        assert!(session.next_frame().expect("next ten"));
        assert_eq!(session.playhead_ticks(), frame_ten);

        let end = rate.frame_to_ticks(120).expect("end");
        assert!(session.jump_playhead_to_end());
        assert_eq!(session.playhead_ticks(), end);
        assert!(session.previous_frame().expect("previous end"));
        assert_eq!(
            session.playhead_ticks(),
            rate.frame_to_ticks(119).expect("frame 119")
        );
        assert!(session.next_frame().expect("next end"));
        assert_eq!(session.playhead_ticks(), end);
        assert!(!session.next_frame().expect("next at end"));
        assert_eq!(session.playhead_ticks(), end);
        assert!(session.jump_playhead_to_start());
        assert_eq!(session.playhead_ticks(), TimelineTicks::ZERO);
    }
}

#[test]
fn empty_projects_keep_the_playhead_at_zero() {
    let mut session = EditorSession::new_native().expect("new native");
    assert_eq!(session.native_content_end(), Some(TimelineTicks::ZERO));
    assert!(!session.set_playhead_ticks(TimelineTicks::new(600_000)));
    assert_eq!(session.playhead_ticks(), TimelineTicks::ZERO);
    assert!(!session.next_frame().expect("empty next"));
    assert!(!session.jump_playhead_to_end());
    assert_eq!(session.playhead_ticks(), TimelineTicks::ZERO);
    assert!(!session.native_is_dirty());
}

#[test]
fn playhead_movement_is_session_only_and_selection_independent() {
    let mut session = EditorSession::with_native_qa().expect("qa");
    assert!(!session.native_is_dirty());
    assert!(!session.can_undo_native());
    assert!(session.set_playhead_ticks(TimelineTicks::new(600_000)));
    assert!(!session.native_is_dirty());
    assert!(!session.can_undo_native());

    assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 1 }, false));
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(600_000));
    session.clear_timeline_selection();
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(600_000));

    session.set_timeline_demo(true);
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(600_000));
    session.set_timeline_demo(false);
}

#[test]
fn playhead_clamps_after_content_shrinks_and_magnet_compaction() {
    let mut session = EditorSession::with_native_qa().expect("qa");
    assert!(session.set_playhead_ticks(TimelineTicks::new(34 * 120_000)));
    assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 3 }, false));
    session.delete_selected_native().expect("delete last clip");
    assert_eq!(
        session.native_content_end(),
        Some(TimelineTicks::new(25 * 120_000))
    );
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(25 * 120_000));

    let mut session = EditorSession::with_native_qa().expect("qa");
    assert!(session.set_playhead_ticks(TimelineTicks::new(34 * 120_000)));
    session.toggle_main_track_magnet();
    assert_eq!(
        session.native_content_end(),
        Some(TimelineTicks::new(20 * 120_000))
    );
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(20 * 120_000));
    assert!(session.main_track_magnet_enabled());
}

#[test]
fn split_at_playhead_splits_exactly_and_undo_redo_preserves_ids() {
    let mut session = EditorSession::with_native_qa().expect("qa");
    assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 1 }, false));
    assert!(session.set_playhead_ticks(TimelineTicks::new(12 * 120_000)));
    assert!(session.can_split_at_playhead());
    assert!(!session.native_is_dirty());

    session.split_at_playhead().expect("split");
    assert!(session.native_is_dirty());
    let native = session.native_timeline.as_ref().expect("native");
    let main = native.project().main_track().expect("main");
    assert_eq!(main.clips().len(), 5);
    let left = &main.clips()[1];
    let right = &main.clips()[2];
    assert_eq!(left.id().get(), 3);
    assert_eq!(left.range().start().get(), 10 * 120_000);
    assert_eq!(left.range().end().get(), 12 * 120_000);
    assert_eq!(right.id().get(), 6);
    assert_eq!(right.range().start().get(), 12 * 120_000);
    assert_eq!(right.range().end().get(), 15 * 120_000);
    assert_eq!(
        left.source().expect("left source").used(),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(240_000)).expect("left")
    );
    assert_eq!(
        right.source().expect("right source").used(),
        SourceRange::new(SourceTicks::new(240_000), SourceTicks::new(600_000)).expect("right")
    );
    assert_eq!(
        native.project().content_end(),
        TimelineTicks::new(35 * 120_000)
    );
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(12 * 120_000));

    session.undo_native().expect("undo split");
    assert_eq!(
        session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main")
            .clips()
            .len(),
        4
    );
    assert!(session.is_timeline_item_selected(TimelineItemId { track: 1, item: 1 }));
    assert_eq!(
        session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .next_clip_id(),
        7,
        "Undo must not roll back the allocated split ID"
    );
    session.redo_native().expect("redo split");
    assert_eq!(
        session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main")
            .clips()
            .len(),
        5
    );
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(12 * 120_000));

    session.undo_native().expect("undo again");
    assert!(session.set_playhead_ticks(TimelineTicks::new(13 * 120_000)));
    session.split_at_playhead().expect("split again");
    let right_after = &session
        .native_timeline
        .as_ref()
        .expect("native")
        .project()
        .main_track()
        .expect("main")
        .clips()[2];
    assert_eq!(right_after.id().get(), 7, "discarded ID must not be reused");
}

#[test]
fn split_rejects_no_selection_multi_selection_and_boundary_points() {
    let mut session = EditorSession::with_native_qa().expect("qa");
    assert!(!session.can_split_at_playhead());
    assert!(session.split_at_playhead().is_err());

    let b = TimelineItemId { track: 1, item: 1 };
    assert!(session.select_timeline_item(b, false));
    assert!(session.set_playhead_ticks(TimelineTicks::new(10 * 120_000)));
    assert!(!session.can_split_at_playhead());
    let before = session
        .native_timeline
        .as_ref()
        .expect("native")
        .project()
        .clone();
    assert!(session.split_at_playhead().is_err());
    assert_eq!(
        session.native_timeline.as_ref().expect("native").project(),
        &before
    );
    assert!(!session.native_is_dirty());

    assert!(session.set_playhead_ticks(TimelineTicks::new(15 * 120_000)));
    assert!(!session.can_split_at_playhead());
    assert!(session.split_at_playhead().is_err());

    assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 2 }, true));
    assert_eq!(session.timeline_selection().len(), 2);
    assert!(!session.can_split_at_playhead());
    assert!(session.split_at_playhead().is_err());
}

#[test]
fn split_with_magnet_on_does_not_ripple_or_change_magnet_state() {
    let mut session = EditorSession::with_native_qa().expect("qa");
    session.toggle_main_track_magnet();
    assert!(session.main_track_magnet_enabled());
    assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 1 }, false));
    assert!(session.set_playhead_ticks(TimelineTicks::new(7 * 120_000)));
    assert!(session.can_split_at_playhead());
    session.split_at_playhead().expect("split on");
    let native = session.native_timeline.as_ref().expect("native");
    let starts = native
        .project()
        .main_track()
        .expect("main")
        .clips()
        .iter()
        .map(|clip| clip.range().start().get())
        .collect::<Vec<_>>();
    assert_eq!(
        starts,
        vec![0, 5 * 120_000, 7 * 120_000, 10 * 120_000, 15 * 120_000]
    );
    assert!(session.main_track_magnet_enabled());
    assert_eq!(
        native.project().content_end(),
        TimelineTicks::new(20 * 120_000)
    );
}

#[test]
fn split_rejects_locked_linked_nested_and_source_less_main_clips() {
    let mut locked_project =
        EngineProject::named(FrameRate::new(30, 1).expect("fps"), "Locked").expect("project");
    let locked_track = locked_project
        .add_track("V1", EngineTrackKind::Video, true)
        .expect("track");
    locked_project
        .assign_main_track(locked_track)
        .expect("main");
    locked_project
        .add_clip(locked_track, clip_spec("A", 0, 600_000))
        .expect("clip");
    let mut locked = native_project_session(locked_project);
    assert!(locked.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
    locked.set_playhead_ticks(TimelineTicks::new(240_000));
    assert!(!locked.can_split_at_playhead());
    assert!(locked.split_at_playhead().is_err());

    let mut linked_project =
        EngineProject::named(FrameRate::new(30, 1).expect("fps"), "Linked").expect("project");
    let main = linked_project
        .add_track("V1", EngineTrackKind::Video, false)
        .expect("main track");
    linked_project.assign_main_track(main).expect("main");
    let overlay = linked_project
        .add_track("T1", EngineTrackKind::Video, false)
        .expect("overlay");
    let mut linked_clip = clip_spec("A", 0, 600_000);
    let mut overlay_clip = clip_spec("Caption", 0, 600_000);
    overlay_clip.source = None;
    overlay_clip.kind = ClipKind::Subtitle;
    overlay_clip.linked_clip_ids.insert(ClipId::new(1));
    linked_clip.linked_clip_ids.insert(ClipId::new(2));
    linked_project.add_clip(main, linked_clip).expect("clip");
    linked_project
        .add_clip(overlay, overlay_clip)
        .expect("caption");
    let mut linked = native_project_session(linked_project);
    assert!(linked.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
    linked.set_playhead_ticks(TimelineTicks::new(240_000));
    assert!(!linked.can_split_at_playhead());
    assert!(linked.split_at_playhead().is_err());

    let mut nested_project =
        EngineProject::named(FrameRate::new(30, 1).expect("fps"), "Nested").expect("project");
    let nested_track = nested_project
        .add_track("V1", EngineTrackKind::Video, false)
        .expect("track");
    nested_project
        .assign_main_track(nested_track)
        .expect("main");
    nested_project
        .add_clip(
            nested_track,
            ClipSpec {
                name: "Nested".to_owned(),
                kind: ClipKind::NestedComposition,
                range: TimelineRange::new(TimelineTicks::ZERO, TimelineTicks::new(600_000))
                    .expect("range"),
                source: None,
                media_asset_id: None,
                linked_clip_ids: Default::default(),
                nested: true,
            },
        )
        .expect("nested");
    let mut nested = native_project_session(nested_project);
    assert!(nested.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
    nested.set_playhead_ticks(TimelineTicks::new(240_000));
    assert!(!nested.can_split_at_playhead());
    assert!(nested.split_at_playhead().is_err());

    let mut source_less_project =
        EngineProject::named(FrameRate::new(30, 1).expect("fps"), "No Source").expect("project");
    let source_less_track = source_less_project
        .add_track("V1", EngineTrackKind::Video, false)
        .expect("track");
    source_less_project
        .assign_main_track(source_less_track)
        .expect("main");
    let mut source_less = clip_spec("A", 0, 600_000);
    source_less.source = None;
    source_less_project
        .add_clip(source_less_track, source_less)
        .expect("clip");
    let mut source_less_session = native_project_session(source_less_project);
    assert!(source_less_session.select_timeline_item(TimelineItemId { track: 0, item: 0 }, false));
    source_less_session.set_playhead_ticks(TimelineTicks::new(240_000));
    assert!(!source_less_session.can_split_at_playhead());
    assert!(source_less_session.split_at_playhead().is_err());
}

#[test]
fn shortcut_commands_drive_frame_navigation_and_split() {
    let mut session = EditorSession::with_native_qa().expect("qa");
    assert!(session.execute_shortcut_command(crate::shortcuts::CommandId::NextFrame));
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(4_000));
    assert!(session.execute_shortcut_command(crate::shortcuts::CommandId::JumpToEnd));
    assert_eq!(session.playhead_ticks(), TimelineTicks::new(35 * 120_000));
    assert!(session.execute_shortcut_command(crate::shortcuts::CommandId::JumpToBeginning));
    assert_eq!(session.playhead_ticks(), TimelineTicks::ZERO);

    assert!(session.select_timeline_item(TimelineItemId { track: 1, item: 1 }, false));
    assert!(session.set_playhead_ticks(TimelineTicks::new(12 * 120_000)));
    assert!(session.execute_shortcut_command(crate::shortcuts::CommandId::SplitAtPlayhead));
    assert_eq!(
        session
            .native_timeline
            .as_ref()
            .expect("native")
            .project()
            .main_track()
            .expect("main")
            .clips()
            .len(),
        5
    );
}

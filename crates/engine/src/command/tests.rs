//! Command, normalization and Magnet-state tests for the native engine.

use super::*;
use crate::project::{ClipId, ClipKind, ClipSpec, Project, SourceMapping, TrackKind};
use capcut_core::time::{
    FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks,
};
use std::collections::BTreeSet;

fn rate() -> FrameRate {
    FrameRate::new(30, 1).expect("rate")
}

fn video_asset(name: &str, duration_seconds: i64) -> MediaAssetRegistration {
    let metadata = MediaAssetMetadata {
        container: "mov,mp4".to_owned(),
        video_stream_index: 0,
        codec: "h264".to_owned(),
        width: 1920,
        height: 1080,
        pixel_format: Some("yuv420p".to_owned()),
        timebase_numerator: 1,
        timebase_denominator: 15_360,
        stream_start_ticks: 0,
        duration_ticks: Some(duration_seconds * 15_360),
        nominal_frame_rate: crate::asset::MediaRational::new(30, 1),
        average_frame_rate: crate::asset::MediaRational::new(30, 1),
        rotation_degrees: 0,
        has_audio: false,
        frame_count_hint: None,
    };
    MediaAssetRegistration::video(
        name,
        MediaLocator::absolute(format!("C:\\media\\{name}.mp4")).expect("locator"),
        metadata,
    )
    .expect("asset")
}

fn ticks(seconds: i64) -> TimelineTicks {
    TimelineTicks::new(seconds * 120_000)
}

fn range(start: i64, end: i64) -> TimelineRange {
    TimelineRange::new(ticks(start), ticks(end)).expect("range")
}

fn source(duration: i64) -> SourceMapping {
    SourceMapping::new(
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration * 120_000))
            .expect("available"),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration * 120_000)).expect("used"),
        Timebase::new(120_000).expect("timebase"),
        1,
        1,
    )
    .expect("source")
}

fn spec(name: &str, start: i64, end: i64) -> ClipSpec {
    ClipSpec {
        name: name.to_owned(),
        kind: ClipKind::Video,
        range: range(start, end),
        source: Some(source(end - start)),
        media_asset_id: None,
        linked_clip_ids: BTreeSet::new(),
        nested: false,
    }
}

fn spec_at_ticks(name: &str, start: i64, end: i64) -> ClipSpec {
    let duration = end - start;
    let mapping = SourceMapping::new(
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("available"),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("used"),
        Timebase::new(120_000).expect("timebase"),
        1,
        1,
    )
    .expect("source");
    ClipSpec {
        name: name.to_owned(),
        kind: ClipKind::Video,
        range: TimelineRange::new(TimelineTicks::new(start), TimelineTicks::new(end))
            .expect("range"),
        source: Some(mapping),
        media_asset_id: None,
        linked_clip_ids: BTreeSet::new(),
        nested: false,
    }
}

fn text_spec(name: &str, start: i64, end: i64, links: BTreeSet<ClipId>) -> ClipSpec {
    ClipSpec {
        name: name.to_owned(),
        kind: ClipKind::Subtitle,
        range: range(start, end),
        source: None,
        media_asset_id: None,
        linked_clip_ids: links,
        nested: false,
    }
}

fn nested_spec(name: &str, start: i64, end: i64) -> ClipSpec {
    ClipSpec {
        name: name.to_owned(),
        kind: ClipKind::NestedComposition,
        range: range(start, end),
        source: None,
        media_asset_id: None,
        linked_clip_ids: BTreeSet::new(),
        nested: true,
    }
}

fn project_with_abc() -> (Project, ClipId, ClipId, ClipId) {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let a = project.add_clip(track, spec("A", 0, 5)).expect("A");
    let b = project.add_clip(track, spec("B", 10, 15)).expect("B");
    let c = project.add_clip(track, spec("C", 20, 25)).expect("C");
    (project, a, b, c)
}

fn project_with_abcd() -> (Project, ClipId, ClipId, ClipId, ClipId) {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let a = project.add_clip(track, spec("A", 0, 5)).expect("A");
    let b = project.add_clip(track, spec("B", 10, 13)).expect("B");
    let c = project.add_clip(track, spec("C", 20, 25)).expect("C");
    let d = project.add_clip(track, spec("D", 30, 32)).expect("D");
    (project, a, b, c, d)
}

fn project_contiguous() -> (Project, ClipId, ClipId, ClipId) {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let a = project.add_clip(track, spec("A", 0, 5)).expect("A");
    let b = project.add_clip(track, spec("B", 5, 10)).expect("B");
    let c = project.add_clip(track, spec("C", 10, 15)).expect("C");
    (project, a, b, c)
}

fn make_editor(project: Project) -> NativeEditor {
    NativeEditor::new(project).expect("editor")
}

fn ranges(editor: &NativeEditor) -> Vec<(ClipId, i64, i64)> {
    editor
        .project()
        .main_track()
        .expect("main")
        .clips()
        .iter()
        .map(|clip| {
            (
                clip.id(),
                clip.range().start().get(),
                clip.range().end().get(),
            )
        })
        .collect()
}

fn names(editor: &NativeEditor) -> Vec<String> {
    editor
        .project()
        .main_track()
        .expect("main")
        .clips()
        .iter()
        .map(|clip| clip.name().to_owned())
        .collect()
}

fn used_range(editor: &NativeEditor, clip: ClipId) -> SourceRange {
    editor
        .project()
        .clip(clip)
        .expect("clip")
        .source()
        .expect("source")
        .used()
}

#[test]
fn restored_editor_validates_magnet_continuity_and_starts_clean() {
    let (gapped, a, b, c) = project_with_abc();
    assert_eq!(
        NativeEditor::from_restored(gapped.clone(), true),
        Err(CommandError::MagnetTrackNotContiguous)
    );

    let editor = NativeEditor::from_restored(gapped, false).expect("Magnet OFF");
    assert!(!editor.magnet_enabled());
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 1_200_000, 1_800_000),
            (c, 2_400_000, 3_000_000)
        ],
        "loading with Magnet OFF must never normalize gaps"
    );
    assert_eq!(editor.project().next_clip_id(), 4);

    let (contiguous, _, _, _) = project_contiguous();
    let mut editor = NativeEditor::from_restored(contiguous, true).expect("Magnet ON");
    assert!(editor.magnet_enabled());
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());
    editor
        .execute(Command::DeleteClip {
            clip: ClipId::new(2),
        })
        .expect("continuity-preserving edit after restore");
    assert_eq!(
        ranges(&editor),
        vec![
            (ClipId::new(1), 0, 600_000),
            (ClipId::new(3), 600_000, 1_200_000)
        ]
    );
}

#[test]
fn restored_magnet_on_rejects_missing_main_track() {
    let project = Project::empty(rate()).expect("project");
    assert_eq!(
        NativeEditor::from_restored(project, true),
        Err(CommandError::MissingMainTrack)
    );
}

#[test]
fn enable_compacts_three_gapped_clips_exactly_as_required() {
    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    let sources_before = [
        used_range(&editor, a),
        used_range(&editor, b),
        used_range(&editor, c),
    ];

    let outcome = editor.enable_magnet().expect("enable Magnet");

    assert!(outcome.changed);
    assert_eq!(outcome.affected, vec![b, c]);
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 600_000, 1_200_000),
            (c, 1_200_000, 1_800_000)
        ]
    );
    assert_eq!(names(&editor), vec!["A", "B", "C"]);
    assert!(editor.magnet_enabled());
    assert_eq!(editor.project().revision(), 1);
    assert_eq!(editor.project().next_clip_id(), 4);
    assert_eq!(
        [
            used_range(&editor, a),
            used_range(&editor, b),
            used_range(&editor, c)
        ],
        sources_before
    );
    for clip in editor.project().main_track().expect("main").clips() {
        assert_eq!(clip.range().duration(), ticks(5));
    }
    editor.project().validate().expect("valid invariants");
}

#[test]
fn normalize_four_variable_duration_clips_preserves_order_sources_and_timing() {
    let (project, a, b, c, d) = project_with_abcd();
    let mut editor = make_editor(project);
    let sources_before = [
        used_range(&editor, a),
        used_range(&editor, b),
        used_range(&editor, c),
        used_range(&editor, d),
    ];

    let outcome = editor.enable_magnet().expect("enable Magnet");

    assert!(outcome.changed);
    assert_eq!(outcome.affected, vec![b, c, d]);
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 600_000, 960_000),
            (c, 960_000, 1_560_000),
            (d, 1_560_000, 1_800_000)
        ]
    );
    assert_eq!(names(&editor), vec!["A", "B", "C", "D"]);
    let durations: Vec<i64> = editor
        .project()
        .main_track()
        .expect("main")
        .clips()
        .iter()
        .map(|clip| clip.range().duration().get())
        .collect();
    assert_eq!(durations, vec![600_000, 360_000, 600_000, 240_000]);
    assert_eq!(
        [
            used_range(&editor, a),
            used_range(&editor, b),
            used_range(&editor, c),
            used_range(&editor, d)
        ],
        sources_before
    );
    assert_eq!(editor.project().next_clip_id(), 5);
    assert!(editor.magnet_enabled());
}

#[test]
fn normalize_preserves_frame_boundary_ticks_exactly() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    // 30 fps boundaries: 4,000 ticks per frame, none aligned to whole seconds.
    let a = project
        .add_clip(track, spec_at_ticks("A", 0, 12_000))
        .expect("A");
    let b = project
        .add_clip(track, spec_at_ticks("B", 40_000, 52_000))
        .expect("B");
    let c = project
        .add_clip(track, spec_at_ticks("C", 80_000, 96_000))
        .expect("C");
    let mut editor = make_editor(project);
    let before = used_range(&editor, b);

    editor.enable_magnet().expect("enable");

    assert_eq!(
        ranges(&editor),
        vec![(a, 0, 12_000), (b, 12_000, 24_000), (c, 24_000, 40_000)]
    );
    assert_eq!(used_range(&editor, b), before);
    assert_eq!(
        editor
            .project()
            .main_track()
            .expect("main")
            .clips()
            .iter()
            .map(|clip| clip.range().duration().get())
            .collect::<Vec<_>>(),
        vec![12_000, 12_000, 16_000]
    );
}

#[test]
fn normalize_single_offset_clip_moves_to_zero_and_keeps_duration() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let clip = project.add_clip(track, spec("Only", 10, 15)).expect("clip");
    let mut editor = make_editor(project);

    let outcome = editor.enable_magnet().expect("enable");

    assert!(outcome.changed);
    assert_eq!(outcome.affected, vec![clip]);
    assert_eq!(ranges(&editor), vec![(clip, 0, 600_000)]);
    assert_eq!(used_range(&editor, clip), source(5).used());
    assert!(editor.magnet_enabled());
}

#[test]
fn normalize_empty_or_missing_main_track_is_typed() {
    let mut editor = make_editor(Project::empty(rate()).expect("project"));
    assert_eq!(editor.enable_magnet(), Err(CommandError::MissingMainTrack));
    assert!(!editor.magnet_enabled());
    assert!(!editor.can_undo());

    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let mut editor = make_editor(project);
    let outcome = editor.enable_magnet().expect("empty main track");
    assert!(!outcome.changed);
    assert!(outcome.affected.is_empty());
    assert!(editor.magnet_enabled());
    assert_eq!(editor.project().revision(), 0);
    assert!(!editor.can_undo());
    editor.disable_magnet();
    assert!(!editor.magnet_enabled());
}

#[test]
fn already_contiguous_track_only_changes_the_preference() {
    let (project, _, _, _) = project_contiguous();
    let mut editor = make_editor(project);
    let before = editor.project().clone();

    let outcome = editor.enable_magnet().expect("enable");

    assert!(!outcome.changed);
    assert!(outcome.affected.is_empty());
    assert!(editor.magnet_enabled());
    assert_eq!(editor.project(), &before);
    assert_eq!(editor.project().revision(), 0);
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());

    editor.disable_magnet();
    assert!(!editor.magnet_enabled());
    assert_eq!(editor.project(), &before);

    editor.enable_magnet().expect("re-enable");
    assert!(editor.magnet_enabled());
    assert!(!editor.can_undo());
}

#[test]
fn undo_redo_restore_positions_and_magnet_state_coherently() {
    let (project, a, b, c) = project_with_abc();
    let gaps = project.clone();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    let normalized = editor.project().clone();

    editor.undo().expect("undo activation");
    assert_eq!(editor.project().main_track(), gaps.main_track());
    assert!(!editor.magnet_enabled());
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 1_200_000, 1_800_000),
            (c, 2_400_000, 3_000_000)
        ]
    );
    assert_eq!(editor.project().revision(), 2);

    editor.redo().expect("redo activation");
    assert_eq!(editor.project().main_track(), normalized.main_track());
    assert!(editor.magnet_enabled());
    assert_eq!(editor.project().revision(), 3);
}

#[test]
fn disabling_magnet_is_a_preference_and_undo_still_restores_the_snapshot() {
    let (project, _, _, _) = project_with_abc();
    let gaps = project.clone();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    editor.disable_magnet();
    assert!(!editor.magnet_enabled());
    assert_eq!(
        ranges(&editor),
        vec![
            (ClipId::new(1), 0, 600_000),
            (ClipId::new(2), 600_000, 1_200_000),
            (ClipId::new(3), 1_200_000, 1_800_000)
        ],
        "disabling Magnet never moves clips"
    );

    editor.undo().expect("undo compaction");
    assert_eq!(editor.project().main_track(), gaps.main_track());
    assert!(!editor.magnet_enabled());

    editor.redo().expect("redo compaction");
    assert!(editor.magnet_enabled(), "redo restores its recorded state");
    assert_eq!(ranges(&editor)[2], (ClipId::new(3), 1_200_000, 1_800_000));
}

#[test]
fn activation_rejects_a_locked_track_without_mutation() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, true)
        .expect("track");
    project.assign_main_track(track).expect("main");
    project.add_clip(track, spec("A", 0, 5)).expect("A");
    project.add_clip(track, spec("B", 10, 15)).expect("B");
    let before = project.clone();
    let mut editor = make_editor(project);

    assert_eq!(
        editor.enable_magnet(),
        Err(CommandError::TrackLocked(track))
    );
    assert_eq!(editor.project(), &before);
    assert!(!editor.magnet_enabled());
    assert!(!editor.can_undo());
}

#[test]
fn activation_rejects_cross_track_links_atomically() {
    let mut project = Project::empty(rate()).expect("project");
    let main = project
        .add_track("V1", TrackKind::Video, false)
        .expect("main track");
    project.assign_main_track(main).expect("main");
    project.add_clip(main, spec("A", 0, 5)).expect("A");
    let b = project.add_clip(main, spec("B", 10, 15)).expect("B");
    let text = project
        .add_track("T1", TrackKind::Video, false)
        .expect("text track");
    let links = BTreeSet::from([b]);
    project
        .add_clip(text, text_spec("Caption", 2, 7, links))
        .expect("caption");
    let before = project.clone();
    let mut editor = make_editor(project);

    assert_eq!(
        editor.enable_magnet(),
        Err(CommandError::UnsupportedLinkedMedia)
    );
    assert_eq!(editor.project(), &before);
    assert!(!editor.magnet_enabled());
    assert!(!editor.can_undo());
    let caption = editor
        .project()
        .tracks()
        .iter()
        .find(|track| track.name() == "T1")
        .and_then(|track| track.clips().first())
        .expect("caption");
    assert_eq!(caption.range(), range(2, 7));
}

#[test]
fn activation_rejects_nested_compositions_atomically() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    project.add_clip(track, spec("A", 0, 5)).expect("A");
    project
        .add_clip(track, nested_spec("Nested", 10, 15))
        .expect("nested");
    let before = project.clone();
    let mut editor = make_editor(project);

    assert_eq!(
        editor.enable_magnet(),
        Err(CommandError::UnsupportedNestedComposition)
    );
    assert_eq!(editor.project(), &before);
    assert!(!editor.magnet_enabled());
    assert!(!editor.can_undo());
}

#[test]
fn activation_rejects_an_oversized_history_transaction_atomically() {
    let (project, _, _, _) = project_with_abc();
    let before = project.clone();
    let mut editor = NativeEditor::with_history_budget(project, 1).expect("editor");

    assert!(matches!(
        editor.enable_magnet(),
        Err(CommandError::HistoryBudgetExceeded { .. })
    ));
    assert_eq!(editor.project(), &before);
    assert!(!editor.magnet_enabled());
    assert!(!editor.can_undo());
    assert_eq!(editor.project().next_clip_id(), before.next_clip_id());
}

#[test]
fn magnet_on_delete_keeps_the_main_track_contiguous_and_repairs_on_undo() {
    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");

    editor
        .execute(Command::DeleteClip { clip: b })
        .expect("delete");

    assert_eq!(
        ranges(&editor),
        vec![(a, 0, 600_000), (c, 600_000, 1_200_000)]
    );
    assert!(editor.magnet_enabled());

    editor.undo().expect("undo delete");
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 600_000, 1_200_000),
            (c, 1_200_000, 1_800_000)
        ]
    );
    assert!(editor.magnet_enabled());

    editor.undo().expect("undo activation");
    assert!(!editor.magnet_enabled());
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 1_200_000, 1_800_000),
            (c, 2_400_000, 3_000_000)
        ]
    );

    editor.redo().expect("redo activation");
    editor.redo().expect("redo delete");
    assert_eq!(
        ranges(&editor),
        vec![(a, 0, 600_000), (c, 600_000, 1_200_000)]
    );
    assert!(editor.magnet_enabled());
}

#[test]
fn magnet_on_delete_first_and_last_keep_continuity() {
    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    editor
        .execute(Command::DeleteClip { clip: a })
        .expect("first");
    assert_eq!(
        ranges(&editor),
        vec![(b, 0, 600_000), (c, 600_000, 1_200_000)]
    );
    assert!(editor.magnet_enabled());

    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    editor
        .execute(Command::DeleteClip { clip: c })
        .expect("last");
    assert_eq!(
        ranges(&editor),
        vec![(a, 0, 600_000), (b, 600_000, 1_200_000)]
    );
    assert!(editor.magnet_enabled());
}

#[test]
fn magnet_on_trim_shortening_keeps_continuity_and_updates_source_use() {
    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");

    editor
        .execute(Command::TrimClipEnd {
            clip: b,
            new_end: ticks(8),
        })
        .expect("trim");

    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 600_000, 960_000),
            (c, 960_000, 1_560_000)
        ]
    );
    assert_eq!(
        used_range(&editor, b),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(360_000)).expect("used")
    );
    assert!(editor.magnet_enabled());

    editor.undo().expect("undo trim");
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 600_000, 1_200_000),
            (c, 1_200_000, 1_800_000)
        ]
    );
    assert!(editor.magnet_enabled());
}

#[test]
fn magnet_off_delete_and_trim_leave_suffix_positions_alone() {
    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor
        .execute(Command::DeleteClip { clip: b })
        .expect("delete");
    assert!(!editor.magnet_enabled());
    assert_eq!(
        ranges(&editor),
        vec![(a, 0, 600_000), (c, 2_400_000, 3_000_000)]
    );

    editor.undo().expect("undo delete");
    editor
        .execute(Command::TrimClipEnd {
            clip: b,
            new_end: ticks(13),
        })
        .expect("trim");
    assert_eq!(
        ranges(&editor),
        vec![
            (a, 0, 600_000),
            (b, 1_200_000, 1_560_000),
            (c, 2_400_000, 3_000_000)
        ]
    );
}

#[test]
fn magnet_edits_reject_gapped_state_before_mutating() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.magnet_enabled = true;
    let before = editor.project().clone();

    assert_eq!(
        editor.execute(Command::DeleteClip { clip: b }),
        Err(CommandError::MagnetTrackNotContiguous)
    );
    assert_eq!(
        editor.execute(Command::TrimClipEnd {
            clip: b,
            new_end: ticks(13),
        }),
        Err(CommandError::MagnetTrackNotContiguous)
    );
    assert_eq!(editor.project(), &before);
    assert!(!editor.can_undo());
    assert!(editor.magnet_enabled());
}

#[test]
fn magnet_on_edits_reject_unverified_linked_content_atomically() {
    let mut project = Project::empty(rate()).expect("project");
    let main = project
        .add_track("V1", TrackKind::Video, false)
        .expect("main track");
    project.assign_main_track(main).expect("main");
    let a = project.add_clip(main, spec("A", 0, 5)).expect("A");
    let b = project.add_clip(main, spec("B", 5, 10)).expect("B");
    let text = project
        .add_track("T1", TrackKind::Video, false)
        .expect("text track");
    let links = BTreeSet::from([b]);
    let caption = project
        .add_clip(text, text_spec("Caption", 10, 15, links))
        .expect("caption");
    let mut editor = make_editor(project);
    editor.magnet_enabled = true;
    let before = editor.project().clone();

    assert_eq!(
        editor.execute(Command::DeleteClip { clip: a }),
        Err(CommandError::UnsupportedLinkedMedia)
    );
    assert_eq!(
        editor.execute(Command::DeleteClip { clip: b }),
        Err(CommandError::UnsupportedLinkedMedia)
    );
    assert_eq!(editor.project(), &before);
    assert_eq!(
        editor.project().clip(caption).expect("caption").range(),
        range(10, 15),
        "no unconditional upper-track shift is allowed"
    );
    assert!(!editor.can_undo());
}

#[test]
fn magnet_on_edits_reject_nested_suffix_content_atomically() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let a = project.add_clip(track, spec("A", 0, 5)).expect("A");
    project.add_clip(track, spec("B", 5, 10)).expect("B");
    project
        .add_clip(track, nested_spec("Nested", 10, 15))
        .expect("nested");
    let mut editor = make_editor(project);
    editor.magnet_enabled = true;
    let before = editor.project().clone();

    assert_eq!(
        editor.execute(Command::DeleteClip { clip: a }),
        Err(CommandError::UnsupportedNestedComposition)
    );
    assert_eq!(editor.project(), &before);
    assert!(!editor.can_undo());
}

#[test]
fn locked_linked_nested_and_non_main_targets_are_rejected() {
    let (project, _, _, _) = project_contiguous();
    let mut project = project;
    let main = project.main_track().expect("main").id();
    let text = project
        .add_track("T1", TrackKind::Video, false)
        .expect("text");
    let links = BTreeSet::from([ClipId::new(1)]);
    let caption = project
        .add_clip(text, text_spec("Caption", 10, 15, links))
        .expect("caption");
    let nested = project
        .add_clip(main, nested_spec("Nested", 15, 20))
        .expect("nested");
    let mut editor = make_editor(project);
    assert_eq!(
        editor.execute(Command::DeleteClip { clip: caption }),
        Err(CommandError::UnsupportedTrack(text))
    );
    assert_eq!(
        editor.execute(Command::DeleteClip { clip: nested }),
        Err(CommandError::UnsupportedNestedComposition)
    );
    assert_eq!(
        editor.execute(Command::DeleteClip {
            clip: ClipId::new(1)
        }),
        Err(CommandError::UnsupportedLinkedMedia)
    );

    let mut project = Project::empty(rate()).expect("project");
    let locked = project
        .add_track("V1", TrackKind::Video, true)
        .expect("track");
    project.assign_main_track(locked).expect("main");
    let clip = project.add_clip(locked, spec("A", 0, 5)).expect("clip");
    let mut editor = make_editor(project);
    assert_eq!(
        editor.execute(Command::DeleteClip { clip }),
        Err(CommandError::TrackLocked(locked))
    );
}

#[test]
fn split_works_under_magnet_without_respacing_neighbours() {
    let (project, a, b, c) = project_contiguous();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    assert!(editor.magnet_enabled());

    let outcome = editor
        .execute(Command::SplitClip {
            clip: b,
            at: ticks(7),
        })
        .expect("split");
    assert!(outcome.changed);
    let main = editor.project().main_track().expect("main");
    assert_eq!(main.clips().len(), 4);
    assert_eq!(main.clips()[0].id(), a);
    assert_eq!(main.clips()[1].id(), b);
    assert_eq!(main.clips()[1].range(), range(5, 7));
    assert_eq!(main.clips()[2].range(), range(7, 10));
    assert_eq!(main.clips()[3].id(), c);
    assert_eq!(main.clips()[3].range(), range(10, 15));
    assert!(editor.magnet_enabled());

    editor.undo().expect("undo split");
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        3
    );
    assert!(editor.magnet_enabled());
    editor.redo().expect("redo split");
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        4
    );
    assert!(editor.magnet_enabled());
}

#[test]
fn split_preserves_positions_ids_and_source_ranges() {
    let (project, a, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    let outcome = editor
        .execute(Command::SplitClip {
            clip: b,
            at: ticks(12),
        })
        .expect("split");
    assert!(outcome.changed);
    let main = editor.project().main_track().expect("main");
    assert_eq!(main.clips().len(), 4);
    assert_eq!(main.clips()[0].id(), a);
    let left = &main.clips()[1];
    let right = &main.clips()[2];
    assert_eq!(left.id(), b);
    assert_eq!(left.range(), range(10, 12));
    assert_eq!(right.range(), range(12, 15));
    assert_ne!(right.id(), b);
    assert_eq!(main.clips()[3].id(), c);
    assert_eq!(
        left.source().expect("left source").used(),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(240_000)).expect("left")
    );
    assert_eq!(
        right.source().expect("right source").used(),
        SourceRange::new(SourceTicks::new(240_000), SourceTicks::new(600_000)).expect("right")
    );
    assert_eq!(editor.project().content_end(), ticks(25));
}

#[test]
fn unsupported_commands_are_typed_without_mutation_or_history() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    let before = editor.project().clone();
    for command in [
        Command::InsertClip,
        Command::MoveClip {
            clip: b,
            to: ticks(30),
        },
        Command::TrimClipStart {
            clip: b,
            new_start: ticks(11),
        },
        Command::TrimClipExtension {
            clip: b,
            new_end: ticks(16),
        },
    ] {
        assert_eq!(editor.execute(command), Err(CommandError::Unsupported));
    }
    assert_eq!(editor.project(), &before);
    assert!(!editor.can_undo());
}

#[test]
fn trim_noop_and_unsupported_source_mapping_are_typed() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let mut source_less = spec("No source", 0, 5);
    source_less.source = None;
    let clip = project.add_clip(track, source_less).expect("clip");
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("contiguous preference");
    assert!(!editor.can_undo());

    let outcome = editor
        .execute(Command::TrimClipEnd {
            clip,
            new_end: ticks(5),
        })
        .expect("no-op trim");
    assert!(!outcome.changed);
    assert!(!editor.can_undo());
    assert_eq!(editor.project().revision(), 0);

    assert_eq!(
        editor.execute(Command::TrimClipEnd {
            clip,
            new_end: ticks(4),
        }),
        Err(CommandError::UnsupportedSourceMapping)
    );
    assert_eq!(
        editor.execute(Command::TrimClipEnd {
            clip,
            new_end: ticks(6),
        }),
        Err(CommandError::UnsupportedTrim)
    );
}

#[test]
fn trim_extension_after_magnet_activation_does_not_move_the_suffix() {
    let (project, _, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    let before = editor.project().clone();
    assert_eq!(
        editor.execute(Command::TrimClipEnd {
            clip: b,
            new_end: ticks(16),
        }),
        Err(CommandError::UnsupportedTrim)
    );
    assert_eq!(editor.project(), &before);
    assert_eq!(ranges(&editor)[1], (b, 600_000, 1_200_000));
    assert_eq!(ranges(&editor)[2], (c, 1_200_000, 1_800_000));
}

#[test]
fn undo_redo_restore_selection_positions_and_allocator() {
    let (project, a, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.select_clip(b);
    editor
        .execute(Command::DeleteClip { clip: b })
        .expect("delete");
    assert!(!editor.selection().contains(b));
    let after = editor.project().clone();
    editor.undo().expect("undo");
    assert_eq!(editor.project().content_end(), ticks(25));
    assert!(editor.selection().contains(b));
    editor.redo().expect("redo");
    assert_eq!(editor.project().main_track(), after.main_track());
    assert_eq!(editor.project().next_clip_id(), after.next_clip_id());
    assert!(!editor.selection().contains(b));
    assert_eq!(editor.project().revision(), 3);
    let _ = a;
}

#[test]
fn split_undo_redo_keeps_monotonic_allocator_and_selection() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.select_clip(b);
    editor
        .execute(Command::SplitClip {
            clip: b,
            at: ticks(12),
        })
        .expect("split");
    assert_eq!(editor.project().next_clip_id(), 5);
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        4
    );
    editor.undo().expect("undo");
    assert_eq!(
        editor.project().next_clip_id(),
        5,
        "undo must not roll back the monotonic allocator"
    );
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        3
    );
    assert!(editor.selection().contains(b));
    editor.redo().expect("redo");
    assert_eq!(editor.project().next_clip_id(), 5);
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        4
    );
}

#[test]
fn undo_then_new_split_never_reuses_a_discarded_id() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor
        .execute(Command::SplitClip {
            clip: b,
            at: ticks(12),
        })
        .expect("first split");
    assert_eq!(editor.project().next_clip_id(), 5);
    editor.undo().expect("undo first split");
    editor
        .execute(Command::SplitClip {
            clip: b,
            at: ticks(12),
        })
        .expect("second split");
    let main = editor.project().main_track().expect("main");
    assert_eq!(main.clips().len(), 4);
    assert_eq!(editor.project().next_clip_id(), 6);
    assert!(
        main.clips().iter().all(|clip| clip.id().get() != 4),
        "discarded branch ID must not be reused"
    );
    assert!(
        !editor.can_redo(),
        "a new branch must invalidate the discarded redo branch"
    );
}

#[test]
fn split_undo_then_different_edit_keeps_allocator_and_clears_redo() {
    let (project, _, b, c) = project_with_abc();
    let mut editor = make_editor(project);
    editor
        .execute(Command::SplitClip {
            clip: b,
            at: ticks(12),
        })
        .expect("split");
    editor.undo().expect("undo");
    editor
        .execute(Command::DeleteClip { clip: c })
        .expect("different edit");
    assert_eq!(editor.project().next_clip_id(), 5);
    assert!(!editor.can_redo());
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        2
    );
}

#[test]
fn failed_split_does_not_consume_an_id_or_create_history() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    let next_id = editor.project().next_clip_id();
    assert_eq!(
        editor.execute(Command::SplitClip {
            clip: b,
            at: ticks(10),
        }),
        Err(CommandError::InvalidSplitPoint)
    );
    assert_eq!(editor.project().next_clip_id(), next_id);
    assert!(!editor.can_undo());
    assert_eq!(
        editor.project().main_track().expect("main").clips().len(),
        3
    );
}

#[test]
fn history_budget_rejects_an_oversized_edit_before_mutation() {
    let (project, _, b, _) = project_with_abc();
    let before = project.clone();
    let mut editor = NativeEditor::with_history_budget(project, 1).expect("editor");
    assert!(matches!(
        editor.execute(Command::DeleteClip { clip: b }),
        Err(CommandError::HistoryBudgetExceeded { .. })
    ));
    assert_eq!(editor.project(), &before);
    assert_eq!(editor.project().next_clip_id(), before.next_clip_id());
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());
}

#[test]
fn history_budget_rejection_preserves_existing_redo_and_ids() {
    let (project, a, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    editor
        .execute(Command::DeleteClip { clip: b })
        .expect("delete");
    editor.undo().expect("undo");
    assert!(editor.can_redo());
    let before = editor.project().clone();
    editor.history_budget = 1;
    assert!(matches!(
        editor.execute(Command::DeleteClip { clip: a }),
        Err(CommandError::HistoryBudgetExceeded { .. })
    ));
    assert_eq!(editor.project(), &before);
    assert_eq!(editor.project().next_clip_id(), before.next_clip_id());
    assert!(editor.can_redo(), "rejected command must not clear redo");
}

#[test]
fn revision_overflow_rejects_without_mutation() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.project.revision = u64::MAX;
    let before = editor.project().clone();
    assert_eq!(
        editor.execute(Command::DeleteClip { clip: b }),
        Err(CommandError::RevisionOverflow)
    );
    assert_eq!(editor.project(), &before);
    assert!(!editor.can_undo());
}

#[test]
fn repeated_delete_undo_is_deterministic_under_magnet() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    let compacted = editor.project().main_track().cloned();
    for _ in 0..20 {
        editor
            .execute(Command::DeleteClip { clip: b })
            .expect("delete");
        editor.undo().expect("undo");
    }
    assert_eq!(editor.project().main_track(), compacted.as_ref());
    assert!(editor.magnet_enabled());
}

#[test]
fn failed_commands_do_not_consume_ids_or_clear_redo() {
    let (project, _, b, _) = project_with_abc();
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("enable");
    editor
        .execute(Command::DeleteClip { clip: b })
        .expect("delete");
    editor.undo().expect("undo");
    let next_id = editor.project().next_clip_id();
    let before = editor.project().clone();
    assert!(editor
        .execute(Command::DeleteClip {
            clip: ClipId::new(999)
        })
        .is_err());
    assert_eq!(editor.project(), &before);
    assert_eq!(editor.project().next_clip_id(), next_id);
    assert!(editor.can_redo());
}

#[test]
fn empty_and_single_clip_projects_behave_safely() {
    let mut editor = make_editor(Project::empty(rate()).expect("empty"));
    assert_eq!(
        editor.execute(Command::DeleteClip {
            clip: ClipId::new(1)
        }),
        Err(CommandError::MissingMainTrack)
    );

    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let clip = project.add_clip(track, spec("Only", 0, 5)).expect("clip");
    let mut editor = make_editor(project);
    editor.enable_magnet().expect("preference only");
    editor
        .execute(Command::DeleteClip { clip })
        .expect("delete");
    assert!(editor
        .project()
        .main_track()
        .expect("main")
        .clips()
        .is_empty());
    assert_eq!(editor.project().content_end(), TimelineTicks::ZERO);
    assert!(editor.magnet_enabled());
}

#[test]
fn add_append_split_relink_and_rename_keep_asset_identity() {
    let mut editor = NativeEditor::new(Project::empty(rate()).expect("project")).expect("editor");
    let outcome = editor
        .execute(Command::AddMediaAsset {
            asset: video_asset("A", 3),
        })
        .expect("add asset");
    let asset = outcome.created_asset.expect("created asset");
    assert_eq!(asset.get(), 1);
    let appended = editor
        .execute(Command::AppendMediaClip { asset })
        .expect("append");
    let clip = appended.affected[0];
    let project = editor.project();
    let main = project.main_track().expect("main");
    assert_eq!(main.name(), "V1");
    let native_clip = project.clip(clip).expect("clip");
    assert_eq!(native_clip.media_asset_id(), Some(asset));
    assert_eq!(native_clip.range(), range(0, 3));
    assert_eq!(
        native_clip.source().expect("source").used(),
        SourceRange::new(SourceTicks::ZERO, SourceTicks::new(360_000)).expect("used")
    );

    editor
        .execute(Command::SplitClip { clip, at: ticks(1) })
        .expect("split");
    let clips = editor.project().main_track().expect("main").clips();
    assert_eq!(clips.len(), 2);
    assert!(clips
        .iter()
        .all(|clip| clip.media_asset_id() == Some(asset)));

    let MediaAssetRegistration { metadata, .. } = video_asset("short", 1);
    let error = editor
        .execute(Command::RelinkMediaAsset {
            asset,
            locator: MediaLocator::absolute(r"C:\media\short.mp4").expect("locator"),
            metadata,
            file_size_bytes: None,
        })
        .expect_err("too short");
    assert!(matches!(error, CommandError::RelinkDurationTooShort { .. }));
    assert_eq!(
        editor
            .project()
            .asset(asset)
            .expect("asset")
            .locator()
            .value(),
        r"C:\media\A.mp4"
    );

    editor
        .execute(Command::RenameProject {
            name: "Renamed".to_owned(),
        })
        .expect("rename");
    assert_eq!(editor.project().name(), "Renamed");
    let next_asset = editor.project().next_asset_id();
    while editor.can_undo() {
        editor.undo().expect("undo");
    }
    assert!(editor.project().assets().is_empty());
    assert_eq!(editor.project().next_asset_id(), next_asset);
    while editor.can_redo() {
        editor.redo().expect("redo");
    }
    assert_eq!(editor.project().assets().len(), 1);
    assert_eq!(editor.project().name(), "Renamed");
}

#[test]
fn append_rejects_locked_or_ambiguous_main_tracks() {
    let mut project = Project::empty(rate()).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, true)
        .expect("track");
    project.assign_main_track(track).expect("main");
    let mut editor = NativeEditor::new(project).expect("editor");
    let asset = editor
        .execute(Command::AddMediaAsset {
            asset: video_asset("Locked", 1),
        })
        .expect("asset")
        .created_asset
        .expect("id");
    assert!(matches!(
        editor.execute(Command::AppendMediaClip { asset }),
        Err(CommandError::TrackLocked(_))
    ));

    let mut ambiguous = Project::empty(rate()).expect("project");
    ambiguous
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    let mut ambiguous = NativeEditor::new(ambiguous).expect("editor");
    let asset = ambiguous
        .execute(Command::AddMediaAsset {
            asset: video_asset("NoMain", 1),
        })
        .expect("asset")
        .created_asset
        .expect("id");
    assert!(matches!(
        ambiguous.execute(Command::AppendMediaClip { asset }),
        Err(CommandError::MissingMainTrack)
    ));
}

#[test]
fn unknown_duration_asset_registers_but_cannot_be_appended() {
    let mut editor = NativeEditor::new(Project::empty(rate()).expect("project")).expect("editor");
    let mut registration = video_asset("Unknown", 1);
    registration.metadata.duration_ticks = None;
    let asset = editor
        .execute(Command::AddMediaAsset {
            asset: registration,
        })
        .expect("asset")
        .created_asset
        .expect("id");
    assert!(matches!(
        editor.execute(Command::AppendMediaClip { asset }),
        Err(CommandError::InvalidAsset(_))
    ));
    assert!(editor.project().main_track().is_none());
}

#[test]
fn duplicate_asset_locator_is_rejected_without_consuming_an_id() {
    let mut editor = NativeEditor::new(Project::empty(rate()).expect("project")).expect("editor");
    editor
        .execute(Command::AddMediaAsset {
            asset: video_asset("A", 1),
        })
        .expect("first");
    let before = editor.project().next_asset_id();
    assert!(matches!(
        editor.execute(Command::AddMediaAsset {
            asset: video_asset("A", 2),
        }),
        Err(CommandError::DuplicateAssetLocator(_))
    ));
    assert_eq!(editor.project().next_asset_id(), before);
    assert_eq!(editor.project().assets().len(), 1);
}

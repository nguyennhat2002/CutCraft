//! Shared validation and ripple helpers for the limited Main Track Magnet.

use capcut_core::time::TimelineTicks;

use capcut_core::time::TimelineRange;

use super::{CommandError, CommandOutcome};
use crate::project::{ClipId, Project, Track};

pub(super) fn main_track_index(project: &Project, clip_id: ClipId) -> Result<usize, CommandError> {
    let main = project.main_track().ok_or(CommandError::MissingMainTrack)?;
    if main.locked() {
        return Err(CommandError::TrackLocked(main.id()));
    }
    let Some(index) = main.index_of(clip_id) else {
        if let Some(track) = project
            .tracks()
            .iter()
            .find(|track| track.index_of(clip_id).is_some())
        {
            return Err(CommandError::UnsupportedTrack(track.id()));
        }
        return Err(CommandError::UnknownClip(clip_id));
    };
    let clip = main.clip(index).expect("indexed clip");
    if clip.nested() {
        return Err(CommandError::UnsupportedNestedComposition);
    }
    if project.links_for(clip_id) {
        return Err(CommandError::UnsupportedLinkedMedia);
    }
    Ok(index)
}

/// Remove every positive gap on the explicit main video track while preserving
/// order, IDs, durations and source ranges. Linked or nested content is rejected
/// because no verified linked-media movement model exists in this milestone.
pub(super) fn normalize_main_track(project: &mut Project) -> Result<CommandOutcome, CommandError> {
    let main = project.main_track().ok_or(CommandError::MissingMainTrack)?;
    if main.locked() {
        return Err(CommandError::TrackLocked(main.id()));
    }
    for clip in main.clips() {
        if clip.nested() {
            return Err(CommandError::UnsupportedNestedComposition);
        }
        if project.links_for(clip.id()) {
            return Err(CommandError::UnsupportedLinkedMedia);
        }
    }

    let mut cursor = TimelineTicks::ZERO;
    let mut affected = Vec::new();
    let track = project
        .main_track_mut()
        .ok_or(CommandError::MissingMainTrack)?;
    for clip in &mut track.clips {
        let duration = clip.range().duration();
        let end = cursor
            .checked_add(duration)
            .ok_or(CommandError::ArithmeticOverflow)?;
        if clip.range().start() != cursor || clip.range().end() != end {
            let range =
                TimelineRange::new(cursor, end).map_err(|_| CommandError::ArithmeticOverflow)?;
            affected.push(clip.id());
            clip.set_range(range);
        }
        cursor = end;
    }
    Ok(CommandOutcome {
        changed: !affected.is_empty(),
        affected,
        created_asset: None,
    })
}

pub(super) fn ensure_main_track_is_contiguous(project: &Project) -> Result<(), CommandError> {
    let main = project.main_track().ok_or(CommandError::MissingMainTrack)?;
    let mut cursor = TimelineTicks::ZERO;
    for clip in main.clips() {
        if clip.range().start() != cursor {
            return Err(CommandError::MagnetTrackNotContiguous);
        }
        cursor = clip.range().end();
    }
    Ok(())
}

pub(super) fn ensure_suffix_is_safe(
    project: &Project,
    track: &Track,
    start_index: usize,
) -> Result<(), CommandError> {
    for clip in track.clips().iter().skip(start_index) {
        if clip.nested() {
            return Err(CommandError::UnsupportedNestedComposition);
        }
        if project.links_for(clip.id()) {
            return Err(CommandError::UnsupportedLinkedMedia);
        }
    }
    Ok(())
}

pub(super) fn shift_suffix_left(
    track: &mut Track,
    start_index: usize,
    delta: TimelineTicks,
) -> Result<(), CommandError> {
    for clip in track.clips.iter_mut().skip(start_index) {
        let shifted = clip
            .range()
            .shifted_left(delta)
            .map_err(|_| CommandError::ArithmeticOverflow)?;
        clip.set_range(shifted);
    }
    Ok(())
}

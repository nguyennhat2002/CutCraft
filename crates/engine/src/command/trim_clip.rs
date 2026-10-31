//! TrimClipEnd shortening command.

use capcut_core::time::{SourceRange, SourceTicks, TimelineRange, TimelineTicks};

use super::magnet::{
    ensure_main_track_is_contiguous, ensure_suffix_is_safe, main_track_index, shift_suffix_left,
};
use super::{CommandError, CommandOutcome};
use crate::project::{ClipId, Project};
use crate::selection::Selection;

pub(super) fn apply(
    project: &mut Project,
    _selection: &mut Selection,
    clip_id: ClipId,
    new_end: TimelineTicks,
    magnet: bool,
) -> Result<CommandOutcome, CommandError> {
    let index = main_track_index(project, clip_id)?;
    if magnet {
        ensure_main_track_is_contiguous(project)?;
    }
    let clip = project
        .main_track()
        .and_then(|track| track.clip(index))
        .ok_or(CommandError::UnknownClip(clip_id))?;
    let old_range = clip.range();
    if new_end == old_range.end() {
        return Ok(CommandOutcome {
            changed: false,
            affected: Vec::new(),
            created_asset: None,
        });
    }
    if new_end > old_range.end() {
        return Err(CommandError::UnsupportedTrim);
    }
    if new_end <= old_range.start() {
        return Err(CommandError::InvalidRange);
    }

    let delta = old_range
        .end()
        .checked_sub(new_end)
        .ok_or(CommandError::ArithmeticOverflow)?;
    let mut source = clip
        .source()
        .cloned()
        .ok_or(CommandError::UnsupportedSourceMapping)?;
    let new_source_end = source
        .used()
        .end()
        .checked_sub(SourceTicks::new(delta.get()))
        .ok_or(CommandError::ArithmeticOverflow)?;
    let new_used = SourceRange::new(source.used().start(), new_source_end)
        .map_err(|_| CommandError::InvalidRange)?;
    source
        .set_used(new_used)
        .map_err(|_| CommandError::InvalidRange)?;
    let new_range =
        TimelineRange::new(old_range.start(), new_end).map_err(|_| CommandError::InvalidRange)?;

    if magnet {
        let track = project.main_track().expect("main track was validated");
        ensure_suffix_is_safe(project, track, index + 1)?;
    }

    let track = project
        .main_track_mut()
        .ok_or(CommandError::MissingMainTrack)?;
    let clip = track.clips.get_mut(index).expect("indexed clip");
    clip.set_range(new_range);
    clip.set_source(Some(source));
    if magnet {
        shift_suffix_left(track, index + 1, delta)?;
    }

    Ok(CommandOutcome {
        changed: true,
        affected: vec![clip_id],
        created_asset: None,
    })
}

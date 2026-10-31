//! DeleteClip command and its limited Magnet ripple.

use super::magnet::{
    ensure_main_track_is_contiguous, ensure_suffix_is_safe, main_track_index, shift_suffix_left,
};
use super::{CommandError, CommandOutcome};
use crate::project::{ClipId, Project};
use crate::selection::Selection;

pub(super) fn apply(
    project: &mut Project,
    selection: &mut Selection,
    clip_id: ClipId,
    magnet: bool,
) -> Result<CommandOutcome, CommandError> {
    let index = main_track_index(project, clip_id)?;
    if magnet {
        ensure_main_track_is_contiguous(project)?;
    }

    let mut affected = Vec::new();
    if magnet {
        let track = project.main_track().expect("main track was validated");
        ensure_suffix_is_safe(project, track, index + 1)?;
        affected.extend(track.clips().iter().skip(index + 1).map(|clip| clip.id()));
    }
    affected.push(clip_id);

    let duration = project
        .main_track()
        .and_then(|track| track.clip(index))
        .map(|clip| clip.range().duration())
        .ok_or(CommandError::UnknownClip(clip_id))?;

    let track = project
        .main_track_mut()
        .ok_or(CommandError::MissingMainTrack)?;
    track.clips.remove(index);
    selection.deselect(clip_id);

    if magnet {
        shift_suffix_left(track, index, duration)?;
    }

    Ok(CommandOutcome {
        changed: true,
        affected,
        created_asset: None,
    })
}

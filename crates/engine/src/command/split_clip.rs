//! SplitClip command. Positions are preserved; no Magnet ripple is applied.

use std::collections::BTreeSet;

use capcut_core::time::{SourceRange, SourceTicks, TimelineRange, TimelineTicks};

use super::magnet::main_track_index;
use super::{CommandError, CommandOutcome};
use crate::project::{Clip, ClipId, ClipSpec, Project, SourceMapping};
use crate::selection::Selection;

pub(super) fn apply(
    project: &mut Project,
    _selection: &mut Selection,
    clip_id: ClipId,
    at: TimelineTicks,
) -> Result<CommandOutcome, CommandError> {
    let index = main_track_index(project, clip_id)?;
    let clip = project
        .main_track()
        .and_then(|track| track.clip(index))
        .cloned()
        .ok_or(CommandError::UnknownClip(clip_id))?;
    let old_range = clip.range();
    if at <= old_range.start() || at >= old_range.end() {
        return Err(CommandError::InvalidSplitPoint);
    }
    let source = clip
        .source()
        .cloned()
        .ok_or(CommandError::UnsupportedSourceMapping)?;
    let offset = at
        .checked_sub(old_range.start())
        .ok_or(CommandError::ArithmeticOverflow)?;
    let source_split = source
        .used()
        .start()
        .checked_add(SourceTicks::new(offset.get()))
        .ok_or(CommandError::ArithmeticOverflow)?;
    if source_split <= source.used().start() || source_split >= source.used().end() {
        return Err(CommandError::UnsupportedSourceMapping);
    }

    let left_range =
        TimelineRange::new(old_range.start(), at).map_err(|_| CommandError::InvalidSplitPoint)?;
    let right_range =
        TimelineRange::new(at, old_range.end()).map_err(|_| CommandError::InvalidSplitPoint)?;
    let left_used = SourceRange::new(source.used().start(), source_split)
        .map_err(|_| CommandError::UnsupportedSourceMapping)?;
    let right_used = SourceRange::new(source_split, source.used().end())
        .map_err(|_| CommandError::UnsupportedSourceMapping)?;

    let mut left_source = source.clone();
    left_source
        .set_used(left_used)
        .map_err(|_| CommandError::UnsupportedSourceMapping)?;
    let (speed_numerator, speed_denominator) = source.speed();
    let right_source = SourceMapping::new(
        source.available(),
        right_used,
        source.timebase(),
        speed_numerator,
        speed_denominator,
    )
    .map_err(|_| CommandError::UnsupportedSourceMapping)?;

    let next_id = project
        .next_clip_id
        .checked_add(1)
        .ok_or(CommandError::IdOverflow)?;
    let right_id = ClipId::new(project.next_clip_id);
    let right_clip = Clip::from_spec(
        right_id,
        ClipSpec {
            name: clip.name().to_owned(),
            kind: clip.kind(),
            range: right_range,
            source: Some(right_source),
            media_asset_id: clip.media_asset_id(),
            linked_clip_ids: BTreeSet::new(),
            nested: false,
        },
    )
    .map_err(CommandError::InvalidProject)?;

    project.next_clip_id = next_id;
    let track = project
        .main_track_mut()
        .ok_or(CommandError::MissingMainTrack)?;
    let left = track.clips.get_mut(index).expect("indexed clip");
    left.set_range(left_range);
    left.set_source(Some(left_source));
    track.clips.insert(index + 1, right_clip);

    Ok(CommandOutcome {
        changed: true,
        affected: vec![clip_id, right_id],
        created_asset: None,
    })
}

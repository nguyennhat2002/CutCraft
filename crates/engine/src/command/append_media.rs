//! AppendMediaClip: place a registered video asset at the end of the main track.

use std::collections::BTreeSet;

use capcut_core::time::{
    SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks, TIMELINE_TICKS_PER_SECOND,
};

use crate::asset::{MediaAssetId, MediaAssetKind};
use crate::project::{ClipKind, ClipSpec, Project, SourceMapping, TrackId, TrackKind};

use super::{CommandError, CommandOutcome};

pub(super) fn apply(
    project: &mut Project,
    asset_id: MediaAssetId,
) -> Result<CommandOutcome, CommandError> {
    let (duration, name) = {
        let asset = project
            .asset(asset_id)
            .ok_or(CommandError::UnknownAsset(asset_id))?;
        if asset.kind() != MediaAssetKind::Video {
            return Err(CommandError::UnsupportedAssetKind);
        }
        let duration = asset
            .metadata()
            .duration_source_ticks()
            .map_err(|error| CommandError::InvalidAsset(error.to_string()))?;
        (duration, asset.display_name().to_owned())
    };
    let track_id = ensure_main_track(project)?;
    let track = project
        .track(track_id)
        .ok_or(CommandError::MissingMainTrack)?;
    if track.locked() {
        return Err(CommandError::TrackLocked(track_id));
    }
    let start = track
        .clips()
        .last()
        .map(|clip| clip.range().end())
        .unwrap_or(TimelineTicks::ZERO);
    let end = start
        .checked_add(TimelineTicks::new(duration.get()))
        .ok_or(CommandError::ArithmeticOverflow)?;
    let range = TimelineRange::new(start, end).map_err(|_| CommandError::InvalidRange)?;
    let available = SourceRange::new(SourceTicks::ZERO, duration)
        .map_err(|_| CommandError::InvalidAsset("source range is empty".to_owned()))?;
    let source = SourceMapping::new(
        available,
        available,
        Timebase::new(TIMELINE_TICKS_PER_SECOND)
            .map_err(|_| CommandError::UnsupportedSourceMapping)?,
        1,
        1,
    )
    .map_err(|_| CommandError::UnsupportedSourceMapping)?;
    let clip = project
        .add_clip(
            track_id,
            ClipSpec {
                name,
                kind: ClipKind::Video,
                range,
                source: Some(source),
                media_asset_id: Some(asset_id),
                linked_clip_ids: BTreeSet::new(),
                nested: false,
            },
        )
        .map_err(CommandError::InvalidProject)?;
    Ok(CommandOutcome {
        changed: true,
        affected: vec![clip],
        created_asset: None,
    })
}

fn ensure_main_track(project: &mut Project) -> Result<TrackId, CommandError> {
    if let Some(track) = project.main_video_track() {
        return Ok(track);
    }
    if !project.tracks().is_empty() {
        return Err(CommandError::MissingMainTrack);
    }
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .map_err(CommandError::InvalidProject)?;
    project
        .assign_main_track(track)
        .map_err(CommandError::InvalidProject)?;
    Ok(track)
}

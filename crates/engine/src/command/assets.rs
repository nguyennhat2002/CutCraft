//! Persistent media-asset registration, relink and project rename commands.

use crate::asset::{MediaAssetId, MediaAssetMetadata, MediaAssetRegistration, MediaLocator};
use crate::project::{Project, ProjectError};

use super::{CommandError, CommandOutcome};

pub(super) fn add(
    project: &mut Project,
    registration: MediaAssetRegistration,
) -> Result<CommandOutcome, CommandError> {
    let asset = project.add_asset(registration).map_err(map_project_error)?;
    Ok(CommandOutcome {
        changed: true,
        affected: Vec::new(),
        created_asset: Some(asset),
    })
}

pub(super) fn relink(
    project: &mut Project,
    asset_id: MediaAssetId,
    locator: MediaLocator,
    metadata: MediaAssetMetadata,
    file_size_bytes: Option<u64>,
) -> Result<CommandOutcome, CommandError> {
    let current = project
        .asset(asset_id)
        .ok_or(CommandError::UnknownAsset(asset_id))?;
    if project
        .assets()
        .iter()
        .any(|asset| asset.id() != asset_id && asset.locator() == &locator)
    {
        return Err(CommandError::DuplicateAssetLocator(
            locator.value().to_owned(),
        ));
    }
    metadata
        .validate()
        .map_err(|error| CommandError::InvalidAsset(error.to_string()))?;
    if file_size_bytes == Some(0) {
        return Err(CommandError::InvalidAsset(
            "source file size must be nonzero".to_owned(),
        ));
    }
    let required_end = project
        .tracks()
        .iter()
        .flat_map(|track| track.clips())
        .filter(|clip| clip.media_asset_id() == Some(asset_id))
        .filter_map(|clip| clip.source())
        .map(|source| {
            source
                .available()
                .end()
                .get()
                .max(source.used().end().get())
        })
        .max();
    if let Some(required_end) = required_end {
        let duration = metadata
            .duration_source_ticks()
            .map_err(|error| CommandError::InvalidAsset(error.to_string()))?
            .get();
        if duration < required_end {
            return Err(CommandError::RelinkDurationTooShort {
                required_ticks: required_end,
                actual_ticks: duration,
            });
        }
    }
    let changed = current.locator() != &locator
        || current.metadata() != &metadata
        || current.file_size_bytes() != file_size_bytes;
    if !changed {
        return Ok(CommandOutcome {
            changed: false,
            affected: Vec::new(),
            created_asset: None,
        });
    }
    project
        .asset_mut(asset_id)
        .ok_or(CommandError::UnknownAsset(asset_id))?
        .relink(locator, metadata, file_size_bytes)
        .map_err(|error| CommandError::InvalidAsset(error.to_string()))?;
    Ok(CommandOutcome {
        changed: true,
        affected: Vec::new(),
        created_asset: None,
    })
}

pub(super) fn rename(project: &mut Project, name: String) -> Result<CommandOutcome, CommandError> {
    let changed = project.name() != name;
    if changed {
        project.rename(name).map_err(map_project_error)?;
    }
    Ok(CommandOutcome {
        changed,
        affected: Vec::new(),
        created_asset: None,
    })
}

fn map_project_error(error: ProjectError) -> CommandError {
    match error {
        ProjectError::TooManyMediaAssets => CommandError::TooManyAssets,
        ProjectError::DuplicateAssetLocator(locator) => {
            CommandError::DuplicateAssetLocator(locator)
        }
        other => CommandError::InvalidProject(other),
    }
}

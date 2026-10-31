//! Native edit commands, transaction runner and Magnet policy.

pub mod add_draft;
pub mod append_media;
pub mod assets;
pub mod delete_clip;
pub mod history;
pub mod magnet;
pub mod split_clip;
pub mod trait_cmd;
pub mod trim_clip;

use std::fmt;

use capcut_core::time::TimelineTicks;

use crate::asset::{MediaAssetId, MediaAssetMetadata, MediaAssetRegistration, MediaLocator};
use crate::project::{ClipId, Project, ProjectError, TrackId};
use crate::selection::Selection;

use self::history::{EditHistory, StateSnapshot, Transaction, MAX_HISTORY_BYTES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    DeleteClip {
        clip: ClipId,
    },
    TrimClipEnd {
        clip: ClipId,
        new_end: TimelineTicks,
    },
    SplitClip {
        clip: ClipId,
        at: TimelineTicks,
    },
    NormalizeMainTrack,
    AddMediaAsset {
        asset: MediaAssetRegistration,
    },
    RelinkMediaAsset {
        asset: MediaAssetId,
        locator: MediaLocator,
        metadata: MediaAssetMetadata,
        file_size_bytes: Option<u64>,
    },
    AppendMediaClip {
        asset: MediaAssetId,
    },
    RenameProject {
        name: String,
    },
    InsertClip,
    MoveClip {
        clip: ClipId,
        to: TimelineTicks,
    },
    TrimClipStart {
        clip: ClipId,
        new_start: TimelineTicks,
    },
    TrimClipExtension {
        clip: ClipId,
        new_end: TimelineTicks,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome {
    pub changed: bool,
    pub affected: Vec<ClipId>,
    pub created_asset: Option<MediaAssetId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEditor {
    project: Project,
    selection: Selection,
    history: EditHistory,
    history_budget: usize,
    magnet_enabled: bool,
}

impl NativeEditor {
    pub fn new(project: Project) -> Result<Self, CommandError> {
        project.validate().map_err(CommandError::InvalidProject)?;
        Ok(Self {
            project,
            selection: Selection::new(),
            history: EditHistory::new(),
            history_budget: MAX_HISTORY_BYTES,
            magnet_enabled: false,
        })
    }

    /// Restore a persisted document. History starts empty; Magnet ON requires
    /// an existing, exactly contiguous main track. Loading never normalizes.
    pub fn from_restored(project: Project, magnet_enabled: bool) -> Result<Self, CommandError> {
        project.validate().map_err(CommandError::InvalidProject)?;
        let editor = Self {
            project,
            selection: Selection::new(),
            history: EditHistory::new(),
            history_budget: MAX_HISTORY_BYTES,
            magnet_enabled,
        };
        editor.validate_magnet_state()?;
        Ok(editor)
    }

    /// Validate the engine-owned Magnet preference against the project layout.
    pub fn validate_magnet_state(&self) -> Result<(), CommandError> {
        if self.magnet_enabled {
            if self.project.main_track().is_none() {
                return Err(CommandError::MissingMainTrack);
            }
            magnet::ensure_main_track_is_contiguous(&self.project)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn with_history_budget(
        project: Project,
        history_budget: usize,
    ) -> Result<Self, CommandError> {
        let mut editor = Self::new(project)?;
        editor.history_budget = history_budget;
        Ok(editor)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn select_clip(&mut self, clip: ClipId) {
        self.selection.select(clip);
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn magnet_enabled(&self) -> bool {
        self.magnet_enabled
    }

    /// Immutable project content suitable for a background save. History,
    /// selection and transient UI state are deliberately excluded.
    pub fn snapshot(&self) -> crate::project::ProjectSnapshot {
        crate::project::ProjectSnapshot {
            project: self.project.clone(),
            magnet_enabled: self.magnet_enabled,
        }
    }

    /// Enable magnetic main-track layout. Existing gaps are normalized in one
    /// transaction; an already-contiguous track only changes the preference.
    pub fn enable_magnet(&mut self) -> Result<CommandOutcome, CommandError> {
        if self.magnet_enabled {
            return Ok(CommandOutcome {
                changed: false,
                affected: Vec::new(),
                created_asset: None,
            });
        }
        self.execute(Command::NormalizeMainTrack)
    }

    /// Disable the preference without changing project positions.
    pub fn disable_magnet(&mut self) {
        self.magnet_enabled = false;
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Validate a command against a clone without mutating editor state.
    pub fn validate_command(&self, command: Command) -> Result<(), CommandError> {
        let mut project = self.project.clone();
        let mut selection = self.selection.clone();
        let magnet = self.magnet_enabled;
        let outcome = match command {
            Command::DeleteClip { clip } => {
                delete_clip::apply(&mut project, &mut selection, clip, magnet)?
            }
            Command::TrimClipEnd { clip, new_end } => {
                trim_clip::apply(&mut project, &mut selection, clip, new_end, magnet)?
            }
            Command::SplitClip { clip, at } => {
                split_clip::apply(&mut project, &mut selection, clip, at)?
            }
            Command::NormalizeMainTrack => magnet::normalize_main_track(&mut project)?,
            Command::AddMediaAsset { asset } => assets::add(&mut project, asset)?,
            Command::RelinkMediaAsset {
                asset,
                locator,
                metadata,
                file_size_bytes,
            } => assets::relink(&mut project, asset, locator, metadata, file_size_bytes)?,
            Command::AppendMediaClip { asset } => append_media::apply(&mut project, asset)?,
            Command::RenameProject { name } => assets::rename(&mut project, name)?,
            Command::InsertClip
            | Command::MoveClip { .. }
            | Command::TrimClipStart { .. }
            | Command::TrimClipExtension { .. } => return Err(CommandError::Unsupported),
        };
        if outcome.changed {
            project.validate().map_err(CommandError::InvalidProject)?;
        }
        Ok(())
    }

    /// Run a command against the editor's own Magnet preference. Magnet state
    /// is not a caller-provided parameter, so Undo/Redo cannot desynchronize it
    /// from the project snapshot it belongs to.
    pub fn execute(&mut self, command: Command) -> Result<CommandOutcome, CommandError> {
        let before_project = self.project.clone();
        let before_selection = self.selection.clone();
        let before_magnet = self.magnet_enabled;
        let magnet = self.magnet_enabled;
        let is_normalize = matches!(&command, Command::NormalizeMainTrack);
        let after_magnet = if is_normalize { true } else { before_magnet };
        let mut project = before_project.clone();
        let mut selection = before_selection.clone();
        let outcome = match command {
            Command::DeleteClip { clip } => {
                delete_clip::apply(&mut project, &mut selection, clip, magnet)?
            }
            Command::TrimClipEnd { clip, new_end } => {
                trim_clip::apply(&mut project, &mut selection, clip, new_end, magnet)?
            }
            Command::SplitClip { clip, at } => {
                split_clip::apply(&mut project, &mut selection, clip, at)?
            }
            Command::NormalizeMainTrack => magnet::normalize_main_track(&mut project)?,
            Command::AddMediaAsset { asset } => assets::add(&mut project, asset)?,
            Command::RelinkMediaAsset {
                asset,
                locator,
                metadata,
                file_size_bytes,
            } => assets::relink(&mut project, asset, locator, metadata, file_size_bytes)?,
            Command::AppendMediaClip { asset } => append_media::apply(&mut project, asset)?,
            Command::RenameProject { name } => assets::rename(&mut project, name)?,
            Command::InsertClip
            | Command::MoveClip { .. }
            | Command::TrimClipStart { .. }
            | Command::TrimClipExtension { .. } => return Err(CommandError::Unsupported),
        };
        if !outcome.changed {
            if is_normalize {
                self.magnet_enabled = true;
            }
            return Ok(outcome);
        }
        project.validate().map_err(CommandError::InvalidProject)?;
        let revision = project
            .revision
            .checked_add(1)
            .ok_or(CommandError::RevisionOverflow)?;
        project.revision = revision;
        let transaction = Transaction::new(
            StateSnapshot {
                project: before_project,
                selection: before_selection,
                magnet_enabled: before_magnet,
            },
            StateSnapshot {
                project: project.clone(),
                selection: selection.clone(),
                magnet_enabled: after_magnet,
            },
        );
        let transaction_bytes = transaction.estimated_bytes;
        if transaction_bytes > self.history_budget {
            return Err(CommandError::HistoryBudgetExceeded {
                transaction_bytes,
                budget: self.history_budget,
            });
        }
        if !self.history.record(transaction, self.history_budget) {
            return Err(CommandError::HistoryBudgetExceeded {
                transaction_bytes,
                budget: self.history_budget,
            });
        }
        self.project = project;
        self.selection = selection;
        self.magnet_enabled = after_magnet;
        Ok(outcome)
    }

    pub fn undo(&mut self) -> Result<(), CommandError> {
        let revision = self
            .project
            .revision
            .checked_add(1)
            .ok_or(CommandError::RevisionOverflow)?;
        let transaction = self.history.pop_undo().ok_or(CommandError::NothingToUndo)?;
        let mut project = transaction.before.project.clone();
        project.next_clip_id = project.next_clip_id.max(self.project.next_clip_id);
        project.next_track_id = project.next_track_id.max(self.project.next_track_id);
        project.next_asset_id = project.next_asset_id.max(self.project.next_asset_id);
        project.revision = revision;
        if let Err(error) = project.validate() {
            self.history.push_undo(transaction);
            return Err(CommandError::InvalidProject(error));
        }
        self.project = project;
        self.selection = transaction.before.selection.clone();
        self.magnet_enabled = transaction.before.magnet_enabled;
        self.history.push_redo(transaction);
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), CommandError> {
        let revision = self
            .project
            .revision
            .checked_add(1)
            .ok_or(CommandError::RevisionOverflow)?;
        let transaction = self.history.pop_redo().ok_or(CommandError::NothingToRedo)?;
        let mut project = transaction.after.project.clone();
        project.next_clip_id = project.next_clip_id.max(self.project.next_clip_id);
        project.next_track_id = project.next_track_id.max(self.project.next_track_id);
        project.next_asset_id = project.next_asset_id.max(self.project.next_asset_id);
        project.revision = revision;
        if let Err(error) = project.validate() {
            self.history.push_redo(transaction);
            return Err(CommandError::InvalidProject(error));
        }
        self.project = project;
        self.selection = transaction.after.selection.clone();
        self.magnet_enabled = transaction.after.magnet_enabled;
        self.history.push_undo(transaction);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    MissingMainTrack,
    UnsupportedTrack(TrackId),
    UnknownClip(ClipId),
    TrackLocked(TrackId),
    UnsupportedLinkedMedia,
    UnsupportedNestedComposition,
    MagnetTrackNotContiguous,
    UnsupportedTrim,
    UnsupportedSourceMapping,
    UnknownAsset(MediaAssetId),
    UnsupportedAssetKind,
    TooManyAssets,
    DuplicateAssetLocator(String),
    InvalidAsset(String),
    RelinkDurationTooShort {
        required_ticks: i64,
        actual_ticks: i64,
    },
    InvalidRange,
    InvalidSplitPoint,
    ArithmeticOverflow,
    RevisionOverflow,
    IdOverflow,
    HistoryBudgetExceeded {
        transaction_bytes: usize,
        budget: usize,
    },
    InvalidProject(ProjectError),
    Unsupported,
    NothingToUndo,
    NothingToRedo,
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMainTrack => formatter.write_str("project has no main video track"),
            Self::UnsupportedTrack(id) => {
                write!(formatter, "track {} is not the main video track", id.get())
            }
            Self::UnknownClip(id) => write!(formatter, "unknown clip {}", id.get()),
            Self::TrackLocked(id) => write!(formatter, "track {} is locked", id.get()),
            Self::UnsupportedLinkedMedia => {
                formatter.write_str("linked media cannot be ripple-edited in this milestone")
            }
            Self::UnsupportedNestedComposition => {
                formatter.write_str("nested compositions cannot be edited in this milestone")
            }
            Self::MagnetTrackNotContiguous => {
                formatter.write_str("main track is not contiguous; enable Magnet to normalize it")
            }
            Self::UnsupportedTrim => formatter.write_str("trim operation is not supported"),
            Self::UnsupportedSourceMapping => {
                formatter.write_str("source mapping is not supported")
            }
            Self::UnknownAsset(id) => write!(formatter, "unknown media asset {}", id.get()),
            Self::UnsupportedAssetKind => {
                formatter.write_str("media asset kind is not supported for video clips")
            }
            Self::TooManyAssets => formatter.write_str("too many media assets"),
            Self::DuplicateAssetLocator(locator) => {
                write!(formatter, "media locator is already registered: {locator}")
            }
            Self::InvalidAsset(message) => write!(formatter, "invalid media asset: {message}"),
            Self::RelinkDurationTooShort {
                required_ticks,
                actual_ticks,
            } => write!(
                formatter,
                "relinked media is too short: clips require {required_ticks} source ticks, file provides {actual_ticks}"
            ),
            Self::InvalidRange => formatter.write_str("invalid edit range"),
            Self::InvalidSplitPoint => formatter.write_str("split point is not inside the clip"),
            Self::ArithmeticOverflow => formatter.write_str("edit time arithmetic overflow"),
            Self::RevisionOverflow => formatter.write_str("project revision overflow"),
            Self::IdOverflow => formatter.write_str("clip ID allocation overflow"),
            Self::HistoryBudgetExceeded {
                transaction_bytes,
                budget,
            } => write!(
                formatter,
                "history budget exceeded: transaction {transaction_bytes} bytes, budget {budget}"
            ),
            Self::InvalidProject(error) => write!(formatter, "project invariant failed: {error}"),
            Self::Unsupported => formatter.write_str("command is not supported"),
            Self::NothingToUndo => formatter.write_str("nothing to undo"),
            Self::NothingToRedo => formatter.write_str("nothing to redo"),
        }
    }
}

impl std::error::Error for CommandError {}

#[cfg(test)]
mod tests;

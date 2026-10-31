//! Typed failures for native project open and save.

use std::fmt;
use std::path::PathBuf;

use capcut_engine::command::CommandError;
use capcut_engine::project::ProjectError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectOpenError {
    InvalidExtension(PathBuf),
    NotAFile(PathBuf),
    Io { path: PathBuf, message: String },
    TooLarge { size: u64, limit: u64 },
    JsonDepthExceeded { limit: usize },
    InvalidJson(String),
    UnsupportedFormat(String),
    UnsupportedSchemaVersion(u32),
    UnsupportedTimebase(i64),
    InvalidFps(String),
    InvalidProjectId(String),
    InvalidField(&'static str),
    InvalidSource(String),
    InvalidAsset(String),
    Project(ProjectError),
    Engine(CommandError),
}

impl fmt::Display for ProjectOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExtension(path) => write!(
                formatter,
                "{} is not a .cutcraft project file",
                path.display()
            ),
            Self::NotAFile(path) => write!(formatter, "{} is not a file", path.display()),
            Self::Io { path, message } => {
                write!(formatter, "could not read {}: {message}", path.display())
            }
            Self::TooLarge { size, limit } => write!(
                formatter,
                "project file is too large ({size} bytes; limit {limit})"
            ),
            Self::JsonDepthExceeded { limit } => {
                write!(formatter, "project JSON nesting exceeds {limit} levels")
            }
            Self::InvalidJson(message) => write!(formatter, "invalid project file: {message}"),
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported project format {format:?}")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported project schema version {version}")
            }
            Self::UnsupportedTimebase(timebase) => {
                write!(formatter, "unsupported project timebase {timebase}")
            }
            Self::InvalidFps(message) => write!(formatter, "invalid project frame rate: {message}"),
            Self::InvalidProjectId(message) => {
                write!(formatter, "invalid project identity: {message}")
            }
            Self::InvalidField(field) => write!(formatter, "invalid project field {field}"),
            Self::InvalidSource(message) => {
                write!(formatter, "invalid source mapping: {message}")
            }
            Self::InvalidAsset(message) => {
                write!(formatter, "invalid media asset: {message}")
            }
            Self::Project(error) => write!(formatter, "invalid project: {error}"),
            Self::Engine(error) => write!(formatter, "invalid Magnet state: {error}"),
        }
    }
}

impl std::error::Error for ProjectOpenError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSaveError {
    InvalidExtension(PathBuf),
    NotAFile(PathBuf),
    InvalidProject(ProjectError),
    InvalidMagnet(CommandError),
    InvalidSnapshot(String),
    TargetExists(PathBuf),
    ExternalModified(PathBuf),
    ExternalMissing(PathBuf),
    Serialize(String),
    TooLarge { size: usize, limit: u64 },
    Io { path: PathBuf, message: String },
    StagedVerification(String),
    CompareFailed,
}

impl fmt::Display for ProjectSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExtension(path) => write!(
                formatter,
                "save target must end in .cutcraft: {}",
                path.display()
            ),
            Self::NotAFile(path) => write!(formatter, "{} is not a file", path.display()),
            Self::InvalidProject(error) => write!(formatter, "invalid project: {error}"),
            Self::InvalidMagnet(error) => write!(formatter, "invalid Magnet state: {error}"),
            Self::InvalidSnapshot(message) => {
                write!(formatter, "invalid project snapshot: {message}")
            }
            Self::TargetExists(path) => write!(
                formatter,
                "{} already exists and is not the active project file",
                path.display()
            ),
            Self::ExternalModified(path) => write!(
                formatter,
                "{} changed outside CutCraft; save was not applied",
                path.display()
            ),
            Self::ExternalMissing(path) => write!(
                formatter,
                "{} was removed outside CutCraft; save was not applied",
                path.display()
            ),
            Self::Serialize(message) => {
                write!(formatter, "could not serialize project: {message}")
            }
            Self::TooLarge { size, limit } => write!(
                formatter,
                "serialized project is too large ({size} bytes; limit {limit})"
            ),
            Self::Io { path, message } => {
                write!(formatter, "could not write {}: {message}", path.display())
            }
            Self::StagedVerification(message) => {
                write!(formatter, "staged project failed verification: {message}")
            }
            Self::CompareFailed => {
                formatter.write_str("staged project differs from the active project")
            }
        }
    }
}

impl std::error::Error for ProjectSaveError {}

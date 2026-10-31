//! Stable document identity for native projects.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_PROJECT_ID_BYTES: usize = 64;
const MIN_PROJECT_ID_BYTES: usize = 8;
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    /// Generate a process-unique identity from wall-clock nanoseconds, process
    /// ID and a per-process counter. This is a document identifier, not a
    /// security token.
    pub fn generate() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(format!(
            "p{nanos:032x}{:08x}{counter:016x}",
            std::process::id()
        ))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, ProjectIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ProjectIdError::Empty);
        }
        if value.len() > MAX_PROJECT_ID_BYTES {
            return Err(ProjectIdError::TooLong(value.len()));
        }
        if value.len() < MIN_PROJECT_ID_BYTES {
            return Err(ProjectIdError::TooShort(value.len()));
        }
        if let Some(character) = value.chars().find(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        }) {
            return Err(ProjectIdError::InvalidCharacter(character));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectIdError {
    Empty,
    TooShort(usize),
    TooLong(usize),
    InvalidCharacter(char),
}

impl fmt::Display for ProjectIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("project ID is empty"),
            Self::TooShort(length) => {
                write!(formatter, "project ID is too short ({length} bytes)")
            }
            Self::TooLong(length) => {
                write!(formatter, "project ID is too long ({length} bytes)")
            }
            Self::InvalidCharacter(character) => {
                write!(
                    formatter,
                    "project ID contains invalid character {character:?}"
                )
            }
        }
    }
}

impl std::error::Error for ProjectIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_valid_unique_and_stable_strings() {
        let first = ProjectId::generate();
        let second = ProjectId::generate();
        assert_ne!(first, second);
        assert!(first.as_str().len() >= MIN_PROJECT_ID_BYTES);
        assert!(first
            .as_str()
            .chars()
            .all(|character| character.is_ascii_alphanumeric()));
        assert_eq!(ProjectId::parse(first.to_string()), Ok(first));
    }

    #[test]
    fn invalid_ids_are_rejected() {
        assert_eq!(ProjectId::parse(""), Err(ProjectIdError::Empty));
        assert_eq!(ProjectId::parse("short"), Err(ProjectIdError::TooShort(5)));
        assert_eq!(
            ProjectId::parse("a".repeat(MAX_PROJECT_ID_BYTES + 1)),
            Err(ProjectIdError::TooLong(MAX_PROJECT_ID_BYTES + 1))
        );
        assert_eq!(
            ProjectId::parse("valid-id-123"),
            Ok(ProjectId("valid-id-123".to_owned()))
        );
        assert_eq!(
            ProjectId::parse("invalid id 123"),
            Err(ProjectIdError::InvalidCharacter(' '))
        );
    }
}

//! Persistent media-asset value types and validation.
//!
//! Asset metadata describes the decoder stream (timebase, start, duration),
//! while clip source mappings remain in the 120,000-tick native source domain.

use std::fmt;

use capcut_core::time::{SourceTicks, TIMELINE_TICKS_PER_SECOND};

pub const MAX_MEDIA_ASSETS: usize = 4096;
pub const MAX_ASSET_LABEL_BYTES: usize = 256;
pub const MAX_LOCATOR_BYTES: usize = 4096;
pub const MAX_CONTAINER_BYTES: usize = 128;
pub const MAX_CODEC_BYTES: usize = 64;
pub const MAX_PIXEL_FORMAT_BYTES: usize = 64;
pub const MAX_ASSET_DIMENSION: u32 = 8192;
pub const MAX_ASSET_PIXELS: u64 = 33_554_432;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaAssetId(u64);

impl MediaAssetId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAssetKind {
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MediaLocatorKind {
    ProjectRelative,
    Absolute,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaLocator {
    kind: MediaLocatorKind,
    value: String,
}

impl MediaLocator {
    pub fn project_relative(value: impl Into<String>) -> Result<Self, MediaAssetError> {
        let value = value.into();
        validate_relative_locator(&value)?;
        Ok(Self {
            kind: MediaLocatorKind::ProjectRelative,
            value,
        })
    }

    pub fn absolute(value: impl Into<String>) -> Result<Self, MediaAssetError> {
        let value = value.into();
        validate_text(&value, MAX_LOCATOR_BYTES, "media locator")?;
        if !looks_absolute(&value) {
            return Err(MediaAssetError::InvalidLocator(
                "absolute locator is not an absolute path".to_owned(),
            ));
        }
        Ok(Self {
            kind: MediaLocatorKind::Absolute,
            value,
        })
    }

    pub const fn kind(&self) -> MediaLocatorKind {
        self.kind
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaRational {
    pub numerator: u32,
    pub denominator: u32,
}

impl MediaRational {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        Some(Self {
            numerator,
            denominator,
        })
    }

    pub fn as_f64(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }
}

impl fmt::Display for MediaRational {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.numerator, self.denominator)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAssetMetadata {
    pub container: String,
    pub video_stream_index: u32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub pixel_format: Option<String>,
    pub timebase_numerator: u32,
    pub timebase_denominator: u32,
    pub stream_start_ticks: i64,
    pub duration_ticks: Option<i64>,
    pub nominal_frame_rate: Option<MediaRational>,
    pub average_frame_rate: Option<MediaRational>,
    pub rotation_degrees: u16,
    pub has_audio: bool,
    pub frame_count_hint: Option<u64>,
}

impl MediaAssetMetadata {
    pub fn validate(&self) -> Result<(), MediaAssetError> {
        validate_text(&self.container, MAX_CONTAINER_BYTES, "container")?;
        validate_text(&self.codec, MAX_CODEC_BYTES, "codec")?;
        if let Some(pixel_format) = &self.pixel_format {
            validate_text(pixel_format, MAX_PIXEL_FORMAT_BYTES, "pixel format")?;
        }
        if self.width == 0
            || self.height == 0
            || self.width > MAX_ASSET_DIMENSION
            || self.height > MAX_ASSET_DIMENSION
            || u64::from(self.width) * u64::from(self.height) > MAX_ASSET_PIXELS
        {
            return Err(MediaAssetError::InvalidMetadata(
                "invalid video dimensions".to_owned(),
            ));
        }
        if self.timebase_numerator == 0 || self.timebase_denominator == 0 {
            return Err(MediaAssetError::InvalidMetadata(
                "stream timebase must be nonzero".to_owned(),
            ));
        }
        if self.duration_ticks.is_some_and(|ticks| ticks <= 0) {
            return Err(MediaAssetError::InvalidMetadata(
                "media duration must be positive".to_owned(),
            ));
        }
        if !matches!(self.rotation_degrees, 0 | 90 | 180 | 270) {
            return Err(MediaAssetError::InvalidMetadata(
                "rotation must be 0, 90, 180 or 270 degrees".to_owned(),
            ));
        }
        Ok(())
    }

    /// Convert the stream duration into the native 120,000-tick source domain.
    /// This quantization happens once; callers persist the resulting clip range.
    pub fn duration_source_ticks(&self) -> Result<SourceTicks, MediaAssetError> {
        let duration = self
            .duration_ticks
            .ok_or(MediaAssetError::UnknownDuration)?;
        if duration <= 0 {
            return Err(MediaAssetError::InvalidMetadata(
                "media duration must be positive".to_owned(),
            ));
        }
        if self.timebase_numerator == 0 || self.timebase_denominator == 0 {
            return Err(MediaAssetError::InvalidMetadata(
                "stream timebase must be nonzero".to_owned(),
            ));
        }
        let numerator = i128::from(duration)
            .checked_mul(i128::from(self.timebase_numerator))
            .and_then(|value| value.checked_mul(i128::from(TIMELINE_TICKS_PER_SECOND)))
            .ok_or(MediaAssetError::DurationOverflow)?;
        let denominator = i128::from(self.timebase_denominator);
        if denominator <= 0 {
            return Err(MediaAssetError::InvalidMetadata(
                "stream timebase denominator must be positive".to_owned(),
            ));
        }
        let quotient = numerator.div_euclid(denominator);
        let remainder = numerator.rem_euclid(denominator);
        let rounded = quotient + i128::from(remainder * 2 >= denominator);
        if rounded <= 0 {
            return Err(MediaAssetError::InvalidMetadata(
                "media duration rounds to zero native ticks".to_owned(),
            ));
        }
        let ticks = i64::try_from(rounded).map_err(|_| MediaAssetError::DurationOverflow)?;
        Ok(SourceTicks::new(ticks))
    }

    pub fn display_dimensions(&self) -> (u32, u32) {
        if self.rotation_degrees % 180 == 90 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAssetRegistration {
    pub kind: MediaAssetKind,
    pub display_name: String,
    pub locator: MediaLocator,
    pub metadata: MediaAssetMetadata,
    pub file_size_bytes: Option<u64>,
}

impl MediaAssetRegistration {
    pub fn video(
        display_name: impl Into<String>,
        locator: MediaLocator,
        metadata: MediaAssetMetadata,
    ) -> Result<Self, MediaAssetError> {
        let registration = Self {
            kind: MediaAssetKind::Video,
            display_name: display_name.into(),
            locator,
            metadata,
            file_size_bytes: None,
        };
        registration.validate()?;
        Ok(registration)
    }

    pub fn validate(&self) -> Result<(), MediaAssetError> {
        validate_text(&self.display_name, MAX_ASSET_LABEL_BYTES, "asset name")?;
        self.metadata.validate()?;
        if self.file_size_bytes == Some(0) {
            return Err(MediaAssetError::InvalidMetadata(
                "source file size must be nonzero".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAssetParts {
    pub id: u64,
    pub kind: MediaAssetKind,
    pub display_name: String,
    pub locator: MediaLocator,
    pub metadata: MediaAssetMetadata,
    pub file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAsset {
    id: MediaAssetId,
    kind: MediaAssetKind,
    display_name: String,
    locator: MediaLocator,
    metadata: MediaAssetMetadata,
    file_size_bytes: Option<u64>,
}

impl MediaAsset {
    pub(crate) fn from_registration(
        id: MediaAssetId,
        registration: MediaAssetRegistration,
    ) -> Result<Self, MediaAssetError> {
        registration.validate()?;
        Ok(Self {
            id,
            kind: registration.kind,
            display_name: registration.display_name,
            locator: registration.locator,
            metadata: registration.metadata,
            file_size_bytes: registration.file_size_bytes,
        })
    }

    pub(crate) fn from_parts(parts: MediaAssetParts) -> Result<Self, MediaAssetError> {
        validate_text(&parts.display_name, MAX_ASSET_LABEL_BYTES, "asset name")?;
        parts.metadata.validate()?;
        if parts.id == 0 {
            return Err(MediaAssetError::InvalidMetadata(
                "asset ID must be nonzero".to_owned(),
            ));
        }
        if parts.file_size_bytes == Some(0) {
            return Err(MediaAssetError::InvalidMetadata(
                "source file size must be nonzero".to_owned(),
            ));
        }
        Ok(Self {
            id: MediaAssetId::new(parts.id),
            kind: parts.kind,
            display_name: parts.display_name,
            locator: parts.locator,
            metadata: parts.metadata,
            file_size_bytes: parts.file_size_bytes,
        })
    }

    pub const fn id(&self) -> MediaAssetId {
        self.id
    }

    pub const fn kind(&self) -> MediaAssetKind {
        self.kind
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn locator(&self) -> &MediaLocator {
        &self.locator
    }

    pub fn metadata(&self) -> &MediaAssetMetadata {
        &self.metadata
    }

    pub fn file_size_bytes(&self) -> Option<u64> {
        self.file_size_bytes
    }

    pub(crate) fn relink(
        &mut self,
        locator: MediaLocator,
        metadata: MediaAssetMetadata,
        file_size_bytes: Option<u64>,
    ) -> Result<(), MediaAssetError> {
        metadata.validate()?;
        if file_size_bytes == Some(0) {
            return Err(MediaAssetError::InvalidMetadata(
                "source file size must be nonzero".to_owned(),
            ));
        }
        self.locator = locator;
        self.metadata = metadata;
        self.file_size_bytes = file_size_bytes;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaAssetError {
    InvalidLocator(String),
    InvalidLabel(String),
    InvalidMetadata(String),
    UnknownDuration,
    DurationOverflow,
}

impl fmt::Display for MediaAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocator(message) => write!(formatter, "invalid media locator: {message}"),
            Self::InvalidLabel(message) => write!(formatter, "invalid media asset name: {message}"),
            Self::InvalidMetadata(message) => {
                write!(formatter, "invalid media asset metadata: {message}")
            }
            Self::UnknownDuration => formatter.write_str("media duration is unavailable"),
            Self::DurationOverflow => formatter.write_str("media duration conversion overflowed"),
        }
    }
}

impl std::error::Error for MediaAssetError {}

fn validate_text(value: &str, limit: usize, description: &str) -> Result<(), MediaAssetError> {
    if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        return Err(MediaAssetError::InvalidLabel(format!(
            "{description} must be 1..={limit} bytes without control characters"
        )));
    }
    Ok(())
}

fn validate_relative_locator(value: &str) -> Result<(), MediaAssetError> {
    validate_text(value, MAX_LOCATOR_BYTES, "project-relative locator")?;
    if looks_absolute(value) || value.starts_with('/') || value.starts_with('\\') {
        return Err(MediaAssetError::InvalidLocator(
            "project-relative locator must not be absolute".to_owned(),
        ));
    }
    for component in value.split(['/', '\\']) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(MediaAssetError::InvalidLocator(
                "project-relative locator contains an unsafe path component".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn looks_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with('/')
        || value.starts_with("\\\\")
        || value.starts_with("//")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> MediaAssetMetadata {
        MediaAssetMetadata {
            container: "mov,mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase_numerator: 1,
            timebase_denominator: 15_360,
            stream_start_ticks: 0,
            duration_ticks: Some(8 * 15_360),
            nominal_frame_rate: MediaRational::new(30, 1),
            average_frame_rate: MediaRational::new(30, 1),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: Some(240),
        }
    }

    #[test]
    fn relative_locator_rejects_traversal_and_absolute_forms() {
        for bad in [
            "",
            "../x.mp4",
            "a/../b.mp4",
            "a//b.mp4",
            "\\server\\share\\x.mp4",
            "C:\\x.mp4",
            "/x.mp4",
            ".\\x.mp4",
        ] {
            assert!(MediaLocator::project_relative(bad).is_err(), "{bad}");
        }
        assert!(MediaLocator::project_relative("media/clip.mp4").is_ok());
        assert!(MediaLocator::absolute("C:\\Media\\clip.mp4").is_ok());
        assert!(MediaLocator::absolute("\\\\server\\share\\clip.mp4").is_ok());
        assert!(MediaLocator::absolute("media/clip.mp4").is_err());
    }

    #[test]
    fn duration_conversion_is_checked_and_rounded_once() {
        let mut metadata = metadata();
        metadata.timebase_numerator = 1001;
        metadata.timebase_denominator = 30_000;
        metadata.duration_ticks = Some(90);
        assert_eq!(
            metadata.duration_source_ticks().expect("duration"),
            SourceTicks::new(360_360)
        );
        metadata.duration_ticks = None;
        assert_eq!(
            metadata.duration_source_ticks(),
            Err(MediaAssetError::UnknownDuration)
        );
    }

    #[test]
    fn metadata_validation_rejects_invalid_dimensions_timebase_and_rotation() {
        let mut value = metadata();
        value.width = 0;
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.timebase_denominator = 0;
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.rotation_degrees = 45;
        assert!(value.validate().is_err());
    }

    #[test]
    fn display_dimensions_follow_quarter_turn_rotation() {
        let mut value = metadata();
        assert_eq!(value.display_dimensions(), (1920, 1080));
        value.rotation_degrees = 90;
        assert_eq!(value.display_dimensions(), (1080, 1920));
    }
}

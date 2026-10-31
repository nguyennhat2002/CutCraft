//! Versioned `.cutcraft` JSON schema and engine conversion.

use std::collections::BTreeSet;

use capcut_core::time::{
    FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks,
    TIMELINE_TICKS_PER_SECOND,
};
use capcut_engine::asset::{
    MediaAssetId, MediaAssetKind, MediaAssetMetadata, MediaAssetParts, MediaLocator, MediaRational,
};
use capcut_engine::command::NativeEditor;
use capcut_engine::project::{
    ClipId, ClipKind, ClipParts, Project, ProjectParts, ProjectSnapshot, SourceMapping, TrackId,
    TrackKind, TrackParts,
};
use serde::{Deserialize, Serialize};

use crate::error::{ProjectOpenError, ProjectSaveError};
use crate::id::ProjectId;

pub const FORMAT_MARKER: &str = "cutcraft-project";
pub const SCHEMA_VERSION: u32 = 2;
pub const MIN_SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_FILE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_JSON_NESTING_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDocumentMetadata {
    pub project_id: ProjectId,
    pub created_at_unix_ms: u64,
    pub save_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedProjectFile {
    pub project_id: ProjectId,
    pub editor: NativeEditor,
    pub schema_version: u32,
    pub created_at_unix_ms: u64,
    pub last_saved_unix_ms: u64,
    pub save_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedProjectFile {
    V1(ProjectFileV1),
    V2(ProjectFileV2),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectFileV1 {
    pub format: String,
    pub schema_version: u32,
    pub project_id: String,
    pub name: String,
    pub fps: FpsFile,
    pub timebase: i64,
    pub main_video_track: Option<u64>,
    pub next_track_id: u64,
    pub next_clip_id: u64,
    pub magnet_enabled: bool,
    pub tracks: Vec<TrackFileV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectFileV2 {
    pub format: String,
    pub schema_version: u32,
    pub project_id: String,
    pub name: String,
    pub fps: FpsFile,
    pub timebase: i64,
    pub main_video_track: Option<u64>,
    pub next_track_id: u64,
    pub next_clip_id: u64,
    pub next_asset_id: u64,
    pub magnet_enabled: bool,
    pub created_at_unix_ms: u64,
    pub last_saved_unix_ms: u64,
    pub save_generation: u64,
    pub assets: Vec<MediaAssetFile>,
    pub tracks: Vec<TrackFileV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FpsFile {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrackFileV1 {
    pub id: u64,
    pub name: String,
    pub kind: TrackKindFile,
    pub locked: bool,
    pub clips: Vec<ClipFileV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrackFileV2 {
    pub id: u64,
    pub name: String,
    pub kind: TrackKindFile,
    pub locked: bool,
    pub clips: Vec<ClipFileV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClipFileV1 {
    pub id: u64,
    pub name: String,
    pub kind: ClipKindFile,
    pub start: i64,
    pub end: i64,
    pub source: Option<SourceFile>,
    pub linked_clip_ids: Vec<u64>,
    pub nested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClipFileV2 {
    pub id: u64,
    pub name: String,
    pub kind: ClipKindFile,
    pub start: i64,
    pub end: i64,
    pub source: Option<SourceFile>,
    pub media_asset_id: Option<u64>,
    pub linked_clip_ids: Vec<u64>,
    pub nested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceFile {
    pub available_start: i64,
    pub available_end: i64,
    pub used_start: i64,
    pub used_end: i64,
    pub timebase: i64,
    pub speed_numerator: u32,
    pub speed_denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MediaAssetFile {
    pub id: u64,
    pub kind: MediaAssetKindFile,
    pub display_name: String,
    pub locator: LocatorFile,
    pub metadata: MediaAssetMetadataFile,
    pub file_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocatorFile {
    pub kind: LocatorKindFile,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MediaAssetMetadataFile {
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
    pub nominal_frame_rate: Option<RationalFile>,
    pub average_frame_rate: Option<RationalFile>,
    pub rotation_degrees: u16,
    pub has_audio: bool,
    pub frame_count_hint: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RationalFile {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrackKindFile {
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClipKindFile {
    Video,
    Audio,
    Subtitle,
    Effect,
    NestedComposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MediaAssetKindFile {
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocatorKindFile {
    ProjectRelative,
    Absolute,
}

#[derive(Deserialize)]
struct FileHeader {
    format: String,
    schema_version: u32,
}

impl ParsedProjectFile {
    pub(crate) fn into_opened(
        self,
        fallback_created_at_unix_ms: u64,
        fallback_last_saved_unix_ms: u64,
    ) -> Result<OpenedProjectFile, ProjectOpenError> {
        let (file, schema_version) = match self {
            Self::V1(file) => (
                file.into_v2(fallback_created_at_unix_ms, fallback_last_saved_unix_ms),
                1,
            ),
            Self::V2(file) => (file, 2),
        };
        let created_at_unix_ms = file.created_at_unix_ms;
        let last_saved_unix_ms = file.last_saved_unix_ms;
        let save_generation = file.save_generation;
        let opened = file.into_opened()?;
        Ok(OpenedProjectFile {
            project_id: opened.0,
            editor: opened.1,
            schema_version,
            created_at_unix_ms,
            last_saved_unix_ms,
            save_generation,
        })
    }
}

impl ProjectFileV1 {
    fn into_v2(self, created_at_unix_ms: u64, last_saved_unix_ms: u64) -> ProjectFileV2 {
        ProjectFileV2 {
            format: self.format,
            schema_version: 2,
            project_id: self.project_id,
            name: self.name,
            fps: self.fps,
            timebase: self.timebase,
            main_video_track: self.main_video_track,
            next_track_id: self.next_track_id,
            next_clip_id: self.next_clip_id,
            next_asset_id: 1,
            magnet_enabled: self.magnet_enabled,
            created_at_unix_ms,
            last_saved_unix_ms,
            save_generation: 0,
            assets: Vec::new(),
            tracks: self
                .tracks
                .into_iter()
                .map(|track| TrackFileV2 {
                    id: track.id,
                    name: track.name,
                    kind: track.kind,
                    locked: track.locked,
                    clips: track
                        .clips
                        .into_iter()
                        .map(|clip| ClipFileV2 {
                            id: clip.id,
                            name: clip.name,
                            kind: clip.kind,
                            start: clip.start,
                            end: clip.end,
                            source: clip.source,
                            media_asset_id: None,
                            linked_clip_ids: clip.linked_clip_ids,
                            nested: clip.nested,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl ProjectFileV2 {
    pub(crate) fn from_snapshot(
        snapshot: &ProjectSnapshot,
        metadata: &ProjectDocumentMetadata,
        last_saved_unix_ms: u64,
    ) -> Self {
        let project = &snapshot.project;
        Self {
            format: FORMAT_MARKER.to_owned(),
            schema_version: SCHEMA_VERSION,
            project_id: metadata.project_id.to_string(),
            name: project.name().to_owned(),
            fps: FpsFile {
                numerator: project.fps().numerator,
                denominator: project.fps().denominator,
            },
            timebase: TIMELINE_TICKS_PER_SECOND,
            main_video_track: project.main_video_track().map(TrackId::get),
            next_track_id: project.next_track_id(),
            next_clip_id: project.next_clip_id(),
            next_asset_id: project.next_asset_id(),
            magnet_enabled: snapshot.magnet_enabled,
            created_at_unix_ms: metadata.created_at_unix_ms,
            last_saved_unix_ms,
            save_generation: metadata.save_generation,
            assets: project
                .assets()
                .iter()
                .map(MediaAssetFile::from_asset)
                .collect(),
            tracks: project
                .tracks()
                .iter()
                .map(|track| TrackFileV2 {
                    id: track.id().get(),
                    name: track.name().to_owned(),
                    kind: match track.kind() {
                        TrackKind::Video => TrackKindFile::Video,
                        TrackKind::Audio => TrackKindFile::Audio,
                    },
                    locked: track.locked(),
                    clips: track
                        .clips()
                        .iter()
                        .map(|clip| ClipFileV2 {
                            id: clip.id().get(),
                            name: clip.name().to_owned(),
                            kind: match clip.kind() {
                                ClipKind::Video => ClipKindFile::Video,
                                ClipKind::Audio => ClipKindFile::Audio,
                                ClipKind::Subtitle => ClipKindFile::Subtitle,
                                ClipKind::Effect => ClipKindFile::Effect,
                                ClipKind::NestedComposition => ClipKindFile::NestedComposition,
                            },
                            start: clip.range().start().get(),
                            end: clip.range().end().get(),
                            source: clip.source().map(SourceFile::from_mapping),
                            media_asset_id: clip.media_asset_id().map(MediaAssetId::get),
                            linked_clip_ids: clip
                                .linked_clip_ids()
                                .iter()
                                .map(|id| id.get())
                                .collect(),
                            nested: clip.nested(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub(crate) fn into_opened(self) -> Result<(ProjectId, NativeEditor), ProjectOpenError> {
        let project_id = ProjectId::parse(self.project_id)
            .map_err(|error| ProjectOpenError::InvalidProjectId(error.to_string()))?;
        if self.timebase != TIMELINE_TICKS_PER_SECOND {
            return Err(ProjectOpenError::UnsupportedTimebase(self.timebase));
        }
        let fps = FrameRate::new(self.fps.numerator, self.fps.denominator)
            .map_err(|error| ProjectOpenError::InvalidFps(error.to_string()))?;
        let mut assets = Vec::with_capacity(self.assets.len());
        for asset in self.assets {
            assets.push(asset.into_parts()?);
        }
        let mut tracks = Vec::with_capacity(self.tracks.len());
        for track in self.tracks {
            let mut clips = Vec::with_capacity(track.clips.len());
            for clip in track.clips {
                clips.push(ClipParts {
                    id: ClipId::new(clip.id),
                    name: clip.name,
                    kind: match clip.kind {
                        ClipKindFile::Video => ClipKind::Video,
                        ClipKindFile::Audio => ClipKind::Audio,
                        ClipKindFile::Subtitle => ClipKind::Subtitle,
                        ClipKindFile::Effect => ClipKind::Effect,
                        ClipKindFile::NestedComposition => ClipKind::NestedComposition,
                    },
                    range: TimelineRange::new(
                        TimelineTicks::new(clip.start),
                        TimelineTicks::new(clip.end),
                    )
                    .map_err(|_| ProjectOpenError::InvalidField("clip range"))?,
                    source: clip.source.map(SourceFile::into_mapping).transpose()?,
                    media_asset_id: clip.media_asset_id.map(MediaAssetId::new),
                    linked_clip_ids: clip
                        .linked_clip_ids
                        .into_iter()
                        .map(ClipId::new)
                        .collect::<BTreeSet<_>>(),
                    nested: clip.nested,
                });
            }
            tracks.push(TrackParts {
                id: TrackId::new(track.id),
                name: track.name,
                kind: match track.kind {
                    TrackKindFile::Video => TrackKind::Video,
                    TrackKindFile::Audio => TrackKind::Audio,
                },
                locked: track.locked,
                clips,
            });
        }
        let project = Project::from_parts(ProjectParts {
            name: self.name,
            fps,
            main_video_track: self.main_video_track.map(TrackId::new),
            tracks,
            assets,
            next_track_id: self.next_track_id,
            next_clip_id: self.next_clip_id,
            next_asset_id: self.next_asset_id,
        })
        .map_err(ProjectOpenError::Project)?;
        let editor = NativeEditor::from_restored(project, self.magnet_enabled)
            .map_err(ProjectOpenError::Engine)?;
        Ok((project_id, editor))
    }
}

impl MediaAssetFile {
    fn from_asset(asset: &capcut_engine::asset::MediaAsset) -> Self {
        Self {
            id: asset.id().get(),
            kind: match asset.kind() {
                MediaAssetKind::Video => MediaAssetKindFile::Video,
            },
            display_name: asset.display_name().to_owned(),
            locator: LocatorFile::from_locator(asset.locator()),
            metadata: MediaAssetMetadataFile::from_metadata(asset.metadata()),
            file_size_bytes: asset.file_size_bytes(),
        }
    }

    fn into_parts(self) -> Result<MediaAssetParts, ProjectOpenError> {
        Ok(MediaAssetParts {
            id: self.id,
            kind: match self.kind {
                MediaAssetKindFile::Video => MediaAssetKind::Video,
            },
            display_name: self.display_name,
            locator: self.locator.into_locator()?,
            metadata: self.metadata.into_metadata()?,
            file_size_bytes: self.file_size_bytes,
        })
    }
}

impl LocatorFile {
    fn from_locator(locator: &MediaLocator) -> Self {
        Self {
            kind: match locator.kind() {
                capcut_engine::asset::MediaLocatorKind::ProjectRelative => {
                    LocatorKindFile::ProjectRelative
                }
                capcut_engine::asset::MediaLocatorKind::Absolute => LocatorKindFile::Absolute,
            },
            value: locator.value().to_owned(),
        }
    }

    fn into_locator(self) -> Result<MediaLocator, ProjectOpenError> {
        match self.kind {
            LocatorKindFile::ProjectRelative => MediaLocator::project_relative(self.value)
                .map_err(|error| ProjectOpenError::InvalidAsset(error.to_string())),
            LocatorKindFile::Absolute => MediaLocator::absolute(self.value)
                .map_err(|error| ProjectOpenError::InvalidAsset(error.to_string())),
        }
    }
}

impl MediaAssetMetadataFile {
    fn from_metadata(metadata: &MediaAssetMetadata) -> Self {
        Self {
            container: metadata.container.clone(),
            video_stream_index: metadata.video_stream_index,
            codec: metadata.codec.clone(),
            width: metadata.width,
            height: metadata.height,
            pixel_format: metadata.pixel_format.clone(),
            timebase_numerator: metadata.timebase_numerator,
            timebase_denominator: metadata.timebase_denominator,
            stream_start_ticks: metadata.stream_start_ticks,
            duration_ticks: metadata.duration_ticks,
            nominal_frame_rate: metadata.nominal_frame_rate.map(RationalFile::from),
            average_frame_rate: metadata.average_frame_rate.map(RationalFile::from),
            rotation_degrees: metadata.rotation_degrees,
            has_audio: metadata.has_audio,
            frame_count_hint: metadata.frame_count_hint,
        }
    }

    fn into_metadata(self) -> Result<MediaAssetMetadata, ProjectOpenError> {
        let metadata = MediaAssetMetadata {
            container: self.container,
            video_stream_index: self.video_stream_index,
            codec: self.codec,
            width: self.width,
            height: self.height,
            pixel_format: self.pixel_format,
            timebase_numerator: self.timebase_numerator,
            timebase_denominator: self.timebase_denominator,
            stream_start_ticks: self.stream_start_ticks,
            duration_ticks: self.duration_ticks,
            nominal_frame_rate: self.nominal_frame_rate.map(RationalFile::into_rational),
            average_frame_rate: self.average_frame_rate.map(RationalFile::into_rational),
            rotation_degrees: self.rotation_degrees,
            has_audio: self.has_audio,
            frame_count_hint: self.frame_count_hint,
        };
        metadata
            .validate()
            .map_err(|error| ProjectOpenError::InvalidAsset(error.to_string()))?;
        Ok(metadata)
    }
}

impl RationalFile {
    fn from(value: MediaRational) -> Self {
        Self {
            numerator: value.numerator,
            denominator: value.denominator,
        }
    }

    fn into_rational(self) -> MediaRational {
        MediaRational {
            numerator: self.numerator,
            denominator: self.denominator,
        }
    }
}

impl SourceFile {
    fn from_mapping(source: &SourceMapping) -> Self {
        Self {
            available_start: source.available().start().get(),
            available_end: source.available().end().get(),
            used_start: source.used().start().get(),
            used_end: source.used().end().get(),
            timebase: source.timebase().ticks_per_second,
            speed_numerator: source.speed().0,
            speed_denominator: source.speed().1,
        }
    }

    fn into_mapping(self) -> Result<SourceMapping, ProjectOpenError> {
        let available = SourceRange::new(
            SourceTicks::new(self.available_start),
            SourceTicks::new(self.available_end),
        )
        .map_err(|error| ProjectOpenError::InvalidSource(error.to_string()))?;
        let used = SourceRange::new(
            SourceTicks::new(self.used_start),
            SourceTicks::new(self.used_end),
        )
        .map_err(|error| ProjectOpenError::InvalidSource(error.to_string()))?;
        let timebase = Timebase::new(self.timebase)
            .map_err(|error| ProjectOpenError::InvalidSource(error.to_string()))?;
        SourceMapping::new(
            available,
            used,
            timebase,
            self.speed_numerator,
            self.speed_denominator,
        )
        .map_err(|error| ProjectOpenError::InvalidSource(error.to_string()))
    }
}

/// Parse one project document after bounded reading. Depth and version are
/// checked before full strict deserialization.
pub(crate) fn parse_project_bytes(bytes: &[u8]) -> Result<ParsedProjectFile, ProjectOpenError> {
    if !json_depth_within(bytes, MAX_JSON_NESTING_DEPTH) {
        return Err(ProjectOpenError::JsonDepthExceeded {
            limit: MAX_JSON_NESTING_DEPTH,
        });
    }
    let header: FileHeader = serde_json::from_slice(bytes)
        .map_err(|error| ProjectOpenError::InvalidJson(error.to_string()))?;
    if header.format != FORMAT_MARKER {
        return Err(ProjectOpenError::UnsupportedFormat(header.format));
    }
    match header.schema_version {
        1 => serde_json::from_slice(bytes)
            .map(ParsedProjectFile::V1)
            .map_err(|error| ProjectOpenError::InvalidJson(error.to_string())),
        2 => serde_json::from_slice(bytes)
            .map(ParsedProjectFile::V2)
            .map_err(|error| ProjectOpenError::InvalidJson(error.to_string())),
        version => Err(ProjectOpenError::UnsupportedSchemaVersion(version)),
    }
}

pub(crate) fn serialize_project(file: &ProjectFileV2) -> Result<Vec<u8>, ProjectSaveError> {
    serde_json::to_vec_pretty(file).map_err(|error| ProjectSaveError::Serialize(error.to_string()))
}

pub(crate) fn parse_staged_v2(bytes: &[u8]) -> Result<ProjectFileV2, ProjectSaveError> {
    match parse_project_bytes(bytes) {
        Ok(ParsedProjectFile::V2(file)) => Ok(file),
        Ok(ParsedProjectFile::V1(_)) => Err(ProjectSaveError::StagedVerification(
            "staged project unexpectedly used schema v1".to_owned(),
        )),
        Err(error) => Err(ProjectSaveError::StagedVerification(error.to_string())),
    }
}

pub(crate) fn json_depth_within(bytes: &[u8], limit: usize) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > limit {
                    return false;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ProjectId;
    use capcut_core::time::{FrameRate, SourceRange as CoreSourceRange, SourceTicks, Timebase};

    fn source(duration: i64) -> SourceMapping {
        SourceMapping::new(
            CoreSourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("available"),
            CoreSourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration)).expect("used"),
            Timebase::new(TIMELINE_TICKS_PER_SECOND).expect("timebase"),
            1,
            1,
        )
        .expect("source")
    }

    fn sample_project() -> Project {
        let mut project =
            Project::named(FrameRate::new(30, 1).expect("fps"), "Sample").expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        project
            .add_clip(
                track,
                capcut_engine::project::ClipSpec {
                    name: "A".to_owned(),
                    kind: ClipKind::Video,
                    range: TimelineRange::new(TimelineTicks::new(4000), TimelineTicks::new(12_000))
                        .expect("range"),
                    source: Some(source(8000)),
                    media_asset_id: None,
                    linked_clip_ids: BTreeSet::new(),
                    nested: false,
                },
            )
            .expect("clip");
        project
    }

    fn snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            project: sample_project(),
            magnet_enabled: false,
        }
    }

    fn encoded_v2(project_id: &ProjectId) -> Vec<u8> {
        let file = ProjectFileV2::from_snapshot(
            &snapshot(),
            &ProjectDocumentMetadata {
                project_id: project_id.clone(),
                created_at_unix_ms: 1,
                save_generation: 3,
            },
            2,
        );
        serde_json::to_vec(&file).expect("json")
    }

    fn encoded_v1(project_id: &ProjectId) -> Vec<u8> {
        let v1 = ProjectFileV1 {
            format: FORMAT_MARKER.to_owned(),
            schema_version: 1,
            project_id: project_id.to_string(),
            name: "Legacy".to_owned(),
            fps: FpsFile {
                numerator: 30,
                denominator: 1,
            },
            timebase: TIMELINE_TICKS_PER_SECOND,
            main_video_track: None,
            next_track_id: 1,
            next_clip_id: 1,
            magnet_enabled: false,
            tracks: Vec::new(),
        };
        serde_json::to_vec(&v1).expect("json")
    }

    #[test]
    fn schema_v2_round_trip_preserves_exact_parts_and_metadata() {
        let id = ProjectId::generate();
        let bytes = encoded_v2(&id);
        let opened = parse_project_bytes(&bytes)
            .expect("parse")
            .into_opened(10, 20)
            .expect("opened");
        assert_eq!(opened.project_id, id);
        assert_eq!(opened.schema_version, 2);
        assert_eq!(opened.created_at_unix_ms, 1);
        assert_eq!(opened.last_saved_unix_ms, 2);
        assert_eq!(opened.save_generation, 3);
        assert_eq!(
            opened.editor.project().tracks(),
            snapshot().project.tracks()
        );
        assert_eq!(opened.editor.project().next_clip_id(), 2);
        assert!(!opened.editor.magnet_enabled());
        assert!(!opened.editor.can_undo());
    }

    #[test]
    fn schema_v1_migrates_in_memory_and_preserves_identity() {
        let id = ProjectId::generate();
        let bytes = encoded_v1(&id);
        let opened = parse_project_bytes(&bytes)
            .expect("parse")
            .into_opened(111, 222)
            .expect("opened");
        assert_eq!(opened.schema_version, 1);
        assert_eq!(opened.created_at_unix_ms, 111);
        assert_eq!(opened.last_saved_unix_ms, 222);
        assert_eq!(opened.save_generation, 0);
        assert_eq!(opened.project_id, id);
        assert!(opened.editor.project().assets().is_empty());
        assert_eq!(opened.editor.project().next_asset_id(), 1);
    }

    fn asset_snapshot() -> ProjectSnapshot {
        use capcut_engine::asset::{
            MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
        };
        use capcut_engine::command::Command;
        let mut editor = NativeEditor::new(sample_project()).expect("editor");
        let metadata = MediaAssetMetadata {
            container: "mov,mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase_numerator: 1,
            timebase_denominator: 15_360,
            stream_start_ticks: 512,
            duration_ticks: Some(3 * 15_360),
            nominal_frame_rate: MediaRational::new(30, 1),
            average_frame_rate: MediaRational::new(30, 1),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: Some(90),
        };
        let registration = MediaAssetRegistration::video(
            "Media A",
            MediaLocator::absolute("C:/media/a.mp4").expect("locator"),
            metadata,
        )
        .expect("asset");
        let asset = editor
            .execute(Command::AddMediaAsset {
                asset: registration,
            })
            .expect("add asset")
            .created_asset
            .expect("asset id");
        editor
            .execute(Command::AppendMediaClip { asset })
            .expect("append");
        editor.snapshot()
    }

    #[test]
    fn schema_v2_round_trips_assets_and_clip_references() {
        let id = ProjectId::generate();
        let snapshot = asset_snapshot();
        let file = ProjectFileV2::from_snapshot(
            &snapshot,
            &ProjectDocumentMetadata {
                project_id: id.clone(),
                created_at_unix_ms: 9,
                save_generation: 4,
            },
            10,
        );
        let bytes = serde_json::to_vec(&file).expect("json");
        let opened = parse_project_bytes(&bytes)
            .expect("parse")
            .into_opened(0, 0)
            .expect("opened");
        let project = opened.editor.project();
        assert_eq!(project.assets().len(), 1);
        let asset = &project.assets()[0];
        assert_eq!(asset.metadata().timebase_denominator, 15_360);
        assert_eq!(asset.metadata().stream_start_ticks, 512);
        assert_eq!(
            asset.metadata().duration_source_ticks().unwrap(),
            SourceTicks::new(360_000)
        );
        let media_clip = project
            .main_track()
            .expect("main")
            .clips()
            .iter()
            .find(|clip| clip.media_asset_id() == Some(asset.id()))
            .expect("media clip");
        assert_eq!(
            media_clip.range(),
            TimelineRange::new(TimelineTicks::new(12_000), TimelineTicks::new(372_000)).unwrap()
        );
        assert_eq!(opened.schema_version, 2);
        assert_eq!(project.next_asset_id(), 2);
    }

    #[test]
    fn dangling_asset_reference_is_rejected() {
        let id = ProjectId::generate();
        let bytes = serde_json::to_vec(&ProjectFileV2::from_snapshot(
            &asset_snapshot(),
            &ProjectDocumentMetadata {
                project_id: id,
                created_at_unix_ms: 9,
                save_generation: 1,
            },
            10,
        ))
        .expect("json");
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        let tracks = value["tracks"].as_array_mut().expect("tracks");
        let clip = &mut tracks[0]["clips"][0];
        clip["media_asset_id"] = serde_json::json!(99);
        let opened = parse_project_bytes(&serde_json::to_vec(&value).expect("json"))
            .expect("parse")
            .into_opened(0, 0);
        assert!(matches!(opened, Err(ProjectOpenError::Project(_))));
    }

    #[test]
    fn header_rejects_future_schema_before_full_parse() {
        let id = ProjectId::generate();
        let bytes = encoded_v2(&id);
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        value["schema_version"] = serde_json::json!(3);
        value["future_field"] = serde_json::json!({ "unseen": true });
        let bytes = serde_json::to_vec(&value).expect("json");
        assert_eq!(
            parse_project_bytes(&bytes),
            Err(ProjectOpenError::UnsupportedSchemaVersion(3))
        );
    }

    #[test]
    fn unknown_fields_and_invalid_json_are_rejected() {
        let id = ProjectId::generate();
        let bytes = encoded_v2(&id);
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        value["unknown"] = serde_json::json!(1);
        let bytes = serde_json::to_vec(&value).expect("json");
        assert!(matches!(
            parse_project_bytes(&bytes),
            Err(ProjectOpenError::InvalidJson(_))
        ));
        assert!(matches!(
            parse_project_bytes(b"{not-json"),
            Err(ProjectOpenError::InvalidJson(_))
        ));
    }

    #[test]
    fn unsupported_format_timebase_and_invalid_fields_are_typed() {
        let id = ProjectId::generate();
        let bytes = encoded_v2(&id);
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        value["format"] = serde_json::json!("other-project");
        assert_eq!(
            parse_project_bytes(&serde_json::to_vec(&value).expect("json")),
            Err(ProjectOpenError::UnsupportedFormat(
                "other-project".to_owned()
            ))
        );

        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        value["timebase"] = serde_json::json!(90_000);
        let opened = parse_project_bytes(&serde_json::to_vec(&value).expect("json"))
            .expect("parse")
            .into_opened(0, 0);
        assert_eq!(opened, Err(ProjectOpenError::UnsupportedTimebase(90_000)));

        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        value["tracks"][0]["clips"][0]["start"] = serde_json::json!(20_000);
        value["tracks"][0]["clips"][0]["end"] = serde_json::json!(10_000);
        let opened = parse_project_bytes(&serde_json::to_vec(&value).expect("json"))
            .expect("parse")
            .into_opened(0, 0);
        assert_eq!(opened, Err(ProjectOpenError::InvalidField("clip range")));
    }

    #[test]
    fn depth_scanner_ignores_braces_inside_strings() {
        assert!(json_depth_within(br#"{"a":"{{{{"}"#, 2));
        assert!(!json_depth_within(br#"{"a":{"b":1}}"#, 1));
        assert!(json_depth_within(br#"{"a":"\\\"{"}"#, 2));
    }
}

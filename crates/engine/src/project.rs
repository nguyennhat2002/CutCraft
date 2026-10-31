//! Native project, timeline, track and clip model.
//!
//! This model is independent of CapCut JSON. UI code only projects it for
//! display; editing commands own mutation and invariants.

use std::collections::BTreeSet;
use std::fmt;

use capcut_core::time::{
    FrameRate, SourceRange, Timebase, TimelineRange, TimelineTicks, TIMELINE_TICKS_PER_SECOND,
};

use crate::asset::{
    MediaAsset, MediaAssetError, MediaAssetId, MediaAssetKind, MediaAssetParts,
    MediaAssetRegistration, MediaLocator, MAX_MEDIA_ASSETS,
};

pub const MAX_TRACKS: usize = 64;
pub const MAX_CLIPS_PER_TRACK: usize = 10_000;
pub const MAX_LABEL_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(u64);

impl TrackId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClipId(u64);

impl ClipId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipKind {
    Video,
    Audio,
    Subtitle,
    Effect,
    NestedComposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapping {
    available: SourceRange,
    used: SourceRange,
    timebase: Timebase,
    speed_numerator: u32,
    speed_denominator: u32,
}

impl SourceMapping {
    pub fn new(
        available: SourceRange,
        used: SourceRange,
        timebase: Timebase,
        speed_numerator: u32,
        speed_denominator: u32,
    ) -> Result<Self, ProjectError> {
        if timebase.ticks_per_second != TIMELINE_TICKS_PER_SECOND
            || speed_numerator == 0
            || speed_denominator == 0
            || speed_numerator != speed_denominator
        {
            return Err(ProjectError::UnsupportedSourceMapping);
        }
        if used.start() < available.start() || used.end() > available.end() {
            return Err(ProjectError::InvalidSourceRange);
        }
        Ok(Self {
            available,
            used,
            timebase,
            speed_numerator,
            speed_denominator,
        })
    }

    pub const fn available(&self) -> SourceRange {
        self.available
    }

    pub const fn used(&self) -> SourceRange {
        self.used
    }

    pub const fn timebase(&self) -> Timebase {
        self.timebase
    }

    pub const fn speed(&self) -> (u32, u32) {
        (self.speed_numerator, self.speed_denominator)
    }

    pub(crate) fn set_used(&mut self, used: SourceRange) -> Result<(), ProjectError> {
        if used.start() < self.available.start() || used.end() > self.available.end() {
            return Err(ProjectError::InvalidSourceRange);
        }
        self.used = used;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipSpec {
    pub name: String,
    pub kind: ClipKind,
    pub range: TimelineRange,
    pub source: Option<SourceMapping>,
    pub media_asset_id: Option<MediaAssetId>,
    pub linked_clip_ids: BTreeSet<ClipId>,
    pub nested: bool,
}

/// Validated parts used to restore an existing document without reallocating
/// IDs or reordering content. Persistence loads through this path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectParts {
    pub name: String,
    pub fps: FrameRate,
    pub main_video_track: Option<TrackId>,
    pub tracks: Vec<TrackParts>,
    pub assets: Vec<MediaAssetParts>,
    pub next_track_id: u64,
    pub next_clip_id: u64,
    pub next_asset_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackParts {
    pub id: TrackId,
    pub name: String,
    pub kind: TrackKind,
    pub locked: bool,
    pub clips: Vec<ClipParts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipParts {
    pub id: ClipId,
    pub name: String,
    pub kind: ClipKind,
    pub range: TimelineRange,
    pub source: Option<SourceMapping>,
    pub media_asset_id: Option<MediaAssetId>,
    pub linked_clip_ids: BTreeSet<ClipId>,
    pub nested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    id: ClipId,
    name: String,
    kind: ClipKind,
    range: TimelineRange,
    source: Option<SourceMapping>,
    media_asset_id: Option<MediaAssetId>,
    linked_clip_ids: BTreeSet<ClipId>,
    nested: bool,
}

impl Clip {
    pub(crate) fn from_spec(id: ClipId, spec: ClipSpec) -> Result<Self, ProjectError> {
        validate_label(&spec.name)?;
        if spec.nested != matches!(spec.kind, ClipKind::NestedComposition) {
            return Err(ProjectError::InvalidNestedClip);
        }
        if let Some(source) = &spec.source {
            let timeline_duration = spec.range.duration().get();
            let source_duration = source.used().duration().get();
            if timeline_duration != source_duration {
                return Err(ProjectError::SourceDurationMismatch);
            }
        }
        if spec.media_asset_id.is_some() && (spec.source.is_none() || spec.kind != ClipKind::Video)
        {
            return Err(ProjectError::InvalidAssetClip);
        }
        Ok(Self {
            id,
            name: spec.name,
            kind: spec.kind,
            range: spec.range,
            source: spec.source,
            media_asset_id: spec.media_asset_id,
            linked_clip_ids: spec.linked_clip_ids,
            nested: spec.nested,
        })
    }

    pub const fn id(&self) -> ClipId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> ClipKind {
        self.kind
    }

    pub const fn range(&self) -> TimelineRange {
        self.range
    }

    pub const fn source(&self) -> Option<&SourceMapping> {
        self.source.as_ref()
    }

    pub const fn media_asset_id(&self) -> Option<MediaAssetId> {
        self.media_asset_id
    }

    pub fn linked_clip_ids(&self) -> &BTreeSet<ClipId> {
        &self.linked_clip_ids
    }

    pub const fn nested(&self) -> bool {
        self.nested
    }

    pub(crate) fn set_range(&mut self, range: TimelineRange) {
        self.range = range;
    }

    pub(crate) fn set_source(&mut self, source: Option<SourceMapping>) {
        self.source = source;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    id: TrackId,
    name: String,
    kind: TrackKind,
    locked: bool,
    pub(crate) clips: Vec<Clip>,
}

impl Track {
    fn new(id: TrackId, name: &str, kind: TrackKind, locked: bool) -> Result<Self, ProjectError> {
        validate_label(name)?;
        Ok(Self {
            id,
            name: name.to_owned(),
            kind,
            locked,
            clips: Vec::new(),
        })
    }

    pub const fn id(&self) -> TrackId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn kind(&self) -> TrackKind {
        self.kind
    }

    pub const fn locked(&self) -> bool {
        self.locked
    }

    pub fn clips(&self) -> &[Clip] {
        &self.clips
    }

    pub fn clip(&self, index: usize) -> Option<&Clip> {
        self.clips.get(index)
    }

    pub fn index_of(&self, clip: ClipId) -> Option<usize> {
        self.clips.iter().position(|entry| entry.id() == clip)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub project: Project,
    pub magnet_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    name: String,
    fps: FrameRate,
    main_video_track: Option<TrackId>,
    tracks: Vec<Track>,
    assets: Vec<MediaAsset>,
    pub(crate) next_track_id: u64,
    pub(crate) next_clip_id: u64,
    pub(crate) next_asset_id: u64,
    pub(crate) revision: u64,
}

impl Project {
    pub fn empty(fps: FrameRate) -> Result<Self, ProjectError> {
        fps.ticks_per_frame()
            .map_err(|_| ProjectError::UnsupportedFrameRate)?;
        Ok(Self {
            name: "Untitled".to_owned(),
            fps,
            main_video_track: None,
            tracks: Vec::new(),
            assets: Vec::new(),
            next_track_id: 1,
            next_clip_id: 1,
            next_asset_id: 1,
            revision: 0,
        })
    }

    pub fn named(fps: FrameRate, name: impl Into<String>) -> Result<Self, ProjectError> {
        let mut project = Self::empty(fps)?;
        project.name = name.into();
        validate_label(&project.name)?;
        Ok(project)
    }

    /// Restore exact IDs, order, main-track identity and allocator high-water
    /// marks, then run the full project validation. Revision restarts at zero.
    pub fn from_parts(parts: ProjectParts) -> Result<Self, ProjectError> {
        parts
            .fps
            .ticks_per_frame()
            .map_err(|_| ProjectError::UnsupportedFrameRate)?;
        if parts.tracks.len() > MAX_TRACKS {
            return Err(ProjectError::TooManyTracks);
        }
        if parts
            .tracks
            .iter()
            .any(|track| track.clips.len() > MAX_CLIPS_PER_TRACK)
        {
            return Err(ProjectError::TooManyClips);
        }
        if parts.assets.len() > MAX_MEDIA_ASSETS {
            return Err(ProjectError::TooManyMediaAssets);
        }

        let mut assets = Vec::with_capacity(parts.assets.len());
        for asset_parts in parts.assets {
            assets.push(MediaAsset::from_parts(asset_parts).map_err(ProjectError::InvalidAsset)?);
        }

        let mut tracks = Vec::with_capacity(parts.tracks.len());
        for track_parts in parts.tracks {
            let mut track = Track::new(
                track_parts.id,
                &track_parts.name,
                track_parts.kind,
                track_parts.locked,
            )?;
            track.clips.reserve(track_parts.clips.len());
            for clip_parts in track_parts.clips {
                track.clips.push(Clip::from_spec(
                    clip_parts.id,
                    ClipSpec {
                        name: clip_parts.name,
                        kind: clip_parts.kind,
                        range: clip_parts.range,
                        source: clip_parts.source,
                        media_asset_id: clip_parts.media_asset_id,
                        linked_clip_ids: clip_parts.linked_clip_ids,
                        nested: clip_parts.nested,
                    },
                )?);
            }
            tracks.push(track);
        }

        let project = Self {
            name: parts.name,
            fps: parts.fps,
            main_video_track: parts.main_video_track,
            tracks,
            assets,
            next_track_id: parts.next_track_id,
            next_clip_id: parts.next_clip_id,
            next_asset_id: parts.next_asset_id,
            revision: 0,
        };
        project.validate()?;
        Ok(project)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn fps(&self) -> FrameRate {
        self.fps
    }

    pub const fn main_video_track(&self) -> Option<TrackId> {
        self.main_video_track
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id() == id)
    }

    pub fn main_track(&self) -> Option<&Track> {
        self.main_video_track.and_then(|id| self.track(id))
    }

    pub fn clip(&self, id: ClipId) -> Option<&Clip> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips())
            .find(|clip| clip.id() == id)
    }

    pub fn assets(&self) -> &[MediaAsset] {
        &self.assets
    }

    pub fn asset(&self, id: MediaAssetId) -> Option<&MediaAsset> {
        self.assets.iter().find(|asset| asset.id() == id)
    }

    pub(crate) fn asset_mut(&mut self, id: MediaAssetId) -> Option<&mut MediaAsset> {
        self.assets.iter_mut().find(|asset| asset.id() == id)
    }

    pub fn asset_by_locator(&self, locator: &MediaLocator) -> Option<&MediaAsset> {
        self.assets.iter().find(|asset| asset.locator() == locator)
    }

    pub fn add_asset(
        &mut self,
        registration: MediaAssetRegistration,
    ) -> Result<MediaAssetId, ProjectError> {
        if self.assets.len() >= MAX_MEDIA_ASSETS {
            return Err(ProjectError::TooManyMediaAssets);
        }
        registration
            .validate()
            .map_err(ProjectError::InvalidAsset)?;
        if self
            .assets
            .iter()
            .any(|asset| asset.locator() == &registration.locator)
        {
            return Err(ProjectError::DuplicateAssetLocator(
                registration.locator.value().to_owned(),
            ));
        }
        let id = MediaAssetId::new(self.next_asset_id);
        let next = self
            .next_asset_id
            .checked_add(1)
            .ok_or(ProjectError::IdOverflow)?;
        let asset =
            MediaAsset::from_registration(id, registration).map_err(ProjectError::InvalidAsset)?;
        self.next_asset_id = next;
        self.assets.push(asset);
        Ok(id)
    }

    pub fn rename(&mut self, name: impl Into<String>) -> Result<(), ProjectError> {
        let name = name.into();
        validate_label(&name)?;
        self.name = name;
        Ok(())
    }

    pub(crate) fn main_track_mut(&mut self) -> Option<&mut Track> {
        let id = self.main_video_track?;
        self.tracks.iter_mut().find(|track| track.id() == id)
    }

    pub fn content_end(&self) -> TimelineTicks {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .map(|clip| clip.range().end())
            .max()
            .unwrap_or(TimelineTicks::ZERO)
    }

    pub fn add_track(
        &mut self,
        name: &str,
        kind: TrackKind,
        locked: bool,
    ) -> Result<TrackId, ProjectError> {
        if self.tracks.len() >= MAX_TRACKS {
            return Err(ProjectError::TooManyTracks);
        }
        validate_label(name)?;
        let id = TrackId::new(self.next_track_id);
        let next = self
            .next_track_id
            .checked_add(1)
            .ok_or(ProjectError::IdOverflow)?;
        let track = Track::new(id, name, kind, locked)?;
        self.next_track_id = next;
        self.tracks.push(track);
        Ok(id)
    }

    pub fn assign_main_track(&mut self, id: TrackId) -> Result<(), ProjectError> {
        let track = self.track(id).ok_or(ProjectError::UnknownTrack(id))?;
        if track.kind() != TrackKind::Video {
            return Err(ProjectError::NotVideoTrack(id));
        }
        self.main_video_track = Some(id);
        Ok(())
    }

    pub fn add_clip(&mut self, track_id: TrackId, spec: ClipSpec) -> Result<ClipId, ProjectError> {
        let track = self
            .tracks
            .iter_mut()
            .find(|track| track.id() == track_id)
            .ok_or(ProjectError::UnknownTrack(track_id))?;
        if track.clips.len() >= MAX_CLIPS_PER_TRACK {
            return Err(ProjectError::TooManyClips);
        }
        let id = ClipId::new(self.next_clip_id);
        let next = self
            .next_clip_id
            .checked_add(1)
            .ok_or(ProjectError::IdOverflow)?;
        let clip = Clip::from_spec(id, spec)?;
        if track
            .clips
            .iter()
            .any(|entry| entry.range().overlaps(clip.range()))
        {
            return Err(ProjectError::OverlappingClips);
        }
        self.next_clip_id = next;
        let index = track
            .clips
            .iter()
            .position(|entry| entry.range().start() > clip.range().start())
            .unwrap_or(track.clips.len());
        track.clips.insert(index, clip);
        Ok(id)
    }

    pub fn next_track_id(&self) -> u64 {
        self.next_track_id
    }

    pub fn next_clip_id(&self) -> u64 {
        self.next_clip_id
    }

    pub fn next_asset_id(&self) -> u64 {
        self.next_asset_id
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Approximate heap/inline bytes for metadata-only history budgeting.
    /// This intentionally excludes media, frames, GPU resources and CapCut
    /// document bytes, which the native project does not own.
    pub fn estimated_bytes(&self) -> usize {
        let mut total = std::mem::size_of::<Self>() + self.name.capacity();
        total += self.tracks.capacity() * std::mem::size_of::<Track>();
        for track in &self.tracks {
            total += track.name.capacity();
            total += track.clips.capacity() * std::mem::size_of::<Clip>();
            for clip in &track.clips {
                total += clip.name.capacity();
                total += clip.linked_clip_ids.len() * (std::mem::size_of::<ClipId>() + 48);
            }
        }
        total += self.assets.capacity() * std::mem::size_of::<MediaAsset>();
        for asset in &self.assets {
            total += asset.display_name().len();
            total += asset.locator().value().len();
            total += asset.metadata().container.len();
            total += asset.metadata().codec.len();
            total += asset
                .metadata()
                .pixel_format
                .as_ref()
                .map(String::len)
                .unwrap_or(0);
        }
        total
    }

    pub fn links_for(&self, clip_id: ClipId) -> bool {
        if let Some(clip) = self
            .tracks
            .iter()
            .flat_map(|track| track.clips())
            .find(|clip| clip.id() == clip_id)
        {
            if !clip.linked_clip_ids().is_empty() {
                return true;
            }
        }
        self.tracks
            .iter()
            .flat_map(|track| track.clips())
            .any(|clip| clip.linked_clip_ids().contains(&clip_id))
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.tracks.len() > MAX_TRACKS {
            return Err(ProjectError::TooManyTracks);
        }
        validate_label(&self.name)?;
        if self.next_track_id == 0 || self.next_clip_id == 0 || self.next_asset_id == 0 {
            return Err(ProjectError::ZeroAllocator);
        }
        if self.assets.len() > MAX_MEDIA_ASSETS {
            return Err(ProjectError::TooManyMediaAssets);
        }
        let mut asset_ids = BTreeSet::new();
        let mut locators = BTreeSet::new();
        for asset in &self.assets {
            if asset.id().get() == 0 {
                return Err(ProjectError::ZeroAssetId);
            }
            if self.next_asset_id <= asset.id().get() {
                return Err(ProjectError::AssetAllocatorOutOfSync);
            }
            if !asset_ids.insert(asset.id()) {
                return Err(ProjectError::DuplicateAssetId(asset.id()));
            }
            if !locators.insert(asset.locator().clone()) {
                return Err(ProjectError::DuplicateAssetLocator(
                    asset.locator().value().to_owned(),
                ));
            }
            asset
                .metadata()
                .validate()
                .map_err(ProjectError::InvalidAsset)?;
        }
        if let Some(main) = self.main_video_track {
            let track = self
                .track(main)
                .ok_or(ProjectError::InvalidMainTrack(main))?;
            if track.kind() != TrackKind::Video {
                return Err(ProjectError::NotVideoTrack(main));
            }
        }
        let mut track_ids = BTreeSet::new();
        let mut clip_ids = BTreeSet::new();
        for track in &self.tracks {
            if track.id().get() == 0 {
                return Err(ProjectError::ZeroTrackId);
            }
            if self.next_track_id <= track.id().get() {
                return Err(ProjectError::AllocatorOutOfSync);
            }
            if !track_ids.insert(track.id()) {
                return Err(ProjectError::DuplicateTrackId(track.id()));
            }
            validate_label(track.name())?;
            if track.clips.len() > MAX_CLIPS_PER_TRACK {
                return Err(ProjectError::TooManyClips);
            }
            let mut previous_end: Option<TimelineTicks> = None;
            for clip in &track.clips {
                if clip.id().get() == 0 {
                    return Err(ProjectError::ZeroClipId);
                }
                if self.next_clip_id <= clip.id().get() {
                    return Err(ProjectError::AllocatorOutOfSync);
                }
                if !clip_ids.insert(clip.id()) {
                    return Err(ProjectError::DuplicateClipId(clip.id()));
                }
                validate_label(clip.name())?;
                if let Some(end) = previous_end {
                    if clip.range().start() < end {
                        return Err(ProjectError::OverlappingClips);
                    }
                }
                previous_end = Some(clip.range().end());
                if let Some(source) = clip.source() {
                    if source.timebase().ticks_per_second != TIMELINE_TICKS_PER_SECOND
                        || source.speed() != (1, 1)
                    {
                        return Err(ProjectError::UnsupportedSourceMapping);
                    }
                    if source.used().duration().get() != clip.range().duration().get() {
                        return Err(ProjectError::SourceDurationMismatch);
                    }
                }
                if let Some(asset_id) = clip.media_asset_id() {
                    if clip.kind() != ClipKind::Video || clip.source().is_none() {
                        return Err(ProjectError::InvalidAssetClip);
                    }
                    let asset = self
                        .asset(asset_id)
                        .ok_or(ProjectError::UnknownMediaAsset(asset_id))?;
                    if asset.kind() != MediaAssetKind::Video {
                        return Err(ProjectError::InvalidAssetClip);
                    }
                    let duration = asset
                        .metadata()
                        .duration_source_ticks()
                        .map_err(ProjectError::InvalidAsset)?;
                    let source = clip.source().ok_or(ProjectError::InvalidAssetClip)?;
                    if source.available().end() > duration || source.used().end() > duration {
                        return Err(ProjectError::AssetSourceOutOfBounds(asset_id));
                    }
                }
            }
        }
        for track in &self.tracks {
            for clip in &track.clips {
                for linked in clip.linked_clip_ids() {
                    if *linked == clip.id() {
                        return Err(ProjectError::SelfLinkedClip(clip.id()));
                    }
                    if !clip_ids.contains(linked) {
                        return Err(ProjectError::UnknownLinkedClip(*linked));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    UnsupportedFrameRate,
    TooManyTracks,
    TooManyClips,
    InvalidLabel,
    UnknownTrack(TrackId),
    UnknownClip(ClipId),
    NotVideoTrack(TrackId),
    InvalidMainTrack(TrackId),
    DuplicateTrackId(TrackId),
    DuplicateClipId(ClipId),
    OverlappingClips,
    InvalidSourceRange,
    UnsupportedSourceMapping,
    SourceDurationMismatch,
    InvalidNestedClip,
    ZeroTrackId,
    ZeroClipId,
    ZeroAssetId,
    ZeroAllocator,
    UnknownLinkedClip(ClipId),
    SelfLinkedClip(ClipId),
    AllocatorOutOfSync,
    AssetAllocatorOutOfSync,
    TooManyMediaAssets,
    DuplicateAssetId(MediaAssetId),
    UnknownMediaAsset(MediaAssetId),
    DuplicateAssetLocator(String),
    InvalidAsset(MediaAssetError),
    InvalidAssetClip,
    AssetSourceOutOfBounds(MediaAssetId),
    IdOverflow,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFrameRate => formatter.write_str("unsupported project frame rate"),
            Self::TooManyTracks => formatter.write_str("too many tracks"),
            Self::TooManyClips => formatter.write_str("too many clips"),
            Self::InvalidLabel => formatter.write_str("invalid label"),
            Self::UnknownTrack(id) => write!(formatter, "unknown track {}", id.get()),
            Self::UnknownClip(id) => write!(formatter, "unknown clip {}", id.get()),
            Self::NotVideoTrack(id) => write!(formatter, "track {} is not video", id.get()),
            Self::InvalidMainTrack(id) => write!(formatter, "invalid main track {}", id.get()),
            Self::DuplicateTrackId(id) => write!(formatter, "duplicate track {}", id.get()),
            Self::DuplicateClipId(id) => write!(formatter, "duplicate clip {}", id.get()),
            Self::OverlappingClips => formatter.write_str("clips overlap"),
            Self::InvalidSourceRange => formatter.write_str("invalid source range"),
            Self::UnsupportedSourceMapping => formatter.write_str("unsupported source mapping"),
            Self::SourceDurationMismatch => {
                formatter.write_str("timeline/source duration mismatch")
            }
            Self::InvalidNestedClip => formatter.write_str("invalid nested clip metadata"),
            Self::ZeroTrackId => formatter.write_str("track IDs must be nonzero"),
            Self::ZeroClipId => formatter.write_str("clip IDs must be nonzero"),
            Self::ZeroAssetId => formatter.write_str("media asset IDs must be nonzero"),
            Self::ZeroAllocator => formatter.write_str("ID allocators must be nonzero"),
            Self::UnknownLinkedClip(id) => {
                write!(formatter, "linked clip {} does not exist", id.get())
            }
            Self::SelfLinkedClip(id) => {
                write!(formatter, "clip {} links to itself", id.get())
            }
            Self::AllocatorOutOfSync => {
                formatter.write_str("ID allocator is not ahead of existing IDs")
            }
            Self::AssetAllocatorOutOfSync => {
                formatter.write_str("media asset ID allocator is not ahead of existing IDs")
            }
            Self::TooManyMediaAssets => formatter.write_str("too many media assets"),
            Self::DuplicateAssetId(id) => write!(formatter, "duplicate media asset {}", id.get()),
            Self::UnknownMediaAsset(id) => write!(formatter, "unknown media asset {}", id.get()),
            Self::DuplicateAssetLocator(locator) => {
                write!(
                    formatter,
                    "media asset locator is already registered: {locator}"
                )
            }
            Self::InvalidAsset(error) => write!(formatter, "invalid media asset: {error}"),
            Self::InvalidAssetClip => {
                formatter.write_str("clip media asset reference is inconsistent")
            }
            Self::AssetSourceOutOfBounds(id) => write!(
                formatter,
                "clip source range exceeds media asset {} duration",
                id.get()
            ),
            Self::IdOverflow => formatter.write_str("stable ID allocation overflow"),
        }
    }
}

impl std::error::Error for ProjectError {}

fn validate_label(value: &str) -> Result<(), ProjectError> {
    if value.is_empty() || value.len() > MAX_LABEL_BYTES {
        return Err(ProjectError::InvalidLabel);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use capcut_core::time::{
        FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks,
    };
    use std::collections::BTreeSet;

    use crate::asset::{MediaAssetMetadata, MediaRational};

    fn rate() -> FrameRate {
        FrameRate::new(30, 1).expect("30/1")
    }

    fn ticks(seconds: i64) -> TimelineTicks {
        TimelineTicks::new(seconds * 120_000)
    }

    fn range(start: i64, end: i64) -> TimelineRange {
        TimelineRange::new(ticks(start), ticks(end)).expect("range")
    }

    fn source(duration: i64) -> SourceMapping {
        SourceMapping::new(
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration * 120_000))
                .expect("available"),
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(duration * 120_000))
                .expect("used"),
            Timebase::new(120_000).expect("timebase"),
            1,
            1,
        )
        .expect("source")
    }

    fn spec(name: &str, start: i64, end: i64) -> ClipSpec {
        ClipSpec {
            name: name.to_owned(),
            kind: ClipKind::Video,
            range: range(start, end),
            source: Some(source(end - start)),
            media_asset_id: None,
            linked_clip_ids: BTreeSet::new(),
            nested: false,
        }
    }

    #[test]
    fn empty_project_has_no_inferred_main_track() {
        let project = Project::empty(rate()).expect("project");
        assert!(project.main_video_track().is_none());
        assert!(project.tracks().is_empty());
        assert_eq!(project.content_end(), TimelineTicks::ZERO);
    }

    #[test]
    fn main_track_must_be_an_existing_video_track() {
        let mut project = Project::empty(rate()).expect("project");
        let video = project
            .add_track("V1", TrackKind::Video, false)
            .expect("video");
        let audio = project
            .add_track("A1", TrackKind::Audio, false)
            .expect("audio");
        assert!(project.assign_main_track(audio).is_err());
        assert!(project.assign_main_track(TrackId::new(999)).is_err());
        project.assign_main_track(video).expect("main");
        assert_eq!(project.main_video_track(), Some(video));
        project.validate().expect("valid");
    }

    #[test]
    fn clip_ids_are_monotonic_and_failed_adds_do_not_consume_ids() {
        let mut project = Project::empty(rate()).expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        let a = project.add_clip(track, spec("A", 0, 5)).expect("A");
        let before = project.next_clip_id();
        assert!(project.add_clip(track, spec("overlap", 4, 6)).is_err());
        assert_eq!(project.next_clip_id(), before);
        let b = project.add_clip(track, spec("B", 10, 15)).expect("B");
        assert_ne!(a, b);
        assert_eq!(project.tracks()[0].clips().len(), 2);
        assert_eq!(project.content_end(), ticks(15));
    }

    #[test]
    fn source_mapping_requires_exact_supported_media_timebase() {
        let result = SourceMapping::new(
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(600_000)).expect("available"),
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(600_000)).expect("used"),
            Timebase::new(90_000).expect("timebase"),
            1,
            1,
        );
        assert!(matches!(
            result,
            Err(ProjectError::UnsupportedSourceMapping)
        ));

        let variable_speed = SourceMapping::new(
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(600_000)).expect("available"),
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(600_000)).expect("used"),
            Timebase::new(120_000).expect("timebase"),
            2,
            1,
        );
        assert!(matches!(
            variable_speed,
            Err(ProjectError::UnsupportedSourceMapping)
        ));

        let outside_available = SourceMapping::new(
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(600_000)).expect("available"),
            SourceRange::new(SourceTicks::new(120_000), SourceTicks::new(720_000)).expect("used"),
            Timebase::new(120_000).expect("timebase"),
            1,
            1,
        );
        assert!(matches!(
            outside_available,
            Err(ProjectError::InvalidSourceRange)
        ));
    }

    #[test]
    fn links_are_detected_in_both_directions() {
        let mut project = Project::empty(rate()).expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        let first = project.add_clip(track, spec("A", 0, 5)).expect("A");
        let mut linked = spec("B", 5, 10);
        linked.linked_clip_ids.insert(first);
        let second = project.add_clip(track, linked).expect("B");
        assert!(project.links_for(first));
        assert!(project.links_for(second));
    }

    #[test]
    fn invalid_project_limits_are_rejected_without_panicking() {
        let mut project = Project::empty(rate()).expect("project");
        assert!(project.add_track("", TrackKind::Video, false).is_err());
        assert!(project
            .add_track(&"x".repeat(MAX_LABEL_BYTES + 1), TrackKind::Video, false)
            .is_err());
    }

    fn project_with_clips(count: usize) -> Project {
        let mut project = Project::empty(rate()).expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        let track = project.tracks.first_mut().expect("track");
        for index in 0..count {
            let start = index as i64 * 120_000;
            let end = start + 120_000;
            let source = SourceMapping::new(
                SourceRange::new(SourceTicks::ZERO, SourceTicks::new(120_000)).expect("available"),
                SourceRange::new(SourceTicks::ZERO, SourceTicks::new(120_000)).expect("used"),
                Timebase::new(120_000).expect("timebase"),
                1,
                1,
            )
            .expect("source");
            track.clips.push(
                Clip::from_spec(
                    ClipId::new(index as u64 + 1),
                    ClipSpec {
                        name: format!("Clip {index}"),
                        kind: ClipKind::Video,
                        range: TimelineRange::new(
                            TimelineTicks::new(start),
                            TimelineTicks::new(end),
                        )
                        .expect("range"),
                        source: Some(source),
                        media_asset_id: None,
                        linked_clip_ids: BTreeSet::new(),
                        nested: false,
                    },
                )
                .expect("clip"),
            );
        }
        project.next_clip_id = count as u64 + 1;
        project
    }

    #[test]
    fn estimated_bytes_accounts_for_asset_metadata() {
        let mut project = project_with_clips(2);
        let before = project.estimated_bytes();
        for index in 0..40usize {
            let metadata = MediaAssetMetadata {
                container: "mov,mp4".to_owned(),
                video_stream_index: 0,
                codec: "h264".to_owned(),
                width: 1920,
                height: 1080,
                pixel_format: Some("yuv420p".to_owned()),
                timebase_numerator: 1,
                timebase_denominator: 15_360,
                stream_start_ticks: 0,
                duration_ticks: Some(15_360),
                nominal_frame_rate: MediaRational::new(30, 1),
                average_frame_rate: MediaRational::new(30, 1),
                rotation_degrees: 0,
                has_audio: false,
                frame_count_hint: None,
            };
            let registration = MediaAssetRegistration::video(
                format!("Asset {index} with a descriptive name"),
                MediaLocator::absolute(format!("C:/media/asset-{index}.mp4")).expect("locator"),
                metadata,
            )
            .expect("asset");
            project.add_asset(registration).expect("registered");
        }
        let after = project.estimated_bytes();
        assert!(
            after > before + 40 * std::mem::size_of::<MediaAsset>(),
            "asset metadata must be included in history accounting: before={before} after={after}"
        );
    }

    #[test]
    fn estimated_bytes_grow_small_medium_large_and_max_shape_exceeds_history_budget() {
        let small = project_with_clips(3).estimated_bytes();
        let medium = project_with_clips(1_000).estimated_bytes();
        let large = project_with_clips(10_000).estimated_bytes();
        println!("estimate small={small} medium={medium} large={large}");
        assert!(small < medium && medium < large);
        assert!(large > 1_000_000, "large fixture estimate was {large}");

        let per_clip = std::mem::size_of::<Clip>() + MAX_LABEL_BYTES + 64;
        let max_estimate = std::mem::size_of::<Project>()
            + MAX_TRACKS * (std::mem::size_of::<Track>() + MAX_LABEL_BYTES)
            + MAX_TRACKS * MAX_CLIPS_PER_TRACK * per_clip;
        println!("maximum-shape estimate={max_estimate}");
        assert!(
            max_estimate > crate::command::history::MAX_HISTORY_BYTES * 2,
            "maximum-shape estimate must exceed the history budget"
        );
    }

    #[test]
    fn allocator_must_stay_ahead_of_existing_ids() {
        let mut project = project_with_clips(1);
        project.next_clip_id = 1;
        assert_eq!(project.validate(), Err(ProjectError::AllocatorOutOfSync));
    }

    fn restored_parts() -> ProjectParts {
        ProjectParts {
            name: "Restored".to_owned(),
            fps: rate(),
            main_video_track: Some(TrackId::new(7)),
            tracks: vec![TrackParts {
                id: TrackId::new(7),
                name: "V1".to_owned(),
                kind: TrackKind::Video,
                locked: false,
                clips: vec![
                    ClipParts {
                        id: ClipId::new(9),
                        name: "A".to_owned(),
                        kind: ClipKind::Video,
                        range: range(0, 1),
                        source: Some(source(1)),
                        media_asset_id: None,
                        linked_clip_ids: BTreeSet::new(),
                        nested: false,
                    },
                    ClipParts {
                        id: ClipId::new(10),
                        name: "B".to_owned(),
                        kind: ClipKind::Video,
                        range: range(2, 3),
                        source: Some(source(1)),
                        media_asset_id: None,
                        linked_clip_ids: BTreeSet::new(),
                        nested: false,
                    },
                ],
            }],
            assets: Vec::new(),
            next_track_id: 8,
            next_clip_id: 11,
            next_asset_id: 1,
        }
    }

    #[test]
    fn from_parts_restores_ids_order_main_track_and_allocators() {
        let project = Project::from_parts(restored_parts()).expect("restored");
        assert_eq!(project.name(), "Restored");
        assert_eq!(project.next_track_id(), 8);
        assert_eq!(project.next_clip_id(), 11);
        assert_eq!(project.revision(), 0);
        assert_eq!(project.main_video_track(), Some(TrackId::new(7)));
        let main = project.main_track().expect("main");
        assert_eq!(main.id(), TrackId::new(7));
        assert_eq!(
            main.clips()
                .iter()
                .map(|clip| clip.id())
                .collect::<Vec<_>>(),
            vec![ClipId::new(9), ClipId::new(10)]
        );
        assert_eq!(project.content_end(), ticks(3));
    }

    #[test]
    fn restore_rejects_zero_ids_broken_links_and_self_links() {
        let mut zero_track = restored_parts();
        zero_track.tracks[0].id = TrackId::new(0);
        zero_track.main_video_track = None;
        assert_eq!(
            Project::from_parts(zero_track),
            Err(ProjectError::ZeroTrackId)
        );

        let mut zero_clip = restored_parts();
        zero_clip.tracks[0].clips[0].id = ClipId::new(0);
        assert_eq!(
            Project::from_parts(zero_clip),
            Err(ProjectError::ZeroClipId)
        );

        let mut zero_allocator = restored_parts();
        zero_allocator.next_clip_id = 0;
        assert_eq!(
            Project::from_parts(zero_allocator),
            Err(ProjectError::ZeroAllocator)
        );

        let mut broken_link = restored_parts();
        broken_link.tracks[0].clips[0]
            .linked_clip_ids
            .insert(ClipId::new(3));
        assert_eq!(
            Project::from_parts(broken_link),
            Err(ProjectError::UnknownLinkedClip(ClipId::new(3)))
        );

        let mut self_link = restored_parts();
        self_link.tracks[0].clips[0]
            .linked_clip_ids
            .insert(ClipId::new(9));
        assert_eq!(
            Project::from_parts(self_link),
            Err(ProjectError::SelfLinkedClip(ClipId::new(9)))
        );
    }

    #[test]
    fn named_project_validates_the_name() {
        let project = Project::named(rate(), "Saved Project").expect("named");
        assert_eq!(project.name(), "Saved Project");
        assert!(Project::named(rate(), "").is_err());
    }

    #[test]
    fn restore_enforces_track_and_clip_count_limits() {
        let mut too_many_tracks = restored_parts();
        too_many_tracks.main_video_track = None;
        too_many_tracks.tracks = (1..=65)
            .map(|id| TrackParts {
                id: TrackId::new(id),
                name: format!("T{id}"),
                kind: TrackKind::Video,
                locked: false,
                clips: Vec::new(),
            })
            .collect();
        too_many_tracks.next_track_id = 66;
        assert_eq!(
            Project::from_parts(too_many_tracks),
            Err(ProjectError::TooManyTracks)
        );

        let mut too_many_clips = restored_parts();
        let template = too_many_clips.tracks[0].clips[0].clone();
        too_many_clips.tracks[0].clips = vec![template; MAX_CLIPS_PER_TRACK + 1];
        too_many_clips.next_clip_id = (MAX_CLIPS_PER_TRACK + 2) as u64;
        assert_eq!(
            Project::from_parts(too_many_clips),
            Err(ProjectError::TooManyClips)
        );
    }
}

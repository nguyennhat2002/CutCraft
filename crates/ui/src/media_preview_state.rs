//! Clip-aware asynchronous Preview.
//!
//! The active native clip determines the media asset. There is no session-global
//! manual binding. Decoding remains in `capcut-media`; this module owns request
//! identity, stale-result checks, one decoded frame and one reusable texture.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use capcut_core::time::{SourceTicks, TimelineTicks, TIMELINE_TICKS_PER_SECOND};
use capcut_engine::asset::{MediaAssetId, MediaAssetMetadata, MediaLocator};
use capcut_engine::project::{Clip, ClipId, ClipKind};
use capcut_media::{
    source_ticks_to_media_timestamp, DecodedFrame, FrameRequest, MediaError, MediaProbe,
    MediaTimestamp, PixelFormat, PreviewEngine, PreviewEvent,
};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use crate::editor_state::EditorSession;
use crate::media_library_state::{probe_matches_asset, AssetAvailability, MediaLibraryState};

const TEXTURE_NAME: &str = "cutcraft-preview-media";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipPreviewStatus {
    NoProject,
    NoActiveClip,
    ClipWithoutMedia,
    UnsupportedComposition,
    UnsupportedVisualLayer,
    AssetMissing,
    AssetChanged(String),
    AssetUnreadable(String),
    LoadingAsset,
    Seeking,
    Frame,
    EndOfMedia,
    Failed(String),
}

impl ClipPreviewStatus {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::LoadingAsset | Self::Seeking)
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::AssetChanged(message)
            | Self::AssetUnreadable(message)
            | Self::Failed(message) => Some(message),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MediaPreviewMetrics {
    pub probe_latency_ms: Option<u64>,
    pub last_seek_latency_ms: Option<u64>,
    pub last_decode_elapsed_ms: Option<u64>,
    pub last_texture_submit_ms: Option<u64>,
    pub frame_requests: u64,
    pub stale_results_ignored: u64,
    pub texture_uploads: u64,
    pub max_outstanding_requests: u32,
    pub decoded_bytes: u64,
    pub source_opens: u64,
}

#[derive(Debug, Clone)]
struct ActiveRequest {
    generation: u64,
    target: MediaTimestamp,
    clip_id: ClipId,
    asset_id: MediaAssetId,
    requested_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequestedTarget {
    target: MediaTimestamp,
    clip_id: ClipId,
    asset_id: MediaAssetId,
}

#[derive(Debug)]
enum SourceState {
    Probing,
    Ready(MediaProbe),
    Failed(String),
}

#[derive(Debug)]
struct OpenSource {
    asset_id: MediaAssetId,
    locator: MediaLocator,
    metadata: MediaAssetMetadata,
    path: PathBuf,
    state: SourceState,
}

impl OpenSource {
    fn matches(
        &self,
        asset_id: MediaAssetId,
        locator: &MediaLocator,
        metadata: &MediaAssetMetadata,
        path: &Path,
    ) -> bool {
        self.asset_id == asset_id
            && &self.locator == locator
            && &self.metadata == metadata
            && self.path == path
    }
}

#[derive(Debug)]
enum FramePlan {
    Ready {
        source_ticks: SourceTicks,
        target: MediaTimestamp,
    },
    AtOrAfterMediaEnd,
}

pub struct MediaPreview {
    engine: Option<PreviewEngine>,
    source_id: u64,
    next_generation: u64,
    project_id: Option<capcut_project::ProjectId>,
    open_source: Option<OpenSource>,
    active_clip_id: Option<ClipId>,
    active_asset_id: Option<MediaAssetId>,
    active_playhead: Option<TimelineTicks>,
    status: ClipPreviewStatus,
    requested: Option<RequestedTarget>,
    active: Option<ActiveRequest>,
    frame: Option<DecodedFrame>,
    texture: Option<TextureHandle>,
    texture_frame: Option<(u64, u64)>,
    metrics: MediaPreviewMetrics,
}

impl Default for MediaPreview {
    fn default() -> Self {
        Self {
            engine: None,
            source_id: 0,
            next_generation: 0,
            project_id: None,
            open_source: None,
            active_clip_id: None,
            active_asset_id: None,
            active_playhead: None,
            status: ClipPreviewStatus::NoProject,
            requested: None,
            active: None,
            frame: None,
            texture: None,
            texture_frame: None,
            metrics: MediaPreviewMetrics::default(),
        }
    }
}

impl fmt::Debug for MediaPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MediaPreview")
            .field("source_id", &self.source_id)
            .field(
                "open_asset",
                &self.open_source.as_ref().map(|source| source.asset_id),
            )
            .field("active_clip_id", &self.active_clip_id)
            .field("active_asset_id", &self.active_asset_id)
            .field("status", &self.status)
            .field("has_frame", &self.frame.is_some())
            .field("has_texture", &self.texture.is_some())
            .field("pending", &self.is_pending())
            .field("metrics", &self.metrics)
            .finish()
    }
}

impl MediaPreview {
    pub fn status(&self) -> &ClipPreviewStatus {
        &self.status
    }

    pub fn frame(&self) -> Option<&DecodedFrame> {
        self.frame.as_ref()
    }

    pub fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }

    pub fn probe(&self) -> Option<&MediaProbe> {
        match self.open_source.as_ref().map(|source| &source.state) {
            Some(SourceState::Ready(probe)) => Some(probe),
            _ => None,
        }
    }

    pub fn active_clip_id(&self) -> Option<ClipId> {
        self.active_clip_id
    }

    pub fn active_asset_id(&self) -> Option<MediaAssetId> {
        self.active_asset_id
    }

    pub fn active_playhead(&self) -> Option<TimelineTicks> {
        self.active_playhead
    }

    pub fn metrics(&self) -> MediaPreviewMetrics {
        self.metrics
    }

    pub fn is_pending(&self) -> bool {
        self.active.is_some()
            || matches!(
                self.status,
                ClipPreviewStatus::LoadingAsset | ClipPreviewStatus::Seeking
            )
    }

    pub fn reset_for_project(&mut self) {
        self.bump_source();
        if let Some(engine) = &self.engine {
            let _ = engine.clear();
        }
        self.project_id = None;
        self.open_source = None;
        self.active_clip_id = None;
        self.active_asset_id = None;
        self.active_playhead = None;
        self.reset_request();
        self.invalidate_frame();
        self.status = ClipPreviewStatus::NoProject;
    }

    pub fn clear(&mut self) {
        self.reset_for_project();
    }

    /// Resolve the active clip and request its frame. Never blocks.
    pub fn sync(&mut self, session: &EditorSession, library: &MediaLibraryState) {
        let project_id = session
            .native_document()
            .map(|document| document.project_id().clone());
        if self.project_id != project_id {
            self.reset_for_project();
            self.project_id = project_id;
        }
        if session.timeline_display_mode != crate::timeline_state::TimelineDisplayMode::Project {
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::NoProject);
            return;
        }
        let Some(native) = session.native_timeline.as_ref() else {
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::NoProject);
            return;
        };
        let project = native.project();
        let playhead = session.playhead_ticks();
        let mut active = Vec::new();
        for track in project.tracks() {
            for clip in track.clips() {
                if clip.kind() != ClipKind::Audio
                    && clip.range().start() <= playhead
                    && playhead < clip.range().end()
                {
                    active.push((track.id(), clip.clone()));
                }
            }
        }
        if active.len() > 1 {
            self.active_clip_id = None;
            self.active_asset_id = None;
            self.active_playhead = Some(playhead);
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::UnsupportedComposition);
            return;
        }
        let Some((_track_id, clip)) = active.pop() else {
            self.active_clip_id = None;
            self.active_asset_id = None;
            self.active_playhead = Some(playhead);
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::NoActiveClip);
            return;
        };
        if clip.kind() != ClipKind::Video || clip.nested() {
            self.active_clip_id = Some(clip.id());
            self.active_asset_id = clip.media_asset_id();
            self.active_playhead = Some(playhead);
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::UnsupportedVisualLayer);
            return;
        }
        let Some(asset_id) = clip.media_asset_id() else {
            self.active_clip_id = Some(clip.id());
            self.active_asset_id = None;
            self.active_playhead = Some(playhead);
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::ClipWithoutMedia);
            return;
        };
        let Some(asset) = project.asset(asset_id) else {
            self.active_clip_id = Some(clip.id());
            self.active_asset_id = Some(asset_id);
            self.active_playhead = Some(playhead);
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::Failed(
                "The clip references a missing media asset.".to_owned(),
            ));
            return;
        };
        match library.availability(asset_id) {
            Some(AssetAvailability::Missing) => {
                self.active_clip_id = Some(clip.id());
                self.active_asset_id = Some(asset_id);
                self.active_playhead = Some(playhead);
                self.clear_frame_only();
                self.set_status(ClipPreviewStatus::AssetMissing);
                return;
            }
            Some(AssetAvailability::Unreadable(message)) => {
                self.active_clip_id = Some(clip.id());
                self.active_asset_id = Some(asset_id);
                self.active_playhead = Some(playhead);
                self.clear_frame_only();
                self.set_status(ClipPreviewStatus::AssetUnreadable(message.clone()));
                return;
            }
            Some(AssetAvailability::Changed(message)) => {
                self.active_clip_id = Some(clip.id());
                self.active_asset_id = Some(asset_id);
                self.active_playhead = Some(playhead);
                self.clear_frame_only();
                self.set_status(ClipPreviewStatus::AssetChanged(message.clone()));
                return;
            }
            _ => {}
        }
        let Some(path) = library.resolved_path(asset_id).map(Path::to_path_buf) else {
            self.active_clip_id = Some(clip.id());
            self.active_asset_id = Some(asset_id);
            self.active_playhead = Some(playhead);
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::AssetMissing);
            return;
        };
        self.active_clip_id = Some(clip.id());
        self.active_asset_id = Some(asset_id);
        self.active_playhead = Some(playhead);
        if let Err(error) = self.ensure_source(asset_id, asset.locator(), asset.metadata(), &path) {
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::Failed(error));
            return;
        }
        let source_state = self.open_source.as_ref().map(|open| match &open.state {
            SourceState::Probing => None,
            SourceState::Failed(message) => Some(Err(message.clone())),
            SourceState::Ready(probe) => Some(Ok(probe.clone())),
        });
        let probe = match source_state {
            None => {
                self.set_status(ClipPreviewStatus::LoadingAsset);
                return;
            }
            Some(None) => {
                self.invalidate_frame();
                self.set_status(ClipPreviewStatus::LoadingAsset);
                return;
            }
            Some(Some(Err(message))) => {
                self.invalidate_frame();
                self.set_status(ClipPreviewStatus::Failed(message));
                return;
            }
            Some(Some(Ok(probe))) => probe,
        };
        if !probe_matches_asset(&probe, asset.metadata()) {
            self.clear_frame_only();
            self.set_status(ClipPreviewStatus::AssetChanged(
                "The media file changed after it was registered. Relink is required.".to_owned(),
            ));
            return;
        }
        let plan = match plan_clip_frame(&clip, playhead, &probe) {
            Ok(plan) => plan,
            Err(message) => {
                self.clear_frame_only();
                self.set_status(ClipPreviewStatus::Failed(message));
                return;
            }
        };
        let (source_ticks, target) = match plan {
            FramePlan::AtOrAfterMediaEnd => {
                self.clear_frame_only();
                self.set_status(ClipPreviewStatus::EndOfMedia);
                return;
            }
            FramePlan::Ready {
                source_ticks,
                target,
            } => (source_ticks, target),
        };
        let requested = RequestedTarget {
            target,
            clip_id: clip.id(),
            asset_id,
        };
        if self.requested.as_ref() == Some(&requested) {
            if self.active.is_some() {
                self.set_status(ClipPreviewStatus::Seeking);
            } else if self.frame.is_some() {
                self.set_status(ClipPreviewStatus::Frame);
            }
            return;
        }
        self.invalidate_frame();
        self.requested = Some(requested);
        let generation = self.next_generation();
        let requested_at = Instant::now();
        self.active = Some(ActiveRequest {
            generation,
            target,
            clip_id: clip.id(),
            asset_id,
            requested_at,
        });
        self.metrics.frame_requests = self.metrics.frame_requests.saturating_add(1);
        self.metrics.max_outstanding_requests = self.metrics.max_outstanding_requests.max(1);
        self.set_status(ClipPreviewStatus::Seeking);
        media_log(format_args!(
            "request source={} generation={generation} clip={} asset={} playhead={} source_ticks={} target={} tb={}",
            self.source_id,
            clip.id().get(),
            asset_id.get(),
            playhead.get(),
            source_ticks.get(),
            target.ticks,
            target.timebase
        ));
        if let Some(engine) = &self.engine {
            let request = FrameRequest {
                source_id: self.source_id,
                generation,
                clip_tag: clip.id().get(),
                asset_tag: asset_id.get(),
                stream_index: probe.video_stream_index,
                target,
            };
            if let Err(error) = engine.request(request) {
                self.active = None;
                self.set_status(ClipPreviewStatus::Failed(error.to_string()));
            }
        }
    }

    pub fn poll(&mut self, library: &mut MediaLibraryState) -> bool {
        let Some(event) = self.engine.as_ref().and_then(PreviewEngine::poll) else {
            return false;
        };
        self.handle_event(event, library)
    }

    pub fn ensure_texture(&mut self, context: &Context) -> bool {
        let Some(frame) = self.frame.as_ref() else {
            self.texture = None;
            self.texture_frame = None;
            return false;
        };
        let identity = (frame.source_id, frame.generation);
        if self.texture_frame == Some(identity) && self.texture.is_some() {
            return false;
        }
        if frame.format != PixelFormat::Rgba8 {
            self.invalidate_frame();
            self.set_status(ClipPreviewStatus::Failed(
                "the decoder returned an unsupported pixel format".to_owned(),
            ));
            return false;
        }
        let Ok(width) = usize::try_from(frame.width) else {
            self.invalidate_frame();
            return false;
        };
        let Ok(height) = usize::try_from(frame.height) else {
            self.invalidate_frame();
            return false;
        };
        let expected = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4));
        if width == 0 || height == 0 || expected != Some(frame.rgba.len()) {
            let width = frame.width;
            let height = frame.height;
            self.invalidate_frame();
            self.set_status(ClipPreviewStatus::Failed(format!(
                "decoded frame dimensions {width}x{height} do not match the RGBA buffer"
            )));
            return false;
        }
        let image = ColorImage::from_rgba_unmultiplied([width, height], &frame.rgba);
        let started = Instant::now();
        match &mut self.texture {
            Some(texture) => texture.set(image, TextureOptions::LINEAR),
            None => {
                self.texture =
                    Some(context.load_texture(TEXTURE_NAME, image, TextureOptions::LINEAR));
            }
        }
        let elapsed = started.elapsed().as_millis() as u64;
        self.texture_frame = Some(identity);
        self.metrics.texture_uploads = self.metrics.texture_uploads.saturating_add(1);
        self.metrics.last_texture_submit_ms = Some(elapsed);
        media_log(format_args!(
            "texture source={} generation={} {}x{} submit_ms={elapsed}",
            frame.source_id, frame.generation, frame.width, frame.height
        ));
        true
    }

    fn ensure_source(
        &mut self,
        asset_id: MediaAssetId,
        locator: &MediaLocator,
        metadata: &MediaAssetMetadata,
        path: &Path,
    ) -> Result<(), String> {
        if self
            .open_source
            .as_ref()
            .is_some_and(|source| source.matches(asset_id, locator, metadata, path))
        {
            return Ok(());
        }
        if self.engine.is_none() {
            self.engine = Some(PreviewEngine::spawn().map_err(|error| error.to_string())?);
        }
        self.bump_source();
        self.active = None;
        self.requested = None;
        self.invalidate_frame();
        self.open_source = Some(OpenSource {
            asset_id,
            locator: locator.clone(),
            metadata: metadata.clone(),
            path: path.to_path_buf(),
            state: SourceState::Probing,
        });
        self.set_status(ClipPreviewStatus::LoadingAsset);
        self.metrics.source_opens = self.metrics.source_opens.saturating_add(1);
        media_log(format_args!(
            "open source={} asset={} path={}",
            self.source_id,
            asset_id.get(),
            path.display()
        ));
        let opened = match &self.engine {
            Some(engine) => engine
                .open_source(self.source_id, path.to_path_buf())
                .map_err(|error| error.to_string()),
            None => Ok(()),
        };
        if let Err(message) = opened {
            if let Some(open) = self.open_source.as_mut() {
                open.state = SourceState::Failed(message.clone());
            }
            return Err(message);
        }
        Ok(())
    }

    fn handle_event(&mut self, event: PreviewEvent, library: &mut MediaLibraryState) -> bool {
        match event {
            PreviewEvent::Probe {
                source_id,
                elapsed_ms,
                result,
            } => {
                if source_id != self.source_id {
                    self.metrics.stale_results_ignored =
                        self.metrics.stale_results_ignored.saturating_add(1);
                    return false;
                }
                self.metrics.probe_latency_ms = Some(elapsed_ms);
                let Some(open) = self.open_source.as_mut() else {
                    return false;
                };
                match result {
                    Ok(probe) => {
                        if !probe_matches_asset(&probe, &open.metadata) {
                            let message = "The media file changed after it was registered. Relink is required.".to_owned();
                            open.state = SourceState::Failed(message.clone());
                            library.note_asset_changed(open.asset_id, message.clone());
                            self.invalidate_frame();
                            self.reset_request();
                            self.set_status(ClipPreviewStatus::AssetChanged(message));
                            return true;
                        }
                        let asset_id = open.asset_id;
                        open.state = SourceState::Ready(probe);
                        library.note_asset_available(asset_id);
                        self.set_status(ClipPreviewStatus::Seeking);
                        media_log(format_args!(
                            "probe source={source_id} elapsed_ms={elapsed_ms}"
                        ));
                        true
                    }
                    Err(error) => {
                        let asset_id = open.asset_id;
                        let missing = std::fs::metadata(&open.path).is_err();
                        let message = error.to_string();
                        open.state = SourceState::Failed(message.clone());
                        library.note_asset_unreadable(asset_id, message.clone());
                        self.invalidate_frame();
                        self.reset_request();
                        self.set_status(if missing {
                            ClipPreviewStatus::AssetMissing
                        } else {
                            ClipPreviewStatus::AssetUnreadable(message)
                        });
                        true
                    }
                }
            }
            PreviewEvent::Frame {
                source_id,
                generation,
                result,
            } => {
                let Some(active) = self.active.as_ref() else {
                    self.metrics.stale_results_ignored =
                        self.metrics.stale_results_ignored.saturating_add(1);
                    return false;
                };
                if source_id != self.source_id
                    || generation != active.generation
                    || Some(active.clip_id) != self.active_clip_id
                    || Some(active.asset_id) != self.active_asset_id
                {
                    self.metrics.stale_results_ignored =
                        self.metrics.stale_results_ignored.saturating_add(1);
                    media_log(format_args!(
                        "stale frame ignored source={source_id} generation={generation} current_source={} current_generation={}",
                        self.source_id, active.generation
                    ));
                    return false;
                }
                let target = active.target;
                let clip_id = active.clip_id;
                let asset_id = active.asset_id;
                let elapsed_ms = active.requested_at.elapsed().as_millis() as u64;
                self.active = None;
                self.metrics.last_seek_latency_ms = Some(elapsed_ms);
                match result {
                    Ok(frame) => {
                        if frame.source_id != source_id
                            || frame.generation != generation
                            || frame.requested_target != target
                            || frame.clip_tag != clip_id.get()
                            || frame.asset_tag != asset_id.get()
                        {
                            self.invalidate_frame();
                            self.set_status(ClipPreviewStatus::Failed(
                                "decoder returned a frame for a different request".to_owned(),
                            ));
                            return true;
                        }
                        self.metrics.last_decode_elapsed_ms = Some(frame.decode_elapsed_ms);
                        self.metrics.decoded_bytes = self
                            .metrics
                            .decoded_bytes
                            .saturating_add(frame.rgba.len() as u64);
                        media_log(format_args!(
                            "frame source={source_id} generation={generation} clip={} asset={} pts={} tb={} {}x{} bytes={} decode_ms={} seek_ms={elapsed_ms}",
                            frame.clip_tag,
                            frame.asset_tag,
                            frame.pts.ticks,
                            frame.pts.timebase,
                            frame.width,
                            frame.height,
                            frame.rgba.len(),
                            frame.decode_elapsed_ms
                        ));
                        self.frame = Some(frame);
                        self.set_status(ClipPreviewStatus::Frame);
                        true
                    }
                    Err(error) => {
                        self.invalidate_frame();
                        let status = match error {
                            MediaError::EndOfStream => ClipPreviewStatus::EndOfMedia,
                            other => ClipPreviewStatus::Failed(other.to_string()),
                        };
                        self.set_status(status);
                        true
                    }
                }
            }
        }
    }

    fn bump_source(&mut self) {
        self.source_id = self.source_id.wrapping_add(1).max(1);
    }

    fn next_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        self.next_generation
    }

    fn reset_request(&mut self) {
        self.requested = None;
        self.active = None;
    }

    fn clear_frame_only(&mut self) {
        self.reset_request();
        self.invalidate_frame();
    }

    fn invalidate_frame(&mut self) {
        self.frame = None;
        self.texture = None;
        self.texture_frame = None;
    }

    fn set_status(&mut self, status: ClipPreviewStatus) -> bool {
        if self.status == status {
            return false;
        }
        self.status = status;
        true
    }
}

fn plan_clip_frame(
    clip: &Clip,
    playhead: TimelineTicks,
    probe: &MediaProbe,
) -> Result<FramePlan, String> {
    if playhead < clip.range().start() || playhead >= clip.range().end() {
        return Err("The playhead is outside the active clip.".to_owned());
    }
    let source = clip
        .source()
        .ok_or_else(|| "The active clip has no source mapping.".to_owned())?;
    let (numerator, denominator) = source.speed();
    if numerator != denominator {
        return Err(format!(
            "Clip speed {numerator}/{denominator} is not supported by M3-D Preview."
        ));
    }
    if source.timebase().ticks_per_second != TIMELINE_TICKS_PER_SECOND {
        return Err("The clip source timebase is not supported by M3-D Preview.".to_owned());
    }
    let delta = playhead
        .get()
        .checked_sub(clip.range().start().get())
        .ok_or_else(|| "Playhead offset overflowed.".to_owned())?;
    let source_tick_value = source
        .used()
        .start()
        .get()
        .checked_add(delta)
        .ok_or_else(|| "Source time overflowed.".to_owned())?;
    if source_tick_value < source.used().start().get()
        || source_tick_value >= source.used().end().get()
    {
        return Err("The playhead maps outside the clip source range.".to_owned());
    }
    let source_ticks = SourceTicks::new(source_tick_value);
    let target = source_ticks_to_media_timestamp(source_ticks, probe.stream_start)
        .map_err(|error| error.to_string())?;
    if probe
        .end_timestamp()
        .is_some_and(|end| target.ticks >= end.ticks)
    {
        return Ok(FramePlan::AtOrAfterMediaEnd);
    }
    Ok(FramePlan::Ready {
        source_ticks,
        target,
    })
}

fn media_log(arguments: fmt::Arguments<'_>) {
    if std::env::var_os("CUTCRAFT_MEDIA_LOG").is_some() {
        eprintln!("[cutcraft-media] {arguments}");
    }
}

#[cfg(test)]
impl MediaPreview {
    pub(crate) fn inject_displayed_frame_for_test(&mut self, frame: DecodedFrame) {
        self.source_id = frame.source_id;
        self.frame = Some(frame);
        self.status = ClipPreviewStatus::Frame;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use capcut_core::time::{FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange};
    use capcut_engine::project::{ClipKind, ClipSpec, Project, SourceMapping, TrackKind};
    use capcut_media::{MediaRational as ProbeRational, MediaTimebase};

    use super::*;

    fn probe() -> MediaProbe {
        let timebase = MediaTimebase::new(1, 15_360).expect("timebase");
        MediaProbe {
            path: PathBuf::from("clip.mp4"),
            container: "mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase,
            nominal_frame_rate: Some(ProbeRational::new(30, 1).unwrap()),
            average_frame_rate: Some(ProbeRational::new(30, 1).unwrap()),
            duration: Some(MediaTimestamp::new(5 * 15_360, timebase)),
            stream_start: MediaTimestamp::new(0, timebase),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: Some(150),
            probe_elapsed_ms: 1,
        }
    }

    fn clip(start: i64, end: i64, source_start: i64) -> Clip {
        use capcut_engine::asset::{
            MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
        };
        let duration = end - start;
        let mut project = Project::named(FrameRate::new(30, 1).unwrap(), "Clip").expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        let metadata = MediaAssetMetadata {
            container: "mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase_numerator: 1,
            timebase_denominator: 15_360,
            stream_start_ticks: 0,
            duration_ticks: Some(5 * 15_360),
            nominal_frame_rate: MediaRational::new(30, 1),
            average_frame_rate: MediaRational::new(30, 1),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: None,
        };
        let registration = MediaAssetRegistration::video(
            "clip.mp4",
            MediaLocator::absolute("C:/media/clip.mp4").expect("locator"),
            metadata,
        )
        .expect("asset");
        let asset = project.add_asset(registration).expect("add asset");
        let source = SourceMapping::new(
            SourceRange::new(
                SourceTicks::new(source_start),
                SourceTicks::new(source_start + 600_000),
            )
            .unwrap(),
            SourceRange::new(
                SourceTicks::new(source_start),
                SourceTicks::new(source_start + duration),
            )
            .unwrap(),
            Timebase::new(120_000).unwrap(),
            1,
            1,
        )
        .unwrap();
        let clip_id = project
            .add_clip(
                track,
                ClipSpec {
                    name: "A".to_owned(),
                    kind: ClipKind::Video,
                    range: TimelineRange::new(TimelineTicks::new(start), TimelineTicks::new(end))
                        .unwrap(),
                    source: Some(source),
                    media_asset_id: Some(asset),
                    linked_clip_ids: BTreeSet::new(),
                    nested: false,
                },
            )
            .expect("clip");
        project.clip(clip_id).expect("stored clip").clone()
    }

    #[test]
    fn clip_plan_maps_trim_exclusive_end_and_media_end() {
        let clip = clip(120_000, 240_000, 0);
        let plan = plan_clip_frame(&clip, TimelineTicks::new(120_000), &probe()).unwrap();
        assert!(matches!(
            plan,
            FramePlan::Ready {
                source_ticks,
                target
            } if source_ticks == SourceTicks::ZERO && target.ticks == 0
        ));
        let plan = plan_clip_frame(&clip, TimelineTicks::new(180_000), &probe()).unwrap();
        assert!(matches!(
            plan,
            FramePlan::Ready { target, .. } if target.ticks == 7_680
        ));
        assert!(plan_clip_frame(&clip, TimelineTicks::new(240_000), &probe()).is_err());
    }

    #[test]
    fn active_clip_collection_is_half_open_and_detects_multiple_layers() {
        let mut project =
            Project::named(FrameRate::new(30, 1).unwrap(), "Preview").expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        let make = |name: &str, start: i64, end: i64| ClipSpec {
            name: name.to_owned(),
            kind: ClipKind::Video,
            range: TimelineRange::new(TimelineTicks::new(start), TimelineTicks::new(end)).unwrap(),
            source: Some(
                SourceMapping::new(
                    SourceRange::new(SourceTicks::ZERO, SourceTicks::new(end - start)).unwrap(),
                    SourceRange::new(SourceTicks::ZERO, SourceTicks::new(end - start)).unwrap(),
                    Timebase::new(120_000).unwrap(),
                    1,
                    1,
                )
                .unwrap(),
            ),
            media_asset_id: None,
            linked_clip_ids: BTreeSet::new(),
            nested: false,
        };
        project.add_clip(track, make("A", 0, 120_000)).expect("A");
        project
            .add_clip(track, make("B", 120_000, 240_000))
            .expect("B");
        fn active_at(project: &Project, ticks: i64) -> usize {
            project
                .tracks()
                .iter()
                .flat_map(|track| track.clips())
                .filter(|clip| {
                    clip.range().start() <= TimelineTicks::new(ticks)
                        && TimelineTicks::new(ticks) < clip.range().end()
                })
                .count()
        }
        assert_eq!(active_at(&project, 119_999), 1);
        assert_eq!(active_at(&project, 120_000), 1);
        assert_eq!(active_at(&project, 240_000), 0);
        let other = project
            .add_track("V2", TrackKind::Video, false)
            .expect("track");
        project.add_clip(other, make("C", 0, 240_000)).expect("C");
        assert_eq!(active_at(&project, 60_000), 2);
    }

    fn session_with(project: Project) -> EditorSession {
        EditorSession {
            native_timeline: Some(
                crate::native_timeline::NativeTimeline::from_project(project).expect("native"),
            ),
            ..EditorSession::default()
        }
    }

    fn bare_clip(name: &str, start: i64, end: i64) -> ClipSpec {
        ClipSpec {
            name: name.to_owned(),
            kind: ClipKind::Video,
            range: TimelineRange::new(TimelineTicks::new(start), TimelineTicks::new(end)).unwrap(),
            source: Some(
                SourceMapping::new(
                    SourceRange::new(SourceTicks::ZERO, SourceTicks::new(end - start)).unwrap(),
                    SourceRange::new(SourceTicks::ZERO, SourceTicks::new(end - start)).unwrap(),
                    Timebase::new(120_000).unwrap(),
                    1,
                    1,
                )
                .unwrap(),
            ),
            media_asset_id: None,
            linked_clip_ids: BTreeSet::new(),
            nested: false,
        }
    }

    fn two_track_project() -> Project {
        let mut project =
            Project::named(FrameRate::new(30, 1).unwrap(), "Preview").expect("project");
        let first = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(first).expect("main");
        project
            .add_clip(first, bare_clip("A", 0, 120_000))
            .expect("A");
        let second = project
            .add_track("V2", TrackKind::Video, false)
            .expect("track");
        project
            .add_clip(second, bare_clip("B", 0, 120_000))
            .expect("B");
        project
    }

    #[test]
    fn preview_reports_composition_gap_and_missing_media_states() {
        let library = MediaLibraryState::default();

        let mut session = session_with(two_track_project());
        session.set_playhead_ticks(TimelineTicks::new(60_000));
        let mut preview = MediaPreview::default();
        preview.sync(&session, &library);
        assert_eq!(preview.status(), &ClipPreviewStatus::UnsupportedComposition);
        assert_eq!(preview.active_clip_id(), None);

        let mut single =
            Project::named(FrameRate::new(30, 1).unwrap(), "Preview").expect("project");
        let track = single
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        single.assign_main_track(track).expect("main");
        single
            .add_clip(track, bare_clip("A", 0, 120_000))
            .expect("A");
        let mut session = session_with(single);
        session.set_playhead_ticks(TimelineTicks::new(60_000));
        let mut preview = MediaPreview::default();
        preview.sync(&session, &library);
        assert_eq!(preview.status(), &ClipPreviewStatus::ClipWithoutMedia);

        session.set_playhead_ticks(TimelineTicks::new(120_000));
        preview.sync(&session, &library);
        assert_eq!(preview.status(), &ClipPreviewStatus::NoActiveClip);
    }

    #[test]
    fn preview_reports_missing_asset_for_asset_backed_clip() {
        use capcut_engine::asset::{
            MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
        };
        let mut project =
            Project::named(FrameRate::new(30, 1).unwrap(), "Preview").expect("project");
        let track = project
            .add_track("V1", TrackKind::Video, false)
            .expect("track");
        project.assign_main_track(track).expect("main");
        let metadata = MediaAssetMetadata {
            container: "mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase_numerator: 1,
            timebase_denominator: 15_360,
            stream_start_ticks: 0,
            duration_ticks: Some(5 * 15_360),
            nominal_frame_rate: MediaRational::new(30, 1),
            average_frame_rate: MediaRational::new(30, 1),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: None,
        };
        let registration = MediaAssetRegistration::video(
            "clip.mp4",
            MediaLocator::absolute("C:/media/clip.mp4").expect("locator"),
            metadata,
        )
        .expect("asset");
        let asset = project.add_asset(registration).expect("add asset");
        let mut spec = bare_clip("A", 0, 120_000);
        spec.media_asset_id = Some(asset);
        project.add_clip(track, spec).expect("clip");
        let mut session = session_with(project);
        session.set_playhead_ticks(TimelineTicks::new(60_000));
        let library = MediaLibraryState::default();
        let mut preview = MediaPreview::default();
        preview.sync(&session, &library);
        assert_eq!(preview.status(), &ClipPreviewStatus::AssetMissing);
        assert_eq!(preview.active_asset_id(), Some(asset));
    }
}

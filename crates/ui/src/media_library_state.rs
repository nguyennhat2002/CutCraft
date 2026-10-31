//! Real Media Pool import, validation, availability and relink state.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use capcut_engine::asset::{
    MediaAssetId, MediaAssetKind, MediaAssetMetadata, MediaAssetRegistration, MediaLocator,
    MediaLocatorKind, MediaRational,
};
use capcut_media::{
    MediaError, MediaProbe, MediaRational as ProbeRational, ProbeQueue, ProbeRequest,
};
use capcut_project::{locator_for_import, paths_equivalent, resolve_media_locator};

use crate::editor_state::EditorSession;

const MAX_ASSET_NAME_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetAvailability {
    Checking,
    Available,
    Missing,
    Unreadable(String),
    Changed(String),
}

impl AssetAvailability {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Checking => "Checking...",
            Self::Available => "Available",
            Self::Missing => "Offline",
            Self::Unreadable(_) => "Unreadable",
            Self::Changed(_) => "Changed - relink",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Unreadable(message) | Self::Changed(message) => Some(message),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaDialogMode {
    Import,
    Relink(MediaAssetId),
}

#[derive(Debug, Clone)]
pub struct MediaDialogState {
    pub mode: MediaDialogMode,
    pub path: String,
    pub error: Option<String>,
    pub message: Option<String>,
    pub pending: bool,
    pub asset_name: Option<String>,
}

enum PendingProbe {
    Import,
    Relink(MediaAssetId),
    Validate(MediaAssetId, MediaAssetMetadata),
}

pub struct MediaLibraryState {
    queue: ProbeQueue,
    next_request_id: u64,
    dialog: Option<MediaDialogState>,
    pending: BTreeMap<u64, PendingProbe>,
    availability: BTreeMap<MediaAssetId, AssetAvailability>,
    resolved: BTreeMap<MediaAssetId, PathBuf>,
    project_key: Option<(capcut_project::ProjectId, PathBuf, u64)>,
    notice: Option<String>,
    metrics: MediaLibraryMetrics,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MediaLibraryMetrics {
    pub probe_requests: u64,
    pub validation_requests: u64,
    pub import_successes: u64,
    pub relink_successes: u64,
}

impl Default for MediaLibraryState {
    fn default() -> Self {
        Self {
            queue: ProbeQueue::spawn(),
            next_request_id: 1,
            dialog: None,
            pending: BTreeMap::new(),
            availability: BTreeMap::new(),
            resolved: BTreeMap::new(),
            project_key: None,
            notice: None,
            metrics: MediaLibraryMetrics::default(),
        }
    }
}

impl fmt::Debug for MediaLibraryState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MediaLibraryState")
            .field("dialog", &self.dialog)
            .field("availability", &self.availability)
            .field("metrics", &self.metrics)
            .finish()
    }
}

impl MediaLibraryState {
    pub fn reset(&mut self) {
        self.dialog = None;
        self.pending.clear();
        self.availability.clear();
        self.resolved.clear();
        self.project_key = None;
    }

    pub fn dialog(&self) -> Option<&MediaDialogState> {
        self.dialog.as_ref()
    }

    pub fn dialog_mut(&mut self) -> Option<&mut MediaDialogState> {
        self.dialog.as_mut()
    }

    pub fn availability(&self, asset: MediaAssetId) -> Option<&AssetAvailability> {
        self.availability.get(&asset)
    }

    pub fn resolved_path(&self, asset: MediaAssetId) -> Option<&Path> {
        self.resolved.get(&asset).map(PathBuf::as_path)
    }

    pub fn note_asset_changed(&mut self, asset: MediaAssetId, message: impl Into<String>) {
        self.availability
            .insert(asset, AssetAvailability::Changed(message.into()));
    }

    pub fn note_asset_unreadable(&mut self, asset: MediaAssetId, message: impl Into<String>) {
        self.availability
            .insert(asset, AssetAvailability::Unreadable(message.into()));
    }

    pub fn note_asset_available(&mut self, asset: MediaAssetId) {
        self.availability
            .insert(asset, AssetAvailability::Available);
    }

    pub fn is_pending(&self) -> bool {
        self.queue.is_pending()
    }

    pub fn metrics(&self) -> MediaLibraryMetrics {
        self.metrics
    }

    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    pub fn open_import_dialog(&mut self) {
        self.dialog = Some(MediaDialogState {
            mode: MediaDialogMode::Import,
            path: String::new(),
            error: None,
            message: None,
            pending: false,
            asset_name: None,
        });
    }

    pub fn open_relink_dialog(&mut self, session: &EditorSession, asset: MediaAssetId) {
        let name = session
            .native_timeline
            .as_ref()
            .and_then(|native| native.project().asset(asset))
            .map(|asset| asset.display_name().to_owned());
        let path = self
            .resolved
            .get(&asset)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.dialog = Some(MediaDialogState {
            mode: MediaDialogMode::Relink(asset),
            path,
            error: None,
            message: None,
            pending: false,
            asset_name: name,
        });
    }

    pub fn close_dialog(&mut self) {
        if self.dialog.is_some() {
            self.pending.retain(|_, pending| {
                !matches!(pending, PendingProbe::Import | PendingProbe::Relink(_))
            });
        }
        self.dialog = None;
    }

    pub fn start_dialog_probe(
        &mut self,
        session: &EditorSession,
        raw_path: &str,
    ) -> Result<(), String> {
        let Some(dialog) = self.dialog.as_ref() else {
            return Err("No media dialog is open.".to_owned());
        };
        if dialog.pending {
            return Err("A media probe is already running.".to_owned());
        }
        let mode = dialog.mode.clone();
        let raw = raw_path.trim();
        if raw.is_empty() {
            return self.set_dialog_error("Enter a local video file path.");
        }
        let absolute = match normalize_path(Path::new(raw)) {
            Ok(path) => path,
            Err(error) => return self.set_dialog_error(error),
        };
        let (request_id, kind) = if let MediaDialogMode::Relink(asset) = mode {
            (self.next_request(), PendingProbe::Relink(asset))
        } else {
            (self.next_request(), PendingProbe::Import)
        };
        if let Err(error) = self.queue.submit(ProbeRequest {
            request_id,
            path: absolute.clone(),
        }) {
            return self.set_dialog_error(error.to_string());
        }
        if let Some(dialog) = &mut self.dialog {
            dialog.pending = true;
            dialog.error = None;
            dialog.message = None;
            dialog.path = absolute.to_string_lossy().into_owned();
        }
        self.pending.insert(request_id, kind);
        self.metrics.probe_requests = self.metrics.probe_requests.saturating_add(1);
        let _ = session;
        Ok(())
    }

    pub fn update(&mut self, session: &mut EditorSession) -> bool {
        let mut changed = self.poll_probes(session);
        if self.refresh_assets(session) {
            changed = true;
        }
        changed
    }

    fn poll_probes(&mut self, session: &mut EditorSession) -> bool {
        let mut changed = false;
        while let Some(event) = self.queue.poll() {
            let Some(kind) = self.pending.remove(&event.request_id) else {
                continue;
            };
            match kind {
                PendingProbe::Import => {
                    let Some(dialog) = &mut self.dialog else {
                        continue;
                    };
                    let Some(project_path) = session
                        .native_document()
                        .and_then(|document| document.path())
                        .cloned()
                    else {
                        dialog.pending = false;
                        dialog.error = Some("This session has no managed project.".to_owned());
                        continue;
                    };
                    match event.result {
                        Ok(probe) => {
                            match self.register_imported_asset(
                                session,
                                &project_path,
                                &event.path,
                                &probe,
                            ) {
                                Ok(created) => {
                                    let message = if created {
                                        "Media imported and registered."
                                    } else {
                                        "This media file is already imported."
                                    };
                                    self.notice = Some(message.to_owned());
                                    self.dialog = None;
                                    if created {
                                        self.metrics.import_successes =
                                            self.metrics.import_successes.saturating_add(1);
                                    }
                                }
                                Err(error) => {
                                    if let Some(dialog) = &mut self.dialog {
                                        dialog.pending = false;
                                        dialog.error = Some(error);
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            if let Some(dialog) = &mut self.dialog {
                                dialog.pending = false;
                                dialog.error = Some(error.to_string());
                            }
                        }
                    }
                    changed = true;
                }
                PendingProbe::Relink(asset) => {
                    let Some(dialog) = &mut self.dialog else {
                        continue;
                    };
                    let Some(project_path) = session
                        .native_document()
                        .and_then(|document| document.path())
                        .cloned()
                    else {
                        dialog.pending = false;
                        dialog.error = Some("This session has no managed project.".to_owned());
                        continue;
                    };
                    match event.result {
                        Ok(probe) => {
                            let metadata = metadata_from_probe(&probe);
                            let locator = locator_for_import(Some(&project_path), &event.path)
                                .map_err(|error| error.to_string());
                            let size = std::fs::metadata(&event.path)
                                .ok()
                                .map(|metadata| metadata.len());
                            match locator.and_then(|locator| {
                                session
                                    .relink_media_asset(asset, locator, metadata, size)
                                    .map_err(|error| error.to_string())
                            }) {
                                Ok(_) => {
                                    self.notice = Some("Media relinked.".to_owned());
                                    self.metrics.relink_successes =
                                        self.metrics.relink_successes.saturating_add(1);
                                    self.dialog = None;
                                    self.project_key = None;
                                }
                                Err(error) => {
                                    if let Some(dialog) = &mut self.dialog {
                                        dialog.pending = false;
                                        dialog.error = Some(error);
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            if let Some(dialog) = &mut self.dialog {
                                dialog.pending = false;
                                dialog.error = Some(error.to_string());
                            }
                        }
                    }
                    changed = true;
                }
                PendingProbe::Validate(asset, expected) => {
                    let status = match event.result {
                        Ok(probe) => {
                            if probe_matches_asset(&probe, &expected) {
                                AssetAvailability::Available
                            } else {
                                AssetAvailability::Changed(
                                    "The file metadata differs from the saved asset. Relink is required."
                                        .to_owned(),
                                )
                            }
                        }
                        Err(error) if is_missing_probe_error(&error, &event.path) => {
                            AssetAvailability::Missing
                        }
                        Err(error) => AssetAvailability::Unreadable(error.to_string()),
                    };
                    self.availability.insert(asset, status);
                    changed = true;
                }
            }
        }
        changed
    }

    fn register_imported_asset(
        &mut self,
        session: &mut EditorSession,
        project_path: &Path,
        path: &Path,
        probe: &MediaProbe,
    ) -> Result<bool, String> {
        let absolute = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        for asset in session
            .native_timeline
            .as_ref()
            .map(|native| native.project().assets())
            .unwrap_or(&[])
        {
            if let Some(resolved) = self.resolved.get(&asset.id()) {
                if paths_equivalent(resolved, &absolute) {
                    if probe_matches_asset(probe, asset.metadata()) {
                        return Ok(false);
                    }
                    return Err(
                        "This file is already registered with different metadata. Use Relink on the existing asset."
                            .to_owned(),
                    );
                }
            }
        }
        let metadata = metadata_from_probe(probe);
        let locator =
            locator_for_import(Some(project_path), path).map_err(|error| error.to_string())?;
        let display_name = display_name_for_path(path);
        let mut registration = MediaAssetRegistration::video(display_name, locator, metadata)
            .map_err(|error| error.to_string())?;
        registration.file_size_bytes = std::fs::metadata(path).ok().map(|metadata| metadata.len());
        session.add_media_asset(registration)?;
        self.project_key = None;
        Ok(true)
    }

    fn refresh_assets(&mut self, session: &mut EditorSession) -> bool {
        let Some(native) = session.native_timeline.as_ref() else {
            let changed = !self.availability.is_empty();
            self.availability.clear();
            self.resolved.clear();
            self.project_key = None;
            return changed;
        };
        let Some(document) = session.native_document() else {
            return false;
        };
        let Some(project_path) = document.path() else {
            return false;
        };
        let project = native.project();
        let key = (
            document.project_id().clone(),
            project_path.clone(),
            project.revision(),
        );
        if self.project_key.as_ref() == Some(&key) {
            return false;
        }
        self.project_key = Some(key);
        self.availability.clear();
        self.resolved.clear();
        let mut validation_budget = 48usize;
        for asset in project.assets() {
            let resolved = match resolve_media_locator(project_path, asset.locator()) {
                Ok(path) => std::fs::canonicalize(&path).unwrap_or(path),
                Err(error) => {
                    self.availability
                        .insert(asset.id(), AssetAvailability::Unreadable(error.to_string()));
                    continue;
                }
            };
            if asset.locator().kind() == MediaLocatorKind::Absolute && !resolved.is_absolute() {
                self.availability.insert(
                    asset.id(),
                    AssetAvailability::Unreadable("asset locator is not absolute".to_owned()),
                );
                continue;
            }
            self.resolved.insert(asset.id(), resolved.clone());
            match std::fs::metadata(&resolved) {
                Ok(metadata) if metadata.is_file() => {
                    if validation_budget == 0 {
                        self.availability
                            .insert(asset.id(), AssetAvailability::Available);
                        continue;
                    }
                    validation_budget -= 1;
                    let request_id = self.next_request();
                    self.availability
                        .insert(asset.id(), AssetAvailability::Checking);
                    self.pending.insert(
                        request_id,
                        PendingProbe::Validate(asset.id(), asset.metadata().clone()),
                    );
                    self.metrics.validation_requests =
                        self.metrics.validation_requests.saturating_add(1);
                    if self
                        .queue
                        .submit(ProbeRequest {
                            request_id,
                            path: resolved,
                        })
                        .is_err()
                    {
                        self.pending.remove(&request_id);
                        self.availability
                            .insert(asset.id(), AssetAvailability::Available);
                    }
                }
                Ok(_) => {
                    self.availability.insert(
                        asset.id(),
                        AssetAvailability::Unreadable("asset path is not a file".to_owned()),
                    );
                }
                Err(_) => {
                    self.availability
                        .insert(asset.id(), AssetAvailability::Missing);
                }
            }
        }
        true
    }

    fn next_request(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        id
    }

    fn set_dialog_error<T>(&mut self, error: impl Into<String>) -> Result<T, String> {
        let error = error.into();
        if let Some(dialog) = &mut self.dialog {
            dialog.error = Some(error.clone());
        }
        Err(error)
    }
}

fn normalize_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|error| format!("Could not resolve the media path: {error}"))
    }
}

fn display_name_for_path(path: &Path) -> String {
    let raw = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Imported media".to_owned());
    let mut name = String::new();
    for character in raw.chars() {
        if character.is_control() {
            continue;
        }
        if name.len() + character.len_utf8() > MAX_ASSET_NAME_BYTES {
            break;
        }
        name.push(character);
    }
    if name.is_empty() {
        "Imported media".to_owned()
    } else {
        name
    }
}

fn metadata_from_probe(probe: &MediaProbe) -> MediaAssetMetadata {
    MediaAssetMetadata {
        container: probe.container.clone(),
        video_stream_index: probe.video_stream_index,
        codec: probe.codec.clone(),
        width: probe.width,
        height: probe.height,
        pixel_format: probe.pixel_format.clone(),
        timebase_numerator: probe.timebase.numerator,
        timebase_denominator: probe.timebase.denominator,
        stream_start_ticks: probe.stream_start.ticks,
        duration_ticks: probe.duration.map(|duration| duration.ticks),
        nominal_frame_rate: probe.nominal_frame_rate.and_then(convert_rate),
        average_frame_rate: probe.average_frame_rate.and_then(convert_rate),
        rotation_degrees: probe.rotation_degrees,
        has_audio: probe.has_audio,
        frame_count_hint: probe.frame_count_hint,
    }
}

fn convert_rate(rate: ProbeRational) -> Option<MediaRational> {
    MediaRational::new(rate.numerator, rate.denominator)
}

pub(crate) fn probe_matches_asset(probe: &MediaProbe, expected: &MediaAssetMetadata) -> bool {
    let live = metadata_from_probe(probe);
    live.video_stream_index == expected.video_stream_index
        && live.codec.eq_ignore_ascii_case(&expected.codec)
        && live.width == expected.width
        && live.height == expected.height
        && live.timebase_numerator == expected.timebase_numerator
        && live.timebase_denominator == expected.timebase_denominator
        && live.stream_start_ticks == expected.stream_start_ticks
        && live.duration_ticks == expected.duration_ticks
        && live.nominal_frame_rate == expected.nominal_frame_rate
        && live.average_frame_rate == expected.average_frame_rate
        && live.rotation_degrees == expected.rotation_degrees
}

fn is_missing_probe_error(error: &MediaError, path: &Path) -> bool {
    if matches!(error, MediaError::Io(_)) {
        return std::fs::metadata(path).is_err();
    }
    false
}

pub fn asset_is_playable(availability: Option<&AssetAvailability>) -> bool {
    matches!(
        availability,
        None | Some(AssetAvailability::Available) | Some(AssetAvailability::Checking)
    )
}

pub fn asset_kind_is_video(kind: MediaAssetKind) -> bool {
    matches!(kind, MediaAssetKind::Video)
}

pub fn asset_locator_is_supported(locator: &MediaLocator) -> bool {
    matches!(
        locator.kind(),
        MediaLocatorKind::ProjectRelative | MediaLocatorKind::Absolute
    )
}

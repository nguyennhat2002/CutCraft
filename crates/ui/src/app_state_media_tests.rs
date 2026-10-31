//! Real-media managed workflow: import, append, clip-aware Preview, AutoSave,
//! reopen, offline detection and relink. Requires discoverable FFmpeg tools.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use capcut_core::time::TimelineTicks;
use capcut_engine::asset::{MediaAssetId, MediaLocatorKind};
use capcut_media::source_ticks_to_media_timestamp;
use capcut_project::ProjectLibrary;

use super::*;
use crate::media_library_state::AssetAvailability;
use crate::media_preview_state::ClipPreviewStatus;

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-media-ui-{tag}-{}-{}",
            std::process::id(),
            unix_millis_now()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ffmpeg_available() -> bool {
    let ffmpeg = std::env::var_os("CUTCRAFT_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let ffprobe = std::env::var_os("CUTCRAFT_FFPROBE").unwrap_or_else(|| "ffprobe".into());
    [ffmpeg, ffprobe].into_iter().all(|tool| {
        Command::new(tool)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

fn generate_video(dir: &TempDir, name: &str, color: &str) -> Option<PathBuf> {
    let ffmpeg = std::env::var_os("CUTCRAFT_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let path = dir.path(name);
    let output = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s=320x180:d=1:r=30"),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "15",
            "-bf",
            "0",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&path)
        .output()
        .ok()?;
    output.status.success().then_some(path)
}

fn managed_state(tag: &str) -> (AppState, TempDir, egui::Context) {
    let dir = TempDir::new(tag);
    let mut state = AppState::default();
    state.install_project_library(ProjectLibrary::new(&dir.0));
    let context = egui::Context::default();
    crate::theme::configure(&context);
    (state, dir, context)
}

fn pump_auto_save(
    state: &mut AppState,
    context: &egui::Context,
    start_ms: u64,
    mut predicate: impl FnMut(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut step = 0u64;
    loop {
        state.update_auto_save_at(context, start_ms + step * 20);
        if predicate(state) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for AutoSave state"
        );
        std::thread::sleep(Duration::from_millis(10));
        step += 1;
    }
}

fn pump_media(
    state: &mut AppState,
    context: &egui::Context,
    mut predicate: impl FnMut(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        state.update_media_library(context);
        state.update_media_preview(context);
        if predicate(state) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for media state: preview={:?} dialog={:?}",
            state.media_preview().status(),
            state
                .media_library()
                .dialog()
                .map(|dialog| (dialog.pending, dialog.error.clone()))
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn create_project(state: &mut AppState, context: &egui::Context) {
    state.start_new_managed_project().expect("start creation");
    pump_auto_save(state, context, 0, |state| {
        state.screen() == Screen::Editor && state.session().is_some()
    });
}

fn import_video(state: &mut AppState, context: &egui::Context, path: &Path) -> MediaAssetId {
    state.open_media_import_dialog();
    state.media_library_mut().dialog_mut().expect("dialog").path =
        path.to_string_lossy().into_owned();
    state.confirm_media_import().expect("start import");
    pump_media(state, context, |state| {
        state.media_library().dialog().is_none()
            && state
                .session()
                .and_then(|session| session.native_timeline.as_ref())
                .is_some_and(|native| {
                    native.project().assets().iter().any(|asset| {
                        state
                            .media_library()
                            .resolved_path(asset.id())
                            .is_some_and(|resolved| {
                                capcut_project::paths_equivalent(resolved, path)
                            })
                    })
                })
    });
    let session = state.session().expect("session");
    let native = session.native_timeline.as_ref().expect("native");
    native
        .project()
        .assets()
        .iter()
        .find(|asset| {
            state
                .media_library()
                .resolved_path(asset.id())
                .is_some_and(|resolved| capcut_project::paths_equivalent(resolved, path))
        })
        .map(|asset| asset.id())
        .expect("imported asset")
}

struct ClipInfo {
    id: capcut_engine::project::ClipId,
    asset: MediaAssetId,
    start: i64,
    end: i64,
    source_start: i64,
}

fn main_clips(state: &AppState) -> Vec<ClipInfo> {
    let project = state
        .session()
        .and_then(|session| session.native_timeline.as_ref())
        .map(|native| native.project())
        .expect("project");
    project
        .main_track()
        .expect("main track")
        .clips()
        .iter()
        .map(|clip| ClipInfo {
            id: clip.id(),
            asset: clip.media_asset_id().expect("asset-backed clip"),
            start: clip.range().start().get(),
            end: clip.range().end().get(),
            source_start: clip.source().expect("source").used().start().get(),
        })
        .collect()
}

fn wait_for_frame(
    state: &mut AppState,
    context: &egui::Context,
    clip: capcut_engine::project::ClipId,
    asset: MediaAssetId,
) -> (i64, i64) {
    pump_media(state, context, |state| {
        matches!(state.media_preview().status(), ClipPreviewStatus::Frame)
            && state
                .media_preview()
                .frame()
                .is_some_and(|frame| frame.clip_tag == clip.get() && frame.asset_tag == asset.get())
    });
    let frame = state.media_preview().frame().expect("frame");
    (frame.pts.ticks, frame.requested_target.ticks)
}

fn assert_no_media_copied(project_path: &Path) {
    let root = project_path.parent().expect("project directory");
    fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"))
            {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(root, &mut found);
    assert!(
        found.is_empty(),
        "media was copied into the project: {found:?}"
    );
}

#[test]
fn managed_media_workflow_imports_previews_reopens_offline_and_relinks() {
    if !ffmpeg_available() {
        eprintln!("skipping managed media workflow: FFmpeg tools unavailable");
        return;
    }
    let (mut state, media_dir, context) = managed_state("workflow");
    let Some(video_a) = generate_video(&media_dir, "clïp á.mp4", "red") else {
        eprintln!("skipping managed media workflow: fixture generation failed");
        return;
    };
    let Some(video_b) = generate_video(&media_dir, "clïp b.mp4", "blue") else {
        eprintln!("skipping managed media workflow: fixture generation failed");
        return;
    };
    create_project(&mut state, &context);

    let asset_a = import_video(&mut state, &context, &video_a);
    let asset_b = import_video(&mut state, &context, &video_b);
    assert_ne!(asset_a, asset_b);
    pump_media(&mut state, &context, |state| {
        [asset_a, asset_b].iter().all(|asset| {
            matches!(
                state.media_library().availability(*asset),
                Some(AssetAvailability::Available)
            )
        })
    });

    {
        let session = state.session_mut().expect("session");
        let project_path = session
            .native_document()
            .and_then(|document| document.path())
            .expect("managed path")
            .to_path_buf();
        assert_no_media_copied(&project_path);
    }
    state.append_media_asset(asset_a).expect("append A");
    state.append_media_asset(asset_b).expect("append B");
    let clips = main_clips(&state);
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].asset, asset_a);
    assert_eq!(clips[1].asset, asset_b);
    assert_eq!(clips[0].start, 0);
    assert_eq!(clips[1].start, clips[0].end);
    let locator_kind = state
        .session()
        .and_then(|session| session.native_timeline.as_ref())
        .and_then(|native| native.project().asset(asset_a))
        .map(|asset| asset.locator().kind())
        .expect("asset A");
    assert_eq!(locator_kind, MediaLocatorKind::Absolute);

    let playhead_a = TimelineTicks::new((clips[0].start + clips[0].end) / 2);
    state
        .session_mut()
        .expect("session")
        .set_playhead_ticks(playhead_a);
    let (pts_a, target_a) = wait_for_frame(&mut state, &context, clips[0].id, asset_a);
    let expected_target = source_ticks_to_media_timestamp(
        capcut_core::time::SourceTicks::new(
            clips[0].source_start + (playhead_a.get() - clips[0].start),
        ),
        state.media_preview().probe().expect("probe").stream_start,
    )
    .expect("target");
    assert_eq!(target_a, expected_target.ticks);
    assert!(pts_a <= target_a);

    let playhead_b = TimelineTicks::new((clips[1].start + clips[1].end) / 2);
    state
        .session_mut()
        .expect("session")
        .set_playhead_ticks(playhead_b);
    let (_, target_b) = wait_for_frame(&mut state, &context, clips[1].id, asset_b);
    let expected_b = source_ticks_to_media_timestamp(
        capcut_core::time::SourceTicks::new(
            clips[1].source_start + (playhead_b.get() - clips[1].start),
        ),
        state.media_preview().probe().expect("probe").stream_start,
    )
    .expect("target");
    assert_eq!(target_b, expected_b.ticks);

    let project_path = state
        .session()
        .and_then(EditorSession::native_document)
        .and_then(|document| document.path())
        .cloned()
        .expect("path");
    let project_id = state
        .session()
        .and_then(EditorSession::native_document)
        .map(|document| document.project_id().clone())
        .expect("id");
    state.go_home();
    pump_auto_save(&mut state, &context, 10_000, |state| {
        state.screen() == Screen::Home
    });
    let saved = capcut_project::open(&project_path).expect("reopen saved");
    assert_eq!(saved.editor.project().assets().len(), 2);
    assert_eq!(
        saved
            .editor
            .project()
            .main_track()
            .expect("main")
            .clips()
            .len(),
        2
    );
    assert_eq!(saved.editor.project().assets()[0].id(), asset_a);
    assert_eq!(saved.editor.project().assets()[1].id(), asset_b);

    state.refresh_project_library();
    state.open_library_project(project_id.clone());
    pump_auto_save(&mut state, &context, 20_000, |state| {
        state.screen() == Screen::Editor && state.session().is_some()
    });
    let reopened = main_clips(&state);
    assert_eq!(reopened.len(), 2);
    assert_eq!(reopened[0].asset, asset_a);
    assert_eq!(reopened[1].asset, asset_b);

    state
        .session_mut()
        .expect("session")
        .set_playhead_ticks(playhead_a);
    wait_for_frame(&mut state, &context, reopened[0].id, asset_a);

    let offline_b = media_dir.path("clïp b offline.mp4");
    std::fs::rename(&video_b, &offline_b).expect("move B offline");
    state.media_library_mut().reset();
    state
        .session_mut()
        .expect("session")
        .set_playhead_ticks(playhead_b);
    pump_media(&mut state, &context, |state| {
        matches!(
            state.media_library().availability(asset_b),
            Some(AssetAvailability::Missing)
        ) && matches!(
            state.media_preview().status(),
            ClipPreviewStatus::AssetMissing
        )
    });

    state.open_media_relink_dialog(asset_b);
    state
        .media_library_mut()
        .dialog_mut()
        .expect("relink dialog")
        .path = offline_b.to_string_lossy().into_owned();
    state.confirm_media_import().expect("start relink");
    pump_media(&mut state, &context, |state| {
        state.media_library().dialog().is_none()
            && matches!(
                state.media_library().availability(asset_b),
                Some(AssetAvailability::Available)
            )
    });
    pump_auto_save(&mut state, &context, 30_000, |state| {
        state.session().is_some_and(|session| {
            session.persistent_generation() == session.persisted_generation()
        })
    });
    let saved = capcut_project::open(&project_path).expect("reopen relinked");
    let relinked = saved
        .editor
        .project()
        .asset(asset_b)
        .expect("asset B preserved");
    assert_eq!(relinked.id(), asset_b);
    assert!(relinked.locator().value().contains("clïp b offline.mp4"));

    let relinked_clips = main_clips(&state);
    state
        .session_mut()
        .expect("session")
        .set_playhead_ticks(playhead_b);
    wait_for_frame(&mut state, &context, relinked_clips[1].id, asset_b);
}

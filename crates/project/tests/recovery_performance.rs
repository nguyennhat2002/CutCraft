//! Diagnostic measurements for project serialization, staged save and recovery.
//! No thresholds are asserted; output is recorded in milestone evidence.

use std::path::PathBuf;
use std::time::Instant;

use capcut_core::time::{
    FrameRate, SourceRange, SourceTicks, Timebase, TimelineRange, TimelineTicks,
};
use capcut_engine::asset::{
    MediaAssetId, MediaAssetMetadata, MediaAssetRegistration, MediaLocator, MediaRational,
};
use capcut_engine::command::NativeEditor;
use capcut_engine::project::{ClipKind, ClipSpec, Project, TrackKind};
use capcut_project::{
    publish_recovery_snapshot, read_recovery_document, save_snapshot, serialize_snapshot,
    ProjectDocumentMetadata, ProjectId, RecoveryJob,
};

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
        duration_ticks: Some(3 * 15_360),
        nominal_frame_rate: MediaRational::new(30, 1),
        average_frame_rate: MediaRational::new(30, 1),
        rotation_degrees: 0,
        has_audio: false,
        frame_count_hint: None,
    }
}

fn synthetic(asset_count: usize, clip_count: usize) -> NativeEditor {
    let mut project = Project::empty(FrameRate::new(30, 1).expect("fps")).expect("project");
    let track = project
        .add_track("V1", TrackKind::Video, false)
        .expect("track");
    project.assign_main_track(track).expect("main");
    for index in 0..asset_count {
        project
            .add_asset(
                MediaAssetRegistration::video(
                    format!("Asset {index}"),
                    MediaLocator::absolute(format!("C:/media/asset-{index}.mp4")).expect("locator"),
                    metadata(),
                )
                .expect("asset"),
            )
            .expect("add asset");
    }
    for index in 0..clip_count {
        let start = index as i64 * 120_000;
        let source = capcut_engine::project::SourceMapping::new(
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(360_000)).expect("available"),
            SourceRange::new(SourceTicks::ZERO, SourceTicks::new(120_000)).expect("used"),
            Timebase::new(120_000).expect("timebase"),
            1,
            1,
        )
        .expect("source");
        project
            .add_clip(
                track,
                ClipSpec {
                    name: format!("Clip {index}"),
                    kind: ClipKind::Video,
                    range: TimelineRange::new(
                        TimelineTicks::new(start),
                        TimelineTicks::new(start + 120_000),
                    )
                    .expect("range"),
                    source: Some(source),
                    media_asset_id: if asset_count == 0 {
                        None
                    } else {
                        Some(MediaAssetId::new((index % asset_count) as u64 + 1))
                    },
                    linked_clip_ids: std::collections::BTreeSet::new(),
                    nested: false,
                },
            )
            .expect("clip");
    }
    NativeEditor::new(project).expect("editor")
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-perf-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).expect("temp");
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn percentile(samples: &mut [f64], fraction: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((samples.len() - 1) as f64 * fraction).round() as usize;
    samples[index]
}

fn measure(mut operation: impl FnMut()) -> (f64, f64) {
    let mut samples = Vec::with_capacity(20);
    for _ in 0..20 {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    (
        percentile(&mut samples, 0.50),
        percentile(&mut samples, 0.95),
    )
}

#[test]
fn serialization_save_and_recovery_latency_probe() {
    for (name, assets, clips) in [
        ("empty", 0, 0),
        ("2-assets-2-clips", 2, 2),
        ("100-clips-20-assets", 20, 100),
        ("1000-clips-100-assets", 100, 1_000),
    ] {
        let editor = synthetic(assets, clips);
        let snapshot = editor.snapshot();
        let project_id = ProjectId::generate();
        let temp = TempDir::new(name);
        let target = temp.0.join("project.cutcraft");
        let mut generation = 0u64;
        let metadata_for = |generation: u64| ProjectDocumentMetadata {
            project_id: project_id.clone(),
            created_at_unix_ms: 1,
            save_generation: generation,
        };
        let serialized = serialize_snapshot(&snapshot, &metadata_for(1), 1).expect("serialize");
        let (serialize_p50, serialize_p95) = measure(|| {
            let _ = serialize_snapshot(&snapshot, &metadata_for(1), 1).expect("serialize");
        });
        let mut allowed = None;
        let (save_p50, save_p95) = measure(|| {
            generation += 1;
            let saved = save_snapshot(
                &snapshot,
                &metadata_for(generation),
                &target,
                allowed.as_ref(),
            )
            .expect("save");
            allowed = Some(saved.snapshot);
        });
        let save_bytes = std::fs::metadata(&target)
            .map(|value| value.len())
            .unwrap_or(0);
        let mut recovery_generation = 0u64;
        let recovery_root = TempDir::new(&format!("{name}-recovery"));
        let (recovery_p50, recovery_p95) = measure(|| {
            recovery_generation += 1;
            let _ = publish_recovery_snapshot(&RecoveryJob {
                snapshot: snapshot.clone(),
                project_id: project_id.clone(),
                original_project_path: target.clone(),
                document_generation: recovery_generation,
                last_saved_generation: recovery_generation.saturating_sub(1),
                last_saved_unix_ms: 1,
                created_at_unix_ms: 1,
                library_root: recovery_root.0.clone(),
            })
            .expect("recovery");
        });
        let recovery_path = std::fs::read_dir(recovery_root.0.join(".recovery"))
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .find(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .ends_with(".cutcraft-recovery")
                    })
                    .map(|entry| entry.path())
            })
            .expect("recovery artifact");
        let recovery_bytes = std::fs::metadata(&recovery_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let (validation_p50, validation_p95) = measure(|| {
            let _ = read_recovery_document(&recovery_path).expect("validate recovery");
        });
        println!(
            "PERF {name}: assets={assets} clips={clips} serialized_bytes={} saved_bytes={save_bytes} recovery_bytes={recovery_bytes} serialize_p50_ms={serialize_p50:.3} serialize_p95_ms={serialize_p95:.3} save_p50_ms={save_p50:.3} save_p95_ms={save_p95:.3} recovery_p50_ms={recovery_p50:.3} recovery_p95_ms={recovery_p95:.3} recovery_validate_p50_ms={validation_p50:.3} recovery_validate_p95_ms={validation_p95:.3}",
            serialized.len()
        );
        assert!(save_bytes > 0 && recovery_bytes > 0);
    }
}

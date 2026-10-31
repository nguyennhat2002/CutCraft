//! Real FFmpeg-backed probe/decode tests.
//!
//! Fixtures are generated at test time with the locally configured FFmpeg CLI.
//! Ordinary runs report a skip when the tools are unavailable. Set
//! `CUTCRAFT_REQUIRE_MEDIA=1` for the explicit media gate: missing tools or
//! fixture generation then fail loudly instead of counting as a pass.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use capcut_media::{
    probe_media, DecodedFrame, FrameRequest, MediaError, MediaProbe, MediaTimebase, MediaTimestamp,
    PreviewEngine, PreviewEvent,
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn required_media() -> bool {
    std::env::var("CUTCRAFT_REQUIRE_MEDIA").is_ok_and(|value| value == "1")
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cutcraft-m3c-{}-{}",
            std::process::id(),
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("temp dir");
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

struct Tools {
    ffmpeg: OsString,
    _ffprobe: OsString,
}

impl Tools {
    fn discover() -> Option<Self> {
        let ffmpeg =
            std::env::var_os("CUTCRAFT_FFMPEG").unwrap_or_else(|| OsString::from("ffmpeg"));
        let ffprobe =
            std::env::var_os("CUTCRAFT_FFPROBE").unwrap_or_else(|| OsString::from("ffprobe"));
        for tool in [&ffmpeg, &ffprobe] {
            let output = Command::new(tool).arg("-version").output();
            let available = output.as_ref().is_ok_and(|output| output.status.success());
            if !available {
                let message = format!("media tool {tool:?} is unavailable");
                if required_media() {
                    panic!("required media gate failed: {message}");
                }
                eprintln!("skipping media integration test: {message}");
                return None;
            }
            if required_media() {
                let version = output
                    .ok()
                    .map(|output| {
                        String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .next()
                            .unwrap_or("")
                            .to_owned()
                    })
                    .unwrap_or_default();
                eprintln!("media gate: {tool:?} -> {version}");
            }
        }
        Some(Self {
            ffmpeg,
            _ffprobe: ffprobe,
        })
    }
}

fn generate_colors(
    tools: &Tools,
    dir: &TempDir,
    name: &str,
    rate: &str,
    extra: &[&str],
) -> Option<PathBuf> {
    let path = dir.path(name);
    let mut command = Command::new(&tools.ffmpeg);
    command.args(["-hide_banner", "-loglevel", "error", "-y"]);
    for color in ["red", "green", "blue"] {
        command.args([
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s=320x180:d=1:r={rate}"),
        ]);
    }
    command.args([
        "-filter_complex",
        "[0:v][1:v][2:v]concat=n=3:v=1:a=0,format=yuv420p",
    ]);
    command.args([
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-g",
        "15",
        "-bf",
        "2",
    ]);
    command.args(extra);
    command.arg(&path);
    let output = command.output().ok()?;
    if !output.status.success() {
        eprintln!(
            "skipping media integration test: fixture generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    Some(path)
}

fn generate_offset(tools: &Tools, dir: &TempDir) -> Option<PathBuf> {
    let path = dir.path("offset.mp4");
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=purple:s=320x180:d=1:r=30",
            "-output_ts_offset",
            "1.5",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "15",
            "-bf",
            "2",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&path)
        .output()
        .ok()?;
    output.status.success().then_some(path)
}

fn generate_vfr(tools: &Tools, dir: &TempDir) -> Option<PathBuf> {
    let path = dir.path("vfr.mp4");
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=orange:s=320x180:d=1:r=30",
            "-vf",
            "setpts='if(lt(N,10),N/30/TB,(0.333333+(N-10)/10)/TB)'",
            "-fps_mode",
            "vfr",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-g",
            "30",
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

fn generate_rotated(tools: &Tools, dir: &TempDir, source: &Path) -> Option<PathBuf> {
    let path = dir.path("rotated.mp4");
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-display_rotation",
            "90",
            "-i",
        ])
        .arg(source)
        .args(["-c", "copy"])
        .arg(&path)
        .output()
        .ok()?;
    output.status.success().then_some(path)
}

fn generate_unsupported(tools: &Tools, dir: &TempDir) -> Option<PathBuf> {
    let path = dir.path("mpeg4.mp4");
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x180:d=1:r=30",
            "-c:v",
            "mpeg4",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&path)
        .output()
        .ok()?;
    output.status.success().then_some(path)
}

fn timestamp(seconds: f64, timebase: MediaTimebase) -> MediaTimestamp {
    let ticks = (seconds * timebase.ticks_per_second()).round() as i64;
    MediaTimestamp::new(ticks, timebase)
}

fn event_label(event: &PreviewEvent) -> String {
    match event {
        PreviewEvent::Probe {
            source_id,
            elapsed_ms,
            result,
        } => format!(
            "probe source={source_id} elapsed={elapsed_ms}ms ok={}",
            result.is_ok()
        ),
        PreviewEvent::Frame {
            source_id,
            generation,
            result,
        } => format!(
            "frame source={source_id} generation={generation} ok={}",
            result.is_ok()
        ),
    }
}

fn wait_for(engine: &PreviewEngine, predicate: impl Fn(&PreviewEvent) -> bool) -> PreviewEvent {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        while let Some(event) = engine.poll() {
            eprintln!("media event: {}", event_label(&event));
            if predicate(&event) {
                return event;
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for media event"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn open(engine: &PreviewEngine, source_id: u64, path: &Path) -> MediaProbe {
    engine
        .open_source(source_id, path.to_path_buf())
        .expect("open request");
    match wait_for(
        engine,
        |event| matches!(event, PreviewEvent::Probe { source_id: id, .. } if *id == source_id),
    ) {
        PreviewEvent::Probe { result, .. } => result.expect("probe"),
        _ => unreachable!(),
    }
}

fn request(
    engine: &PreviewEngine,
    source_id: u64,
    generation: u64,
    stream_index: u32,
    target: MediaTimestamp,
) -> Result<DecodedFrame, MediaError> {
    engine
        .request(FrameRequest {
            source_id,
            generation,
            clip_tag: 0,
            asset_tag: 0,
            stream_index,
            target,
        })
        .expect("frame request");
    match wait_for(
        engine,
        |event| matches!(event, PreviewEvent::Frame { source_id: id, generation: g, .. } if *id == source_id && *g == generation),
    ) {
        PreviewEvent::Frame { result, .. } => result,
        _ => unreachable!(),
    }
}

fn center_pixel(frame: &DecodedFrame) -> [u8; 4] {
    let x = frame.width / 2;
    let y = frame.height / 2;
    let offset = ((y * frame.width + x) * 4) as usize;
    [
        frame.rgba[offset],
        frame.rgba[offset + 1],
        frame.rgba[offset + 2],
        frame.rgba[offset + 3],
    ]
}

fn assert_colored(pixel: [u8; 4], expected: &str) {
    let (r, g, b) = (
        i32::from(pixel[0]),
        i32::from(pixel[1]),
        i32::from(pixel[2]),
    );
    eprintln!("center pixel {pixel:?}, expected {expected}");
    match expected {
        "red" => assert!(r > 180 && g < 80 && b < 80, "not red: {pixel:?}"),
        "green" => assert!(r < 80 && g > 120 && b < 80, "not green: {pixel:?}"),
        "blue" => assert!(r < 80 && g < 120 && b > 150, "not blue: {pixel:?}"),
        _ => panic!("unknown expected color"),
    }
}

#[test]
fn probe_and_decode_color_segments_with_b_frames() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "colors.mp4", "30", &[]) else {
        return;
    };
    let probe = probe_media(&path).expect("probe");
    eprintln!(
        "probe codec={} {}x{} timebase={} nominal={:?} avg={:?} start={:?} duration={:?} elapsed={}ms",
        probe.codec,
        probe.width,
        probe.height,
        probe.timebase,
        probe.nominal_frame_rate,
        probe.average_frame_rate,
        probe.stream_start,
        probe.duration,
        probe.probe_elapsed_ms
    );
    assert_eq!(probe.codec, "h264");
    assert!(probe.container.contains("mp4"));
    assert_eq!((probe.width, probe.height), (320, 180));
    assert_eq!(probe.pixel_format.as_deref(), Some("yuv420p"));
    assert_eq!(probe.timebase, MediaTimebase::new(1, 15_360).unwrap());
    assert_eq!(probe.nominal_frame_rate.unwrap().as_f64(), 30.0);
    assert_eq!(probe.stream_start.ticks, 0);
    assert_eq!(probe.duration.unwrap().ticks, 3 * 15_360);
    assert!(!probe.has_audio);

    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 7, &path);
    for (seconds, color) in [(0.5, "red"), (1.5, "green"), (2.5, "blue")] {
        let target = timestamp(seconds, probe.timebase);
        let frame = request(&engine, 7, seconds as u64, probe.video_stream_index, target)
            .expect("decoded frame");
        assert_eq!(frame.generation, seconds as u64);
        assert_eq!((frame.width, frame.height), (320, 180));
        assert!(frame.pts.seconds_f64() <= seconds + 0.04);
        assert_colored(center_pixel(&frame), color);
        eprintln!(
            "seek target={:.3}s pts={:.6}s decode={}ms bytes={}",
            seconds,
            frame.pts.seconds_f64(),
            frame.decode_elapsed_ms,
            frame.rgba.len()
        );
    }
    let metrics = engine.metrics();
    assert!(metrics.worker_count <= 1);
    assert!(metrics.queued_requests <= 1);
}

#[test]
fn seek_uses_presentation_interval_and_handles_segment_boundary() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "colors.mp4", "30", &[]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 8, &path);
    let before = timestamp(0.99, probe.timebase);
    let frame = request(&engine, 8, 1, probe.video_stream_index, before).expect("before");
    assert_colored(center_pixel(&frame), "red");
    let boundary = timestamp(1.0, probe.timebase);
    let frame = request(&engine, 8, 2, probe.video_stream_index, boundary).expect("boundary");
    assert_colored(center_pixel(&frame), "green");
    assert_eq!(frame.pts.ticks, boundary.ticks);
}

#[test]
fn rational_2997_probe_and_decode_are_exact() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "colors-2997.mp4", "30000/1001", &[]) else {
        return;
    };
    let probe = probe_media(&path).expect("probe");
    assert_eq!(probe.nominal_frame_rate.unwrap().numerator, 30_000);
    assert_eq!(probe.nominal_frame_rate.unwrap().denominator, 1_001);
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 9, &path);
    let target = timestamp(1.5, probe.timebase);
    let frame = request(&engine, 9, 1, probe.video_stream_index, target).expect("frame");
    assert_colored(center_pixel(&frame), "green");
    assert!(frame.pts.ticks <= target.ticks);
}

#[test]
fn vfr_requests_use_actual_presentation_timestamps() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_vfr(&tools, &dir) else {
        return;
    };
    let probe = probe_media(&path).expect("probe");
    assert_ne!(
        probe.nominal_frame_rate.unwrap().as_f64(),
        probe.average_frame_rate.unwrap().as_f64(),
        "VFR fixture should expose differing rate hints"
    );
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 10, &path);
    let target = timestamp(0.45, probe.timebase);
    let frame = request(&engine, 10, 1, probe.video_stream_index, target).expect("frame");
    assert_eq!(frame.pts.ticks, 6_144);
    let target = timestamp(0.55, probe.timebase);
    let frame = request(&engine, 10, 2, probe.video_stream_index, target).expect("frame");
    assert_eq!(frame.pts.ticks, 7_680);
}

#[test]
fn nonzero_stream_start_maps_from_source_zero() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_offset(&tools, &dir) else {
        return;
    };
    let probe = probe_media(&path).expect("probe");
    assert_eq!(probe.stream_start.ticks, 23_040);
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 11, &path);
    let target = capcut_media::source_ticks_to_media_timestamp(
        capcut_core::time::SourceTicks::ZERO,
        probe.stream_start,
    )
    .expect("mapped start");
    let frame = request(&engine, 11, 1, probe.video_stream_index, target).expect("frame");
    assert!(frame.pts.ticks >= probe.stream_start.ticks);
}

#[test]
fn rotated_media_probe_and_decoded_orientation() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(source) = generate_colors(&tools, &dir, "source.mp4", "30", &[]) else {
        return;
    };
    let Some(path) = generate_rotated(&tools, &dir, &source) else {
        return;
    };
    let probe = probe_media(&path).expect("probe");
    assert_eq!(probe.rotation_degrees, 90);
    assert_eq!(probe.display_dimensions(), (180, 320));
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 12, &path);
    let target = timestamp(0.5, probe.timebase);
    let frame = request(&engine, 12, 1, probe.video_stream_index, target).expect("frame");
    assert_eq!((frame.width, frame.height), (180, 320));
    assert_colored(center_pixel(&frame), "red");
}

#[test]
fn unsupported_corrupt_and_end_of_stream_are_typed() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(unsupported) = generate_unsupported(&tools, &dir) else {
        return;
    };
    assert!(matches!(
        probe_media(&unsupported),
        Err(MediaError::UnsupportedCodec(_))
    ));
    let corrupt = dir.path("corrupt.mp4");
    std::fs::write(&corrupt, b"not an mp4").expect("corrupt fixture");
    assert!(probe_media(&corrupt).is_err());

    let Some(path) = generate_colors(&tools, &dir, "colors.mp4", "30", &[]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 13, &path);
    let duration = probe.duration.expect("duration");
    assert!(matches!(
        request(&engine, 13, 1, probe.video_stream_index, duration),
        Err(MediaError::EndOfStream)
    ));
}

#[test]
fn missing_and_truncated_files_fail_without_panicking() {
    let missing = std::env::temp_dir().join("cutcraft-definitely-missing-m3c.mp4");
    let _ = std::fs::remove_file(&missing);
    let error = probe_media(&missing).expect_err("missing file must fail");
    assert!(matches!(error, MediaError::Io(_)), "{error:?}");

    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "colors.mp4", "30", &[]) else {
        return;
    };
    let bytes = std::fs::read(&path).expect("fixture bytes");
    let truncated = dir.path("truncated.mp4");
    std::fs::write(&truncated, &bytes[..bytes.len() / 2]).expect("truncated fixture");
    let error = probe_media(&truncated).expect_err("truncated file must fail");
    eprintln!("truncated media error: {error}");
}

#[test]
fn long_gop_seek_decodes_forward_to_a_non_keyframe() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "long-gop.mp4", "30", &["-g", "300"]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 15, &path);
    let target = timestamp(2.2, probe.timebase);
    let frame = request(&engine, 15, 1, probe.video_stream_index, target).expect("frame");
    assert_eq!(frame.pts.ticks, target.ticks);
    assert_colored(center_pixel(&frame), "blue");
}

#[test]
fn b_frame_presentation_timestamps_increase_in_display_order() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "bframes.mp4", "30", &[]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 16, &path);
    let mut previous = None;
    for (generation, seconds) in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7].into_iter().enumerate() {
        let target = timestamp(seconds, probe.timebase);
        let frame = request(
            &engine,
            16,
            generation as u64,
            probe.video_stream_index,
            target,
        )
        .expect("frame");
        assert!(frame.pts.ticks <= target.ticks);
        if let Some(previous) = previous {
            assert!(
                frame.pts.ticks > previous,
                "presentation timestamps must advance: {previous} -> {}",
                frame.pts.ticks
            );
        }
        previous = Some(frame.pts.ticks);
        assert_colored(center_pixel(&frame), "red");
    }
}

#[test]
fn decoder_recovers_after_end_of_stream_and_frames_are_owned() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "recovery.mp4", "30", &[]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 17, &path);
    let first = request(
        &engine,
        17,
        1,
        probe.video_stream_index,
        timestamp(0.5, probe.timebase),
    )
    .expect("first frame");
    let duration = probe.duration.expect("duration");
    assert!(matches!(
        request(&engine, 17, 2, probe.video_stream_index, duration),
        Err(MediaError::EndOfStream)
    ));
    let recovered = request(
        &engine,
        17,
        3,
        probe.video_stream_index,
        timestamp(1.5, probe.timebase),
    )
    .expect("recovered frame");
    assert_eq!(first.source_id, 17);
    assert_eq!(recovered.generation, 3);
    assert_eq!(first.rgba.len(), 320 * 180 * 4);
    assert_colored(center_pixel(&first), "red");
    assert_colored(center_pixel(&recovered), "green");
}

#[test]
fn switching_sources_keeps_decoded_frame_identity() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(first_path) = generate_colors(&tools, &dir, "first.mp4", "30", &[]) else {
        return;
    };
    let Some(second_path) = generate_colors(&tools, &dir, "second.mp4", "30", &[]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let first_probe = open(&engine, 31, &first_path);
    let first = request(
        &engine,
        31,
        1,
        first_probe.video_stream_index,
        timestamp(1.5, first_probe.timebase),
    )
    .expect("first");
    let second_probe = open(&engine, 32, &second_path);
    let second = request(
        &engine,
        32,
        2,
        second_probe.video_stream_index,
        timestamp(0.5, second_probe.timebase),
    )
    .expect("second");
    assert_eq!(first.source_id, 31);
    assert_eq!(second.source_id, 32);
    assert_colored(center_pixel(&first), "green");
    assert_colored(center_pixel(&second), "red");
}

#[test]
fn probed_audio_presence_and_nonzero_video_stream_index_are_reported() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let path = dir.path("with-audio.mp4");
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-f",
            "lavfi",
            "-i",
            "color=c=teal:s=320x180:d=1:r=30",
            "-map",
            "0:a",
            "-map",
            "1:v",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&path)
        .output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        eprintln!(
            "skipping audio probe test: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let probe = probe_media(&path).expect("probe");
    assert!(probe.has_audio);
    assert_eq!(probe.video_stream_index, 1);
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 33, &path);
    let frame = request(
        &engine,
        33,
        1,
        probe.video_stream_index,
        timestamp(0.5, probe.timebase),
    )
    .expect("video stream frame");
    assert_eq!(frame.width, 320);
    assert_eq!(frame.height, 180);
}

#[test]
fn audio_only_media_reports_no_video_stream() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let path = dir.path("audio-only.m4a");
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:a",
            "aac",
        ])
        .arg(&path)
        .output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        eprintln!(
            "skipping audio-only probe test: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    assert!(matches!(probe_media(&path), Err(MediaError::NoVideoStream)));
}

#[test]
fn rapid_requests_coalesce_and_return_the_latest_generation() {
    let Some(tools) = Tools::discover() else {
        return;
    };
    let dir = TempDir::new();
    let Some(path) = generate_colors(&tools, &dir, "colors.mp4", "30", &[]) else {
        return;
    };
    let engine = PreviewEngine::spawn().expect("engine");
    let probe = open(&engine, 14, &path);
    let metrics = engine.metrics();
    assert!(metrics.queued_requests <= 1);
    for generation in 0..8 {
        let target = timestamp(0.3 + generation as f64 * 0.1, probe.timebase);
        engine
            .request(FrameRequest {
                source_id: 14,
                generation,
                clip_tag: 0,
                asset_tag: 0,
                stream_index: probe.video_stream_index,
                target,
            })
            .expect("request");
    }
    assert!(engine.metrics().queued_requests <= 1);
    let final_generation = 7;
    let event = wait_for(&engine, |event| {
        matches!(
            event,
            PreviewEvent::Frame {
                source_id: 14,
                generation: 7,
                ..
            }
        )
    });
    match event {
        PreviewEvent::Frame { result, .. } => {
            let frame = result.expect("latest frame");
            assert_eq!(frame.generation, final_generation);
        }
        _ => unreachable!(),
    }
    assert_eq!(engine.metrics().worker_count, 1);
}

#[test]
fn required_media_gate_checks_tools_and_h264_generation() {
    if !required_media() {
        return;
    }
    let tools = Tools::discover().expect("required media tools");
    let temp = TempDir::new();
    let fixture = generate_colors(&tools, &temp, "media-gate.mp4", "30", &[])
        .expect("required media gate fixture generation");
    assert!(fixture.exists());
    let probe = probe_media(&fixture).expect("required media gate probe");
    assert_eq!(probe.codec, "h264");
}

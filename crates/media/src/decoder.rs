//! Bounded asynchronous media probing and frame decoding.
//!
//! One worker thread owns all backend processes. UI requests are coalesced to a
//! single latest command, results are generation-tagged, and the UI never waits
//! for decode work.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use serde_json::Value;

use crate::error::MediaError;
use crate::probe::{
    probe_with_tools, MediaProbe, MediaTimestamp, MAX_FRAME_BYTES, MAX_FRAME_DIMENSION,
};
use crate::process::{os, run_tool, MediaTools, DECODE_TIMEOUT, FRAME_QUERY_TIMEOUT};

const MAX_FRAME_QUERY_STDOUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_FRAME_QUERY_STDERR_BYTES: usize = 256 * 1024;
const MAX_DECODE_STDERR_BYTES: usize = 1024 * 1024;
const FRAME_QUERY_LEAD_SECONDS: f64 = 2.0;
const FRAME_QUERY_FALLBACK_LEAD_SECONDS: f64 = 10.0;
const FRAME_QUERY_TAIL_SECONDS: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8,
}

#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub source_id: u64,
    pub generation: u64,
    pub clip_tag: u64,
    pub asset_tag: u64,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub rgba: Vec<u8>,
    pub pts: MediaTimestamp,
    pub requested_target: MediaTimestamp,
    pub decode_elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRequest {
    pub source_id: u64,
    pub generation: u64,
    pub clip_tag: u64,
    pub asset_tag: u64,
    pub stream_index: u32,
    pub target: MediaTimestamp,
}

#[derive(Debug, Clone)]
pub enum PreviewEvent {
    Probe {
        source_id: u64,
        elapsed_ms: u64,
        result: Result<MediaProbe, MediaError>,
    },
    Frame {
        source_id: u64,
        generation: u64,
        result: Result<DecodedFrame, MediaError>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineMetrics {
    pub queued_requests: usize,
    pub worker_count: usize,
}

enum Command {
    Open { source_id: u64, path: PathBuf },
    Seek(FrameRequest),
    Clear,
}

struct Shared {
    pending: Mutex<Option<Command>>,
    result: Mutex<Option<PreviewEvent>>,
    wake: Condvar,
    stop: AtomicBool,
}

struct CurrentSource {
    path: PathBuf,
    probe: MediaProbe,
}

/// One background decoder worker with a one-slot latest-request queue.
pub struct PreviewEngine {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl PreviewEngine {
    pub fn spawn() -> Result<Self, MediaError> {
        Self::spawn_with_tools(MediaTools::discover())
    }

    fn spawn_with_tools(tools: MediaTools) -> Result<Self, MediaError> {
        let shared = Arc::new(Shared {
            pending: Mutex::new(None),
            result: Mutex::new(None),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("cutcraft-media".to_owned())
            .spawn(move || worker_loop(worker_shared, tools))
            .map_err(|error| MediaError::Io(error.to_string()))?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }

    pub fn open_source(&self, source_id: u64, path: PathBuf) -> Result<(), MediaError> {
        self.send(Command::Open { source_id, path })
    }

    pub fn request(&self, request: FrameRequest) -> Result<(), MediaError> {
        self.send(Command::Seek(request))
    }

    pub fn clear(&self) -> Result<(), MediaError> {
        self.send(Command::Clear)
    }

    pub fn poll(&self) -> Option<PreviewEvent> {
        self.shared
            .result
            .lock()
            .ok()
            .and_then(|mut result| result.take())
    }

    pub fn metrics(&self) -> EngineMetrics {
        EngineMetrics {
            queued_requests: usize::from(
                self.shared
                    .pending
                    .lock()
                    .map(|pending| pending.is_some())
                    .unwrap_or(false),
            ),
            worker_count: usize::from(self.worker.is_some()),
        }
    }

    fn send(&self, command: Command) -> Result<(), MediaError> {
        if self.shared.stop.load(Ordering::SeqCst) {
            return Err(MediaError::EngineClosed);
        }
        let mut pending = self
            .shared
            .pending
            .lock()
            .map_err(|_| MediaError::EngineClosed)?;
        *pending = Some(command);
        self.shared.wake.notify_one();
        Ok(())
    }
}

impl Drop for PreviewEngine {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Ok(mut pending) = self.shared.pending.lock() {
            *pending = None;
        }
        self.shared.wake.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn worker_loop(shared: Arc<Shared>, tools: MediaTools) {
    let mut source: Option<CurrentSource> = None;
    loop {
        let command = {
            let mut pending = match shared.pending.lock() {
                Ok(pending) => pending,
                Err(_) => break,
            };
            while pending.is_none() && !shared.stop.load(Ordering::SeqCst) {
                pending = match shared.wake.wait(pending) {
                    Ok(pending) => pending,
                    Err(_) => return,
                };
            }
            if shared.stop.load(Ordering::SeqCst) {
                return;
            }
            pending.take()
        };
        match command {
            Some(Command::Open { source_id, path }) => {
                let started = Instant::now();
                let result = probe_with_tools(&path, &tools);
                let elapsed_ms = started.elapsed().as_millis() as u64;
                if let Ok(probe) = &result {
                    source = Some(CurrentSource {
                        path,
                        probe: probe.clone(),
                    });
                } else {
                    source = None;
                }
                publish(
                    &shared,
                    PreviewEvent::Probe {
                        source_id,
                        elapsed_ms,
                        result,
                    },
                );
            }
            Some(Command::Seek(request)) => {
                let result = decode_request(source.as_ref(), request, &tools);
                publish(
                    &shared,
                    PreviewEvent::Frame {
                        source_id: request.source_id,
                        generation: request.generation,
                        result,
                    },
                );
            }
            Some(Command::Clear) => source = None,
            None => return,
        }
    }
}

fn publish(shared: &Shared, event: PreviewEvent) {
    if let Ok(mut result) = shared.result.lock() {
        *result = Some(event);
    }
}

fn decode_request(
    source: Option<&CurrentSource>,
    request: FrameRequest,
    tools: &MediaTools,
) -> Result<DecodedFrame, MediaError> {
    let source = source.ok_or(MediaError::SourceNotOpen(request.source_id))?;
    if request.stream_index != source.probe.video_stream_index {
        return Err(MediaError::InvalidInput(format!(
            "requested stream {} but decoded stream is {}",
            request.stream_index, source.probe.video_stream_index
        )));
    }
    if request.target.timebase != source.probe.timebase {
        return Err(MediaError::InvalidInput(
            "request timebase does not match the probed stream".to_owned(),
        ));
    }
    decode_frame(&source.path, &source.probe, request, tools)
}

fn decode_frame(
    path: &std::path::Path,
    probe: &MediaProbe,
    request: FrameRequest,
    tools: &MediaTools,
) -> Result<DecodedFrame, MediaError> {
    let started = Instant::now();
    if let Some(end) = probe.end_timestamp() {
        if request.target.ticks >= end.ticks {
            return Err(MediaError::EndOfStream);
        }
    }
    let chosen_pts = find_frame_pts(path, probe, request.target, tools)?;
    let chosen = MediaTimestamp::new(chosen_pts, probe.timebase);
    let (expected_width, expected_height) = probe.display_dimensions();
    let expected_bytes = u64::from(expected_width) * u64::from(expected_height) * 4;
    if expected_width == 0
        || expected_height == 0
        || expected_width > MAX_FRAME_DIMENSION
        || expected_height > MAX_FRAME_DIMENSION
        || expected_bytes > MAX_FRAME_BYTES as u64
    {
        return Err(MediaError::FrameTooLarge {
            width: expected_width,
            height: expected_height,
        });
    }
    let relative_ticks = chosen
        .ticks
        .checked_sub(probe.stream_start.ticks)
        .ok_or(MediaError::InvalidInput(
            "frame timestamp precedes the stream start".to_owned(),
        ))?
        .max(0);
    let seek_seconds = format_seconds(relative_ticks as f64 * probe.timebase.seconds_per_tick());
    let args = vec![
        os("-hide_banner"),
        os("-nostdin"),
        os("-ss"),
        OsString::from(seek_seconds),
        os("-i"),
        path.as_os_str().to_os_string(),
        os("-copyts"),
        os("-map"),
        OsString::from(format!("0:{}", request.stream_index)),
        os("-frames:v"),
        os("1"),
        os("-an"),
        os("-sn"),
        os("-dn"),
        os("-vf"),
        os("showinfo"),
        os("-f"),
        os("rawvideo"),
        os("-pix_fmt"),
        os("rgba"),
        os("pipe:1"),
    ];
    let output = run_tool(
        &tools.ffmpeg,
        &args,
        MAX_FRAME_BYTES,
        MAX_DECODE_STDERR_BYTES,
        DECODE_TIMEOUT,
    )?;
    if !output.success {
        return Err(MediaError::DecodeFailed(if output.stderr.is_empty() {
            "ffmpeg exited with an error".to_owned()
        } else {
            output.stderr
        }));
    }
    if output.stdout.is_empty() {
        return Err(MediaError::EndOfStream);
    }
    let (actual_pts, width, height) = parse_showinfo(&output.stderr)?;
    if actual_pts != chosen_pts {
        return Err(MediaError::PtsMismatch {
            expected: chosen_pts,
            actual: actual_pts,
        });
    }
    if width == 0 || height == 0 || width > MAX_FRAME_DIMENSION || height > MAX_FRAME_DIMENSION {
        return Err(MediaError::FrameTooLarge { width, height });
    }
    let expected = usize::try_from(u64::from(width) * u64::from(height) * 4)
        .map_err(|_| MediaError::FrameTooLarge { width, height })?;
    if expected > MAX_FRAME_BYTES {
        return Err(MediaError::FrameTooLarge { width, height });
    }
    if output.stdout.len() != expected {
        return Err(MediaError::FrameSizeMismatch {
            expected,
            actual: output.stdout.len(),
        });
    }
    Ok(DecodedFrame {
        source_id: request.source_id,
        generation: request.generation,
        clip_tag: request.clip_tag,
        asset_tag: request.asset_tag,
        width,
        height,
        format: PixelFormat::Rgba8,
        rgba: output.stdout,
        pts: MediaTimestamp::new(actual_pts, probe.timebase),
        requested_target: request.target,
        decode_elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn find_frame_pts(
    path: &std::path::Path,
    probe: &MediaProbe,
    target: MediaTimestamp,
    tools: &MediaTools,
) -> Result<i64, MediaError> {
    let target_seconds = target.seconds_f64();
    let floor = if probe.stream_start.seconds_f64() < 0.0 {
        probe.stream_start.seconds_f64()
    } else {
        0.0
    };
    let start_seconds = (target_seconds - FRAME_QUERY_LEAD_SECONDS).max(floor);
    let end_seconds = target_seconds + FRAME_QUERY_TAIL_SECONDS;
    let mut frames = query_frames(path, probe, start_seconds, end_seconds, tools)?;
    if frames.is_empty() {
        let start_seconds = (target_seconds - FRAME_QUERY_FALLBACK_LEAD_SECONDS).max(floor);
        frames = query_frames(path, probe, start_seconds, end_seconds, tools)?;
    }
    frames.sort_unstable();
    frames.dedup();
    if frames.is_empty() {
        return Err(MediaError::NoFrameAtTarget);
    }
    if let Some(pts) = frames
        .iter()
        .copied()
        .filter(|pts| *pts <= target.ticks)
        .max()
    {
        return Ok(pts);
    }
    // A target before the first available presentation frame shows the first
    // available frame; this keeps clip-start preview useful for streams whose
    // first PTS is nonzero.
    frames.first().copied().ok_or(MediaError::NoFrameAtTarget)
}

fn query_frames(
    path: &std::path::Path,
    probe: &MediaProbe,
    start_seconds: f64,
    end_seconds: f64,
    tools: &MediaTools,
) -> Result<Vec<i64>, MediaError> {
    let interval = format!(
        "{}%{}",
        format_seconds(start_seconds),
        format_seconds(end_seconds.max(start_seconds))
    );
    let args = vec![
        os("-hide_banner"),
        os("-v"),
        os("error"),
        os("-select_streams"),
        OsString::from(probe.video_stream_index.to_string()),
        os("-read_intervals"),
        OsString::from(interval),
        os("-show_frames"),
        os("-show_entries"),
        os("frame=best_effort_timestamp,pts,key_frame"),
        os("-print_format"),
        os("json"),
        path.as_os_str().to_os_string(),
    ];
    let output = run_tool(
        &tools.ffprobe,
        &args,
        MAX_FRAME_QUERY_STDOUT_BYTES,
        MAX_FRAME_QUERY_STDERR_BYTES,
        FRAME_QUERY_TIMEOUT,
    )?;
    if !output.success {
        return Err(MediaError::DecodeFailed(if output.stderr.is_empty() {
            "frame timestamp query failed".to_owned()
        } else {
            output.stderr
        }));
    }
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| MediaError::MissingPresentationTimestamp)?;
    let frames = json
        .get("frames")
        .and_then(Value::as_array)
        .ok_or(MediaError::MissingPresentationTimestamp)?;
    let mut output = Vec::with_capacity(frames.len());
    for frame in frames {
        if let Some(pts) = frame
            .get("best_effort_timestamp")
            .and_then(Value::as_i64)
            .or_else(|| frame.get("pts").and_then(Value::as_i64))
        {
            output.push(pts);
        }
    }
    Ok(output)
}

fn parse_showinfo(stderr: &str) -> Result<(i64, u32, u32), MediaError> {
    let line = stderr
        .lines()
        .find(|line| line.contains("Parsed_showinfo") && line.contains("pts:"))
        .ok_or(MediaError::MissingPresentationTimestamp)?;
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let mut pts = None;
    for (index, token) in tokens.iter().enumerate() {
        if *token == "pts:" {
            pts = tokens
                .get(index + 1)
                .and_then(|value| value.parse::<i64>().ok());
        } else if let Some(value) = token.strip_prefix("pts:") {
            pts = value.parse::<i64>().ok();
        }
        if pts.is_some() {
            break;
        }
    }
    let pts = pts.ok_or(MediaError::MissingPresentationTimestamp)?;
    let size = line
        .split_whitespace()
        .find_map(|token| token.strip_prefix("s:"))
        .and_then(|value| value.split_once('x'))
        .and_then(|(width, height)| Some((width.parse::<u32>().ok()?, height.parse::<u32>().ok()?)))
        .ok_or(MediaError::MissingPresentationTimestamp)?;
    Ok((pts, size.0, size.1))
}

fn format_seconds(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "0.000000000".to_owned();
    }
    format!("{seconds:.9}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showinfo_parsing_rejects_missing_fields() {
        let line = "[Parsed_showinfo_0] n:0 pts: 512 pts_time:0.033 s:640x360";
        assert_eq!(parse_showinfo(line).expect("parsed"), (512, 640, 360));
        assert_eq!(
            parse_showinfo("ffmpeg stderr only"),
            Err(MediaError::MissingPresentationTimestamp)
        );
    }

    #[test]
    fn format_seconds_is_finite_and_nan_safe() {
        assert_eq!(format_seconds(1.25), "1.250000000");
        assert_eq!(format_seconds(f64::NAN), "0.000000000");
    }
}

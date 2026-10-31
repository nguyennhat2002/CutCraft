//! Backend-independent media probe metadata and source-time conversion.
//!
//! M3-C uses a runtime-invoked FFmpeg CLI adapter. Decoder timestamps are kept
//! in the stream's rational timebase; no TimelineTicks cast is used as an
//! FFmpeg timestamp.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use capcut_core::time::{SourceTicks, TIMELINE_TICKS_PER_SECOND};
use serde_json::Value;

use crate::error::MediaError;
use crate::process::{os, run_tool, MediaTools, PROBE_TIMEOUT};

pub const SUPPORTED_VIDEO_CODEC: &str = "h264";
pub const MAX_FRAME_DIMENSION: u32 = 8192;
pub const MAX_FRAME_PIXELS: u64 = 33_554_432;
pub const MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_MEDIA_FILE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_PROBE_STDOUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROBE_STDERR_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn from_ffmpeg(value: &str) -> Option<Self> {
        let (numerator, denominator) = value.split_once('/')?;
        let numerator = numerator.trim().parse::<u32>().ok()?;
        let denominator = denominator.trim().parse::<u32>().ok()?;
        Self::new(numerator, denominator)
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

/// Seconds per decoder timestamp tick, e.g. `1/15360`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaTimebase {
    pub numerator: u32,
    pub denominator: u32,
}

impl MediaTimebase {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        Some(Self {
            numerator,
            denominator,
        })
    }

    pub fn from_ffmpeg(value: &str) -> Option<Self> {
        let (numerator, denominator) = value.split_once('/')?;
        let numerator = numerator.trim().parse::<u32>().ok()?;
        let denominator = denominator.trim().parse::<u32>().ok()?;
        Self::new(numerator, denominator)
    }

    pub fn seconds_per_tick(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }

    pub fn ticks_per_second(self) -> f64 {
        f64::from(self.denominator) / f64::from(self.numerator)
    }
}

impl fmt::Display for MediaTimebase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.numerator, self.denominator)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaTimestamp {
    pub ticks: i64,
    pub timebase: MediaTimebase,
}

impl MediaTimestamp {
    pub fn new(ticks: i64, timebase: MediaTimebase) -> Self {
        Self { ticks, timebase }
    }

    pub fn seconds_f64(self) -> f64 {
        self.ticks as f64 * self.timebase.seconds_per_tick()
    }

    pub fn checked_add_ticks(self, delta: i64) -> Option<Self> {
        self.ticks
            .checked_add(delta)
            .map(|ticks| Self::new(ticks, self.timebase))
    }
}

#[derive(Debug, Clone)]
pub struct MediaProbe {
    pub path: PathBuf,
    pub container: String,
    pub video_stream_index: u32,
    pub codec: String,
    /// Source-oriented dimensions before display rotation is applied.
    pub width: u32,
    pub height: u32,
    pub pixel_format: Option<String>,
    pub timebase: MediaTimebase,
    pub nominal_frame_rate: Option<MediaRational>,
    pub average_frame_rate: Option<MediaRational>,
    pub duration: Option<MediaTimestamp>,
    pub stream_start: MediaTimestamp,
    pub rotation_degrees: u16,
    pub has_audio: bool,
    pub frame_count_hint: Option<u64>,
    pub probe_elapsed_ms: u64,
}

impl MediaProbe {
    /// Display dimensions after rotation metadata is applied. The decoder
    /// auto-rotates frames, so this is also the expected decoded orientation.
    pub fn display_dimensions(&self) -> (u32, u32) {
        if self.rotation_degrees % 180 == 90 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        }
    }

    pub fn duration_seconds_f64(&self) -> Option<f64> {
        self.duration.map(MediaTimestamp::seconds_f64)
    }

    /// Exclusive presentation end timestamp. Durations in ffprobe are lengths,
    /// so a nonzero stream start shifts the exclusive end.
    pub fn end_timestamp(&self) -> Option<MediaTimestamp> {
        self.duration
            .and_then(|duration| self.stream_start.checked_add_ticks(duration.ticks))
    }
}

/// Probe a local MP4 with the configured FFmpeg tools.
pub fn probe_media(path: &Path) -> Result<MediaProbe, MediaError> {
    probe_with_tools(path, &MediaTools::discover())
}

pub(crate) fn probe_with_tools(path: &Path, tools: &MediaTools) -> Result<MediaProbe, MediaError> {
    let started = Instant::now();
    let metadata = fs::metadata(path).map_err(|error| MediaError::Io(error.to_string()))?;
    if !metadata.is_file() {
        return Err(MediaError::InvalidInput(format!(
            "{} is not a media file",
            path.display()
        )));
    }
    if metadata.len() == 0 {
        return Err(MediaError::InvalidInput("media file is empty".to_owned()));
    }
    if metadata.len() > MAX_MEDIA_FILE_BYTES {
        return Err(MediaError::InvalidInput(format!(
            "media file is too large ({} bytes)",
            metadata.len()
        )));
    }

    let args = vec![
        os("-hide_banner"),
        os("-v"),
        os("error"),
        os("-print_format"),
        os("json"),
        os("-show_streams"),
        os("-show_format"),
        path.as_os_str().to_os_string(),
    ];
    let output = run_tool(
        &tools.ffprobe,
        &args,
        MAX_PROBE_STDOUT_BYTES,
        MAX_PROBE_STDERR_BYTES,
        PROBE_TIMEOUT,
    )?;
    if !output.success {
        return Err(MediaError::ProbeFailed(if output.stderr.is_empty() {
            "ffprobe exited with an error".to_owned()
        } else {
            output.stderr
        }));
    }
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| MediaError::InvalidMetadata(error.to_string()))?;
    parse_probe(path, &json, started.elapsed().as_millis() as u64)
}

fn parse_probe(path: &Path, json: &Value, probe_elapsed_ms: u64) -> Result<MediaProbe, MediaError> {
    let streams = json
        .get("streams")
        .and_then(Value::as_array)
        .ok_or_else(|| MediaError::InvalidMetadata("missing streams array".to_owned()))?;
    let format = json.get("format").and_then(Value::as_object);
    let video = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("video"))
        .ok_or(MediaError::NoVideoStream)?;
    let codec = string_field(video, "codec_name")
        .ok_or_else(|| MediaError::InvalidMetadata("missing video codec".to_owned()))?;
    if !codec.eq_ignore_ascii_case(SUPPORTED_VIDEO_CODEC) {
        return Err(MediaError::UnsupportedCodec(codec.to_owned()));
    }
    let video_stream_index = u64_field(video, "index")
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| MediaError::InvalidMetadata("missing video stream index".to_owned()))?;
    let width = u32_field(video, "width")
        .ok_or_else(|| MediaError::InvalidMetadata("missing video width".to_owned()))?;
    let height = u32_field(video, "height")
        .ok_or_else(|| MediaError::InvalidMetadata("missing video height".to_owned()))?;
    if width == 0
        || height == 0
        || width > MAX_FRAME_DIMENSION
        || height > MAX_FRAME_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_FRAME_PIXELS
    {
        return Err(MediaError::FrameTooLarge { width, height });
    }
    let timebase_raw = string_field(video, "time_base")
        .ok_or_else(|| MediaError::InvalidMetadata("missing stream timebase".to_owned()))?;
    let timebase = MediaTimebase::from_ffmpeg(timebase_raw)
        .ok_or_else(|| MediaError::UnsupportedTimebase(timebase_raw.to_owned()))?;
    let stream_start = stream_start(video, timebase)?;
    let duration = stream_duration(video, format, timebase)?;
    let rotation_degrees = rotation_degrees(video);
    let has_audio = streams
        .iter()
        .any(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"));
    let container = format
        .and_then(|format| format.get("format_name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    Ok(MediaProbe {
        path: path.to_path_buf(),
        container,
        video_stream_index,
        codec: codec.to_owned(),
        width,
        height,
        pixel_format: string_field(video, "pix_fmt").map(str::to_owned),
        timebase,
        nominal_frame_rate: string_field(video, "r_frame_rate")
            .and_then(MediaRational::from_ffmpeg),
        average_frame_rate: string_field(video, "avg_frame_rate")
            .and_then(MediaRational::from_ffmpeg),
        duration,
        stream_start,
        rotation_degrees,
        has_audio,
        frame_count_hint: u64_field(video, "nb_frames"),
        probe_elapsed_ms,
    })
}

fn stream_start(video: &Value, timebase: MediaTimebase) -> Result<MediaTimestamp, MediaError> {
    if let Some(ticks) = i64_field(video, "start_pts") {
        return Ok(MediaTimestamp::new(ticks, timebase));
    }
    if let Some(seconds) = string_field(video, "start_time") {
        if let Some(ticks) = seconds_to_ticks(seconds, timebase) {
            return Ok(MediaTimestamp::new(ticks, timebase));
        }
    }
    Ok(MediaTimestamp::new(0, timebase))
}

fn stream_duration(
    video: &Value,
    format: Option<&serde_json::Map<String, Value>>,
    timebase: MediaTimebase,
) -> Result<Option<MediaTimestamp>, MediaError> {
    if let Some(ticks) = i64_field(video, "duration_ts") {
        if ticks > 0 {
            return Ok(Some(MediaTimestamp::new(ticks, timebase)));
        }
    }
    if let Some(seconds) = string_field(video, "duration") {
        if let Some(ticks) = seconds_to_ticks(seconds, timebase) {
            if ticks > 0 {
                return Ok(Some(MediaTimestamp::new(ticks, timebase)));
            }
        }
    }
    if let Some(format) = format {
        if let Some(seconds) = format.get("duration").and_then(Value::as_str) {
            if let Some(ticks) = seconds_to_ticks(seconds, timebase) {
                if ticks > 0 {
                    return Ok(Some(MediaTimestamp::new(ticks, timebase)));
                }
            }
        }
    }
    Ok(None)
}

fn rotation_degrees(video: &Value) -> u16 {
    let Some(side_data) = video.get("side_data_list").and_then(Value::as_array) else {
        return 0;
    };
    for entry in side_data {
        if let Some(rotation) = i64_field(entry, "rotation") {
            return rotation.rem_euclid(360) as u16;
        }
    }
    0
}

fn seconds_to_ticks(seconds: &str, timebase: MediaTimebase) -> Option<i64> {
    let seconds = seconds.trim().parse::<f64>().ok()?;
    if !seconds.is_finite() {
        return None;
    }
    let ticks = seconds * timebase.ticks_per_second();
    if !ticks.is_finite() {
        return None;
    }
    let rounded = ticks.round();
    if rounded < i64::MIN as f64 || rounded > i64::MAX as f64 {
        return None;
    }
    Some(rounded as i64)
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn u64_field(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    })
}

fn u32_field(value: &Value, key: &str) -> Option<u32> {
    u64_field(value, key).and_then(|value| u32::try_from(value).ok())
}

fn i64_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    })
}

/// Convert an absolute source-media tick (120,000/sec) to the stream's
/// rational timebase, adding the stream start timestamp. The final tick is
/// rounded to nearest; all intermediate arithmetic is checked i128.
pub fn source_ticks_to_media_timestamp(
    source_ticks: SourceTicks,
    stream_start: MediaTimestamp,
) -> Result<MediaTimestamp, MediaError> {
    if source_ticks.get() < 0 {
        return Err(MediaError::InvalidInput(
            "source time cannot be negative".to_owned(),
        ));
    }
    let timebase = stream_start.timebase;
    let numerator = i128::from(source_ticks.get()) * i128::from(timebase.denominator);
    let denominator = i128::from(TIMELINE_TICKS_PER_SECOND) * i128::from(timebase.numerator);
    if denominator == 0 {
        return Err(MediaError::UnsupportedTimebase(timebase.to_string()));
    }
    let quotient = numerator.div_euclid(denominator);
    let remainder = numerator.rem_euclid(denominator);
    let offset = quotient + i128::from(remainder * 2 >= denominator);
    let ticks = i128::from(stream_start.ticks)
        .checked_add(offset)
        .ok_or_else(|| MediaError::InvalidInput("media timestamp overflow".to_owned()))?;
    let ticks = i64::try_from(ticks)
        .map_err(|_| MediaError::InvalidInput("media timestamp overflow".to_owned()))?;
    Ok(MediaTimestamp::new(ticks, timebase))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timebase_and_rational_parsing_are_exact() {
        assert_eq!(
            MediaTimebase::from_ffmpeg("1/15360"),
            MediaTimebase::new(1, 15_360)
        );
        assert_eq!(
            MediaTimebase::from_ffmpeg("1001/30000"),
            MediaTimebase::new(1001, 30_000)
        );
        assert_eq!(
            MediaRational::from_ffmpeg("30000/1001").unwrap().as_f64(),
            30_000.0 / 1001.0
        );
        assert!(MediaRational::from_ffmpeg("0/0").is_none());
        assert!(MediaTimebase::from_ffmpeg("0/0").is_none());
    }

    #[test]
    fn source_time_maps_through_rational_timebase_and_start() {
        let timebase = MediaTimebase::new(1, 15_360).expect("timebase");
        let start = MediaTimestamp::new(-512, timebase);
        let zero = source_ticks_to_media_timestamp(SourceTicks::ZERO, start).expect("zero");
        assert_eq!(zero, start);
        let one_second =
            source_ticks_to_media_timestamp(SourceTicks::new(120_000), start).expect("one");
        assert_eq!(one_second.ticks, -512 + 15_360);
        let one_frame =
            source_ticks_to_media_timestamp(SourceTicks::new(4_000), start).expect("frame");
        assert_eq!(one_frame.ticks, -512 + 512);
        let rounded = source_ticks_to_media_timestamp(SourceTicks::new(1), start).expect("tiny");
        assert_eq!(rounded.ticks, -512);
        assert!(source_ticks_to_media_timestamp(SourceTicks::new(-1), start).is_err());
    }

    #[test]
    fn nonzero_and_fractional_fps_timebases_preserve_ticks() {
        for (numerator, denominator, expected_ticks) in
            [(1, 15_360, 15_360), (1001, 30_000, 30), (1, 90_000, 90_000)]
        {
            let timebase = MediaTimebase::new(numerator, denominator).expect("timebase");
            let start = MediaTimestamp::new(1_234, timebase);
            let mapped =
                source_ticks_to_media_timestamp(SourceTicks::new(120_000), start).expect("mapped");
            assert_eq!(mapped.ticks, start.ticks + expected_ticks);
        }
    }

    #[test]
    fn display_dimensions_swap_for_quarter_turns() {
        let timebase = MediaTimebase::new(1, 1000).expect("timebase");
        let mut probe = MediaProbe {
            path: PathBuf::from("x.mp4"),
            container: "mp4".to_owned(),
            video_stream_index: 0,
            codec: "h264".to_owned(),
            width: 1920,
            height: 1080,
            pixel_format: Some("yuv420p".to_owned()),
            timebase,
            nominal_frame_rate: Some(MediaRational::new(30, 1).unwrap()),
            average_frame_rate: Some(MediaRational::new(30, 1).unwrap()),
            duration: Some(MediaTimestamp::new(3000, timebase)),
            stream_start: MediaTimestamp::new(0, timebase),
            rotation_degrees: 0,
            has_audio: false,
            frame_count_hint: Some(90),
            probe_elapsed_ms: 0,
        };
        assert_eq!(probe.display_dimensions(), (1920, 1080));
        probe.rotation_degrees = 90;
        assert_eq!(probe.display_dimensions(), (1080, 1920));
        probe.rotation_degrees = 180;
        assert_eq!(probe.display_dimensions(), (1920, 1080));
        probe.rotation_degrees = 270;
        assert_eq!(probe.display_dimensions(), (1080, 1920));
    }
}

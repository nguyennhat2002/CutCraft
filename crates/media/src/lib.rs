//! Media probing and bounded frame decoding.
//!
//! M3-C uses runtime-discovered FFmpeg CLI tools (`CUTCRAFT_FFMPEG` /
//! `CUTCRAFT_FFPROBE` or `PATH`); binaries are not bundled or linked. See
//! `docs/ADR/0005-ffmpeg-cli-preview-backend.md` for time, licensing, and
//! resource-bound decisions.

pub mod audio_resample;
pub mod cache;
pub mod decoder;
pub mod encoder;
mod error;
pub mod probe;
pub mod probe_queue;
mod process;

pub use decoder::{
    DecodedFrame, EngineMetrics, FrameRequest, PixelFormat, PreviewEngine, PreviewEvent,
};
pub use error::MediaError;
pub use probe::{
    probe_media, source_ticks_to_media_timestamp, MediaProbe, MediaRational, MediaTimebase,
    MediaTimestamp, MAX_FRAME_BYTES, MAX_FRAME_DIMENSION, MAX_FRAME_PIXELS, MAX_MEDIA_FILE_BYTES,
    SUPPORTED_VIDEO_CODEC,
};
pub use probe_queue::{ProbeEvent, ProbeExecutor, ProbeQueue, ProbeQueueMetrics, ProbeRequest};

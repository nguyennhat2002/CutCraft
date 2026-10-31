//! Typed media backend errors. No media operation panics on user input.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    ToolUnavailable(String),
    InvalidInput(String),
    ProbeFailed(String),
    InvalidMetadata(String),
    NoVideoStream,
    UnsupportedCodec(String),
    UnsupportedTimebase(String),
    DecodeFailed(String),
    FrameTooLarge { width: u32, height: u32 },
    FrameSizeMismatch { expected: usize, actual: usize },
    NoFrameAtTarget,
    EndOfStream,
    MissingPresentationTimestamp,
    PtsMismatch { expected: i64, actual: i64 },
    ProcessOutputTooLarge { stream: &'static str, limit: usize },
    Timeout { tool: String, timeout_ms: u64 },
    SourceNotOpen(u64),
    EngineClosed,
    Io(String),
}

impl fmt::Display for MediaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToolUnavailable(tool) => write!(
                formatter,
                "media decoder executable {tool:?} was not found; configure CUTCRAFT_FFMPEG/CUTCRAFT_FFPROBE or add it to PATH"
            ),
            Self::InvalidInput(message) => write!(formatter, "invalid media input: {message}"),
            Self::ProbeFailed(message) => write!(formatter, "media probe failed: {message}"),
            Self::InvalidMetadata(message) => {
                write!(formatter, "unsupported media metadata: {message}")
            }
            Self::NoVideoStream => formatter.write_str("media contains no video stream"),
            Self::UnsupportedCodec(codec) => {
                write!(formatter, "unsupported video codec {codec:?}; M3-C supports H.264")
            }
            Self::UnsupportedTimebase(value) => {
                write!(formatter, "unsupported media timebase {value}")
            }
            Self::DecodeFailed(message) => write!(formatter, "video decode failed: {message}"),
            Self::FrameTooLarge { width, height } => {
                write!(formatter, "decoded frame {width}x{height} exceeds the bounded frame size")
            }
            Self::FrameSizeMismatch { expected, actual } => write!(
                formatter,
                "decoded frame size mismatch: expected {expected} bytes, got {actual}"
            ),
            Self::NoFrameAtTarget => formatter.write_str("no video frame covers the requested time"),
            Self::EndOfStream => formatter.write_str("video stream ended"),
            Self::MissingPresentationTimestamp => {
                formatter.write_str("video frame has no usable presentation timestamp")
            }
            Self::PtsMismatch { expected, actual } => write!(
                formatter,
                "decoder returned presentation timestamp {actual}, expected {expected}"
            ),
            Self::ProcessOutputTooLarge { stream, limit } => write!(
                formatter,
                "media process {stream} output exceeded the {limit}-byte limit"
            ),
            Self::Timeout { tool, timeout_ms } => write!(
                formatter,
                "media process {tool:?} exceeded the {timeout_ms} ms timeout and was terminated"
            ),
            Self::SourceNotOpen(source_id) => {
                write!(formatter, "media source {source_id} is not open in the decoder")
            }
            Self::EngineClosed => formatter.write_str("media decoder engine is closed"),
            Self::Io(message) => write!(formatter, "media I/O failed: {message}"),
        }
    }
}

impl std::error::Error for MediaError {}

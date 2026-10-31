//! Bounded invocation of externally configured FFmpeg command-line tools.
//!
//! No binary path is hardcoded. Discovery uses `CUTCRAFT_FFMPEG` /
//! `CUTCRAFT_FFPROBE` or the normal PATH lookup. The tools are not bundled with
//! CutCraft in M3-C.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::MediaError;

pub(crate) const PROBE_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const FRAME_QUERY_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const DECODE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub(crate) struct MediaTools {
    pub ffmpeg: OsString,
    pub ffprobe: OsString,
}

impl MediaTools {
    pub(crate) fn discover() -> Self {
        Self {
            ffmpeg: std::env::var_os("CUTCRAFT_FFMPEG").unwrap_or_else(|| OsString::from("ffmpeg")),
            ffprobe: std::env::var_os("CUTCRAFT_FFPROBE")
                .unwrap_or_else(|| OsString::from("ffprobe")),
        }
    }
}

pub(crate) struct ToolOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// Run a tool without a shell, bounded by memory and with a hard output cap.
///
/// The pipes are drained even after the cap so a noisy child cannot deadlock
/// while waiting for a full stdout/stderr buffer.
pub(crate) fn run_tool(
    program: &OsStr,
    args: &[OsString],
    max_stdout: usize,
    max_stderr: usize,
    timeout: Duration,
) -> Result<ToolOutput, MediaError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            MediaError::ToolUnavailable(format!("{}: {error}", program.to_string_lossy()))
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| MediaError::Io("media process stdout was unavailable".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| MediaError::Io("media process stderr was unavailable".to_owned()))?;
    let stdout_reader = thread::spawn(move || read_capped(stdout, max_stdout));
    let stderr_reader = thread::spawn(move || read_capped(stderr, max_stderr));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(MediaError::Timeout {
                        tool: program.to_string_lossy().into_owned(),
                        timeout_ms: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                    });
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(MediaError::Io(error.to_string()));
            }
        }
    };
    let (stdout, stdout_overflow) = stdout_reader
        .join()
        .map_err(|_| MediaError::Io("media stdout reader panicked".to_owned()))?
        .map_err(|error| MediaError::Io(error.to_string()))?;
    let (stderr, stderr_overflow) = stderr_reader
        .join()
        .map_err(|_| MediaError::Io("media stderr reader panicked".to_owned()))?
        .map_err(|error| MediaError::Io(error.to_string()))?;
    if stdout_overflow {
        return Err(MediaError::ProcessOutputTooLarge {
            stream: "stdout",
            limit: max_stdout,
        });
    }
    if stderr_overflow {
        return Err(MediaError::ProcessOutputTooLarge {
            stream: "stderr",
            limit: max_stderr,
        });
    }
    Ok(ToolOutput {
        success: status.success(),
        stdout,
        stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
    })
}

fn read_capped(mut reader: impl Read, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::with_capacity(limit.min(64 * 1024));
    let mut total = 0_usize;
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read);
        if output.len() < limit {
            let remaining = limit - output.len();
            output.extend_from_slice(&chunk[..read.min(remaining)]);
        }
    }
    Ok((output, total > limit))
}

pub(crate) fn os(value: impl AsRef<OsStr>) -> OsString {
    value.as_ref().to_os_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_tool_run_returns_bounded_output() {
        let (program, args): (&str, Vec<OsString>) = if cfg!(windows) {
            (
                "cmd",
                vec![OsString::from("/C"), OsString::from("echo hello")],
            )
        } else {
            (
                "sh",
                vec![OsString::from("-c"), OsString::from("echo hello")],
            )
        };
        let output = run_tool(
            OsStr::new(program),
            &args,
            1024,
            1024,
            Duration::from_secs(5),
        )
        .expect("tool");
        assert!(output.success);
        assert!(String::from_utf8_lossy(&output.stdout).contains("hello"));
    }

    #[test]
    fn hung_tool_is_terminated_at_the_timeout() {
        let (program, args): (&str, Vec<OsString>) = if cfg!(windows) {
            (
                "ping",
                vec![
                    OsString::from("-n"),
                    OsString::from("10"),
                    OsString::from("127.0.0.1"),
                ],
            )
        } else {
            ("sleep", vec![OsString::from("5")])
        };
        let started = Instant::now();
        let result = run_tool(
            OsStr::new(program),
            &args,
            1024,
            1024,
            Duration::from_millis(200),
        );
        assert!(matches!(result, Err(MediaError::Timeout { .. })));
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "timeout path took too long: {:?}",
            started.elapsed()
        );
    }
}

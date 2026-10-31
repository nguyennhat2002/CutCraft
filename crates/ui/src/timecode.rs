//! Display-only timecode formatting for preview and status indicators.
//!
//! This is **not** authoritative timeline math. The engine owns integer tick
//! time; these helpers only format already-authoritative values for display.

use capcut_core::time::{FrameRate, TimelineTicks};

/// Format `total_millis` as `HH:MM:SS:FF` for the rational rate
/// `fps_num / fps_den`.
///
/// Returns `None` for a non-positive or implausibly large frame rate. Frames
/// are computed with integer arithmetic so half-open frame boundaries do not
/// drift with floating-point rounding.
pub fn format_timecode(total_millis: u64, fps_num: u32, fps_den: u32) -> Option<String> {
    if fps_num == 0 || fps_den == 0 {
        return None;
    }

    let fps_num = u64::from(fps_num);
    let fps_den = u64::from(fps_den);

    // Reject rates below 1 fps and above 1000 fps; both indicate a caller bug
    // in a display path rather than a supported editor rate.
    if fps_num < fps_den || fps_num > fps_den.saturating_mul(1000) {
        return None;
    }

    let total_seconds = total_millis / 1_000;
    let remaining_millis = u128::from(total_millis % 1_000);
    let frames = remaining_millis * u128::from(fps_num) / (1_000 * u128::from(fps_den));

    Some(format_timecode_parts(
        total_seconds,
        u64::try_from(frames).unwrap_or(0),
    ))
}

/// Format an exact tick position using the project's rational FPS.
///
/// This is a non-drop-frame display timecode: the nominal rate is
/// `ceil(fps_num / fps_den)` frames per second, and the frame index is derived
/// from exact ticks. It never uses an f64 second value as edit time.
pub fn format_timecode_ticks(ticks: TimelineTicks, fps: FrameRate) -> Option<String> {
    if ticks.get() < 0 {
        return None;
    }
    fps.ticks_per_frame().ok()?;
    if fps.numerator == 0 || fps.denominator == 0 {
        return None;
    }
    let nominal = u64::from(fps.numerator).div_ceil(u64::from(fps.denominator));
    if nominal == 0 {
        return None;
    }
    let frame = u64::try_from(fps.ticks_to_frame_floor(ticks).ok()?).ok()?;
    Some(format_timecode_parts(frame / nominal, frame % nominal))
}

fn format_timecode_parts(total_seconds: u64, frames: u64) -> String {
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_zero_timecode() {
        assert_eq!(format_timecode(0, 30, 1).as_deref(), Some("00:00:00:00"));
    }

    #[test]
    fn formats_whole_seconds_and_minutes() {
        assert_eq!(
            format_timecode(1_000, 30, 1).as_deref(),
            Some("00:00:01:00")
        );
        assert_eq!(
            format_timecode(61_000, 30, 1).as_deref(),
            Some("00:01:01:00")
        );
        assert_eq!(
            format_timecode(3_600_000, 30, 1).as_deref(),
            Some("01:00:00:00")
        );
    }

    #[test]
    fn formats_last_frame_before_the_next_second() {
        assert_eq!(format_timecode(999, 30, 1).as_deref(), Some("00:00:00:29"));
    }

    #[test]
    fn supports_ntsc_rational_rates_without_rounding_to_30_fps() {
        assert_eq!(
            format_timecode(999, 30_000, 1_001).as_deref(),
            Some("00:00:00:29")
        );
        assert_eq!(
            format_timecode(1_000, 30_000, 1_001).as_deref(),
            Some("00:00:01:00")
        );
    }

    #[test]
    fn rejects_invalid_or_implausible_rates() {
        assert_eq!(format_timecode(0, 0, 1), None);
        assert_eq!(format_timecode(0, 30, 0), None);
        assert_eq!(format_timecode(0, 1, 2), None);
        assert_eq!(format_timecode(0, 2_000, 1), None);
    }

    #[test]
    fn exact_tick_timecode_uses_frame_indices_at_all_supported_rates() {
        for rate in [
            FrameRate::new(24, 1).expect("24"),
            FrameRate::new(25, 1).expect("25"),
            FrameRate::new(30, 1).expect("30"),
            FrameRate::new(60, 1).expect("60"),
            FrameRate::new(24_000, 1_001).expect("23.976"),
            FrameRate::new(30_000, 1_001).expect("29.97"),
        ] {
            assert_eq!(
                format_timecode_ticks(TimelineTicks::ZERO, rate).as_deref(),
                Some("00:00:00:00")
            );
            let nominal = u64::from(rate.numerator).div_ceil(u64::from(rate.denominator));
            let last = rate.frame_to_ticks(nominal as i64 - 1).expect("frame");
            assert_eq!(
                format_timecode_ticks(last, rate).as_deref(),
                Some(format!("00:00:00:{:02}", nominal - 1).as_str()),
                "rate {}/{}",
                rate.numerator,
                rate.denominator
            );
            let next_second = rate.frame_to_ticks(nominal as i64).expect("frame");
            assert_eq!(
                format_timecode_ticks(next_second, rate).as_deref(),
                Some("00:00:01:00"),
                "rate {}/{}",
                rate.numerator,
                rate.denominator
            );
        }
    }

    #[test]
    fn exact_tick_timecode_rejects_negative_and_returns_none_for_invalid_rates() {
        assert_eq!(
            format_timecode_ticks(TimelineTicks::new(-1), FrameRate::new(30, 1).expect("rate")),
            None
        );
    }
}

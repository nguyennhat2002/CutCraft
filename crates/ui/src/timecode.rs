//! Display-only timecode formatting for preview and status indicators.
//!
//! This is **not** authoritative timeline math. The engine will own integer or
//! rational timeline time; the preview currently shows a static placeholder and
//! only needs a stable string format.

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

    let hours = total_seconds / 3_600;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;

    Some(format!("{hours:02}:{minutes:02}:{seconds:02}:{frames:02}"))
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
}

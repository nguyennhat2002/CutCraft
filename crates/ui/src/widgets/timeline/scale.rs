//! Display conversion between timeline seconds and screen points.
//!
//! This is presentation math only. It does not define the engine's authoritative
//! time representation, which remains a future core/engine decision.

use crate::config;

const MIN_PIXELS_PER_SECOND: f32 = config::TIMELINE_MIN_PIXELS_PER_SECOND;
const MAX_PIXELS_PER_SECOND: f32 = config::TIMELINE_MAX_PIXELS_PER_SECOND;
// The reference uses about 300 points between labels at its 30 pt/s scale.
const MIN_MAJOR_LABEL_SPACING: f32 = 240.0;
const MAX_TICKS: usize = 2_000;
const MAJOR_INTERVALS: [u64; 10] = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600];
const FALLBACK_INTERVAL: u64 = 600;

/// Horizontal mapping for the timeline viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineScale {
    pixels_per_second: f32,
    leading_padding: f32,
}

impl TimelineScale {
    /// Create a scale, clamping implausible values to a safe display range.
    pub fn new(pixels_per_second: f32, leading_padding: f32) -> Self {
        let pixels_per_second = if pixels_per_second.is_finite() {
            pixels_per_second.clamp(MIN_PIXELS_PER_SECOND, MAX_PIXELS_PER_SECOND)
        } else {
            config::TIMELINE_PIXELS_PER_SECOND
        };
        let leading_padding = if leading_padding.is_finite() {
            leading_padding.max(0.0)
        } else {
            0.0
        };

        Self {
            pixels_per_second,
            leading_padding,
        }
    }

    pub fn pixels_per_second(&self) -> f32 {
        self.pixels_per_second
    }

    pub fn leading_padding(&self) -> f32 {
        self.leading_padding
    }

    /// Screen x offset for a duration in seconds.
    ///
    /// Negative or non-finite times collapse to the leading padding.
    pub fn x_for_seconds(&self, seconds: f64) -> f32 {
        if !seconds.is_finite() || seconds <= 0.0 {
            return self.leading_padding;
        }

        let x = f64::from(self.leading_padding) + seconds * f64::from(self.pixels_per_second);
        if x >= f64::from(f32::MAX) {
            f32::MAX
        } else {
            x as f32
        }
    }

    /// Total scrollable content width, including leading and trailing padding.
    pub fn content_width(&self, duration_seconds: u64) -> f32 {
        let width = self.x_for_seconds(duration_seconds as f64) + self.leading_padding;
        width.max(1.0)
    }

    /// Seconds between labeled ruler ticks.
    pub fn major_tick_interval(&self) -> u64 {
        MAJOR_INTERVALS
            .iter()
            .copied()
            .find(|interval| *interval as f32 * self.pixels_per_second >= MIN_MAJOR_LABEL_SPACING)
            .unwrap_or(FALLBACK_INTERVAL)
    }

    /// Visible major ticks, including the preceding tick so its minor divisions
    /// remain visible while horizontally scrolled. Offsets are relative to time
    /// content, not the window or pinned headers. Work is bounded independently
    /// of project duration, and the ruler fills even an empty, wide viewport.
    pub fn ticks_in_view(&self, left: f32, right: f32) -> Vec<u64> {
        if !left.is_finite() || !right.is_finite() || right < left {
            return Vec::new();
        }
        let seconds_at = |x: f32| {
            (f64::from(x - self.leading_padding) / f64::from(self.pixels_per_second)).max(0.0)
        };
        let interval = self.major_tick_interval();
        let first = (seconds_at(left) / interval as f64).floor() as u64;
        let last = (seconds_at(right) / interval as f64).ceil() as u64;
        let count = last
            .saturating_sub(first)
            .saturating_add(1)
            .min(MAX_TICKS as u64);
        (0..count)
            .filter_map(|index| first.checked_add(index)?.checked_mul(interval))
            .collect()
    }

    /// Ruler tick positions in whole seconds, inclusive of the duration.
    ///
    /// The interval widens for very long durations so a malformed config cannot
    /// produce an unbounded widget or allocation.
    pub fn ticks(&self, duration_seconds: u64) -> Vec<u64> {
        let mut interval = self.major_tick_interval();
        let estimated = (duration_seconds / interval).saturating_add(1);

        if estimated > MAX_TICKS as u64 {
            let factor = estimated.div_ceil(MAX_TICKS as u64);
            interval = interval.saturating_mul(factor.max(1));
        }

        let mut ticks = Vec::new();
        let mut second = 0_u64;

        while ticks.len() < MAX_TICKS {
            ticks.push(second);

            let Some(next) = second.checked_add(interval) else {
                break;
            };
            if next > duration_seconds {
                break;
            }
            second = next;
        }

        ticks
    }
}

/// Compact ruler label: `00:00`, `00:05`, `01:30`.
pub fn format_ruler_label(total_seconds: u64) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_scale() -> TimelineScale {
        TimelineScale::new(config::TIMELINE_PIXELS_PER_SECOND, 14.0)
    }

    #[test]
    fn invalid_scale_inputs_are_replaced_or_clamped() {
        let nan = TimelineScale::new(f32::NAN, f32::NAN);
        assert_eq!(nan.pixels_per_second(), config::TIMELINE_PIXELS_PER_SECOND);
        assert_eq!(nan.leading_padding(), 0.0);

        assert_eq!(
            TimelineScale::new(-10.0, -4.0).pixels_per_second(),
            MIN_PIXELS_PER_SECOND
        );
        assert_eq!(
            TimelineScale::new(10_000.0, -4.0).pixels_per_second(),
            MAX_PIXELS_PER_SECOND
        );
        assert_eq!(TimelineScale::new(80.0, -4.0).leading_padding(), 0.0);
    }

    #[test]
    fn maps_seconds_to_points_including_leading_padding() {
        let scale = default_scale();

        assert_eq!(scale.x_for_seconds(0.0), 14.0);
        assert!((scale.x_for_seconds(2.0) - 74.0).abs() < 0.01);
        assert_eq!(scale.x_for_seconds(-1.0), 14.0);
        assert_eq!(scale.x_for_seconds(f64::NAN), 14.0);
    }

    #[test]
    fn content_width_covers_duration_and_padding() {
        let scale = default_scale();

        assert!((scale.content_width(60) - (14.0 + 1800.0 + 14.0)).abs() < 0.01);
        assert!(scale.content_width(0) >= 1.0);
    }

    #[test]
    fn tick_interval_grows_as_pixels_per_second_shrink() {
        assert_eq!(TimelineScale::new(80.0, 0.0).major_tick_interval(), 5);
        assert_eq!(TimelineScale::new(40.0, 0.0).major_tick_interval(), 10);
        assert_eq!(TimelineScale::new(30.0, 0.0).major_tick_interval(), 10);
        assert_eq!(TimelineScale::new(10.0, 0.0).major_tick_interval(), 30);
        assert_eq!(TimelineScale::new(4.0, 0.0).major_tick_interval(), 60);
    }

    #[test]
    fn ticks_include_zero_and_the_duration_boundary() {
        let ticks = default_scale().ticks(30);

        assert_eq!(ticks, vec![0, 10, 20, 30]);
    }

    #[test]
    fn excessive_tick_counts_are_bounded() {
        let ticks = default_scale().ticks(u64::MAX);

        assert!(!ticks.is_empty());
        assert!(ticks.len() <= MAX_TICKS);
        assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn ruler_labels_are_compact_and_stable() {
        assert_eq!(format_ruler_label(0), "00:00");
        assert_eq!(format_ruler_label(5), "00:05");
        assert_eq!(format_ruler_label(90), "01:30");
        assert_eq!(format_ruler_label(3_600), "60:00");
    }
}

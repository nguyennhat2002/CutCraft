//! Display conversion between timeline seconds and screen points.
//!
//! This is presentation math only. It does not define the engine's authoritative
//! time representation, which remains a future core/engine decision.

use crate::config;

const MIN_PIXELS_PER_SECOND: f32 = 20.0;
const MAX_PIXELS_PER_SECOND: f32 = 320.0;
const MIN_MAJOR_LABEL_SPACING: f32 = 72.0;
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

/// Compact ruler label: `0:00`, `0:05`, `1:30`.
pub fn format_ruler_label(total_seconds: u64) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}:{seconds:02}")
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
        assert!((scale.x_for_seconds(2.0) - 174.0).abs() < 0.01);
        assert_eq!(scale.x_for_seconds(-1.0), 14.0);
        assert_eq!(scale.x_for_seconds(f64::NAN), 14.0);
    }

    #[test]
    fn content_width_covers_duration_and_padding() {
        let scale = default_scale();

        assert!((scale.content_width(60) - (14.0 + 4_800.0 + 14.0)).abs() < 0.01);
        assert!(scale.content_width(0) >= 1.0);
    }

    #[test]
    fn tick_interval_grows_as_pixels_per_second_shrink() {
        assert_eq!(TimelineScale::new(80.0, 0.0).major_tick_interval(), 1);
        assert_eq!(TimelineScale::new(40.0, 0.0).major_tick_interval(), 2);
        assert_eq!(TimelineScale::new(20.0, 0.0).major_tick_interval(), 5);
    }

    #[test]
    fn ticks_include_zero_and_the_duration_boundary() {
        let ticks = default_scale().ticks(5);

        assert_eq!(ticks, vec![0, 1, 2, 3, 4, 5]);
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
        assert_eq!(format_ruler_label(0), "0:00");
        assert_eq!(format_ruler_label(5), "0:05");
        assert_eq!(format_ruler_label(90), "1:30");
        assert_eq!(format_ruler_label(3_600), "60:00");
    }
}

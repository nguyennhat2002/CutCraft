//! Single reusable mapping between authoritative timeline ticks and editor
//! viewport points.
//!
//! Pointer coordinates are display input; the returned playhead position is an
//! exact frame tick or the exact project end. No f64 seconds are stored.

use capcut_core::time::{FrameRate, TimelineTicks, TIMELINE_TICKS_PER_SECOND};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineMapping {
    origin_x: f64,
    pixels_per_tick: f64,
    duration: TimelineTicks,
    frame_rate: FrameRate,
}

impl TimelineMapping {
    /// `origin_x` is the screen point of tick zero (content left + header width
    /// + leading padding). `pixels_per_second` is the current horizontal zoom.
    pub fn new(
        origin_x: f32,
        pixels_per_second: f32,
        duration: TimelineTicks,
        frame_rate: FrameRate,
    ) -> Option<Self> {
        if !origin_x.is_finite()
            || !pixels_per_second.is_finite()
            || pixels_per_second <= 0.0
            || duration.get() < 0
            || frame_rate.ticks_per_frame().is_err()
        {
            return None;
        }
        Some(Self {
            origin_x: f64::from(origin_x),
            pixels_per_tick: f64::from(pixels_per_second) / TIMELINE_TICKS_PER_SECOND as f64,
            duration,
            frame_rate,
        })
    }

    pub fn origin_x(&self) -> f32 {
        self.origin_x as f32
    }

    pub fn duration(&self) -> TimelineTicks {
        self.duration
    }

    pub fn frame_rate(&self) -> FrameRate {
        self.frame_rate
    }

    /// Display x for an authoritative tick. Ticks outside the project range are
    /// clamped; the result is bounded to finite f32 range.
    pub fn x_for_ticks(&self, ticks: TimelineTicks) -> f32 {
        let ticks = ticks.get().clamp(0, self.duration.get());
        let x = self.origin_x + ticks as f64 * self.pixels_per_tick;
        if x.is_finite() {
            x.clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32
        } else if x.is_sign_positive() {
            f32::MAX
        } else {
            f32::MIN
        }
    }

    /// Convert a pointer x into the nearest project frame. Invalid coordinates
    /// return `None`; points outside the project clamp to zero/end.
    pub fn nearest_frame_ticks(&self, x: f32) -> Option<TimelineTicks> {
        if !x.is_finite() || !self.origin_x.is_finite() || !self.pixels_per_tick.is_finite() {
            return None;
        }
        if self.duration.get() == 0 {
            return Some(TimelineTicks::ZERO);
        }
        let raw = (f64::from(x) - self.origin_x) / self.pixels_per_tick;
        if !raw.is_finite() {
            return None;
        }
        if raw <= 0.0 {
            return Some(TimelineTicks::ZERO);
        }
        if raw >= self.duration.get() as f64 {
            return Some(self.duration);
        }
        let ticks_per_frame = self.frame_rate.ticks_per_frame().ok()?;
        if ticks_per_frame <= 0 {
            return None;
        }
        let frame = (raw / ticks_per_frame as f64).round();
        if !frame.is_finite() {
            return None;
        }
        if frame <= 0.0 {
            return Some(TimelineTicks::ZERO);
        }
        if frame >= i64::MAX as f64 {
            return Some(self.duration);
        }
        let ticks = self.frame_rate.frame_to_ticks(frame as i64).ok()?;
        Some(if ticks.get() > self.duration.get() {
            self.duration
        } else {
            ticks
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capcut_core::time::TIMELINE_TICKS_PER_SECOND;

    fn rates() -> [FrameRate; 6] {
        [
            FrameRate::new(24, 1).expect("24"),
            FrameRate::new(25, 1).expect("25"),
            FrameRate::new(30, 1).expect("30"),
            FrameRate::new(60, 1).expect("60"),
            FrameRate::new(24_000, 1_001).expect("23.976"),
            FrameRate::new(30_000, 1_001).expect("29.97"),
        ]
    }

    fn duration(seconds: i64) -> TimelineTicks {
        TimelineTicks::new(seconds * TIMELINE_TICKS_PER_SECOND)
    }

    #[test]
    fn exact_frame_boundaries_round_trip_at_all_rates() {
        for rate in rates() {
            let end = duration(120);
            let mapping =
                TimelineMapping::new(173.5, 47.0, end, rate).expect("mapping should be valid");
            for frame in [0, 1, 2, 10, 1_000, 10_000] {
                let ticks = rate.frame_to_ticks(frame).expect("frame ticks");
                if ticks.get() > end.get() {
                    continue;
                }
                let x = mapping.x_for_ticks(ticks);
                assert_eq!(
                    mapping.nearest_frame_ticks(x),
                    Some(ticks),
                    "rate {}/{} frame {frame}",
                    rate.numerator,
                    rate.denominator
                );
            }
        }
    }

    #[test]
    fn scroll_zoom_and_high_dpi_affine_scaling_preserve_ticks() {
        let rate = FrameRate::new(30_000, 1_001).expect("rate");
        let end = duration(60);
        let ticks = rate.frame_to_ticks(720).expect("frame");
        for pixels_per_second in [4.0, 30.0, 120.0, 400.0] {
            for origin in [-900.0_f32, -20.0, 0.0, 173.5, 2_000.0] {
                let mapping =
                    TimelineMapping::new(origin, pixels_per_second, end, rate).expect("mapping");
                let x = mapping.x_for_ticks(ticks);
                assert_eq!(mapping.nearest_frame_ticks(x), Some(ticks));

                let doubled =
                    TimelineMapping::new(origin * 2.0, pixels_per_second * 2.0, end, rate)
                        .expect("doubled mapping");
                let x2 = doubled.x_for_ticks(ticks);
                assert_eq!(doubled.nearest_frame_ticks(x2), Some(ticks));
            }
        }
    }

    #[test]
    fn pointer_outside_project_and_nonfinite_inputs_are_clamped_or_rejected() {
        let rate = FrameRate::new(30, 1).expect("rate");
        let mapping = TimelineMapping::new(100.0, 30.0, duration(20), rate).expect("mapping");
        assert_eq!(
            mapping.nearest_frame_ticks(-100.0),
            Some(TimelineTicks::ZERO)
        );
        assert_eq!(
            mapping.nearest_frame_ticks(100.0),
            Some(TimelineTicks::ZERO)
        );
        assert_eq!(mapping.nearest_frame_ticks(10_000.0), Some(duration(20)));
        assert_eq!(mapping.nearest_frame_ticks(f32::NAN), None);
        assert_eq!(mapping.nearest_frame_ticks(f32::INFINITY), None);
        assert!(TimelineMapping::new(f32::NAN, 30.0, duration(20), rate).is_none());
        assert!(TimelineMapping::new(0.0, f32::NAN, duration(20), rate).is_none());
        assert!(TimelineMapping::new(0.0, -1.0, duration(20), rate).is_none());
        assert!(TimelineMapping::new(0.0, 30.0, TimelineTicks::new(-1), rate).is_none());
    }

    #[test]
    fn zero_duration_and_non_frame_aligned_end_are_safe() {
        let rate = FrameRate::new(30, 1).expect("rate");
        let empty = TimelineMapping::new(50.0, 30.0, TimelineTicks::ZERO, rate).expect("empty");
        assert_eq!(
            empty.nearest_frame_ticks(50_000.0),
            Some(TimelineTicks::ZERO)
        );
        assert_eq!(
            empty.x_for_ticks(rate.frame_to_ticks(100).expect("frame")),
            50.0
        );

        let end = TimelineTicks::new(5 * TIMELINE_TICKS_PER_SECOND + 1);
        let mapping = TimelineMapping::new(0.0, 30.0, end, rate).expect("mapping");
        assert_eq!(mapping.nearest_frame_ticks(f32::MAX), Some(end));
        assert!((mapping.x_for_ticks(end) - 150.0).abs() < 0.01);
    }

    #[test]
    fn nearest_frame_rounds_instead_of_accumulating_drift() {
        let rate = FrameRate::new(30, 1).expect("rate");
        let end = duration(60);
        let mapping = TimelineMapping::new(0.0, 30.0, end, rate).expect("mapping");
        let frame_ticks = rate.ticks_per_frame().expect("tpf");
        let x_between = (frame_ticks as f32) * 0.49 / 120_000.0 * 30.0;
        assert_eq!(
            mapping.nearest_frame_ticks(x_between),
            Some(TimelineTicks::ZERO)
        );
        let x_after = (frame_ticks as f32) * 0.51 / 120_000.0 * 30.0;
        assert_eq!(
            mapping.nearest_frame_ticks(x_after),
            Some(rate.frame_to_ticks(1).expect("frame"))
        );
    }

    #[test]
    fn large_ticks_stay_finite() {
        let rate = FrameRate::new(30, 1).expect("rate");
        let end = TimelineTicks::new(i64::MAX / 4);
        let mapping = TimelineMapping::new(f32::MAX / 2.0, 1.0, end, rate).expect("mapping");
        let x = mapping.x_for_ticks(end);
        assert!(x.is_finite());
        assert_eq!(mapping.nearest_frame_ticks(f32::MAX), Some(end));
    }
}

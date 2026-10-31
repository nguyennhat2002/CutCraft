//! Shared timeline time helpers.
//!
//! Native edit time is an exact integer tick. Display seconds are produced by
//! the UI projection and are never authoritative edit state.

use std::fmt;

/// Exact native timeline timebase.
pub const TIMELINE_TICKS_PER_SECOND: i64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimelineTicks(i64);

impl TimelineTicks {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SourceTicks(i64);

impl SourceTicks {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timebase {
    pub ticks_per_second: i64,
}

impl Timebase {
    pub fn new(ticks_per_second: i64) -> Result<Self, TimeError> {
        if ticks_per_second <= 0 {
            return Err(TimeError::InvalidTimebase);
        }
        Ok(Self { ticks_per_second })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl FrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, TimeError> {
        if numerator == 0 || denominator == 0 {
            return Err(TimeError::InvalidRate);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub fn ticks_per_frame(self) -> Result<i64, TimeError> {
        let supported = matches!(
            (self.numerator, self.denominator),
            (24, 1) | (25, 1) | (30, 1) | (60, 1) | (24_000, 1_001) | (30_000, 1_001)
        );
        if !supported {
            return Err(TimeError::UnsupportedRate);
        }
        let numerator = i128::from(TIMELINE_TICKS_PER_SECOND) * i128::from(self.denominator);
        let denominator = i128::from(self.numerator);
        if numerator % denominator != 0 {
            return Err(TimeError::InexactFrameConversion);
        }
        i64::try_from(numerator / denominator).map_err(|_| TimeError::Overflow)
    }

    pub fn frame_to_ticks(self, frame: i64) -> Result<TimelineTicks, TimeError> {
        if frame < 0 {
            return Err(TimeError::InvalidFrameIndex);
        }
        let ticks = i128::from(frame) * i128::from(self.ticks_per_frame()?);
        i64::try_from(ticks)
            .map(TimelineTicks::new)
            .map_err(|_| TimeError::Overflow)
    }

    pub fn ticks_to_frame_floor(self, ticks: TimelineTicks) -> Result<i64, TimeError> {
        if ticks.get() < 0 {
            return Err(TimeError::InvalidFrameIndex);
        }
        let numerator = i128::from(ticks.get()) * i128::from(self.numerator);
        let denominator = i128::from(TIMELINE_TICKS_PER_SECOND) * i128::from(self.denominator);
        i64::try_from(numerator / denominator).map_err(|_| TimeError::Overflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineRange {
    start: TimelineTicks,
    end: TimelineTicks,
}

impl TimelineRange {
    pub fn new(start: TimelineTicks, end: TimelineTicks) -> Result<Self, TimeError> {
        if start.get() < 0 || end <= start {
            return Err(TimeError::InvalidRange);
        }
        Ok(Self { start, end })
    }

    pub const fn start(self) -> TimelineTicks {
        self.start
    }

    pub const fn end(self) -> TimelineTicks {
        self.end
    }

    pub fn duration(self) -> TimelineTicks {
        TimelineTicks::new(self.end.get() - self.start.get())
    }

    pub fn contains(self, time: TimelineTicks) -> bool {
        self.start <= time && time < self.end
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn shifted_left(self, delta: TimelineTicks) -> Result<Self, TimeError> {
        if delta.get() < 0 {
            return Err(TimeError::InvalidRange);
        }
        Self::new(
            self.start.checked_sub(delta).ok_or(TimeError::Overflow)?,
            self.end.checked_sub(delta).ok_or(TimeError::Overflow)?,
        )
    }

    pub fn shifted_right(self, delta: TimelineTicks) -> Result<Self, TimeError> {
        if delta.get() < 0 {
            return Err(TimeError::InvalidRange);
        }
        Self::new(
            self.start.checked_add(delta).ok_or(TimeError::Overflow)?,
            self.end.checked_add(delta).ok_or(TimeError::Overflow)?,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    start: SourceTicks,
    end: SourceTicks,
}

impl SourceRange {
    pub fn new(start: SourceTicks, end: SourceTicks) -> Result<Self, TimeError> {
        if start.get() < 0 || end <= start {
            return Err(TimeError::InvalidRange);
        }
        Ok(Self { start, end })
    }

    pub const fn start(self) -> SourceTicks {
        self.start
    }

    pub const fn end(self) -> SourceTicks {
        self.end
    }

    pub fn duration(self) -> SourceTicks {
        SourceTicks::new(self.end.get() - self.start.get())
    }

    pub fn contains(self, time: SourceTicks) -> bool {
        self.start <= time && time < self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    InvalidRate,
    UnsupportedRate,
    InexactFrameConversion,
    InvalidFrameIndex,
    InvalidTimebase,
    InvalidRange,
    Overflow,
}

impl fmt::Display for TimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRate => formatter.write_str("frame rate must be a positive fraction"),
            Self::UnsupportedRate => formatter.write_str("unsupported frame rate"),
            Self::InexactFrameConversion => {
                formatter.write_str("frame rate cannot be represented exactly in timeline ticks")
            }
            Self::InvalidFrameIndex => formatter.write_str("frame index must be non-negative"),
            Self::InvalidTimebase => formatter.write_str("timebase must be positive"),
            Self::InvalidRange => formatter.write_str("time range must be positive and half-open"),
            Self::Overflow => formatter.write_str("time arithmetic overflow"),
        }
    }
}

impl std::error::Error for TimeError {}

fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_rates_convert_exactly() {
        for (numerator, denominator, ticks) in [
            (24, 1, 5_000),
            (25, 1, 4_800),
            (30, 1, 4_000),
            (60, 1, 2_000),
            (24_000, 1_001, 5_005),
            (30_000, 1_001, 4_004),
        ] {
            let rate = FrameRate::new(numerator, denominator).expect("valid rate");
            assert_eq!(rate.ticks_per_frame(), Ok(ticks));
            assert_eq!(
                rate.frame_to_ticks(1).expect("frame"),
                TimelineTicks::new(ticks)
            );
            assert_eq!(
                rate.ticks_to_frame_floor(TimelineTicks::new(ticks))
                    .expect("frame index"),
                1
            );
        }
    }

    #[test]
    fn unsupported_and_invalid_rates_are_rejected() {
        assert_eq!(
            FrameRate::new(23, 1)
                .expect("valid rational")
                .ticks_per_frame(),
            Err(TimeError::UnsupportedRate)
        );
        assert_eq!(FrameRate::new(0, 1), Err(TimeError::InvalidRate));
        assert_eq!(FrameRate::new(30, 0), Err(TimeError::InvalidRate));
        assert_eq!(
            FrameRate::new(30_000, 1_000)
                .expect("normalized 30/1")
                .ticks_per_frame(),
            Ok(4_000)
        );
    }

    #[test]
    fn long_sequences_use_index_math_and_reject_overflow() {
        let rate = FrameRate::new(30_000, 1_001).expect("rate");
        assert_eq!(
            rate.frame_to_ticks(1_000_000).expect("frame").get(),
            4_004_000_000
        );
        assert_eq!(rate.frame_to_ticks(i64::MAX), Err(TimeError::Overflow));
        assert_eq!(
            rate.ticks_to_frame_floor(TimelineTicks::new(4_004_000_001))
                .expect("floor"),
            1_000_000
        );
    }

    #[test]
    fn ranges_are_positive_half_open_and_checked() {
        let range =
            TimelineRange::new(TimelineTicks::new(10), TimelineTicks::new(20)).expect("range");
        assert_eq!(range.duration(), TimelineTicks::new(10));
        assert!(range.contains(TimelineTicks::new(10)));
        assert!(range.contains(TimelineTicks::new(19)));
        assert!(!range.contains(TimelineTicks::new(20)));
        assert_eq!(
            TimelineRange::new(TimelineTicks::new(20), TimelineTicks::new(20)),
            Err(TimeError::InvalidRange)
        );
        assert_eq!(
            TimelineRange::new(TimelineTicks::new(20), TimelineTicks::new(10)),
            Err(TimeError::InvalidRange)
        );
        assert_eq!(
            TimelineRange::new(TimelineTicks::new(-1), TimelineTicks::new(10)),
            Err(TimeError::InvalidRange)
        );
        assert!(range.overlaps(
            TimelineRange::new(TimelineTicks::new(19), TimelineTicks::new(30)).expect("range")
        ));
        assert!(!range.overlaps(
            TimelineRange::new(TimelineTicks::new(20), TimelineTicks::new(30)).expect("range")
        ));
    }

    #[test]
    fn large_frame_indices_and_overflow_are_checked_for_every_supported_rate() {
        for (numerator, denominator, ticks_per_frame) in [
            (24, 1, 5_000),
            (25, 1, 4_800),
            (30, 1, 4_000),
            (60, 1, 2_000),
            (24_000, 1_001, 5_005),
            (30_000, 1_001, 4_004),
        ] {
            let rate = FrameRate::new(numerator, denominator).expect("rate");
            let frame = 10_000_000i64;
            let ticks = rate.frame_to_ticks(frame).expect("large frame");
            assert_eq!(ticks.get(), frame * ticks_per_frame);
            assert_eq!(
                rate.ticks_to_frame_floor(ticks).expect("floor"),
                frame,
                "exact boundary must not drift for {numerator}/{denominator}"
            );
            assert_eq!(
                rate.ticks_to_frame_floor(TimelineTicks::new(ticks.get() + ticks_per_frame - 1))
                    .expect("last tick of frame"),
                frame
            );
            assert_eq!(
                rate.frame_to_ticks(i64::MAX),
                Err(TimeError::Overflow),
                "overflow must be rejected for {numerator}/{denominator}"
            );
        }
    }

    #[test]
    fn checked_tick_arithmetic_reports_overflow() {
        assert_eq!(
            TimelineTicks::new(i64::MAX).checked_add(TimelineTicks::new(1)),
            None
        );
        assert_eq!(
            TimelineTicks::new(i64::MIN).checked_sub(TimelineTicks::new(1)),
            None
        );
        assert_eq!(
            SourceTicks::new(10).checked_add(SourceTicks::new(5)),
            Some(SourceTicks::new(15))
        );
    }

    #[test]
    fn source_timebase_must_be_positive() {
        assert!(Timebase::new(120_000).is_ok());
        assert_eq!(Timebase::new(0), Err(TimeError::InvalidTimebase));
        assert_eq!(Timebase::new(-1), Err(TimeError::InvalidTimebase));
    }
}

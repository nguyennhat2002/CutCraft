//! Pure geometry for the editor workspace and timeline split.
//!
//! The percentages from the visual reference are defaults, not hard limits.
//! Small windows preserve a usable center preview by compressing the side
//! panels proportionally.

use crate::config;

/// Widths of the three upper editor columns, excluding splitter handles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkspaceWidths {
    pub left: f32,
    pub center: f32,
    pub right: f32,
}

impl WorkspaceWidths {
    pub fn total(self) -> f32 {
        self.left + self.center + self.right
    }
}

/// Compute the upper workspace widths.
///
/// `total_width` is the width available to the three columns after splitter
/// handles have been removed.
pub fn workspace_widths(
    total_width: f32,
    show_left: bool,
    show_right: bool,
    left_fraction: f32,
    right_fraction: f32,
) -> WorkspaceWidths {
    let total = if total_width.is_finite() && total_width > 0.0 {
        total_width
    } else {
        0.0
    };
    if total <= 0.0 {
        return WorkspaceWidths {
            left: 0.0,
            center: 0.0,
            right: 0.0,
        };
    }

    let left_fraction = sanitized_fraction(
        left_fraction,
        config::WORKSPACE_LEFT_FRACTION,
        config::WORKSPACE_LEFT_MAX_FRACTION,
    );
    let right_fraction = sanitized_fraction(
        right_fraction,
        config::WORKSPACE_RIGHT_FRACTION,
        config::WORKSPACE_RIGHT_MAX_FRACTION,
    );

    let center_min = config::WORKSPACE_CENTER_MIN_WIDTH.min((total * 0.40).max(180.0));
    let left_min = config::WORKSPACE_LEFT_MIN_WIDTH.min(total * 0.30);
    let right_min = config::WORKSPACE_RIGHT_MIN_WIDTH.min(total * 0.30);

    match (show_left, show_right) {
        (true, true) => {
            let max_sides = (total - center_min).max(0.0);
            let mut left = (total * left_fraction).max(left_min);
            let mut right = (total * right_fraction).max(right_min);

            if left + right > max_sides && left + right > 0.0 {
                let factor = max_sides / (left + right);
                left = (left * factor).max(left_min);
                right = (right * factor).max(right_min);
            }

            if left + right > total {
                let factor = total / (left + right);
                left *= factor;
                right *= factor;
            }

            WorkspaceWidths {
                left,
                center: (total - left - right).max(0.0),
                right,
            }
        }
        (true, false) => {
            let left = (total * left_fraction).clamp(left_min, (total - center_min).max(left_min));
            WorkspaceWidths {
                left,
                center: (total - left).max(0.0),
                right: 0.0,
            }
        }
        (false, true) => {
            let right =
                (total * right_fraction).clamp(right_min, (total - center_min).max(right_min));
            WorkspaceWidths {
                left: 0.0,
                center: (total - right).max(0.0),
                right,
            }
        }
        (false, false) => WorkspaceWidths {
            left: 0.0,
            center: total,
            right: 0.0,
        },
    }
}

/// Timeline panel heights derived from the available editor height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineHeights {
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

pub fn timeline_heights(available_height: f32, content_height: f32) -> TimelineHeights {
    let available = positive_or(available_height, config::TIMELINE_DEFAULT_HEIGHT);
    let content = positive_or(content_height, available);
    let max =
        (content - config::WORKSPACE_SPLITTER_WIDTH - config::WORKSPACE_MIN_TOP_HEIGHT).max(0.0);
    let min = config::TIMELINE_MIN_HEIGHT.min(max);
    let default = (available * config::TIMELINE_DEFAULT_FRACTION)
        .min(config::TIMELINE_DEFAULT_HEIGHT)
        .clamp(min, max);

    TimelineHeights { default, min, max }
}

/// Resolve a user's preferred timeline height, or the responsive default when
/// no preference has been saved. The result always leaves the upper workspace
/// at least `WORKSPACE_MIN_TOP_HEIGHT` points when space permits.
pub fn timeline_height(
    available_height: f32,
    content_height: f32,
    preferred_fraction: Option<f32>,
) -> f32 {
    let heights = timeline_heights(available_height, content_height);
    let Some(fraction) = preferred_fraction.filter(|value| value.is_finite()) else {
        return heights.default;
    };
    let available = positive_or(available_height, config::TIMELINE_DEFAULT_HEIGHT);
    (available * fraction.clamp(0.0, 1.0)).clamp(heights.min, heights.max)
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn sanitized_fraction(value: f32, fallback: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.15, max)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.5,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn reference_viewport_uses_33_29_38_columns() {
        let widths = workspace_widths(
            1600.0,
            true,
            true,
            config::WORKSPACE_LEFT_FRACTION,
            config::WORKSPACE_RIGHT_FRACTION,
        );

        assert_close(widths.left, 536.0);
        assert_close(widths.right, 601.6);
        assert_close(widths.center, 462.4);
        assert_close(widths.total(), 1600.0);
    }

    #[test]
    fn smaller_window_preserves_a_usable_center_column() {
        let widths = workspace_widths(960.0, true, true, 0.335, 0.376);

        assert!(widths.center >= config::WORKSPACE_CENTER_MIN_WIDTH - 1.0);
        assert!(widths.left >= config::WORKSPACE_LEFT_MIN_WIDTH);
        assert!(widths.right >= config::WORKSPACE_RIGHT_MIN_WIDTH);
        assert_close(widths.total(), 960.0);
    }

    #[test]
    fn hidden_panels_expand_the_center() {
        let both_hidden = workspace_widths(1600.0, false, false, 0.335, 0.376);
        assert_eq!(both_hidden.center, 1600.0);
        assert_eq!(both_hidden.left + both_hidden.right, 0.0);

        let only_left = workspace_widths(1600.0, true, false, 0.335, 0.376);
        assert_close(only_left.left, 536.0);
        assert_close(only_left.center, 1064.0);
        assert_eq!(only_left.right, 0.0);

        let only_right = workspace_widths(1600.0, false, true, 0.335, 0.376);
        assert_close(only_right.right, 601.6);
        assert_close(only_right.center, 998.4);
        assert_eq!(only_right.left, 0.0);
    }

    #[test]
    fn invalid_inputs_do_not_produce_invalid_geometry() {
        let widths = workspace_widths(f32::NAN, true, true, f32::NAN, f32::INFINITY);
        assert_eq!(widths.total(), 0.0);

        let widths = workspace_widths(1200.0, true, true, -5.0, 99.0);
        assert!(widths.center >= config::WORKSPACE_CENTER_MIN_WIDTH - 1.0);
        assert!(widths.left.is_finite());
        assert!(widths.right.is_finite());
    }

    #[test]
    fn timeline_height_defaults_and_dynamic_max_leave_room_for_workspace() {
        let heights = timeline_heights(828.0, 816.0);

        assert_close(heights.default, 397.44);
        assert_close(heights.min, config::TIMELINE_MIN_HEIGHT);
        assert_close(
            heights.max,
            816.0 - config::WORKSPACE_SPLITTER_WIDTH - config::WORKSPACE_MIN_TOP_HEIGHT,
        );
        assert_close(
            816.0 - heights.max - config::WORKSPACE_SPLITTER_WIDTH,
            config::WORKSPACE_MIN_TOP_HEIGHT,
        );
        assert!(heights.max > 460.0, "52% cap must not limit the timeline");
    }

    #[test]
    fn timeline_height_prefers_user_fraction_and_clamps_to_geometry() {
        let default_height = timeline_height(828.0, 816.0, None);
        assert_close(default_height, 397.44);

        assert_close(timeline_height(828.0, 816.0, Some(0.60)), 828.0 * 0.60);
        assert_close(
            timeline_height(828.0, 816.0, Some(0.01)),
            config::TIMELINE_MIN_HEIGHT,
        );
        assert_close(timeline_height(828.0, 816.0, Some(0.99)), 630.0);
        assert_close(
            timeline_height(828.0, 816.0, Some(f32::NAN)),
            default_height,
        );
        let invalid = timeline_height(f32::NAN, f32::NAN, Some(0.60));
        assert!(invalid.is_finite() && invalid > 0.0);
        assert_close(invalid, timeline_heights(f32::NAN, f32::NAN).default);
    }

    #[test]
    fn compact_window_keeps_the_upper_workspace_usable() {
        let heights = timeline_heights(536.0, 524.0);

        assert_close(heights.default, config::TIMELINE_MIN_HEIGHT);
        assert_close(heights.max, 338.0);
        assert_close(timeline_height(536.0, 524.0, Some(0.99)), heights.max);
        assert_close(timeline_height(536.0, 524.0, Some(0.60)), 536.0 * 0.60);
        assert!(
            524.0 - heights.max - config::WORKSPACE_SPLITTER_WIDTH
                >= config::WORKSPACE_MIN_TOP_HEIGHT - 0.5
        );
    }

    #[test]
    fn too_little_content_collapses_the_timeline_instead_of_overflowing() {
        let heights = timeline_heights(100.0, 100.0);

        assert_eq!(heights.min, 0.0);
        assert_eq!(heights.max, 0.0);
        assert_eq!(timeline_height(100.0, 100.0, Some(0.5)), 0.0);
    }
}

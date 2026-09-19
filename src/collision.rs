//! Overlap tests between simple shapes. Plain Rust, no ECS.
//!
//! Everything compares *squared* distances. That gives the same answer as
//! comparing distances, but skips the square root, which is the only costly
//! step in these checks.

use bevy::math::Vec2;

/// Whether two circles touch or overlap.
#[must_use]
pub fn circles_overlap(a: Vec2, a_radius: f32, b: Vec2, b_radius: f32) -> bool {
    let reach = a_radius + b_radius;
    a.distance_squared(b) <= reach * reach
}

/// Whether a circle touches or overlaps an axis-aligned rectangle.
///
/// Finds the point of the rectangle closest to the circle's center (by
/// clamping the center into the rectangle), then checks that point's
/// distance. It also works when the center is inside the rectangle, because
/// then the closest point is the center itself.
#[must_use]
pub fn circle_overlaps_rect(center: Vec2, radius: f32, rect_center: Vec2, rect_size: Vec2) -> bool {
    let half = rect_size / 2.0;
    let closest = center.clamp(rect_center - half, rect_center + half);
    center.distance_squared(closest) <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circles_touching_edge_to_edge_overlap() {
        assert!(circles_overlap(
            Vec2::ZERO,
            10.0,
            Vec2::new(30.0, 0.0),
            20.0
        ));
        assert!(!circles_overlap(
            Vec2::ZERO,
            10.0,
            Vec2::new(30.1, 0.0),
            20.0
        ));
    }

    #[test]
    fn circle_near_rectangle_side() {
        let rect = (Vec2::ZERO, Vec2::new(100.0, 20.0));
        // Directly above the top edge (y = 10), just touching.
        assert!(circle_overlaps_rect(
            Vec2::new(0.0, 15.0),
            5.0,
            rect.0,
            rect.1
        ));
        assert!(!circle_overlaps_rect(
            Vec2::new(0.0, 15.1),
            5.0,
            rect.0,
            rect.1
        ));
    }

    #[test]
    fn circle_near_rectangle_corner_uses_true_distance() {
        let rect = (Vec2::ZERO, Vec2::new(100.0, 20.0));
        // Diagonal from the corner (50, 10): 4 right and 4 up is ~5.66 away.
        assert!(!circle_overlaps_rect(
            Vec2::new(54.0, 14.0),
            5.0,
            rect.0,
            rect.1
        ));
        assert!(circle_overlaps_rect(
            Vec2::new(53.0, 13.0),
            5.0,
            rect.0,
            rect.1
        ));
    }

    #[test]
    fn circle_inside_rectangle_overlaps() {
        assert!(circle_overlaps_rect(
            Vec2::new(10.0, 0.0),
            1.0,
            Vec2::ZERO,
            Vec2::new(100.0, 20.0)
        ));
    }
}

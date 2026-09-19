//! Overlap tests between simple shapes. Plain Rust, no ECS.
//!
//! Everything compares *squared* distances. That gives the same answer as
//! comparing distances, but skips the square root, which is the only costly
//! step in these checks.

use bevy::math::Vec2;

/// A collision shape the astronaut (a circle) can touch.
///
/// A trait: each shape type (circle now, the wind ribbon's path later)
/// implements it, and code that only needs "does the astronaut touch it?"
/// works with any of them.
pub trait Shape {
    /// Whether a circle at `center` with `radius` touches or overlaps the shape.
    fn overlaps_circle(&self, center: Vec2, radius: f32) -> bool;
}

/// A circle shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleShape {
    /// Center, in world coordinates.
    pub center: Vec2,
    /// Radius, in world units.
    pub radius: f32,
}

impl CircleShape {
    /// A circle at `center` with `radius`.
    #[must_use]
    pub const fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }
}

impl Shape for CircleShape {
    fn overlaps_circle(&self, center: Vec2, radius: f32) -> bool {
        circles_overlap(self.center, self.radius, center, radius)
    }
}

/// A thick path: a chain of segments with rounded ends (capsules), like a
/// wind ribbon.
///
/// It *borrows* its points (`&'a [Vec2]`) instead of copying them: the
/// lifetime `'a` tells the compiler this shape can't outlive the level data
/// it points into, so checking a collision never allocates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathShape<'a> {
    /// The centerline, in order (at least two points).
    pub points: &'a [Vec2],
    /// Distance from the centerline to the edge.
    pub half_width: f32,
}

impl PathShape<'_> {
    /// The segment closest to `point`: its index (from `points[i]` to
    /// `points[i + 1]`) and the squared distance. `None` for fewer than two points.
    #[must_use]
    pub fn closest_segment(&self, point: Vec2) -> Option<(usize, f32)> {
        self.points
            .windows(2)
            .map(|pair| distance_squared_to_segment(point, pair[0], pair[1]))
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Direction of travel along segment `index` (unit length, or zero for a
    /// degenerate segment).
    #[must_use]
    pub fn direction(&self, index: usize) -> Vec2 {
        match (self.points.get(index), self.points.get(index + 1)) {
            (Some(&a), Some(&b)) => (b - a).normalize_or_zero(),
            _ => Vec2::ZERO,
        }
    }
}

impl Shape for PathShape<'_> {
    fn overlaps_circle(&self, center: Vec2, radius: f32) -> bool {
        let reach = self.half_width + radius;
        self.closest_segment(center)
            .is_some_and(|(_, distance_squared)| distance_squared <= reach * reach)
    }
}

/// Squared distance from `point` to the segment from `a` to `b`.
#[must_use]
pub fn distance_squared_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    // How far along the segment the closest point is (0 = at a, 1 = at b).
    let t = if length_squared > 0.0 {
        ((point - a).dot(along) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    point.distance_squared(a + along * t)
}

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
    fn distance_to_a_segment_uses_the_nearest_point() {
        let (a, b) = (Vec2::ZERO, Vec2::new(10.0, 0.0));
        assert!((distance_squared_to_segment(Vec2::new(5.0, 3.0), a, b) - 9.0).abs() < 1e-6);
        // Beyond the end: distance to the end point.
        assert!((distance_squared_to_segment(Vec2::new(13.0, 4.0), a, b) - 25.0).abs() < 1e-6);
        // A zero-length segment is just a point.
        assert!((distance_squared_to_segment(Vec2::new(3.0, 4.0), a, a) - 25.0).abs() < 1e-6);
    }

    #[test]
    fn path_shape_touches_along_its_whole_length() {
        let points = [Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::new(100.0, -100.0)];
        let path = PathShape {
            points: &points,
            half_width: 20.0,
        };
        assert!(path.overlaps_circle(Vec2::new(50.0, 25.0), 10.0));
        assert!(path.overlaps_circle(Vec2::new(125.0, -50.0), 10.0));
        assert!(!path.overlaps_circle(Vec2::new(50.0, 31.0), 10.0));
        assert_eq!(
            path.closest_segment(Vec2::new(110.0, -80.0))
                .map(|(i, _)| i),
            Some(1)
        );
        assert_eq!(path.direction(1), Vec2::new(0.0, -1.0));
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

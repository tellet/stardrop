//! What touching an object does. Pure functions of the object type's tuning
//! and the astronaut's state, so every object of a type behaves identically
//! (the consistency rule) and each effect is unit-tested.

use bevy::math::Vec2;

use super::kind::{ObjectKind, ObjectsConfig};
use crate::physics::movement::Body;

/// The result of touching an object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// The astronaut takes on this velocity (and coasts; see `Body::launched`).
    Launch(Vec2),
    /// The attempt ends (hazard).
    FailAttempt,
}

/// What entering contact with an object of `kind` centered at `center` does
/// to `body`.
///
/// One exhaustive `match`: adding an object type makes the compiler point
/// here until its behavior is written.
#[must_use]
pub fn on_enter(kind: ObjectKind, center: Vec2, config: &ObjectsConfig, body: Body) -> Outcome {
    match kind {
        ObjectKind::CloudPuff => Outcome::Launch(cloud_bounce(center, config, body)),
        ObjectKind::Crystal => Outcome::Launch(crystal_reverse(center, config, body)),
        ObjectKind::StaticCloud => Outcome::FailAttempt,
    }
}

/// Upward and away from the cloud's center, at a fixed speed, tilted at most
/// `max_tilt` from straight up.
fn cloud_bounce(center: Vec2, config: &ObjectsConfig, body: Body) -> Vec2 {
    let cloud = config.cloud_puff;
    // Straight up if the astronaut is exactly at the center.
    let away = (body.position - center).normalize_or(Vec2::Y);
    let tilt = away.x.atan2(away.y).clamp(-cloud.max_tilt, cloud.max_tilt);
    Vec2::new(tilt.sin(), tilt.cos()) * cloud.bounce_speed
}

/// Sideways motion reversed (at least `min_sideways_speed`); the fall is kept.
fn crystal_reverse(center: Vec2, config: &ObjectsConfig, body: Body) -> Vec2 {
    let minimum = config.crystal.min_sideways_speed;
    let reversed = -body.velocity.x;
    // Falling straight onto a crystal: send the astronaut away from its
    // center (right, if exactly on center), so there's always a clear effect.
    let direction = if reversed == 0.0 {
        if body.position.x >= center.x {
            1.0
        } else {
            -1.0
        }
    } else {
        reversed.signum()
    };
    Vec2::new(direction * reversed.abs().max(minimum), body.velocity.y)
}

#[cfg(test)]
mod tests {
    use core::f32::consts::FRAC_PI_4;

    use super::*;
    use crate::objects::kind::{CloudPuffConfig, CrystalConfig, StaticCloudConfig};

    const CONFIG: ObjectsConfig = ObjectsConfig {
        cloud_puff: CloudPuffConfig {
            bounce_speed: 650.0,
            max_tilt: FRAC_PI_4,
        },
        crystal: CrystalConfig {
            min_sideways_speed: 350.0,
        },
        static_cloud: StaticCloudConfig { hitbox_scale: 0.8 },
    };

    fn body(position: Vec2, velocity: Vec2) -> Body {
        Body {
            position,
            velocity,
            ..Body::default()
        }
    }

    fn launched(kind: ObjectKind, center: Vec2, body: Body) -> Vec2 {
        match on_enter(kind, center, &CONFIG, body) {
            Outcome::Launch(velocity) => velocity,
            Outcome::FailAttempt => panic!("{kind:?} should launch"),
        }
    }

    #[test]
    fn static_cloud_always_ends_the_attempt() {
        for velocity in [
            Vec2::ZERO,
            Vec2::new(500.0, -450.0),
            Vec2::new(-300.0, 200.0),
        ] {
            let outcome = on_enter(
                ObjectKind::StaticCloud,
                Vec2::ZERO,
                &CONFIG,
                body(Vec2::Y, velocity),
            );
            assert_eq!(outcome, Outcome::FailAttempt);
        }
    }

    #[test]
    fn cloud_always_bounces_upward_and_away_at_its_speed() {
        let center = Vec2::new(100.0, -500.0);
        // Touching from above, the sides, and even from below.
        for offset in [
            Vec2::new(0.0, 60.0),
            Vec2::new(40.0, 50.0),
            Vec2::new(-90.0, 5.0),
            Vec2::new(70.0, -40.0),
        ] {
            let velocity = launched(
                ObjectKind::CloudPuff,
                center,
                body(center + offset, Vec2::new(0.0, -450.0)),
            );
            assert!(
                (velocity.length() - 650.0).abs() < 1e-3,
                "speed for {offset}"
            );
            assert!(
                velocity.y >= 650.0 * FRAC_PI_4.cos() - 1e-3,
                "upward for {offset}"
            );
            assert!(velocity.x * offset.x >= 0.0, "away for {offset}");
        }
    }

    #[test]
    fn cloud_ignores_incoming_speed() {
        // Consistency: the bounce depends only on where you touch, not how fast.
        let center = Vec2::ZERO;
        let slow = launched(
            ObjectKind::CloudPuff,
            center,
            body(Vec2::new(20.0, 60.0), Vec2::new(0.0, -50.0)),
        );
        let fast = launched(
            ObjectKind::CloudPuff,
            center,
            body(Vec2::new(20.0, 60.0), Vec2::new(400.0, -900.0)),
        );
        assert_eq!(slow, fast);
    }

    #[test]
    fn crystal_reverses_sideways_motion_and_keeps_the_fall() {
        let velocity = launched(
            ObjectKind::Crystal,
            Vec2::ZERO,
            body(Vec2::X, Vec2::new(480.0, -300.0)),
        );
        assert_eq!(velocity, Vec2::new(-480.0, -300.0));
    }

    #[test]
    fn crystal_enforces_its_minimum_sideways_speed() {
        let slow = launched(
            ObjectKind::Crystal,
            Vec2::ZERO,
            body(Vec2::X, Vec2::new(-40.0, -450.0)),
        );
        assert_eq!(slow, Vec2::new(350.0, -450.0));
        // Falling straight down: pushed away from the crystal's center.
        let left_of_center = launched(
            ObjectKind::Crystal,
            Vec2::ZERO,
            body(Vec2::new(-5.0, 30.0), Vec2::new(0.0, -450.0)),
        );
        assert_eq!(left_of_center, Vec2::new(-350.0, -450.0));
    }

    #[test]
    fn every_crystal_behaves_the_same() {
        // Same approach relative to two crystals far apart: same result.
        let incoming = Vec2::new(300.0, -400.0);
        let a = launched(
            ObjectKind::Crystal,
            Vec2::new(-300.0, -400.0),
            body(Vec2::new(-320.0, -380.0), incoming),
        );
        let b = launched(
            ObjectKind::Crystal,
            Vec2::new(250.0, -1500.0),
            body(Vec2::new(230.0, -1480.0), incoming),
        );
        assert_eq!(a, b);
    }
}

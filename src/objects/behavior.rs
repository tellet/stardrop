//! What touching an object does. Pure functions of the object type's tuning
//! and the astronaut's state, so every object of a type behaves identically
//! (the consistency rule) and each effect is unit-tested.

use bevy::math::Vec2;

use super::kind::{ObjectKind, ObjectShape, ObjectsConfig, Placement};
use crate::physics::movement::{Body, Capture, MovementConfig};

/// The result of touching an object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Nothing happens on entering (the object acts continuously instead;
    /// see [`while_inside`]).
    Nothing,
    /// The astronaut's new state.
    Changed(Body),
    /// The attempt ends (hazard).
    FailAttempt,
}

/// What entering contact with an object of `kind` centered at `center` does
/// to `body`.
///
/// One exhaustive `match`: adding an object type makes the compiler point
/// here until its behavior is written.
#[must_use]
pub fn on_enter(
    kind: ObjectKind,
    center: Vec2,
    config: &ObjectsConfig,
    movement: &MovementConfig,
    body: Body,
) -> Outcome {
    match kind {
        ObjectKind::CloudPuff => {
            Outcome::Changed(body.launched(cloud_bounce(center, config, body), movement))
        }
        // These act continuously instead (see `while_inside`).
        ObjectKind::WindRibbon | ObjectKind::GravityOrb => Outcome::Nothing,
        ObjectKind::Crystal => {
            Outcome::Changed(body.launched(crystal_reverse(center, config, body), movement))
        }
        ObjectKind::Bubble => Outcome::Changed(body.captured(Capture {
            seconds_left: config.bubble.seconds,
            rise_speed: config.bubble.rise_speed,
            nudge_speed: config.bubble.nudge_speed,
        })),
        ObjectKind::SporeFlower => {
            Outcome::Changed(body.with_swapped_controls(config.spore_flower.swap_seconds))
        }
        ObjectKind::CometStone => Outcome::Changed(body.diving(
            config.comet_stone.dive_speed,
            config.comet_stone.dive_seconds,
        )),
        ObjectKind::StaticCloud => Outcome::FailAttempt,
    }
}

/// The astronaut's new velocity after one fixed step of `dt` seconds inside
/// `placement`, for objects that act continuously; `None` for the rest.
///
/// A wind ribbon pulls the velocity toward its flow (the direction of the
/// nearest segment, at `flow_speed`) at `pull` units/s². Leaving the ribbon
/// simply stops the pull, so the astronaut keeps its speed: "released at its end".
#[must_use]
pub fn while_inside(
    placement: &Placement,
    config: &ObjectsConfig,
    body: Body,
    dt: f32,
) -> Option<Vec2> {
    match placement {
        Placement::WindRibbon { .. } => {
            let ObjectShape::Path(path) = placement.outline() else {
                return None;
            };
            let (segment, _) = path.closest_segment(body.position)?;
            let wind = config.wind_ribbon;
            let target = path.direction(segment) * wind.flow_speed;
            Some(body.velocity.move_towards(target, wind.pull * dt))
        }
        Placement::GravityOrb { center, radius } => {
            Some(orb_pull(*center, *radius, config, body, dt))
        }
        _ => None,
    }
}

/// One step of an orb's pull: accelerate toward the center (strength divided
/// by distance, capped), with a speed limit so nothing gets extreme. Touching
/// the core gives a soft bounce outward instead.
fn orb_pull(center: Vec2, radius: f32, config: &ObjectsConfig, body: Body, dt: f32) -> Vec2 {
    let orb = config.gravity_orb;
    let to_center = center - body.position;
    let distance = to_center.length();
    // Exactly at the center (never in practice): nothing to pull toward.
    let Some(direction) = to_center.try_normalize() else {
        return body.velocity;
    };
    if distance <= radius {
        return -direction * orb.core_bounce_speed;
    }
    let pull = (orb.pull / distance).min(orb.max_pull);
    (body.velocity + direction * pull * dt).clamp_length_max(orb.max_speed)
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
    use crate::objects::kind::{
        BubbleConfig, CloudPuffConfig, CometStoneConfig, CrystalConfig, GravityOrbConfig,
        SporeFlowerConfig, StaticCloudConfig, WindRibbonConfig,
    };

    const CONFIG: ObjectsConfig = ObjectsConfig {
        cloud_puff: CloudPuffConfig {
            bounce_speed: 650.0,
            max_tilt: FRAC_PI_4,
        },
        wind_ribbon: WindRibbonConfig {
            flow_speed: 500.0,
            pull: 1800.0,
        },
        crystal: CrystalConfig {
            min_sideways_speed: 350.0,
        },
        bubble: BubbleConfig {
            rise_speed: 90.0,
            seconds: 2.5,
            nudge_speed: 120.0,
        },
        gravity_orb: GravityOrbConfig {
            field_scale: 4.0,
            pull: 160_000.0,
            max_pull: 2000.0,
            max_speed: 700.0,
            core_bounce_speed: 420.0,
        },
        spore_flower: SporeFlowerConfig { swap_seconds: 4.0 },
        comet_stone: CometStoneConfig {
            dive_speed: 1100.0,
            dive_seconds: 0.6,
        },
        static_cloud: StaticCloudConfig { hitbox_scale: 0.8 },
    };

    /// Movement tuning for tests (only `control_recovery_time` matters here).
    const MOVEMENT: MovementConfig = MovementConfig {
        gravity: 450.0,
        terminal_velocity: 450.0,
        max_air_speed: 500.0,
        air_acceleration: 2000.0,
        air_braking: 1250.0,
        wall_restitution: 0.6,
        control_recovery_time: 0.5,
        coast_drag: 100.0,
    };

    fn body(position: Vec2, velocity: Vec2) -> Body {
        Body {
            position,
            velocity,
            ..Body::default()
        }
    }

    /// The astronaut's new state after touching `kind`.
    fn touched(kind: ObjectKind, center: Vec2, body: Body) -> Body {
        match on_enter(kind, center, &CONFIG, &MOVEMENT, body) {
            Outcome::Changed(changed) => changed,
            other => panic!("{kind:?} should change the astronaut, got {other:?}"),
        }
    }

    fn launched(kind: ObjectKind, center: Vec2, body: Body) -> Vec2 {
        touched(kind, center, body).velocity
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
                &MOVEMENT,
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

    fn ribbon(points: &[Vec2]) -> Placement {
        Placement::WindRibbon {
            path: points.to_vec(),
            width: 80.0,
        }
    }

    /// Steps inside the ribbon until the velocity stops changing.
    fn settle(placement: &Placement, mut body: Body) -> Vec2 {
        for _ in 0..240 {
            if let Some(velocity) = while_inside(placement, &CONFIG, body, 1.0 / 120.0) {
                body.velocity = velocity;
            }
        }
        body.velocity
    }

    #[test]
    fn wind_carries_the_astronaut_along_its_flow() {
        let rightward = ribbon(&[Vec2::new(-400.0, -500.0), Vec2::new(400.0, -500.0)]);
        let falling = body(Vec2::new(0.0, -500.0), Vec2::new(0.0, -450.0));
        let settled = settle(&rightward, falling);
        assert!(
            (settled - Vec2::new(500.0, 0.0)).length() < 1e-2,
            "{settled}"
        );
    }

    #[test]
    fn wind_can_carry_the_astronaut_upward() {
        let upward = ribbon(&[Vec2::new(0.0, -900.0), Vec2::new(0.0, -300.0)]);
        let settled = settle(
            &upward,
            body(Vec2::new(0.0, -700.0), Vec2::new(0.0, -450.0)),
        );
        assert!(settled.y > 450.0, "{settled}");
    }

    #[test]
    fn wind_follows_the_nearest_segment() {
        let bent = ribbon(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(300.0, 0.0),
            Vec2::new(300.0, -300.0),
        ]);
        // Near the second (downward) segment.
        let settled = settle(&bent, body(Vec2::new(310.0, -200.0), Vec2::ZERO));
        assert!(
            (settled - Vec2::new(0.0, -500.0)).length() < 1e-2,
            "{settled}"
        );
    }

    #[test]
    fn only_wind_acts_continuously() {
        let cloud = Placement::CloudPuff {
            center: Vec2::ZERO,
            radius: 80.0,
        };
        assert_eq!(
            while_inside(&cloud, &CONFIG, body(Vec2::ZERO, Vec2::ZERO), 0.01),
            None
        );
        let entering = on_enter(
            ObjectKind::WindRibbon,
            Vec2::ZERO,
            &CONFIG,
            &MOVEMENT,
            body(Vec2::ZERO, Vec2::ZERO),
        );
        assert_eq!(entering, Outcome::Nothing);
    }

    #[test]
    fn spore_flower_swaps_controls_without_moving_the_astronaut() {
        let before = body(Vec2::new(10.0, -200.0), Vec2::new(120.0, -300.0));
        let after = touched(ObjectKind::SporeFlower, Vec2::ZERO, before);
        assert_eq!(after.velocity, before.velocity, "spores don't push");
        assert!(after.effects.controls_swapped());
        assert!((after.effects.swapped_controls - 4.0).abs() < 1e-6);
    }

    #[test]
    fn touching_a_flower_again_resets_the_timer_instead_of_stacking() {
        let fresh = touched(
            ObjectKind::SporeFlower,
            Vec2::ZERO,
            body(Vec2::ZERO, Vec2::ZERO),
        );
        let nearly_over = Body {
            effects: fresh.effects.after(3.5),
            ..fresh
        };
        let again = touched(ObjectKind::SporeFlower, Vec2::ZERO, nearly_over);
        assert!((again.effects.swapped_controls - 4.0).abs() < 1e-6);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "the dive must pass sideways speed through untouched"
    )]
    fn comet_stone_throws_the_astronaut_into_a_dive() {
        let before = body(Vec2::new(0.0, -300.0), Vec2::new(200.0, -450.0));
        let after = touched(ObjectKind::CometStone, Vec2::ZERO, before);
        assert!(
            (after.velocity.y + 1100.0).abs() < 1e-3,
            "beyond terminal velocity"
        );
        assert_eq!(
            after.velocity.x, before.velocity.x,
            "sideways motion is kept"
        );
        assert!(after.effects.diving());
        assert!((after.effects.comet_dive - 0.6).abs() < 1e-6);
    }

    #[test]
    fn the_latest_push_wins() {
        // A cloud bounce during a dive replaces the velocity: one push at a time.
        let diving = touched(
            ObjectKind::CometStone,
            Vec2::ZERO,
            body(Vec2::new(0.0, -300.0), Vec2::ZERO),
        );
        let bounced = touched(ObjectKind::CloudPuff, Vec2::new(0.0, -400.0), diving);
        assert!(bounced.velocity.y > 0.0, "now going up");
    }

    fn orb(center: Vec2, radius: f32) -> Placement {
        Placement::GravityOrb { center, radius }
    }

    #[test]
    fn an_orb_pulls_the_astronaut_toward_its_center() {
        let center = Vec2::new(0.0, -600.0);
        let above = body(Vec2::new(0.0, -300.0), Vec2::ZERO);
        let pulled = while_inside(&orb(center, 60.0), &CONFIG, above, 1.0 / 120.0).unwrap();
        assert!(pulled.y < 0.0, "pulled downward toward the orb: {pulled}");
        let beside = body(Vec2::new(-300.0, -600.0), Vec2::ZERO);
        let sideways = while_inside(&orb(center, 60.0), &CONFIG, beside, 1.0 / 120.0).unwrap();
        assert!(
            sideways.x > 0.0,
            "pulled rightward toward the orb: {sideways}"
        );
    }

    #[test]
    fn the_pull_is_stronger_closer_in() {
        let center = Vec2::ZERO;
        let near = while_inside(
            &orb(center, 50.0),
            &CONFIG,
            body(Vec2::new(0.0, 100.0), Vec2::ZERO),
            1.0 / 120.0,
        );
        let far = while_inside(
            &orb(center, 50.0),
            &CONFIG,
            body(Vec2::new(0.0, 400.0), Vec2::ZERO),
            1.0 / 120.0,
        );
        assert!(near.unwrap().length() > far.unwrap().length());
    }

    #[test]
    fn an_orb_never_makes_the_astronaut_extremely_fast() {
        let center = Vec2::ZERO;
        let mut moving = body(Vec2::new(0.0, 120.0), Vec2::new(0.0, -600.0));
        // Many steps of falling straight at the orb, the worst case.
        for _ in 0..600 {
            if let Some(velocity) = while_inside(&orb(center, 50.0), &CONFIG, moving, 1.0 / 120.0) {
                moving.velocity = velocity;
            }
            moving.position += moving.velocity / 120.0;
        }
        assert!(
            moving.velocity.length() <= CONFIG.gravity_orb.max_speed + 1e-3,
            "{moving:?}"
        );
    }

    #[test]
    fn touching_the_core_bounces_softly_outward() {
        let center = Vec2::ZERO;
        let inside_core = body(Vec2::new(0.0, 30.0), Vec2::new(0.0, -500.0));
        let bounced = while_inside(&orb(center, 50.0), &CONFIG, inside_core, 1.0 / 120.0).unwrap();
        assert!(bounced.y > 0.0, "pushed back out: {bounced}");
        assert!((bounced.length() - CONFIG.gravity_orb.core_bounce_speed).abs() < 1e-3);
    }

    #[test]
    fn every_orb_pulls_the_same_way() {
        let offset = Vec2::new(90.0, 140.0);
        let a = while_inside(
            &orb(Vec2::new(-300.0, -400.0), 60.0),
            &CONFIG,
            body(Vec2::new(-300.0, -400.0) + offset, Vec2::ZERO),
            1.0 / 120.0,
        );
        let b = while_inside(
            &orb(Vec2::new(250.0, -1500.0), 60.0),
            &CONFIG,
            body(Vec2::new(250.0, -1500.0) + offset, Vec2::ZERO),
            1.0 / 120.0,
        );
        assert_eq!(a, b);
    }
}

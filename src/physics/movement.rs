//! How the astronaut moves: gravity, air control and the soft side walls.
//!
//! Pure functions of the current state, the player's input and the tuning
//! values. No ECS here, only `bevy::math` (a plain vector-math library), so
//! everything can be unit-tested and benchmarked without an `App`.
//!
//! Coordinates: world units, y up. Falling means negative y velocity.

use bevy::math::Vec2;

/// Tuning for falling and drifting. One instance lives in `GameConfig`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementConfig {
    /// Downward acceleration, in units/s².
    pub gravity: f32,
    /// Fall speed that gravity alone never exceeds, in units/s.
    pub terminal_velocity: f32,
    /// Fastest sideways drift the player can reach on their own, in units/s.
    pub max_air_speed: f32,
    /// How quickly sideways input builds up speed, in units/s².
    pub air_acceleration: f32,
    /// How quickly sideways speed fades with no input, in units/s².
    pub air_braking: f32,
    /// Fraction of sideways speed kept (and reversed) when bouncing off a side wall.
    pub wall_restitution: f32,
    /// After a push (a wall bounce, or an object), steering and braking fade
    /// back in over this many seconds, so the push stays visible even while
    /// the player keeps steering against it.
    pub control_recovery_time: f32,
    /// Gentle slow-down while coasting (pushed by a wall or, later, an
    /// object) with no input, in units/s². Much weaker than `air_braking`:
    /// we're in space.
    pub coast_drag: f32,
}

/// The player's sideways steering, from -1 (full left) to 1 (full right).
///
/// A newtype rather than a bare `f32`, so a raw stick value or a speed can't
/// be passed by mistake, and the range is guaranteed by construction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AirInput(f32);

impl AirInput {
    /// No steering.
    pub const NONE: Self = Self(0.0);
    /// Full left.
    pub const LEFT: Self = Self(-1.0);
    /// Full right.
    pub const RIGHT: Self = Self(1.0);

    /// Clamps any value into -1..=1. `NaN` (e.g. from a broken gamepad driver) becomes no input.
    #[must_use]
    pub fn new(value: f32) -> Self {
        if value.is_nan() {
            Self::NONE
        } else {
            Self(value.clamp(-1.0, 1.0))
        }
    }

    /// Combines digital keys with an analog stick. Keys win when pressed, so a
    /// resting stick can never fight the keyboard.
    #[must_use]
    pub fn from_controls(left: bool, right: bool, stick_x: f32) -> Self {
        match (left, right) {
            (true, false) => Self::LEFT,
            (false, true) => Self::RIGHT,
            // Both keys cancel out, like most platformers.
            (true, true) => Self::NONE,
            (false, false) => Self::new(stick_x),
        }
    }

    /// The steering value, in -1..=1.
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

/// Where the astronaut's *center* may go sideways (level edges minus its radius).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walls {
    /// Leftmost allowed x.
    pub left: f32,
    /// Rightmost allowed x.
    pub right: f32,
}

/// Why the astronaut is moving sideways, which decides how that motion fades
/// when the player isn't steering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Drift {
    /// The player's own steering: letting go brakes quickly, for precise control.
    #[default]
    Steered,
    /// Pushed by something else: keeps drifting like in space until the player steers.
    Coasting,
}

/// The astronaut's physical state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Body {
    /// Center position, in world units.
    pub position: Vec2,
    /// Velocity, in units/s.
    pub velocity: Vec2,
    /// Seconds left until air control is fully back after a push (0 = full control).
    pub control_recovery: f32,
    /// Whether sideways motion is the player's own or a push from outside.
    pub drift: Drift,
}

impl Body {
    /// Motionless at `position`, with full control.
    #[must_use]
    pub fn at_rest(position: Vec2) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    /// Pushed by something outside the player's control (an object): takes
    /// on `velocity`, coasts (so braking doesn't swallow the push) and has
    /// weakened steering for a moment, exactly like a wall bounce.
    #[must_use]
    pub fn launched(self, velocity: Vec2, config: &MovementConfig) -> Self {
        Self {
            velocity,
            drift: Drift::Coasting,
            control_recovery: config.control_recovery_time,
            ..self
        }
    }
}

/// Advances the body by one fixed timestep of `dt` seconds.
///
/// Uses semi-implicit Euler: velocity first, then position with the new
/// velocity. It is as cheap as plain Euler but stays stable for orbits,
/// which matters once gravity orbs arrive.
#[must_use]
pub fn step(body: Body, input: AirInput, config: &MovementConfig, walls: Walls, dt: f32) -> Body {
    let mut velocity = body.velocity;

    // Vertical: ease toward terminal velocity at the rate of gravity. From
    // below terminal speed this is ordinary gravity; if an object ever pushes
    // us faster (later milestones), we ease back instead of snapping.
    velocity.y = move_towards(velocity.y, -config.terminal_velocity, config.gravity * dt);

    // Horizontal: accelerate toward the steered speed, or brake toward zero,
    // weakened for a moment after a wall bounce.
    let target = input.value() * config.max_air_speed;
    let control = control_after_bounce(body.control_recovery, config.control_recovery_time);
    // Any steering puts the player back in charge.
    let mut drift = if input == AirInput::NONE {
        body.drift
    } else {
        Drift::Steered
    };
    let rate = match (input == AirInput::NONE, drift) {
        (false, _) => config.air_acceleration * control,
        (true, Drift::Steered) => config.air_braking * control,
        // Coasting isn't the player's doing, so recovery doesn't weaken it.
        (true, Drift::Coasting) => config.coast_drag,
    };
    velocity.x = move_towards(velocity.x, target, rate * dt);
    let mut control_recovery = (body.control_recovery - dt).max(0.0);

    let mut position = body.position + velocity * dt;

    // Side walls: clamp inside and bounce back if moving into the wall.
    let hit_left = position.x < walls.left && velocity.x < 0.0;
    let hit_right = position.x > walls.right && velocity.x > 0.0;
    position.x = position.x.clamp(walls.left, walls.right);
    if hit_left || hit_right {
        velocity.x = -velocity.x * config.wall_restitution;
        control_recovery = config.control_recovery_time;
        drift = Drift::Coasting;
    }

    Body {
        position,
        velocity,
        control_recovery,
        drift,
    }
}

/// How much air control the player has (0..=1) with `remaining` seconds of
/// wall recovery left out of `total`. Eases in smoothly (smoothstep), with no
/// sudden jump back to full control.
fn control_after_bounce(remaining: f32, total: f32) -> f32 {
    if remaining <= 0.0 || total <= 0.0 {
        return 1.0;
    }
    let progress = (1.0 - remaining / total).clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

/// Moves `current` toward `target` by at most `max_delta`, never overshooting.
fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    let difference = target - current;
    if difference.abs() <= max_delta {
        target
    } else {
        current + max_delta.copysign(difference)
    }
}

/// Replays a recorded input sequence from `start`, one fixed step per input.
#[must_use]
pub fn replay(
    start: Body,
    inputs: &[AirInput],
    config: &MovementConfig,
    walls: Walls,
    dt: f32,
) -> Body {
    inputs
        .iter()
        .fold(start, |body, &input| step(body, input, config, walls, dt))
}

/// Blends between the previous and current physics positions for drawing.
/// `alpha` is how far we are into the next fixed step (0..1).
#[must_use]
pub fn interpolate(previous: Vec2, current: Vec2, alpha: f32) -> Vec2 {
    previous.lerp(current, alpha)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;
    const CONFIG: MovementConfig = MovementConfig {
        gravity: 450.0,
        terminal_velocity: 450.0,
        max_air_speed: 500.0,
        air_acceleration: 2000.0,
        air_braking: 1250.0,
        wall_restitution: 0.6,
        control_recovery_time: 0.5,
        coast_drag: 100.0,
    };
    const WALLS: Walls = Walls {
        left: -900.0,
        right: 900.0,
    };

    fn run(mut body: Body, input: AirInput, seconds: f32) -> Body {
        // `seconds` is small in tests, so the cast can't truncate meaningfully.
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (seconds / DT).round() as usize;
        for _ in 0..steps {
            body = step(body, input, &CONFIG, WALLS, DT);
        }
        body
    }

    #[test]
    fn falling_never_exceeds_terminal_velocity() {
        let mut body = Body::default();
        for _ in 0..10_000 {
            body = step(body, AirInput::NONE, &CONFIG, WALLS, DT);
            assert!(body.velocity.y >= -CONFIG.terminal_velocity);
        }
        assert!((body.velocity.y + CONFIG.terminal_velocity).abs() < 1e-3);
    }

    #[test]
    fn astronaut_can_never_rise_on_their_own() {
        let mut body = Body::default();
        for input in [AirInput::LEFT, AirInput::RIGHT, AirInput::NONE] {
            let before = body.position.y;
            body = run(body, input, 1.0);
            assert!(body.position.y < before);
            assert!(body.velocity.y <= 0.0);
        }
    }

    #[test]
    fn faster_than_terminal_eases_back_instead_of_snapping() {
        let fast = Body {
            velocity: Vec2::new(0.0, -2.0 * CONFIG.terminal_velocity),
            ..Body::default()
        };
        let after_one_step = step(fast, AirInput::NONE, &CONFIG, WALLS, DT);
        assert!(after_one_step.velocity.y < -CONFIG.terminal_velocity);
        let later = run(fast, AirInput::NONE, 5.0);
        assert!((later.velocity.y + CONFIG.terminal_velocity).abs() < 1e-3);
    }

    #[test]
    fn horizontal_speed_is_capped() {
        // One second: long enough to reach full speed, short enough not to reach a wall.
        let mut body = Body::default();
        for _ in 0..120 {
            body = step(body, AirInput::RIGHT, &CONFIG, WALLS, DT);
            assert!(body.velocity.x <= CONFIG.max_air_speed);
        }
        assert!((body.velocity.x - CONFIG.max_air_speed).abs() < 1e-3);
    }

    #[test]
    fn half_stick_gives_half_speed() {
        let body = run(Body::default(), AirInput::new(0.5), 1.0);
        assert!((body.velocity.x - CONFIG.max_air_speed * 0.5).abs() < 1e-3);
    }

    #[test]
    fn no_input_slows_down_gradually() {
        let moving = run(Body::default(), AirInput::RIGHT, 1.0);
        let one_step = step(moving, AirInput::NONE, &CONFIG, WALLS, DT);
        // Gradual: one step only removes a little speed...
        assert!(one_step.velocity.x > moving.velocity.x * 0.9);
        // ...but it does come to rest, without reversing.
        let rested = run(moving, AirInput::NONE, 1.0);
        assert!(rested.velocity.x.abs() < f32::EPSILON);
    }

    fn at_right_wall_moving_right() -> Body {
        Body {
            position: Vec2::new(WALLS.right - 1.0, 0.0),
            velocity: Vec2::new(CONFIG.max_air_speed, 0.0),
            ..Body::default()
        }
    }

    #[test]
    fn walls_reverse_direction_with_part_of_the_speed() {
        let before = at_right_wall_moving_right();
        // Holding right keeps the speed at exactly max, so only the bounce changes it.
        let bounced = step(before, AirInput::RIGHT, &CONFIG, WALLS, DT);
        assert!(bounced.position.x <= WALLS.right);
        let expected = -before.velocity.x * CONFIG.wall_restitution;
        assert!((bounced.velocity.x - expected).abs() < 1e-3);
        assert!((bounced.control_recovery - CONFIG.control_recovery_time).abs() < 1e-6);
    }

    /// Furthest distance from the right wall reached within `steps` after bouncing.
    fn rebound_distance(input: AirInput, steps: usize) -> f32 {
        let mut body = step(at_right_wall_moving_right(), input, &CONFIG, WALLS, DT);
        let mut furthest: f32 = 0.0;
        for _ in 0..steps {
            body = step(body, input, &CONFIG, WALLS, DT);
            furthest = furthest.max(WALLS.right - body.position.x);
        }
        furthest
    }

    #[test]
    fn rebound_is_visible_even_while_pushing_into_the_wall() {
        // More than an astronaut's width (56 units), so the player sees it.
        // 1 s and 2 s at 120 steps per second.
        assert!(rebound_distance(AirInput::RIGHT, 120) > 70.0);
        assert!(rebound_distance(AirInput::NONE, 240) > 90.0);
    }

    #[test]
    fn after_a_bounce_the_astronaut_keeps_drifting_like_in_space() {
        let mut body = step(
            at_right_wall_moving_right(),
            AirInput::NONE,
            &CONFIG,
            WALLS,
            DT,
        );
        assert_eq!(body.drift, Drift::Coasting);
        let bounce_x = body.position.x;
        for _ in 0..120 {
            body = step(body, AirInput::NONE, &CONFIG, WALLS, DT);
        }
        // One second later: still moving away, and well clear of the wall.
        assert!(body.velocity.x < -100.0);
        assert!(bounce_x - body.position.x > 200.0);
    }

    #[test]
    fn steering_takes_back_control_from_coasting() {
        let bounced = step(
            at_right_wall_moving_right(),
            AirInput::NONE,
            &CONFIG,
            WALLS,
            DT,
        );
        let steered = step(bounced, AirInput::LEFT, &CONFIG, WALLS, DT);
        assert_eq!(steered.drift, Drift::Steered);
        // After letting go, the usual quick braking applies again.
        let mut body = steered;
        for _ in 0..240 {
            body = step(body, AirInput::NONE, &CONFIG, WALLS, DT);
        }
        assert!(body.velocity.x.abs() < f32::EPSILON);
    }

    #[test]
    fn control_returns_smoothly_after_a_bounce() {
        let total = CONFIG.control_recovery_time;
        assert!(control_after_bounce(total, total).abs() < 1e-6);
        assert!((control_after_bounce(0.0, total) - 1.0).abs() < 1e-6);
        let mut previous = 0.0;
        for i in 1..=10_u16 {
            let remaining = total * (1.0 - f32::from(i) / 10.0);
            let control = control_after_bounce(remaining, total);
            assert!(control >= previous, "never drops while recovering");
            previous = control;
        }
    }

    #[test]
    fn walls_hold_while_pushing_into_them() {
        let body = run(Body::default(), AirInput::LEFT, 10.0);
        assert!(body.position.x >= WALLS.left);
    }

    #[test]
    fn keys_override_stick_and_cancel_each_other() {
        assert_eq!(AirInput::from_controls(true, false, 0.7), AirInput::LEFT);
        assert_eq!(AirInput::from_controls(false, true, -0.7), AirInput::RIGHT);
        assert_eq!(AirInput::from_controls(true, true, 0.7), AirInput::NONE);
        assert_eq!(
            AirInput::from_controls(false, false, 0.4),
            AirInput::new(0.4)
        );
    }

    #[test]
    fn input_is_clamped_and_nan_safe() {
        assert_eq!(AirInput::new(3.0), AirInput::RIGHT);
        assert_eq!(AirInput::new(-3.0), AirInput::LEFT);
        assert_eq!(AirInput::new(f32::NAN), AirInput::NONE);
    }

    /// Determinism: the same recorded inputs always produce exactly the same
    /// fall, bit for bit. Compared with `==` on purpose: "close enough" would
    /// hide nondeterminism.
    #[test]
    fn replaying_inputs_is_deterministic() {
        let inputs = recorded_inputs();
        let start = Body::default();
        let first = replay(start, &inputs, &CONFIG, WALLS, DT);
        for _ in 0..5 {
            let again = replay(start, &inputs, &CONFIG, WALLS, DT);
            assert_eq!(
                first.position.to_array().map(f32::to_bits),
                again.position.to_array().map(f32::to_bits)
            );
            assert_eq!(
                first.velocity.to_array().map(f32::to_bits),
                again.velocity.to_array().map(f32::to_bits)
            );
        }
    }

    /// Ten seconds of varied steering: left, right, analog, idle, wall-pushing.
    pub(crate) fn recorded_inputs() -> Vec<AirInput> {
        (0..1200)
            .map(|tick| match tick / 100 {
                0 | 5 => AirInput::NONE,
                1 | 6 | 7 => AirInput::RIGHT,
                2 => AirInput::new(-0.35),
                3 | 8 => AirInput::LEFT,
                _ => AirInput::new(0.8),
            })
            .collect()
    }
}

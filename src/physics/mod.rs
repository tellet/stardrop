//! Runs movement physics at a fixed rate and draws it smoothly.
//!
//! Physics always steps at [`PHYSICS_HZ`], whatever the display's refresh
//! rate, so the same inputs always produce the same fall. Between steps, the
//! drawn position is blended from the last two physics positions, so motion
//! looks smooth at 60 Hz, 144 Hz or anything else.

use bevy::prelude::*;

use crate::config::GameConfig;
use crate::level::ActiveLevel;

pub mod movement;

use movement::{AirInput, Body};

/// Physics steps per second. Cheap (one circle), and a small step keeps
/// fast-moving objects in later milestones from skipping through each other.
const PHYSICS_HZ: f64 = 120.0;

/// The player's current steering, sampled every frame and consumed by physics.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct AirControl(pub AirInput);

/// A body moved by physics. Keeps the previous position for smooth drawing.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Motion {
    body: Body,
    previous_position: Vec2,
}

impl Motion {
    /// A body at rest at `position`.
    pub fn at_rest(position: Vec2) -> Self {
        Self {
            body: Body::at_rest(position),
            previous_position: position,
        }
    }

    /// The current physical state.
    pub fn body(&self) -> Body {
        self.body
    }

    /// Replaces the velocity-related state (after an object's push) while
    /// keeping the position, so drawing stays smooth.
    pub fn set_body(&mut self, body: Body) {
        self.body = body;
    }

    /// Jumps to a new state without drawing a smeared blend from the old position.
    pub fn teleport(&mut self, body: Body) {
        self.body = body;
        self.previous_position = body.position;
    }
}

/// Ordering labels for other plugins.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum PhysicsSystems {
    /// Advances bodies by one fixed step (in `FixedUpdate`).
    Step,
}

/// Fixed-rate movement physics with interpolated drawing.
pub(crate) struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(PHYSICS_HZ))
            .init_resource::<AirControl>()
            .add_systems(FixedUpdate, step_bodies.in_set(PhysicsSystems::Step))
            .add_systems(
                RunFixedMainLoop,
                draw_interpolated.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            );
    }
}

/// The thin engine layer: read components, call the pure logic, write back.
fn step_bodies(
    time: Res<Time>,
    config: Res<GameConfig>,
    level: Res<ActiveLevel>,
    control: Res<AirControl>,
    mut bodies: Query<&mut Motion>,
) {
    let dt = time.delta_secs();
    let walls = level.walls(config.astronaut_radius);
    for mut motion in &mut bodies {
        motion.previous_position = motion.body.position;
        motion.body = movement::step(motion.body, control.0, &config.movement, walls, dt);
    }
}

/// Places each body's transform between its last two physics positions.
fn draw_interpolated(fixed: Res<Time<Fixed>>, mut bodies: Query<(&Motion, &mut Transform)>) {
    let alpha = fixed.overstep_fraction();
    for (motion, mut transform) in &mut bodies {
        let drawn = movement::interpolate(motion.previous_position, motion.body.position, alpha);
        transform.translation.x = drawn.x;
        transform.translation.y = drawn.y;
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::level::Level;
    use movement::Walls;

    /// Feeds the recorded inputs to physics, one per fixed step.
    #[derive(Resource)]
    struct Script {
        inputs: Vec<AirInput>,
        ticks: usize,
    }

    fn play_script(mut script: ResMut<Script>, mut control: ResMut<AirControl>) {
        control.0 = script
            .inputs
            .get(script.ticks)
            .copied()
            .unwrap_or(AirInput::NONE);
        script.ticks += 1;
    }

    /// What a simulated run ended with.
    struct Outcome {
        body: Body,
        ticks: usize,
        walls: Walls,
        dt: f32,
    }

    /// Runs the real plugin at a given render frame rate for `seconds`.
    fn simulate_at_fps(fps: f64, seconds: f64) -> Outcome {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, crate::config::ConfigPlugin, PhysicsPlugin))
            .insert_resource(ActiveLevel(Level::for_tests()))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / fps,
            )))
            .insert_resource(Script {
                inputs: movement::tests::recorded_inputs(),
                ticks: 0,
            })
            .add_systems(FixedUpdate, play_script.before(PhysicsSystems::Step));
        app.world_mut()
            .spawn((Motion::at_rest(Vec2::ZERO), Transform::default()));
        // `seconds * fps` is a small positive frame count.
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (seconds * fps).round() as usize;
        for _ in 0..frames {
            app.update();
        }
        let world = app.world_mut();
        let body = world.query::<&Motion>().single(world).map(Motion::body);
        Outcome {
            body: body.unwrap_or_default(),
            ticks: world.resource::<Script>().ticks,
            walls: world
                .resource::<ActiveLevel>()
                .walls(GameConfig::default().astronaut_radius),
            dt: world.resource::<Time<Fixed>>().timestep().as_secs_f32(),
        }
    }

    /// The same inputs give the same fall at any frame rate: the engine layer
    /// matches the pure replay exactly, whether the game renders at 30, 60 or 144 FPS.
    #[test]
    fn fall_is_identical_at_any_frame_rate() {
        let config = GameConfig::default();
        for fps in [30.0, 60.0, 144.0] {
            let run = simulate_at_fps(fps, 8.0);
            // Physics may run a step or two more at some frame rates; those get no input.
            let mut inputs = movement::tests::recorded_inputs();
            inputs.resize(run.ticks, AirInput::NONE);
            let expected = movement::replay(
                Body::default(),
                &inputs,
                &config.movement,
                run.walls,
                run.dt,
            );
            assert_eq!(run.body, expected, "at {fps} FPS after {} steps", run.ticks);
        }
    }
}

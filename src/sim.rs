//! Plays a level without Bevy: one attempt, step by step.
//!
//! The solver and the tests use this. It applies the same pure rules the game
//! does, so a level proved beatable here is beatable in the game.

use crate::collision::circle_overlaps_rect;
use crate::config::GameConfig;
use crate::level::{Level, MOTE_RADIUS};
use crate::objects::{ObjectKind, Outcome, on_enter, while_inside};
use crate::physics::movement::{AirInput, Body, Drift, step};
use crate::scoring::StardustSet;

/// Physics steps per second, matching the game.
pub const STEPS_PER_SECOND: f32 = 120.0;
/// Seconds per step.
pub const STEP: f32 = 1.0 / STEPS_PER_SECOND;

/// How an attempt ended, or that it's still going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// Still falling.
    Falling,
    /// Reached the beacon.
    Landed,
    /// Missed the beacon, or touched a hazard.
    Failed,
}

/// One attempt at a level.
#[derive(Debug, Clone)]
pub struct Attempt {
    body: Body,
    /// Whether the astronaut touched each object last step.
    contacts: Vec<bool>,
    /// Bubbles spent this attempt.
    used: Vec<bool>,
    stardust: StardustSet,
    elapsed: f32,
}

impl Attempt {
    /// A fresh attempt at rest on the level's start.
    #[must_use]
    pub fn new(level: &Level) -> Self {
        Self {
            body: Body::at_rest(level.start),
            contacts: vec![false; level.objects.len()],
            used: vec![false; level.objects.len()],
            stardust: StardustSet::default(),
            elapsed: 0.0,
        }
    }

    /// The astronaut's state.
    #[must_use]
    pub fn body(&self) -> Body {
        self.body
    }

    /// Seconds since the fall started.
    #[must_use]
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// How much stardust this attempt has collected.
    #[must_use]
    pub fn stardust(&self) -> usize {
        self.stardust.count()
    }

    /// Advances one physics step with `input` held.
    pub fn step(&mut self, level: &Level, config: &GameConfig, input: AirInput) -> Progress {
        let walls = level.walls(config.astronaut_radius);
        self.body = step(self.body, input, &config.movement, walls, STEP);
        self.elapsed += STEP;

        if let Some(progress) = self.touch_objects(level, config) {
            return progress;
        }
        self.collect_stardust(level, config);

        let beacon = level.beacon;
        if circle_overlaps_rect(
            self.body.position,
            config.astronaut_radius,
            beacon.center,
            beacon.size,
        ) {
            return Progress::Landed;
        }
        if self.body.position.y < level.bottom() - config.astronaut_radius {
            return Progress::Failed;
        }
        Progress::Falling
    }

    /// Applies every object the astronaut touches. `Some` ends the attempt.
    fn touch_objects(&mut self, level: &Level, config: &GameConfig) -> Option<Progress> {
        for (index, placement) in level.objects.iter().enumerate() {
            if self.used[index] {
                continue;
            }
            let touching =
                placement.touches(&config.objects, self.body.position, config.astronaut_radius);
            let entered = touching && !self.contacts[index];
            self.contacts[index] = touching;
            if !touching {
                continue;
            }
            // A bubble shields the astronaut from everything but hazards.
            let kind = placement.kind();
            if self.body.effects.held_by_bubble() && kind != ObjectKind::StaticCloud {
                continue;
            }
            if entered {
                match on_enter(placement, &config.objects, &config.movement, self.body) {
                    Outcome::Nothing => {}
                    Outcome::Changed(changed) | Outcome::Teleported(changed) => {
                        self.body = changed;
                        if kind == ObjectKind::Bubble {
                            self.used[index] = true;
                        }
                    }
                    Outcome::FailAttempt => return Some(Progress::Failed),
                }
            }
            if let Some(velocity) = while_inside(placement, &config.objects, self.body, STEP) {
                self.body = Body {
                    velocity,
                    drift: Drift::Coasting,
                    ..self.body
                };
            }
        }
        None
    }

    fn collect_stardust(&mut self, level: &Level, config: &GameConfig) {
        for (index, &mote) in level.stardust.iter().enumerate() {
            let reach = config.astronaut_radius + MOTE_RADIUS;
            if self.body.position.distance_squared(mote) <= reach * reach {
                self.stardust.insert(index);
            }
        }
    }

    /// A rough measure of how far along the level this attempt is.
    #[must_use]
    pub fn depth(&self) -> f32 {
        -self.body.position.y
    }
}

/// Plays a whole attempt with a fixed list of inputs, one per step.
#[must_use]
pub fn play(level: &Level, config: &GameConfig, inputs: &[AirInput]) -> (Attempt, Progress) {
    let mut attempt = Attempt::new(level);
    for &input in inputs {
        match attempt.step(level, config, input) {
            Progress::Falling => {}
            done => return (attempt, done),
        }
    }
    (attempt, Progress::Falling)
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::math::Vec2;
    use bevy::prelude::*;

    use super::*;
    use crate::objects::Placement;
    use crate::objects::tests::object_app;
    use crate::physics::{AirControl, Motion};

    /// Feeds one input per physics step, the same list the simulator gets.
    #[derive(Resource)]
    struct Script {
        inputs: Vec<AirInput>,
        tick: usize,
    }

    fn play_script(mut script: ResMut<Script>, mut control: ResMut<AirControl>) {
        let input = script.inputs.get(script.tick).copied();
        script.tick += 1;
        control.0 = input.unwrap_or(AirInput::NONE);
    }

    /// A level with something of every kind the astronaut can meet on the way.
    fn busy_level() -> Level {
        let mut level = Level::for_tests();
        level.stardust = vec![Vec2::new(-60.0, -700.0)];
        level.objects = vec![
            Placement::CloudPuff {
                center: Vec2::new(0.0, -600.0),
                radius: 80.0,
            },
            Placement::Crystal {
                center: Vec2::new(-150.0, -1000.0),
                size: 60.0,
            },
            Placement::GravityOrb {
                center: Vec2::new(200.0, -1300.0),
                radius: 50.0,
            },
        ];
        level
    }

    /// The game and the simulator must agree, or a level proved beatable here
    /// could be unbeatable in the game.
    #[test]
    fn the_simulator_matches_the_running_game() {
        let level = busy_level();
        let config = GameConfig::default();
        let inputs: Vec<AirInput> = (0..360)
            .map(|tick| match (tick / 40) % 3 {
                0 => AirInput::RIGHT,
                1 => AirInput::NONE,
                _ => AirInput::LEFT,
            })
            .collect();

        let mut app = object_app(level.clone(), level.start);
        app.insert_resource(Time::<Fixed>::from_hz(f64::from(STEPS_PER_SECOND)))
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f32(STEP),
            ))
            .insert_resource(Script {
                inputs: inputs.clone(),
                tick: 0,
            })
            .add_systems(
                FixedUpdate,
                play_script.before(crate::physics::PhysicsSystems::Step),
            );
        for _ in 0..inputs.len() {
            app.update();
        }
        let world = app.world_mut();
        let ticks = world.resource::<Script>().tick;
        let in_game = world
            .query::<&Motion>()
            .single(world)
            .map(Motion::body)
            .unwrap_or_default();

        let (attempt, _) = play(&level, &config, &inputs[..ticks.min(inputs.len())]);
        let simulated = attempt.body();
        assert!(
            in_game.position.distance(simulated.position) < 1.0,
            "game {:?} vs simulator {:?}",
            in_game.position,
            simulated.position
        );
        assert!(in_game.velocity.distance(simulated.velocity) < 1.0);
    }
}

//! The level being played: loading it from `assets/levels/`, reloading it
//! when the file changes (with the `dev` feature), and spawning its scenery.
//!
//! World coordinates: x = 0 is the center, the top edge is y = 0 and the
//! level extends downward to y = -height.

use bevy::asset::AssetLoadFailedEvent;
use bevy::prelude::*;

mod data;
mod goal;
mod loader;
mod scenery;
mod validate;

pub(crate) use data::{LEVEL_WIDTH, Level};
pub(crate) use goal::LevelCompleted;
use loader::{LevelAsset, LevelLoader};
use scenery::LevelEntity;
pub(crate) use scenery::{CheckpointRing, SceneryAssets};

use crate::physics::movement::Body;
use crate::physics::{Motion, PhysicsSystems};

/// The level loaded at startup, relative to `assets/`. Level selection arrives in Milestone 5.
const LEVEL_PATH: &str = "levels/test.level.ron";

/// Where the current level is in its lifecycle.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum LevelPhase {
    /// Waiting for the level file (or for a broken one to be fixed).
    #[default]
    Loading,
    /// Playing.
    Falling,
    /// Missed the beacon: fading back to the last checkpoint. Physics is paused.
    Respawning,
    /// Reached the beacon; physics is paused.
    Landed,
}

/// The validated level currently being played. Only exists once a valid
/// level has loaded, so systems that need it run in [`LevelPhase::Falling`].
#[derive(Resource, Debug, Deref)]
pub(crate) struct ActiveLevel(pub(crate) Level);

/// What's wrong with the level file, if anything, as a message for the screen.
#[derive(Resource, Debug, Default, PartialEq)]
pub(crate) struct LevelProblem(pub Option<String>);

/// Keeps the level asset loaded (and hot-reloadable) while it's in use.
#[derive(Resource)]
struct LevelHandle(Handle<LevelAsset>);

/// Loads the level and keeps the game in sync with it.
pub(crate) struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        add_play_rules(app);
        app.init_asset::<LevelAsset>()
            .init_asset_loader::<LevelLoader>()
            .init_resource::<LevelProblem>()
            .add_systems(Startup, start_loading)
            .add_systems(
                Update,
                (
                    apply_level_changes.run_if(on_message::<AssetEvent<LevelAsset>>),
                    report_unreadable_file.run_if(on_message::<AssetLoadFailedEvent<LevelAsset>>),
                ),
            );
    }
}

/// The gameplay rules of a level, without any file loading. Kept separate so
/// tests can run the real rules in a minimal app with a level they provide.
fn add_play_rules(app: &mut App) {
    app.init_state::<LevelPhase>()
        .add_message::<LevelCompleted>()
        // Physics only runs while falling through a loaded level.
        .configure_sets(FixedUpdate, PhysicsSystems::Step.run_if(still_falling))
        .add_systems(
            FixedUpdate,
            goal::land_on_beacon
                .in_set(LevelSystems::Rules)
                .run_if(still_falling),
        )
        .configure_sets(FixedUpdate, LevelSystems::Rules.after(PhysicsSystems::Step));
}

/// Ordering label for rules that react to where physics moved the astronaut.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum LevelSystems {
    /// Landing, checkpoints, falling out (in `FixedUpdate`, after physics).
    Rules,
}

/// Run condition: falling, with no phase change waiting.
///
/// Several fixed steps can run in one frame, but a state change only happens
/// between frames. Without the second check, the step right after landing
/// would still move the astronaut (and could land a second time).
pub(crate) fn still_falling(
    phase: Res<State<LevelPhase>>,
    next: Res<NextState<LevelPhase>>,
) -> bool {
    *phase.get() == LevelPhase::Falling && matches!(*next, NextState::Unchanged)
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.init_resource::<SceneryAssets>();
    commands.insert_resource(LevelHandle(server.load(LEVEL_PATH)));
}

/// Runs when the level file finishes loading or changes on disk.
fn apply_level_changes(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<LevelAsset>>,
    handle: Res<LevelHandle>,
    levels: Res<Assets<LevelAsset>>,
    scenery: Res<SceneryAssets>,
    old_scenery: Query<Entity, With<LevelEntity>>,
    mut bodies: Query<&mut Motion>,
    mut problem: ResMut<LevelProblem>,
    phase: Res<State<LevelPhase>>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    let ours = handle.0.id();
    let changed = events.read().any(|event| {
        matches!(event,
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } if *id == ours)
    });
    // `let ... else` : if the asset isn't there, stop here.
    let Some(LevelAsset(result)) = levels.get(ours).filter(|_| changed) else {
        return;
    };

    match result {
        Err(error) => {
            // Keep playing the previous version (if any) and show what's wrong.
            warn!("{LEVEL_PATH}: {error}");
            problem.set_if_neq(LevelProblem(Some(format!("{LEVEL_PATH}\n{error}"))));
        }
        Ok(level) => {
            problem.set_if_neq(LevelProblem(None));
            for entity in &old_scenery {
                commands.entity(entity).despawn();
            }
            scenery::spawn(&mut commands, &scenery, level);
            // Keep the astronaut where it is, unless the edit left it outside the level.
            for mut motion in &mut bodies {
                if !level.contains(motion.body().position) {
                    motion.teleport(Body::at_rest(level.start));
                }
            }
            commands.insert_resource(ActiveLevel(level.clone()));
            if *phase.get() == LevelPhase::Loading {
                next_phase.set(LevelPhase::Falling);
            }
            info!("Loaded level \"{}\"", level.name);
        }
    }
}

/// The file couldn't be read at all (missing, no permission, ...).
fn report_unreadable_file(
    mut failures: MessageReader<AssetLoadFailedEvent<LevelAsset>>,
    mut problem: ResMut<LevelProblem>,
) {
    for failure in failures.read() {
        warn!("{}: {}", failure.path, failure.error);
        problem.set_if_neq(LevelProblem(Some(format!(
            "{}\n{}",
            failure.path, failure.error
        ))));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use core::time::Duration;

    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::astronaut::Astronaut;
    use crate::config::GameConfig;
    use crate::physics::PhysicsPlugin;

    /// A minimal app running the real level rules and physics on
    /// [`Level::for_tests`], already falling, with the astronaut at `start`.
    /// Each `update()` advances `frame` seconds of game time.
    pub(crate) fn play_app(start: Vec2, frame: Duration) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .add_plugins((crate::config::ConfigPlugin, PhysicsPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(frame))
            .insert_resource(ActiveLevel(Level::for_tests()));
        add_play_rules(&mut app);
        app.insert_state(LevelPhase::Falling);
        app.world_mut()
            .spawn((Astronaut, Motion::at_rest(start), Transform::default()));
        app
    }

    #[derive(Resource, Default)]
    struct Completions(usize);

    fn count_completions(mut read: MessageReader<LevelCompleted>, mut count: ResMut<Completions>) {
        count.0 += read.read().count();
    }

    #[test]
    fn landing_on_the_beacon_completes_the_level_exactly_once() {
        let level = Level::for_tests();
        let radius = GameConfig::default().astronaut_radius;
        let pad_top = level.beacon.center.y + level.beacon.size.y / 2.0;
        // Just above the pad, and frames long enough for several physics
        // steps each: the case where a second landing could slip in.
        let start = Vec2::new(0.0, pad_top + radius + 1.0);
        let mut app = play_app(start, Duration::from_millis(50));
        app.init_resource::<Completions>()
            .add_systems(Update, count_completions);
        for _ in 0..30 {
            app.update();
        }
        let world = app.world_mut();
        assert_eq!(world.resource::<Completions>().0, 1);
        assert_eq!(
            *world.resource::<State<LevelPhase>>().get(),
            LevelPhase::Landed
        );
        let body = world.query::<&Motion>().single(world).map(Motion::body);
        let body = body.unwrap_or_default();
        assert!(
            (body.position.y - (pad_top + radius)).abs() < 1e-3,
            "rests on top of the pad"
        );
        assert_eq!(body.velocity, Vec2::ZERO);
    }
}

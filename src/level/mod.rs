//! The level being played: loading it from `assets/levels/`, reloading it
//! when the file changes (with the `dev` feature), and spawning its scenery.
//!
//! World coordinates: x = 0 is the center, the top edge is y = 0 and the
//! level extends downward to y = -height.

use bevy::asset::AssetLoadFailedEvent;
use bevy::prelude::*;

mod data;
mod loader;
mod scenery;
mod validate;

pub(crate) use data::{LEVEL_WIDTH, Level};
use loader::{LevelAsset, LevelLoader};
use scenery::{LevelEntity, SceneryAssets};

use crate::config::GameConfig;
use crate::physics::movement::Body;
use crate::physics::{Motion, PhysicsSystems};

/// The level loaded at startup, relative to `assets/`. Level selection arrives in Milestone 5.
const LEVEL_PATH: &str = "levels/test.level.ron";

/// Where the current level is in its lifecycle. Respawning and landing
/// arrive later in Milestone 2.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum LevelPhase {
    /// Waiting for the level file (or for a broken one to be fixed).
    #[default]
    Loading,
    /// Playing.
    Falling,
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
        app.init_asset::<LevelAsset>()
            .init_asset_loader::<LevelLoader>()
            .init_state::<LevelPhase>()
            .init_resource::<LevelProblem>()
            // Physics only runs while falling through a loaded level.
            .configure_sets(
                FixedUpdate,
                PhysicsSystems::Step.run_if(in_state(LevelPhase::Falling)),
            )
            .add_systems(Startup, start_loading)
            .add_systems(
                Update,
                (
                    apply_level_changes.run_if(on_message::<AssetEvent<LevelAsset>>),
                    report_unreadable_file.run_if(on_message::<AssetLoadFailedEvent<LevelAsset>>),
                ),
            )
            .add_systems(
                FixedUpdate,
                restart_below_bottom
                    .after(PhysicsSystems::Step)
                    .run_if(in_state(LevelPhase::Falling)),
            );
    }
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

/// Placeholder until checkpoints exist (later in Milestone 2): falling past
/// the bottom starts the fall again from the top.
fn restart_below_bottom(
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    mut bodies: Query<&mut Motion>,
) {
    for mut motion in &mut bodies {
        if motion.body().position.y < level.bottom() - config.astronaut_radius {
            motion.teleport(Body::at_rest(level.start));
        }
    }
}

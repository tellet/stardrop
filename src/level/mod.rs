//! The level being played: loading it from `assets/levels/`, reloading it
//! when the file changes (with the `dev` feature), and spawning its scenery.
//!
//! World coordinates: x = 0 is the center, the top edge is y = 0 and the
//! level extends downward to y = -[`LEVEL_HEIGHT`].

use bevy::asset::AssetLoadFailedEvent;
use bevy::prelude::*;

mod data;
mod goal;
mod list;
mod loader;
mod scenery;
mod validate;

pub use data::{LEVEL_HEIGHT, LEVEL_WIDTH, Level, Theme};
pub(crate) use goal::LevelCompleted;
pub(crate) use list::{LevelEntry, LevelList};
use list::{LevelListAsset, LevelListLoader};
use loader::{LevelAsset, LevelLoader};
pub(crate) use scenery::LevelEntity;
use scenery::SceneryAssets;
pub(crate) use scenery::{BeaconHalo, MOTE_RADIUS, StardustMote, Twinkle};
pub use validate::LevelError;

use crate::app_state::AppState;
use crate::attempt::AttemptRestarted;
use crate::physics::PhysicsSystems;

/// Where level files live, relative to `assets/`.
const LEVELS_DIR: &str = "levels";
/// The list of levels, relative to `assets/`.
const LEVEL_LIST_PATH: &str = "levels/levels.list.ron";

/// Where the current attempt is. Physics only runs while `Falling`.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum LevelPhase {
    /// Waiting for the level file (or for a broken one to be fixed).
    #[default]
    Loading,
    /// Floating at the start while the player studies the level. The first
    /// left/right press starts the fall.
    Ready,
    /// Playing.
    Falling,
    /// The attempt failed: fading back to the start.
    Restarting,
    /// Reached the beacon.
    Landed,
}

/// The validated level currently being played. Only exists once a valid
/// level has loaded: systems that need it are gated by phase or by
/// `resource_exists`.
#[derive(Resource, Debug, Deref)]
pub(crate) struct ActiveLevel(pub(crate) Level);

/// What's wrong with the level file, if anything, as a message for the screen.
#[derive(Resource, Debug, Default, PartialEq)]
pub(crate) struct LevelProblem(pub Option<String>);

/// Keeps the level asset loaded (and hot-reloadable) while it's in use.
#[derive(Resource)]
struct LevelHandle(Handle<LevelAsset>);

/// Keeps the level list loaded (and hot-reloadable).
#[derive(Resource)]
struct LevelListHandle(Handle<LevelListAsset>);

/// The levels the game offers, once the list has loaded.
#[derive(Resource, Debug, Deref)]
pub(crate) struct Levels(pub(crate) LevelList);

/// Which level the currently loaded file belongs to.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
struct LoadedLevel(SelectedLevel);

/// Which level is being played or was chosen last (an index into [`Levels`]).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedLevel(pub usize);

impl Levels {
    /// The chosen level's entry, if it exists.
    pub(crate) fn entry(&self, selected: SelectedLevel) -> Option<&LevelEntry> {
        self.0.levels.get(selected.0)
    }
}

/// Loads the level and keeps the game in sync with it.
pub(crate) struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        add_play_rules(app);
        app.init_asset::<LevelAsset>()
            .init_asset_loader::<LevelLoader>()
            .init_asset::<LevelListAsset>()
            .init_asset_loader::<LevelListLoader>()
            .init_resource::<LevelProblem>()
            // Shared meshes and materials, built up front like the objects'.
            .init_resource::<SceneryAssets>()
            .init_resource::<SelectedLevel>()
            .add_systems(Startup, start_loading)
            .add_systems(OnEnter(AppState::Playing), load_selected_level)
            // The list may finish loading after play has already started.
            .add_systems(
                Update,
                load_selected_level.run_if(
                    in_state(AppState::Playing).and_then(resource_exists_and_changed::<Levels>),
                ),
            )
            // Pausing also leaves `Playing`, so the level is freed when a
            // menu is *entered* instead, never when play is merely paused.
            .add_systems(
                OnEnter(AppState::LevelSelect),
                unload_level.run_if(resource_exists::<LevelHandle>),
            )
            .add_systems(
                OnEnter(AppState::MainMenu),
                unload_level.run_if(resource_exists::<LevelHandle>),
            )
            .add_systems(
                Update,
                (
                    apply_level_changes.run_if(on_message::<AssetEvent<LevelAsset>>),
                    apply_level_list.run_if(on_message::<AssetEvent<LevelListAsset>>),
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
    /// Landing, stardust, falling out (in `FixedUpdate`, after physics).
    Rules,
}

/// Run condition: a level is loaded and being played (not paused, not in a
/// menu, and not between levels). Tests build small apps without
/// [`AppState`]; there, "no app state" counts as playing.
///
/// Systems that reach for [`ActiveLevel`] must be gated by this: between
/// finishing one level and loading the next there is no level at all, and a
/// system asking for a missing resource stops the game.
pub(crate) fn playing(
    state: Option<Res<State<AppState>>>,
    level: Option<Res<ActiveLevel>>,
) -> bool {
    level.is_some() && state.is_none_or(|state| *state.get() == AppState::Playing)
}

/// Run condition: falling, with no phase change waiting.
///
/// Several fixed steps can run in one frame, but a state change only happens
/// between frames. Without the second check, the step right after landing
/// would still move the astronaut (and could land a second time).
pub(crate) fn still_falling(
    state: Option<Res<State<AppState>>>,
    level: Option<Res<ActiveLevel>>,
    phase: Res<State<LevelPhase>>,
    next: Res<NextState<LevelPhase>>,
) -> bool {
    playing(state, level)
        && *phase.get() == LevelPhase::Falling
        && matches!(*next, NextState::Unchanged)
}

fn start_loading(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(LevelListHandle(server.load(LEVEL_LIST_PATH)));
}

/// Loads the chosen level when play starts. Handing out a new handle drops
/// the old level, so its assets are freed.
fn load_selected_level(
    mut commands: Commands,
    server: Res<AssetServer>,
    levels: Option<Res<Levels>>,
    selected: Res<SelectedLevel>,
    loaded: Option<Res<LoadedLevel>>,
    mut problem: ResMut<LevelProblem>,
) {
    // Resuming from the pause menu also enters `Playing`: keep the level
    // that's already loaded, so pausing never restarts it.
    if loaded.is_some_and(|loaded| loaded.0 == *selected) {
        return;
    }
    let Some(entry) = levels.as_ref().and_then(|levels| levels.entry(*selected)) else {
        problem.set_if_neq(LevelProblem(Some("no level list".into())));
        return;
    };
    commands.insert_resource(LevelHandle(
        server.load(format!("{LEVELS_DIR}/{}", entry.file)),
    ));
    commands.insert_resource(LoadedLevel(*selected));
}

/// Going back to a menu frees the level, its scenery and its objects.
fn unload_level(mut commands: Commands, scenery: Query<Entity, With<LevelEntity>>) {
    for entity in &scenery {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LevelHandle>();
    commands.remove_resource::<LoadedLevel>();
    commands.remove_resource::<ActiveLevel>();
}

/// Runs when the level list loads or changes on disk.
fn apply_level_list(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<LevelListAsset>>,
    handle: Res<LevelListHandle>,
    lists: Res<Assets<LevelListAsset>>,
    mut problem: ResMut<LevelProblem>,
) {
    let ours = handle.0.id();
    let changed = events.read().any(|event| {
        matches!(event,
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } if *id == ours)
    });
    let Some(LevelListAsset(result)) = lists.get(ours).filter(|_| changed) else {
        return;
    };
    match result {
        Err(error) => {
            warn!("{LEVEL_LIST_PATH}: {error}");
            problem.set_if_neq(LevelProblem(Some(format!("{LEVEL_LIST_PATH}\n{error}"))));
        }
        Ok(list) => {
            info!("{} levels available", list.levels.len());
            commands.insert_resource(Levels(list.clone()));
        }
    }
}

/// Runs when the level file finishes loading or changes on disk.
fn apply_level_changes(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<LevelAsset>>,
    // Optional: leaving a level drops the handle, and the asset system still
    // sends a last event afterwards.
    handle: Option<Res<LevelHandle>>,
    levels: Res<Assets<LevelAsset>>,
    scenery: Res<SceneryAssets>,
    old_scenery: Query<Entity, With<LevelEntity>>,
    mut problem: ResMut<LevelProblem>,
    // The level file this system last put into play.
    mut applied: Local<Option<AssetId<LevelAsset>>>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
    mut restarted: MessageWriter<AttemptRestarted>,
) {
    let Some(handle) = handle else {
        return;
    };
    let ours = handle.0.id();
    let changed = events.read().any(|event| {
        matches!(event,
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } if *id == ours)
    });
    // `let ... else` : if the asset isn't there, stop here.
    let Some(LevelAsset(result)) = levels.get(ours).filter(|_| changed) else {
        return;
    };

    // The file this handle came from, for the message on screen.
    let path = handle
        .0
        .path()
        .map_or_else(|| "level".to_owned(), ToString::to_string);

    match result {
        Err(error) => {
            // Keep playing the previous version (if any) and show what's wrong.
            warn!("{path}: {error}");
            problem.set_if_neq(LevelProblem(Some(format!("{path}\n{error}"))));
        }
        Ok(level) => {
            problem.set_if_neq(LevelProblem(None));
            for entity in &old_scenery {
                commands.entity(entity).despawn();
            }
            scenery::spawn(&mut commands, &scenery, level);
            // Replacing the level also restarts the attempt (see `attempt.rs`):
            // with the whole level on screen, a fresh start shows the edit best.
            let is_new_level = *applied != Some(ours);
            *applied = Some(ours);
            commands.insert_resource(ActiveLevel(level.clone()));
            // A different level always starts a fresh attempt, whatever the
            // last one ended in. A new version of the level being played
            // doesn't: `attempt.rs` restarts that one, in place.
            if is_new_level {
                next_phase.set(LevelPhase::Ready);
                restarted.write(AttemptRestarted);
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

    use bevy::asset::AssetPlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::astronaut::Astronaut;
    use crate::config::GameConfig;
    use crate::physics::movement::Body;
    use crate::physics::{Motion, PhysicsPlugin};

    /// A minimal app running the real level rules and physics on
    /// [`Level::for_tests`], already falling, with the astronaut at `start`.
    /// (It skips `Ready`, so tests don't need to press a key to drop.)
    /// Each `update()` advances `frame` seconds of game time.
    pub(crate) fn play_app(start: Vec2, frame: Duration) -> App {
        play_app_with(Level::for_tests(), start, frame)
    }

    /// Like [`play_app`], on a level of the test's choosing.
    pub(crate) fn play_app_with(level: Level, start: Vec2, frame: Duration) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .add_plugins((crate::config::ConfigPlugin, PhysicsPlugin))
            // Systems read settings (reduce motion, quality) even in a level.
            .init_resource::<crate::settings::Settings>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(frame))
            .insert_resource(ActiveLevel(level));
        add_play_rules(&mut app);
        app.insert_state(LevelPhase::Falling);
        app.world_mut()
            .spawn((Astronaut, Motion::at_rest(start), Transform::default()));
        app
    }

    /// Spawns the level's scenery the same way a (re)load does: remove the
    /// old scenery, spawn fresh from shared assets.
    pub(crate) fn reload_scenery(world: &mut World) {
        let old: Vec<Entity> = world
            .query_filtered::<Entity, With<LevelEntity>>()
            .iter(world)
            .collect();
        for entity in old {
            world.despawn(entity);
        }
        if !world.contains_resource::<SceneryAssets>() {
            world.init_resource::<SceneryAssets>();
        }
        world.resource_scope(|world, scenery: Mut<SceneryAssets>| {
            let level = world.resource::<ActiveLevel>().0.clone();
            let mut commands = world.commands();
            scenery::spawn(&mut commands, &scenery, &level);
        });
        world.flush();
    }

    /// Every level in the shipped list: (name, file, parsed level).
    pub(crate) fn shipped_levels() -> Vec<(String, String, Level)> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/levels");
        let text = std::fs::read_to_string(format!("{dir}/levels.list.ron")).unwrap();
        let list = LevelList::parse(&text).unwrap();
        list.levels
            .iter()
            .map(|entry| {
                let text = std::fs::read_to_string(format!("{dir}/{}", entry.file)).unwrap();
                let level =
                    Level::parse(&text).unwrap_or_else(|error| panic!("{}: {error}", entry.file));
                (entry.name.clone(), entry.file.clone(), level)
            })
            .collect()
    }

    /// The whole game flow, headless: real level files, real loading, the
    /// real screens. No window, no menus, no save file.
    fn game_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_resource::<crate::settings::Settings>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_state::<AppState>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                16,
            )))
            .add_plugins((
                crate::config::ConfigPlugin,
                LevelPlugin,
                PhysicsPlugin,
                crate::astronaut::AstronautPlugin,
                crate::attempt::AttemptPlugin,
                crate::collectibles::CollectiblesPlugin,
                crate::objects::ObjectsPlugin,
                crate::effects::EffectsPlugin,
                crate::particles::ParticlesPlugin,
            ));
        app
    }

    /// Runs frames until `ready` is true, or gives up. Level files load on a
    /// background thread, so the number of frames it takes isn't fixed.
    fn run_until(app: &mut App, what: &str, ready: impl Fn(&App) -> bool) {
        for _ in 0..600 {
            app.update();
            if ready(app) {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("{what} never happened");
    }

    fn go_to(app: &mut App, state: AppState) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        app.update();
    }

    fn start_level(app: &mut App, index: usize) {
        let wanted = app
            .world()
            .resource::<Levels>()
            .entry(SelectedLevel(index))
            .expect("the level is in the list")
            .name
            .clone();
        *app.world_mut().resource_mut::<SelectedLevel>() = SelectedLevel(index);
        go_to(app, AppState::Playing);
        // "Next level" keeps the level that just finished loaded, so waiting
        // for `ActiveLevel` to merely exist would wait for nothing.
        run_until(app, "the level loaded", |app| {
            app.world()
                .get_resource::<ActiveLevel>()
                .is_some_and(|level| level.name == wanted)
        });
    }

    /// Landing, without playing the level: drops the astronaut onto the pad.
    fn land(app: &mut App) {
        let beacon = app.world().resource::<ActiveLevel>().beacon;
        let world = app.world_mut();
        let mut astronaut = world.query_filtered::<&mut Motion, With<Astronaut>>();
        for mut motion in astronaut.iter_mut(world) {
            motion.teleport(Body::at_rest(beacon.center));
        }
        app.world_mut()
            .resource_mut::<NextState<LevelPhase>>()
            .set(LevelPhase::Falling);
        run_until(app, "the landing", |app| {
            *app.world().resource::<State<LevelPhase>>().get() == LevelPhase::Landed
        });
    }

    /// Both ways on to another level after finishing one: through the list,
    /// and straight on with "Next level" (which never leaves the level).
    #[test]
    fn choosing_another_level_after_landing_starts_it_fresh() {
        for through_the_list in [true, false] {
            let mut app = game_app();
            run_until(&mut app, "the level list loaded", |app| {
                app.world().contains_resource::<Levels>()
            });
            start_level(&mut app, 0);
            land(&mut app);
            if through_the_list {
                go_to(&mut app, AppState::LevelSelect);
            }
            start_level(&mut app, 1);
            app.update();
            assert_fresh_attempt(&mut app);
        }
    }

    /// A fresh attempt: waiting at the start, with nothing carried over.
    fn assert_fresh_attempt(app: &mut App) {
        let start = app.world().resource::<ActiveLevel>().start;
        assert_eq!(
            *app.world().resource::<State<LevelPhase>>().get(),
            LevelPhase::Ready,
            "the new level must wait at the start, not carry the old phase"
        );
        let world = app.world_mut();
        let body = world
            .query_filtered::<&Motion, With<Astronaut>>()
            .single(world)
            .map(Motion::body)
            .unwrap_or_default();
        assert!(
            body.position.distance(start) < 10.0,
            "the astronaut waits at the new start, not at {:?}",
            body.position
        );
        let run = *world.resource::<crate::collectibles::CurrentRun>();
        assert_eq!(run.stardust.count(), 0, "stardust carried over");
        assert!(
            run.elapsed < 1e-3,
            "the clock carried over: {}",
            run.elapsed
        );
    }

    /// What our own systems cost per frame, away from the renderer and its
    /// vsync (Section 7.1's 1 ms budget). A measurement, not a check:
    /// `cargo test --release --lib -- --ignored frame_cost --nocapture`.
    #[test]
    #[ignore = "a measurement, not a check"]
    #[expect(clippy::print_stdout, reason = "this test reports a measurement")]
    fn frame_cost_of_the_busiest_level() {
        let busiest = shipped_levels()
            .into_iter()
            .max_by_key(|(_, _, level)| level.objects.len() + level.stardust.len())
            .expect("levels are shipped");
        let mut app = game_app();
        run_until(&mut app, "the level list loaded", |app| {
            app.world().contains_resource::<Levels>()
        });
        let index = shipped_levels()
            .iter()
            .position(|(name, _, _)| *name == busiest.0)
            .unwrap_or(0);
        start_level(&mut app, index);
        app.world_mut()
            .resource_mut::<NextState<LevelPhase>>()
            .set(LevelPhase::Falling);
        // Warm up: first frames allocate and spawn.
        for _ in 0..120 {
            app.update();
        }
        let frames = 600;
        let started = std::time::Instant::now();
        for _ in 0..frames {
            app.update();
        }
        let per_frame = started.elapsed() / frames;
        println!(
            "{}: {} objects, {} stardust, {:.3} ms of our own work per frame",
            busiest.0,
            busiest.2.objects.len(),
            busiest.2.stardust.len(),
            per_frame.as_secs_f64() * 1000.0
        );
        assert!(
            per_frame.as_secs_f64() * 1000.0 < 1.0,
            "over the 1 ms budget: {per_frame:?}"
        );
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

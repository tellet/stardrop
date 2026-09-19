//! Resource-usage tests across whole play flows (Section 7.5): repeating
//! retries, respawns and level reloads must not grow the world or the assets.

use core::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::checkpoint::CheckpointPlugin;
use crate::collectibles::CollectiblesPlugin;
use crate::level::tests::{play_app, reload_scenery};
use crate::level::{ActiveLevel, LevelPhase};
use crate::physics::Motion;
use crate::physics::movement::Body;

/// Entities, meshes and materials: what could leak.
#[derive(Debug, PartialEq, Eq)]
struct Footprint {
    entities: u32,
    meshes: usize,
    materials: usize,
}

fn footprint(app: &App) -> Footprint {
    let world = app.world();
    Footprint {
        entities: world.entities().count_spawned(),
        meshes: world.resource::<Assets<Mesh>>().len(),
        materials: world.resource::<Assets<ColorMaterial>>().len(),
    }
}

fn phase(app: &App) -> LevelPhase {
    *app.world().resource::<State<LevelPhase>>().get()
}

fn press(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
}

/// Puts the astronaut below the level and runs until the respawn finishes.
fn fall_out_and_respawn(app: &mut App) {
    let bottom = app.world().resource::<ActiveLevel>().bottom();
    let world = app.world_mut();
    let mut astronaut = world.query_filtered::<&mut Motion, With<Astronaut>>();
    for mut motion in astronaut.iter_mut(world) {
        motion.teleport(Body::at_rest(Vec2::new(0.0, bottom - 500.0)));
    }
    let mut frames = 0;
    while phase(app) != LevelPhase::Respawning {
        app.update();
        frames += 1;
        assert!(frames < 10, "didn't start respawning");
    }
    while phase(app) == LevelPhase::Respawning {
        app.update();
        frames += 1;
        assert!(frames < 120, "respawn never finished");
    }
}

#[test]
fn twenty_retries_respawns_and_reloads_leave_no_trace() {
    let mut app = play_app(Vec2::new(0.0, -300.0), Duration::from_secs_f64(1.0 / 60.0));
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<ColorMaterial>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins((CheckpointPlugin, CollectiblesPlugin));
    reload_scenery(app.world_mut());
    // Warm up: a few frames of falling, one of each flow, so one-time
    // allocations (fade overlay, first messages) are included in the baseline.
    for _ in 0..30 {
        app.update();
    }
    press(&mut app, KeyCode::KeyR);
    fall_out_and_respawn(&mut app);
    reload_scenery(app.world_mut());
    let before = footprint(&app);

    for _ in 0..20 {
        for _ in 0..30 {
            app.update();
        }
        press(&mut app, KeyCode::KeyR);
        fall_out_and_respawn(&mut app);
        reload_scenery(app.world_mut());
    }

    assert_eq!(footprint(&app), before);
    assert_eq!(phase(&app), LevelPhase::Falling);
}

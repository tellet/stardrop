//! Resource-usage tests across whole play flows (Section 7.5): repeating
//! retries, failed attempts and level reloads must not grow the world or the
//! assets.

use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::collectibles::CollectiblesPlugin;
use crate::level::tests::reload_scenery;
use crate::level::{ActiveLevel, Level, LevelPhase};
use crate::objects::Placement;
use crate::objects::tests::object_app;
use crate::physics::movement::{AirInput, Body};
use crate::physics::{AirControl, Motion};

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

/// Presses "right" for a frame to leave `Ready`, like a player dropping.
fn drop(app: &mut App) {
    app.world_mut().resource_mut::<AirControl>().0 = AirInput::RIGHT;
    app.update();
    app.world_mut().resource_mut::<AirControl>().0 = AirInput::NONE;
    app.update();
    assert_eq!(phase(app), LevelPhase::Falling);
}

/// Puts the astronaut below the level and runs until the fresh attempt is ready.
fn fall_out_and_restart(app: &mut App) {
    let bottom = app.world().resource::<ActiveLevel>().bottom();
    let world = app.world_mut();
    let mut astronaut = world.query_filtered::<&mut Motion, With<Astronaut>>();
    for mut motion in astronaut.iter_mut(world) {
        motion.teleport(Body::at_rest(Vec2::new(0.0, bottom - 500.0)));
    }
    let mut frames = 0;
    while phase(app) != LevelPhase::Restarting {
        app.update();
        frames += 1;
        assert!(frames < 10, "didn't start restarting");
    }
    while phase(app) == LevelPhase::Restarting {
        app.update();
        frames += 1;
        assert!(frames < 120, "restart never finished");
    }
    assert_eq!(phase(app), LevelPhase::Ready);
}

/// Everything a hot reload redoes: scenery respawns, and marking the level
/// changed makes objects respawn and the attempt restart.
fn reload_level(app: &mut App) {
    reload_scenery(app.world_mut());
    app.world_mut().resource_mut::<ActiveLevel>().set_changed();
    app.update();
    app.update();
}

#[test]
fn twenty_retries_failures_and_reloads_leave_no_trace() {
    let mut level = Level::for_tests();
    // A ribbon owns its own meshes: reloads must free them, not pile them up.
    level.objects = vec![
        Placement::StaticCloud {
            center: Vec2::new(300.0, -900.0),
            radius: 90.0,
        },
        Placement::WindRibbon {
            path: vec![Vec2::new(-400.0, -1300.0), Vec2::new(-100.0, -1350.0)],
            width: 80.0,
        },
    ];
    let mut app = object_app(level, Vec2::new(0.0, -200.0));
    app.add_plugins(CollectiblesPlugin);
    reload_scenery(app.world_mut());
    // Warm up: a few frames of falling, one of each flow, so one-time
    // allocations (fade overlay, first messages) are included in the baseline.
    for _ in 0..30 {
        app.update();
    }
    press(&mut app, KeyCode::KeyR);
    drop(&mut app);
    fall_out_and_restart(&mut app);
    reload_level(&mut app);
    let before = footprint(&app);

    for _ in 0..20 {
        drop(&mut app);
        for _ in 0..30 {
            app.update();
        }
        press(&mut app, KeyCode::KeyR);
        drop(&mut app);
        fall_out_and_restart(&mut app);
        reload_level(&mut app);
    }

    assert_eq!(footprint(&app), before);
    assert_eq!(phase(&app), LevelPhase::Ready);
}

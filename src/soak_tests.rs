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

/// Memory over a long session, the thing entity counts can't show: does the
/// process itself keep growing? A measurement, not a check:
/// `cargo test --release --lib -- --ignored long_session --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
#[expect(clippy::print_stdout, reason = "this test reports a measurement")]
fn long_session_memory() {
    /// Resident memory of this process, in kilobytes.
    fn resident_kb() -> u64 {
        let text = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
        let pages: u64 = text
            .split_whitespace()
            .nth(1)
            .and_then(|pages| pages.parse().ok())
            .unwrap_or(0);
        pages * 4
    }

    let mut app = object_app(busy_level(), Vec2::new(0.0, -200.0));
    app.add_plugins(CollectiblesPlugin);
    reload_scenery(app.world_mut());
    // The same warm-up as the footprint soak: one of each flow, so one-time
    // allocations are behind us before the first measurement.
    for _ in 0..30 {
        app.update();
    }
    press(&mut app, KeyCode::KeyR);
    drop(&mut app);
    fall_out_and_restart(&mut app);
    reload_level(&mut app);
    let start = resident_kb();
    let rounds = 400;
    for round in 0..rounds {
        drop(&mut app);
        for _ in 0..40 {
            app.update();
        }
        press(&mut app, KeyCode::KeyR);
        drop(&mut app);
        fall_out_and_restart(&mut app);
        reload_level(&mut app);
        if round % 100 == 0 {
            println!("round {round}: {} KB", resident_kb());
        }
    }
    let end = resident_kb();
    println!(
        "{rounds} rounds of play, failure, retry and reload: {start} KB -> {end} KB ({:+} KB)",
        i64::try_from(end).unwrap_or(0) - i64::try_from(start).unwrap_or(0)
    );
    // Entities are covered by the footprint soak; this one is about memory.
    assert_eq!(phase(&app), LevelPhase::Ready);
}

/// A level with something of every kind, for the soaks.
fn busy_level() -> Level {
    let mut level = Level::for_tests();
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
    level
}

#[test]
fn twenty_retries_failures_and_reloads_leave_no_trace() {
    // A ribbon owns its own meshes: reloads must free them, not pile them up.
    let mut app = object_app(busy_level(), Vec2::new(0.0, -200.0));
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

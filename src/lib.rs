//! Stardrop: a calm, dreamy falling game about discovery.
//!
//! Each feature area is its own module exposing one Bevy plugin.
//! Game logic that doesn't need the engine lives in plain-Rust submodules
//! so it can be unit-tested and benchmarked without a running `App`.

use bevy::prelude::*;
use bevy::window::WindowResolution;

mod app_state;
mod astronaut;
mod attempt;
mod background;
mod camera;
mod collectibles;
pub mod collision;
pub mod config;
pub mod debug;
mod effects;
mod journal;
pub mod level;
mod menu;
pub mod objects;
pub mod physics;
mod rng;
pub mod save;
pub mod scoring;
#[cfg(test)]
mod soak_tests;
mod ui;

/// Window size on first launch, in physical pixels. 16:9, the aspect ratio the game is designed for.
const INITIAL_WINDOW_SIZE: (u32, u32) = (1280, 720);

/// Builds and runs the game until the window is closed.
#[must_use]
pub fn run() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Stardrop".into(),
                resolution: WindowResolution::from(INITIAL_WINDOW_SIZE),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            app_state::AppStatePlugin,
            config::ConfigPlugin,
            level::LevelPlugin,
            physics::PhysicsPlugin,
            astronaut::AstronautPlugin,
            attempt::AttemptPlugin,
            collectibles::CollectiblesPlugin,
            objects::ObjectsPlugin,
            effects::EffectsPlugin,
            camera::CameraPlugin,
            background::BackgroundPlugin,
            debug::DebugPlugin,
        ))
        // A second group: Bevy's `add_plugins` takes at most 15 at a time.
        .add_plugins((
            journal::JournalPlugin,
            save::SavePlugin,
            menu::MenuPlugin,
            ui::UiPlugin,
        ))
        .run()
}

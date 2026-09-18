//! Stardrop: a calm, dreamy falling game about discovery.
//!
//! Each feature area is its own module exposing one Bevy plugin.
//! Game logic that doesn't need the engine lives in plain-Rust submodules
//! so it can be unit-tested and benchmarked without a running `App`.

use bevy::prelude::*;
use bevy::window::WindowResolution;

mod background;
mod camera;
mod config;
pub mod debug;

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
            config::ConfigPlugin,
            camera::CameraPlugin,
            background::BackgroundPlugin,
            debug::DebugPlugin,
        ))
        .run()
}

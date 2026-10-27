//! Stardrop: a calm, dreamy falling game about discovery.
//!
//! Each feature area is its own module exposing one Bevy plugin.
//! Game logic that doesn't need the engine lives in plain-Rust submodules
//! so it can be unit-tested and benchmarked without a running `App`.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowResolution};

mod app_state;
pub mod astronaut;
mod attempt;
mod audio;
mod background;
pub mod camera;
mod collectibles;
pub mod collision;
pub mod config;
pub mod debug;
mod effects;
mod ending;
mod journal;
pub mod level;
mod menu;
pub mod music;
mod note_signs;
pub mod objects;
mod particles;
pub mod physics;
mod rng;
pub mod save;
pub mod scoring;
pub mod settings;
pub mod sim;
#[cfg(test)]
mod soak_tests;
mod soft_mesh;
pub mod solver;
pub mod synth;
mod theme;
mod ui;

/// Window size on first launch, in physical pixels: portrait, like the
/// playfield. `settings.rs` shrinks it on the first frame if the monitor is
/// smaller, and restores the size the player left behind.
const INITIAL_WINDOW_SIZE: (u32, u32) = (675, 1200);

/// Builds and runs the game until the window is closed.
#[must_use]
pub fn run() -> AppExit {
    let mut app = App::new();
    // Shipped builds carry their assets inside the executable, so the game is
    // a single file. Files that aren't embedded (a level someone drops next
    // to the binary) still load from an `assets/` folder beside it.
    // With `--features dev` this is skipped, keeping hot reload working.
    #[cfg(not(feature = "dev"))]
    app.add_plugins(bevy_embedded_assets::EmbeddedAssetPlugin {
        mode: bevy_embedded_assets::PluginMode::ReplaceAndFallback {
            path: "assets".to_owned(),
        },
    });
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Stardrop".into(),
            resolution: WindowResolution::from(INITIAL_WINDOW_SIZE),
            position: WindowPosition::Centered(MonitorSelection::Primary),
            // In a browser there is no window to open: the game draws into
            // one canvas on the page, and follows the size the page gives it.
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#stardrop".to_owned()),
            #[cfg(target_arch = "wasm32")]
            fit_canvas_to_parent: true,
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
        particles::ParticlesPlugin,
        note_signs::NoteSignsPlugin,
        audio::GameAudioPlugin,
        camera::CameraPlugin,
        background::BackgroundPlugin,
        debug::DebugPlugin,
    ))
    // A second group: Bevy's `add_plugins` takes at most 15 at a time.
    .add_plugins((
        journal::JournalPlugin,
        save::SavePlugin,
        settings::SettingsPlugin,
        menu::MenuPlugin,
        ending::EndingPlugin,
        ui::UiPlugin,
    ))
    .run()
}

//! The game camera: 2D, HDR with bloom, fixed on the portrait playfield.
//!
//! Every level is one screen, so the camera never moves. It always shows the
//! whole playfield, centered; windows of other shapes show more scenery
//! around it and never cut gameplay off.

use bevy::camera::{Hdr, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::app_state::AppState;
use crate::background::CurrentTheme;
use crate::journal::JOURNAL_ORIGIN;
use crate::level::{LEVEL_HEIGHT, LEVEL_WIDTH};
use crate::settings::{Quality, Settings};

/// The playfield, in world units: what must always be visible.
pub(crate) const PLAYFIELD: Vec2 = Vec2::new(LEVEL_WIDTH, LEVEL_HEIGHT);

/// The playfield's center in world coordinates (the top edge is y = 0).
pub(crate) const PLAYFIELD_CENTER: Vec2 = Vec2::new(0.0, -LEVEL_HEIGHT / 2.0);

// Bloom strength is per theme (`theme.rs`): Bevy's `NATURAL` preset (0.15) is
// too subtle for our glowing look, and hazier skies want more than clear ones.

/// Marks the one gameplay camera.
#[derive(Component)]
pub(crate) struct MainCamera;

/// Spawns the fixed main camera.
pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            // Where the camera looks depends only on the current screen, so
            // it can't get out of step (and doesn't care what exists yet).
            .add_systems(Update, look_at_current_screen)
            .add_systems(
                Update,
                apply_glow.run_if(
                    resource_changed::<CurrentTheme>.or_eager(resource_changed::<Settings>),
                ),
            );
    }
}

/// The glow the game uses at "full" quality, at this theme's strength.
pub fn bloom(intensity: f32) -> Bloom {
    Bloom {
        intensity,
        ..Bloom::NATURAL
    }
}

/// Glow follows the sky, and switches off entirely at "plain" quality, where
/// bloom is the expensive part of our look.
fn apply_glow(
    theme: Res<CurrentTheme>,
    settings: Res<Settings>,
    mut commands: Commands,
    cameras: Query<Entity, With<Camera2d>>,
) {
    for camera in &cameras {
        let mut entity = commands.entity(camera);
        match settings.quality {
            Quality::Full => {
                entity.insert(bloom(crate::theme::art(**theme).bloom));
            }
            Quality::Plain => {
                entity.remove::<Bloom>();
            }
        }
    }
}

/// The journal lives elsewhere in the world; every other screen shows the
/// playfield. Writes only when the target actually changes.
fn look_at_current_screen(
    state: Res<State<AppState>>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
) {
    let target = if *state.get() == AppState::Journal {
        JOURNAL_ORIGIN
    } else {
        PLAYFIELD_CENTER
    };
    let wanted = target.extend(camera.translation.z);
    if camera.translation != wanted {
        camera.translation = wanted;
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        Camera2d,
        Transform::from_translation(PLAYFIELD_CENTER.extend(0.0)),
        // Render into a high-dynamic-range texture so colors can be brighter
        // than "white"; bloom turns that extra brightness into a soft glow.
        Hdr,
        // Maps HDR colors back to the screen. TonyMcMapface desaturates very
        // bright colors gracefully instead of clipping them (see docs/DESIGN.md).
        Tonemapping::TonyMcMapface,
        bloom(crate::theme::art(crate::theme::MENU_THEME).bloom),
        Projection::Orthographic(OrthographicProjection {
            // Always show at least the whole playfield; extra space on the
            // longer side becomes scenery.
            scaling_mode: ScalingMode::AutoMin {
                min_width: PLAYFIELD.x,
                min_height: PLAYFIELD.y,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

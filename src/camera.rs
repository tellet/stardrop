//! The game camera: 2D, HDR, with bloom so bright things glow.

use bevy::camera::{Hdr, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

/// Size of the playfield in world units. The camera always shows at least this
/// much, so gameplay is never cut off; other aspect ratios reveal extra background.
pub(crate) const DESIGN_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);

/// Bloom strength. Bevy's `NATURAL` preset (0.15) is too subtle for our glowing
/// look; tuned per level from Milestone 7.
const BLOOM_INTENSITY: f32 = 0.3;

/// Marks the one gameplay camera.
#[derive(Component)]
pub(crate) struct MainCamera;

/// Spawns the main camera.
pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        Camera2d,
        // Render into a high-dynamic-range texture so colors can be brighter
        // than "white"; bloom turns that extra brightness into a soft glow.
        Hdr,
        // Maps HDR colors back to the screen. TonyMcMapface desaturates very
        // bright colors gracefully instead of clipping them (see DECISIONS.md).
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: BLOOM_INTENSITY,
            ..Bloom::NATURAL
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: DESIGN_SIZE.x,
                min_height: DESIGN_SIZE.y,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

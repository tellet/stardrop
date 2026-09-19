//! The game camera: 2D, HDR with bloom, fixed on the portrait playfield.
//!
//! Every level is one screen, so the camera never moves. It always shows the
//! whole playfield, centered; windows of other shapes show more scenery
//! around it and never cut gameplay off.

use bevy::camera::{Hdr, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::level::{LEVEL_HEIGHT, LEVEL_WIDTH};

/// The playfield, in world units: what must always be visible.
pub(crate) const PLAYFIELD: Vec2 = Vec2::new(LEVEL_WIDTH, LEVEL_HEIGHT);

/// The playfield's center in world coordinates (the top edge is y = 0).
pub(crate) const PLAYFIELD_CENTER: Vec2 = Vec2::new(0.0, -LEVEL_HEIGHT / 2.0);

/// Bloom strength. Bevy's `NATURAL` preset (0.15) is too subtle for our glowing
/// look; tuned per level from Milestone 7.
const BLOOM_INTENSITY: f32 = 0.3;

/// Marks the one gameplay camera.
#[derive(Component)]
pub(crate) struct MainCamera;

/// Spawns the fixed main camera.
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
        Transform::from_translation(PLAYFIELD_CENTER.extend(0.0)),
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

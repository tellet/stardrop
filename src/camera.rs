//! The game camera: 2D, HDR with bloom, following the fall.
//!
//! The camera only moves vertically. It keeps the astronaut in the upper
//! third of the screen (plus a little extra look-ahead at speed) so the player
//! sees what's coming, and the full level width is always visible.

use bevy::camera::{Hdr, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::level::LevelBounds;
use crate::physics::Motion;

/// Size of the playfield in world units. The camera always shows at least this
/// much, so gameplay is never cut off; other aspect ratios reveal extra background.
pub(crate) const DESIGN_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);

/// Bloom strength. Bevy's `NATURAL` preset (0.15) is too subtle for our glowing
/// look; tuned per level from Milestone 7.
const BLOOM_INTENSITY: f32 = 0.3;

/// The astronaut sits this far above the screen center: center minus one
/// sixth of the height puts them on the upper-third line.
const UPPER_THIRD_OFFSET: f32 = DESIGN_SIZE.y / 6.0;
/// Extra look-ahead per unit of fall speed, in seconds: at terminal velocity
/// (450 units/s) the view reaches ~110 units further down.
const LOOK_AHEAD_SECONDS: f32 = 0.25;
/// How quickly the camera catches up (exponential decay rate, 1/s). Higher is
/// snappier; ~5 feels calm without ever losing the astronaut.
const FOLLOW_RATE: f32 = 5.0;
/// Jumps larger than this (e.g. restarting at the top) cut instead of pan.
const SNAP_DISTANCE: f32 = DESIGN_SIZE.y;

/// Marks the one gameplay camera.
#[derive(Component)]
pub(crate) struct MainCamera;

/// Ordering labels for systems that depend on the camera's position.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum CameraSystems {
    /// Moves the camera for this frame.
    Follow,
}

/// Spawns the main camera and makes it follow the astronaut.
pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, follow_astronaut.in_set(CameraSystems::Follow));
    }
}

fn spawn_camera(mut commands: Commands, level: Res<LevelBounds>) {
    let start_y = camera_target(level.start.y, 0.0, &level);
    commands.spawn((
        MainCamera,
        Camera2d,
        Transform::from_xyz(0.0, start_y, 0.0),
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

/// Where the camera center should be for an astronaut at `astronaut_y` falling
/// at `fall_speed` (positive = downward), kept inside the level.
fn camera_target(astronaut_y: f32, fall_speed: f32, level: &LevelBounds) -> f32 {
    let look_ahead = fall_speed.max(0.0) * LOOK_AHEAD_SECONDS;
    let target = astronaut_y - UPPER_THIRD_OFFSET - look_ahead;
    let half_view = DESIGN_SIZE.y / 2.0;
    // Don't show sky above the top edge or below the bottom edge.
    target.clamp(level.bottom() + half_view, -half_view)
}

/// Runs every frame (after physics interpolation), using the *drawn* astronaut
/// position so the camera moves as smoothly as the astronaut does.
fn follow_astronaut(
    time: Res<Time>,
    level: Res<LevelBounds>,
    astronaut: Single<(&Transform, &Motion), (With<Astronaut>, Without<MainCamera>)>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
) {
    let (astronaut_transform, motion) = *astronaut;
    let target = camera_target(
        astronaut_transform.translation.y,
        -motion.body().velocity.y,
        &level,
    );
    let y = &mut camera.translation.y;
    if (target - *y).abs() > SNAP_DISTANCE {
        *y = target;
    } else {
        // Frame-rate independent smoothing: the same feel at 60 or 144 Hz.
        y.smooth_nudge(&target, FOLLOW_RATE, time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL: LevelBounds = LevelBounds {
        width: DESIGN_SIZE.x,
        height: DESIGN_SIZE.y * 10.0,
        start: Vec2::ZERO,
    };

    #[test]
    fn astronaut_sits_on_the_upper_third_line_at_rest() {
        let astronaut_y = -5000.0;
        let center = camera_target(astronaut_y, 0.0, &LEVEL);
        let screen_top = center + DESIGN_SIZE.y / 2.0;
        let from_top = screen_top - astronaut_y;
        assert!((from_top - DESIGN_SIZE.y / 3.0).abs() < 1e-3);
    }

    #[test]
    fn falling_faster_shows_more_below() {
        let slow = camera_target(-5000.0, 0.0, &LEVEL);
        let fast = camera_target(-5000.0, 450.0, &LEVEL);
        assert!(fast < slow);
    }

    #[test]
    fn view_stays_inside_the_level() {
        let half = DESIGN_SIZE.y / 2.0;
        assert!(camera_target(0.0, 0.0, &LEVEL) <= -half);
        assert!(camera_target(LEVEL.bottom() - 500.0, 450.0, &LEVEL) >= LEVEL.bottom() + half);
    }
}

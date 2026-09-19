//! All gameplay tuning numbers, in one place.
//!
//! This is where the consistency rule lives: every behavior strength is
//! defined here once, never per level or per instance.

use bevy::prelude::*;

use core::f32::consts::FRAC_PI_4;

use crate::objects::{CloudPuffConfig, CrystalConfig, ObjectsConfig, StaticCloudConfig};
use crate::physics::movement::MovementConfig;

/// Falling and drifting feel (world units, seconds). See DECISIONS.md M1-2
/// for the reasoning behind each value.
const MOVEMENT: MovementConfig = MovementConfig {
    // ~1 s from rest to full fall speed: a gentle start after respawning.
    gravity: 450.0,
    // With ~720 units visible below the astronaut, ~1.6 s to see what's coming.
    terminal_velocity: 450.0,
    // Crossing the 1920-wide level takes ~4 s: floaty, not twitchy.
    max_air_speed: 500.0,
    // Full sideways speed in 0.25 s.
    air_acceleration: 2000.0,
    // Glides to a stop in ~0.4 s.
    air_braking: 1250.0,
    // Walls bounce you back at 60 % of your speed (we're in space)...
    wall_restitution: 0.6,
    // ...and steering fades back in over 0.5 s, so the rebound (~85–100
    // units) stays visible even while pushing toward the wall.
    control_recovery_time: 0.5,
    // After a bounce the astronaut coasts, slowing only gently: a full-speed
    // bounce drifts ~450 units over ~3 s unless the player steers.
    coast_drag: 100.0,
};

/// The astronaut's collision radius, in world units. Also used by level
/// validation (nothing may overlap the astronaut's start).
pub(crate) const ASTRONAUT_RADIUS: f32 = 28.0;

/// Object behavior, one set per type (Section 3.3: never per level or instance).
const OBJECTS: ObjectsConfig = ObjectsConfig {
    cloud_puff: CloudPuffConfig {
        // Rises about a quarter of the screen (650² / 2g ≈ 470 units).
        bounce_speed: 650.0,
        // Always at least 45° upward: "up and away", never sideways.
        max_tilt: FRAC_PI_4,
    },
    crystal: CrystalConfig {
        // Clearly visible even when the astronaut was barely moving sideways.
        min_sideways_speed: 350.0,
    },
    static_cloud: StaticCloudConfig {
        // Hazards are more forgiving than they look: the hitbox is 80 % of
        // the visible cloud, so near misses feel fair, never cheap.
        hitbox_scale: 0.8,
    },
};

/// Global gameplay tuning.
#[derive(Resource, Debug, Clone)]
pub(crate) struct GameConfig {
    /// Gravity, terminal velocity and air control.
    pub movement: MovementConfig,
    /// The astronaut is a circle of this radius.
    pub astronaut_radius: f32,
    /// Every object type's behavior.
    pub objects: ObjectsConfig,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            movement: MOVEMENT,
            astronaut_radius: ASTRONAUT_RADIUS,
            objects: OBJECTS,
        }
    }
}

/// Registers [`GameConfig`] as a resource.
pub(crate) struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameConfig>();
    }
}

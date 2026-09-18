//! All gameplay tuning numbers, in one place.
//!
//! This is where the consistency rule lives: every behavior strength is
//! defined here once, never per level or per instance.

use bevy::prelude::*;

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
    // Walls give back 30 % of the speed: a small, soft bump.
    wall_restitution: 0.3,
};

/// The astronaut's collision radius, in world units.
const ASTRONAUT_RADIUS: f32 = 28.0;

/// Global gameplay tuning.
#[derive(Resource, Debug, Clone)]
pub(crate) struct GameConfig {
    /// Gravity, terminal velocity and air control.
    pub movement: MovementConfig,
    /// The astronaut is a circle of this radius.
    pub astronaut_radius: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            movement: MOVEMENT,
            astronaut_radius: ASTRONAUT_RADIUS,
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

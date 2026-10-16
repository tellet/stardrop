//! All gameplay tuning numbers, in one place.
//!
//! This is where the consistency rule lives: every behavior strength is
//! defined here once, never per level or per instance.

use bevy::prelude::*;

use core::f32::consts::FRAC_PI_4;

use crate::objects::{
    BubbleConfig, CloudPuffConfig, CometStoneConfig, CrystalConfig, GravityOrbConfig,
    ObjectsConfig, SporeFlowerConfig, StaticCloudConfig, TwinPortalsConfig, WindRibbonConfig,
};
use crate::physics::movement::MovementConfig;

/// Falling and drifting feel (world units, seconds). See docs/DESIGN.md M1-2
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
    wind_ribbon: WindRibbonConfig {
        // Carries the astronaut at its own top sideways speed.
        flow_speed: 500.0,
        // Four times gravity: ribbons can carry you upward, and you can still
        // steer a little inside them.
        pull: 1800.0,
    },
    crystal: CrystalConfig {
        // Clearly visible even when the astronaut was barely moving sideways.
        min_sideways_speed: 350.0,
    },
    twin_portals: TwinPortalsConfig {
        // Far enough out not to land back inside the exit.
        exit_clearance: 40.0,
        // Long enough that you can't bounce straight back (Section 3.4).
        cooldown: 0.5,
    },
    gravity_orb: GravityOrbConfig {
        // The pull reaches four core radii out, and the field is drawn.
        field_scale: 4.0,
        // Acceleration = pull / distance: 1600 u/s² a core away, 400 at the
        // field's edge, so orbits at different radii feel similar.
        pull: 160_000.0,
        max_pull: 2000.0,
        // "Never produce extreme speeds" (Section 3.4).
        max_speed: 700.0,
        // A soft push back out when touching the core.
        core_bounce_speed: 420.0,
    },
    bubble: BubbleConfig {
        // A slow, cozy float upward...
        rise_speed: 90.0,
        // ...for long enough to cross about an eighth of the screen.
        seconds: 2.5,
        // Gentle steering inside, far below normal air control.
        nudge_speed: 120.0,
    },
    spore_flower: SporeFlowerConfig {
        // Long enough to matter, short enough not to feel punishing.
        swap_seconds: 4.0,
    },
    comet_stone: CometStoneConfig {
        // 2.4x terminal velocity: a real shortcut, and dangerous near hazards.
        dive_speed: 1100.0,
        // Held briefly, then eased back by the usual terminal-velocity rule.
        dive_seconds: 0.6,
    },
    static_cloud: StaticCloudConfig {
        // Hazards are more forgiving than they look: the hitbox is 80 % of
        // the visible cloud, so near misses feel fair, never cheap.
        hitbox_scale: 0.8,
    },
};

/// Global gameplay tuning. Public so benchmarks measure the real values
/// instead of a copy that could drift out of step.
#[derive(Resource, Debug, Clone)]
pub struct GameConfig {
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
pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameConfig>();
    }
}

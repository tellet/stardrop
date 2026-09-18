//! The level being played. For now a hard-coded test level; from Milestone 2
//! this is loaded from a RON file.
//!
//! Level coordinates: x = 0 is the center, the top edge is y = 0 and the
//! level extends downward to y = -height.

use bevy::prelude::*;

use crate::camera::DESIGN_SIZE;
use crate::config::GameConfig;
use crate::physics::movement::{Body, Walls};
use crate::physics::{Motion, PhysicsSystems};

/// Test level: ten screen-heights of open sky.
const TEST_LEVEL_HEIGHT: f32 = DESIGN_SIZE.y * 10.0;
/// Start a little below the top edge.
const TEST_LEVEL_START: Vec2 = Vec2::new(0.0, -300.0);

/// Size and start point of the current level.
#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct LevelBounds {
    /// Width in world units; always fully visible.
    pub width: f32,
    /// Height in world units, from y = 0 down to y = -height.
    pub height: f32,
    /// Where the astronaut starts, at rest.
    pub start: Vec2,
}

impl LevelBounds {
    /// Sideways limits for a circle of `radius`.
    pub fn walls(&self, radius: f32) -> Walls {
        let half = self.width / 2.0 - radius;
        Walls {
            left: -half,
            right: half,
        }
    }

    /// Lowest y of the level.
    pub fn bottom(&self) -> f32 {
        -self.height
    }
}

/// Provides [`LevelBounds`] and handles falling out of the level.
pub(crate) struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LevelBounds {
            width: DESIGN_SIZE.x,
            height: TEST_LEVEL_HEIGHT,
            start: TEST_LEVEL_START,
        })
        .add_systems(
            FixedUpdate,
            restart_below_bottom.after(PhysicsSystems::Step),
        );
    }
}

/// Placeholder until checkpoints exist (Milestone 2): falling past the bottom
/// starts the fall again from the top.
fn restart_below_bottom(
    level: Res<LevelBounds>,
    config: Res<GameConfig>,
    mut bodies: Query<&mut Motion>,
) {
    for mut motion in &mut bodies {
        if motion.body().position.y < level.bottom() - config.astronaut_radius {
            motion.teleport(Body {
                position: level.start,
                velocity: Vec2::ZERO,
            });
        }
    }
}

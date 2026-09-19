//! Reaching the beacon completes the level.

use bevy::prelude::*;

use super::{ActiveLevel, LevelPhase};
use crate::astronaut::Astronaut;
use crate::collision::circle_overlaps_rect;
use crate::config::GameConfig;
use crate::physics::Motion;
use crate::physics::movement::Body;

/// Sent once when the astronaut lands on the beacon.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct LevelCompleted;

/// Checks for touching the beacon after each physics step.
pub(super) fn land_on_beacon(
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut completed: MessageWriter<LevelCompleted>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    let radius = config.astronaut_radius;
    let beacon = level.beacon;
    let position = astronaut.body().position;
    if !circle_overlaps_rect(position, radius, beacon.center, beacon.size) {
        return;
    }
    // Come to rest standing on top of the pad.
    let pad_top = beacon.center.y + beacon.size.y / 2.0;
    astronaut.teleport(Body::at_rest(Vec2::new(position.x, pad_top + radius)));
    completed.write(LevelCompleted);
    next_phase.set(LevelPhase::Landed);
}

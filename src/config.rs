//! All gameplay tuning numbers, in one place.
//!
//! This is where the consistency rule lives: every object type's behavior
//! strength is defined here once, never per level or per instance.

use bevy::prelude::*;

/// Global gameplay tuning. Empty for now; movement values arrive in Milestone 1
/// and object behavior parameters in Milestones 3–4.
#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct GameConfig {}

/// Registers [`GameConfig`] as a resource.
pub(crate) struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameConfig>();
    }
}

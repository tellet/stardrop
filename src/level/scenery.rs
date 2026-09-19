//! Placeholder visuals for a level's beacon, checkpoint rings and stardust.
//! Final art arrives in Milestone 7.
//!
//! Meshes and materials are created once and shared by every level entity.
//! Reloading or switching levels therefore never creates new assets, so asset
//! memory can't grow.

use bevy::prelude::*;

use super::data::Level;

/// Checkpoint ring size, in world units.
const RING_RADIUS: f32 = 70.0;
const RING_THICKNESS: f32 = 6.0;
/// Stardust mote radius, in world units (also its pickup radius).
pub(crate) const MOTE_RADIUS: f32 = 10.0;

/// HDR colors (channels above 1.0 glow through bloom).
const BEACON_GLOW: Color = Color::linear_rgb(0.6, 2.4, 2.0);
const RING_GLOW: Color = Color::linear_rgba(0.5, 1.3, 1.6, 0.8);
/// An activated checkpoint glows brighter and warmer.
const RING_ACTIVE_GLOW: Color = Color::linear_rgb(1.6, 3.0, 2.2);
const STARDUST_GLOW: Color = Color::linear_rgb(3.0, 2.3, 0.7);

/// In front of the background, behind the astronaut.
const SCENERY_DEPTH: f32 = 5.0;

/// Everything spawned for the current level; despawned when it reloads.
#[derive(Component)]
pub(crate) struct LevelEntity;

/// A stardust mote, by its index in the level's stardust list.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct StardustMote(pub usize);

/// A checkpoint ring, by its index in the level's checkpoint list.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct CheckpointRing(pub usize);

/// Shared mesh and material handles for level scenery.
#[derive(Resource)]
pub(crate) struct SceneryAssets {
    /// A 1×1 square, scaled to each beacon's size.
    unit_square: Handle<Mesh>,
    ring: Handle<Mesh>,
    mote: Handle<Mesh>,
    beacon_material: Handle<ColorMaterial>,
    ring_material: Handle<ColorMaterial>,
    ring_active_material: Handle<ColorMaterial>,
    mote_material: Handle<ColorMaterial>,
}

impl SceneryAssets {
    /// The ring look for an inactive or activated checkpoint.
    pub(crate) fn ring_material(&self, active: bool) -> Handle<ColorMaterial> {
        if active {
            self.ring_active_material.clone()
        } else {
            self.ring_material.clone()
        }
    }
}

impl FromWorld for SceneryAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_square = meshes.add(Rectangle::new(1.0, 1.0));
        let ring = meshes.add(Annulus::new(RING_RADIUS - RING_THICKNESS, RING_RADIUS));
        let mote = meshes.add(Circle::new(MOTE_RADIUS));
        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
        Self {
            unit_square,
            ring,
            mote,
            beacon_material: materials.add(BEACON_GLOW),
            ring_material: materials.add(RING_GLOW),
            ring_active_material: materials.add(RING_ACTIVE_GLOW),
            mote_material: materials.add(STARDUST_GLOW),
        }
    }
}

/// Spawns the beacon, rings and stardust for `level`.
pub(super) fn spawn(commands: &mut Commands, assets: &SceneryAssets, level: &Level) {
    let beacon = level.beacon;
    commands.spawn((
        LevelEntity,
        Mesh2d(assets.unit_square.clone()),
        MeshMaterial2d(assets.beacon_material.clone()),
        Transform::from_translation(beacon.center.extend(SCENERY_DEPTH))
            .with_scale(beacon.size.extend(1.0)),
    ));
    for (index, &center) in level.checkpoints.iter().enumerate() {
        commands.spawn((
            LevelEntity,
            CheckpointRing(index),
            Mesh2d(assets.ring.clone()),
            MeshMaterial2d(assets.ring_material.clone()),
            Transform::from_translation(center.extend(SCENERY_DEPTH)),
        ));
    }
    for (index, &position) in level.stardust.iter().enumerate() {
        commands.spawn((
            LevelEntity,
            StardustMote(index),
            Mesh2d(assets.mote.clone()),
            MeshMaterial2d(assets.mote_material.clone()),
            Transform::from_translation(position.extend(SCENERY_DEPTH)),
        ));
    }
}

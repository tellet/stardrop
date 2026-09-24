//! What a level's beacon and stardust look like: the two things the player is
//! aiming at, so both glow and both move a little (`effects.rs` animates them).
//!
//! Meshes and materials are created once and shared by every level entity.
//! Reloading or switching levels therefore never creates new assets, so asset
//! memory can't grow.

use bevy::prelude::*;

use super::data::Level;

/// Stardust mote radius, in world units (also its pickup radius).
pub(crate) const MOTE_RADIUS: f32 = 10.0;

/// HDR colors (channels above 1.0 glow through bloom).
const BEACON_GLOW: Color = Color::linear_rgb(0.6, 2.4, 2.0);
const STARDUST_GLOW: Color = Color::linear_rgb(3.0, 2.3, 0.7);
/// The light each one casts around itself: dimmer, wider, and see-through.
const BEACON_HALO: Color = Color::linear_rgba(0.3, 1.6, 1.4, 0.30);
const STARDUST_HALO: Color = Color::linear_rgba(2.0, 1.5, 0.5, 0.35);
/// Halo sizes, as a multiple of what they surround.
const BEACON_HALO_SIZE: Vec2 = Vec2::new(1.5, 7.0);
const MOTE_HALO_SIZE: f32 = 3.4;

/// In front of the background, behind the astronaut.
const SCENERY_DEPTH: f32 = 5.0;

/// Everything spawned for the current level; despawned when it reloads.
#[derive(Component)]
pub(crate) struct LevelEntity;

/// A stardust mote, by its index in the level's stardust list.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct StardustMote(pub usize);

/// A mote's own place in the twinkle, so a row of them never blinks in step.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Twinkle(pub f32);

/// The soft light around the beacon, which breathes slowly. Carries its own
/// size, so the breath scales around it instead of drifting.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct BeaconHalo(pub Vec2);

/// Shared mesh and material handles for level scenery.
#[derive(Resource)]
pub(super) struct SceneryAssets {
    /// A 1×1 square, scaled to each beacon's size.
    unit_square: Handle<Mesh>,
    mote: Handle<Mesh>,
    /// A soft disc of radius 1, for both halos.
    halo: Handle<Mesh>,
    beacon_material: Handle<ColorMaterial>,
    mote_material: Handle<ColorMaterial>,
    beacon_halo: Handle<ColorMaterial>,
    mote_halo: Handle<ColorMaterial>,
}

impl FromWorld for SceneryAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_square = meshes.add(Rectangle::new(1.0, 1.0));
        let mote = meshes.add(Circle::new(MOTE_RADIUS));
        let halo = meshes.add(crate::soft_mesh::unit_disc(20));
        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
        Self {
            unit_square,
            mote,
            halo,
            beacon_material: materials.add(BEACON_GLOW),
            mote_material: materials.add(STARDUST_GLOW),
            beacon_halo: materials.add(BEACON_HALO),
            mote_halo: materials.add(STARDUST_HALO),
        }
    }
}

/// Spawns the beacon and stardust for `level`.
pub(super) fn spawn(commands: &mut Commands, assets: &SceneryAssets, level: &Level) {
    let beacon = level.beacon;
    commands
        .spawn((
            LevelEntity,
            Transform::from_translation(beacon.center.extend(SCENERY_DEPTH)),
            Visibility::default(),
        ))
        // The light above the pad: a tall soft glow the player can aim at
        // from the top of the screen.
        .with_child((
            BeaconHalo(beacon.size * BEACON_HALO_SIZE),
            Mesh2d(assets.halo.clone()),
            MeshMaterial2d(assets.beacon_halo.clone()),
            Transform::from_xyz(0.0, 0.0, -1.0)
                .with_scale((beacon.size * BEACON_HALO_SIZE).extend(1.0)),
        ))
        .with_child((
            Mesh2d(assets.unit_square.clone()),
            MeshMaterial2d(assets.beacon_material.clone()),
            Transform::from_scale(beacon.size.extend(1.0)),
        ));
    for (index, &position) in level.stardust.iter().enumerate() {
        commands
            .spawn((
                LevelEntity,
                StardustMote(index),
                // Spread through the cycle, so they twinkle one after another.
                #[expect(clippy::cast_precision_loss, reason = "at most 64 motes")]
                Twinkle(index as f32 * 0.9),
                Mesh2d(assets.mote.clone()),
                MeshMaterial2d(assets.mote_material.clone()),
                Transform::from_translation(position.extend(SCENERY_DEPTH)),
            ))
            .with_child((
                Mesh2d(assets.halo.clone()),
                MeshMaterial2d(assets.mote_halo.clone()),
                Transform::from_xyz(0.0, 0.0, -1.0)
                    .with_scale(Vec3::splat(MOTE_RADIUS * MOTE_HALO_SIZE)),
            ));
    }
}

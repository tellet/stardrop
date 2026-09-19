//! Placeholder looks for objects: distinct silhouettes and colors per type.
//! Final art arrives in Milestone 7.
//!
//! Meshes are unit-sized (radius 1) and scaled per object, and every object
//! of a type shares them, so reloading a level never creates new assets.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::kind::Placement;

/// Cloud puffs: three overlapping soft circles (offsets and sizes relative to
/// the radius). Pastel, not glowing: clouds are soft, not light sources.
const PUFF_LOBES: [(Vec2, f32); 3] = [
    (Vec2::new(-0.35, -0.08), 0.62),
    (Vec2::new(0.35, -0.08), 0.62),
    (Vec2::new(0.0, 0.2), 0.72),
];
const PUFF_COLOR: Color = Color::srgb(0.96, 0.86, 0.95);
/// Crystals: a tall diamond with a cool glow (slightly above 1.0: a hint of bloom).
const CRYSTAL_GLOW: Color = Color::linear_rgb(0.55, 1.5, 1.9);
/// Spikes around a static cloud's outline.
const STATIC_SPIKES: u16 = 14;
/// Dark and brooding, with a faint violet crackle around it (no bloom).
const STATIC_CORE: Color = Color::srgb(0.10, 0.07, 0.16);
const STATIC_CRACKLE: Color = Color::srgb(0.42, 0.30, 0.62);

/// Objects sit in front of the background and scenery, behind the astronaut.
const OBJECT_DEPTH: f32 = 6.0;

/// Shared mesh and material handles for every object type.
#[derive(Resource)]
pub(super) struct ObjectAssets {
    unit_circle: Handle<Mesh>,
    diamond: Handle<Mesh>,
    spiky_star: Handle<Mesh>,
    puff: Handle<ColorMaterial>,
    crystal: Handle<ColorMaterial>,
    static_core: Handle<ColorMaterial>,
    static_crackle: Handle<ColorMaterial>,
}

impl FromWorld for ObjectAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_circle = meshes.add(Circle::new(1.0));
        let spiky_star = meshes.add(star_mesh(STATIC_SPIKES, 0.82, 1.0));
        // Narrower than tall, tips at distance 1 from the center.
        let diamond = meshes.add(Rhombus::new(1.3, 2.0));
        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
        Self {
            unit_circle,
            diamond,
            spiky_star,
            puff: materials.add(PUFF_COLOR),
            crystal: materials.add(CRYSTAL_GLOW),
            static_core: materials.add(STATIC_CORE),
            static_crackle: materials.add(STATIC_CRACKLE),
        }
    }
}

/// Spawns the visible parts of `placement` as children of `parent`.
pub(super) fn spawn_look(
    parent: &mut ChildSpawnerCommands,
    assets: &ObjectAssets,
    placement: &Placement,
) {
    match placement {
        Placement::CloudPuff { radius, .. } => {
            for (offset, scale) in PUFF_LOBES {
                parent.spawn((
                    Mesh2d(assets.unit_circle.clone()),
                    MeshMaterial2d(assets.puff.clone()),
                    Transform::from_translation((offset * *radius).extend(0.0))
                        .with_scale(Vec3::splat(radius * scale)),
                ));
            }
        }
        Placement::Crystal { size, .. } => {
            parent.spawn((
                Mesh2d(assets.diamond.clone()),
                MeshMaterial2d(assets.crystal.clone()),
                Transform::from_scale(Vec3::new(*size, *size, 1.0)),
            ));
        }
        Placement::StaticCloud { radius, .. } => {
            let size = Vec3::new(*radius, *radius, 1.0);
            parent.spawn((
                Mesh2d(assets.spiky_star.clone()),
                MeshMaterial2d(assets.static_crackle.clone()),
                Transform::from_scale(size),
            ));
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.static_core.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(size * 0.8),
            ));
        }
    }
}

/// The entity transform for an object (its children carry the shapes).
pub(super) fn transform_for(placement: &Placement) -> Transform {
    Transform::from_translation(placement.center().extend(OBJECT_DEPTH))
}

/// A star-shaped outline with `points` spikes, alternating between `inner`
/// and `outer` radius: a triangle fan around the center.
fn star_mesh(points: u16, inner: f32, outer: f32) -> Mesh {
    let corners = points * 2;
    let mut positions = Vec::with_capacity(usize::from(corners) + 1);
    positions.push([0.0, 0.0, 0.0]);
    for i in 0..corners {
        let angle = f32::from(i) / f32::from(corners) * core::f32::consts::TAU;
        let radius = if i % 2 == 0 { outer } else { inner };
        positions.push([radius * angle.cos(), radius * angle.sin(), 0.0]);
    }
    let mut indices = Vec::with_capacity(usize::from(corners) * 3);
    for i in 0..corners {
        let next = (i + 1) % corners;
        indices.extend([0, i + 1, next + 1]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U16(indices))
}

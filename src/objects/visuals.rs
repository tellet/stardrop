//! What each object looks like: a distinct silhouette and colour per type,
//! readable by shape alone (Section 5's accessibility rule).
//!
//! Meshes are unit-sized (radius 1) and scaled per object, and every object
//! of a type shares them. Wind ribbons are the exception: each one's mesh
//! follows its own path. Their meshes are freed automatically when the
//! ribbon is despawned (the last handle to an asset drops it), so reloads
//! still don't grow memory.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::kind::{ObjectKind, ObjectsConfig, Placement};
use crate::effects::FlowMark;

/// Cloud puffs: three overlapping soft circles (offsets and sizes relative to
/// the radius). Pastel, not glowing: clouds are soft, not light sources.
const PUFF_LOBES: [(Vec2, f32); 3] = [
    (Vec2::new(-0.35, -0.08), 0.62),
    (Vec2::new(0.35, -0.08), 0.62),
    (Vec2::new(0.0, 0.2), 0.72),
];
const PUFF_COLOR: Color = Color::srgb(0.96, 0.86, 0.95);
/// Wind ribbons: a translucent glowing strip, with brighter chevrons every
/// `CHEVRON_SPACING` units pointing along the flow.
const RIBBON_GLOW: Color = Color::linear_rgba(0.35, 1.5, 1.4, 0.42);
const CHEVRON_GLOW: Color = Color::linear_rgba(0.6, 2.2, 1.9, 0.8);
/// How much of a ribbon's width is at full strength; the rest fades out to
/// its edges.
const RIBBON_CORE: f32 = 0.55;
const CHEVRON_SPACING: f32 = 115.0;
pub(crate) const CHEVRON_SIZE: f32 = 30.0;
/// Crystals: a tall diamond with a cool glow (slightly above 1.0: a hint of bloom).
const CRYSTAL_GLOW: Color = Color::linear_rgb(0.55, 1.5, 1.9);
/// Twin portals: two matching rings with a bright rim.
const PORTAL_RIM: Color = Color::linear_rgb(1.5, 0.9, 2.6);
const PORTAL_FILL: Color = Color::linear_rgba(0.5, 0.25, 1.0, 0.35);
/// Gravity orbs: a bright core with a faint ring showing how far the pull
/// reaches (the player has to be able to judge it).
const ORB_CORE: Color = Color::linear_rgb(1.9, 1.2, 2.8);
const ORB_FIELD: Color = Color::linear_rgba(0.7, 0.5, 1.4, 0.13);
const ORB_FIELD_EDGE: Color = Color::linear_rgba(1.2, 0.8, 2.0, 0.35);
/// Field edge thickness as a fraction of the field radius.
const ORB_EDGE: f32 = 0.02;
/// Bubbles: a faint glassy shell with a soft sheen.
const BUBBLE_SHELL: Color = Color::linear_rgba(0.9, 1.6, 2.0, 0.55);
const BUBBLE_FILL: Color = Color::linear_rgba(0.5, 0.9, 1.3, 0.16);
/// Shell thickness as a fraction of the radius.
const BUBBLE_RIM: f32 = 0.08;
/// Spore flowers: five petals around a bright core, in mischievous magenta.
const PETALS: u8 = 5;
const PETAL_COLOR: Color = Color::linear_rgb(1.4, 0.35, 1.1);
const FLOWER_CORE: Color = Color::linear_rgb(2.4, 1.9, 0.6);
/// Comet stones: a dark stone with a bright flame pointing down (where it throws you).
const STONE_COLOR: Color = Color::srgb(0.24, 0.20, 0.26);
const FLAME_COLOR: Color = Color::linear_rgb(3.0, 1.1, 0.35);
/// Spikes around a static cloud's outline.
const STATIC_SPIKES: u16 = 14;
/// Dark and brooding, with a faint violet crackle around it (no bloom).
const STATIC_CORE: Color = Color::srgb(0.10, 0.07, 0.16);
const STATIC_CRACKLE: Color = Color::srgb(0.42, 0.30, 0.62);

/// Objects sit in front of the background and scenery, behind the astronaut.
const OBJECT_DEPTH: f32 = 6.0;

/// Shared mesh and material handles for every object type.
#[derive(Resource)]
pub(crate) struct ObjectAssets {
    unit_circle: Handle<Mesh>,
    /// A ring of unit outer radius (the bubble's shell).
    unit_ring: Handle<Mesh>,
    /// A thinner ring of unit outer radius (an orb's field edge).
    thin_ring: Handle<Mesh>,
    /// A 1x1 triangle pointing down (the comet stone's flame).
    down_triangle: Handle<Mesh>,
    /// A unit arrow pointing along +x: one chevron of a wind ribbon.
    chevron_arrow: Handle<Mesh>,
    diamond: Handle<Mesh>,
    spiky_star: Handle<Mesh>,
    puff: Handle<ColorMaterial>,
    crystal: Handle<ColorMaterial>,
    ribbon: Handle<ColorMaterial>,
    chevron: Handle<ColorMaterial>,
    portal_rim: Handle<ColorMaterial>,
    portal_fill: Handle<ColorMaterial>,
    orb_core: Handle<ColorMaterial>,
    orb_field: Handle<ColorMaterial>,
    orb_field_edge: Handle<ColorMaterial>,
    bubble_shell: Handle<ColorMaterial>,
    bubble_fill: Handle<ColorMaterial>,
    petal: Handle<ColorMaterial>,
    flower_core: Handle<ColorMaterial>,
    stone: Handle<ColorMaterial>,
    flame: Handle<ColorMaterial>,
    static_core: Handle<ColorMaterial>,
    static_crackle: Handle<ColorMaterial>,
}

impl FromWorld for ObjectAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_circle = meshes.add(Circle::new(1.0));
        let unit_ring = meshes.add(Annulus::new(1.0 - BUBBLE_RIM, 1.0));
        let thin_ring = meshes.add(Annulus::new(1.0 - ORB_EDGE, 1.0));
        let spiky_star = meshes.add(star_mesh(STATIC_SPIKES, 0.82, 1.0));
        // Narrower than tall, tips at distance 1 from the center.
        let diamond = meshes.add(Rhombus::new(1.3, 2.0));
        let chevron_arrow = meshes.add(Triangle2d::new(
            Vec2::new(0.5, 0.0),
            Vec2::new(-0.5, 0.6),
            Vec2::new(-0.5, -0.6),
        ));
        let down_triangle = meshes.add(Triangle2d::new(
            Vec2::new(-0.5, 0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(0.0, -0.5),
        ));
        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
        Self {
            unit_circle,
            unit_ring,
            thin_ring,
            down_triangle,
            chevron_arrow,
            diamond,
            spiky_star,
            puff: materials.add(PUFF_COLOR),
            crystal: materials.add(CRYSTAL_GLOW),
            ribbon: materials.add(RIBBON_GLOW),
            chevron: materials.add(CHEVRON_GLOW),
            portal_rim: materials.add(PORTAL_RIM),
            portal_fill: materials.add(PORTAL_FILL),
            orb_core: materials.add(ORB_CORE),
            orb_field: materials.add(ORB_FIELD),
            orb_field_edge: materials.add(ORB_FIELD_EDGE),
            bubble_shell: materials.add(BUBBLE_SHELL),
            bubble_fill: materials.add(BUBBLE_FILL),
            petal: materials.add(PETAL_COLOR),
            flower_core: materials.add(FLOWER_CORE),
            stone: materials.add(STONE_COLOR),
            flame: materials.add(FLAME_COLOR),
            static_core: materials.add(STATIC_CORE),
            static_crackle: materials.add(STATIC_CRACKLE),
        }
    }
}

/// Spawns the visible parts of `placement` as children of `parent` (which
/// sits at `placement.center()`).
pub(super) fn spawn_look(
    parent: &mut ChildSpawnerCommands,
    assets: &ObjectAssets,
    meshes: &mut Assets<Mesh>,
    config: &ObjectsConfig,
    placement: &Placement,
) {
    match placement {
        Placement::TwinPortals { a, b, radius } => {
            // Children are placed relative to the pair's midpoint.
            let middle = placement.center();
            for end in [*a, *b] {
                let local = (end - middle).extend(0.0);
                parent.spawn((
                    Mesh2d(assets.unit_circle.clone()),
                    MeshMaterial2d(assets.portal_fill.clone()),
                    Transform::from_translation(local).with_scale(Vec3::splat(*radius)),
                ));
                parent.spawn((
                    Mesh2d(assets.unit_ring.clone()),
                    MeshMaterial2d(assets.portal_rim.clone()),
                    Transform::from_translation(local + Vec3::Z * 0.1)
                        .with_scale(Vec3::splat(*radius)),
                ));
            }
        }
        Placement::GravityOrb { radius, .. } => {
            let field = radius * config.gravity_orb.field_scale;
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.orb_field.clone()),
                Transform::from_scale(Vec3::splat(field)),
            ));
            parent.spawn((
                Mesh2d(assets.thin_ring.clone()),
                MeshMaterial2d(assets.orb_field_edge.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(field)),
            ));
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.orb_core.clone()),
                Transform::from_xyz(0.0, 0.0, 0.2).with_scale(Vec3::splat(*radius)),
            ));
        }
        Placement::WindRibbon { path, width } => {
            // Mesh points are relative to the parent, which sits at the center.
            let center = placement.center();
            let local: Vec<Vec2> = path.iter().map(|&point| point - center).collect();
            parent.spawn((
                Mesh2d(meshes.add(ribbon_mesh(&local, *width))),
                MeshMaterial2d(assets.ribbon.clone()),
            ));
            for (at, along, distance) in chevrons(&local) {
                parent.spawn((
                    FlowMark { distance },
                    Mesh2d(assets.chevron_arrow.clone()),
                    MeshMaterial2d(assets.chevron.clone()),
                    Transform::from_translation(at.extend(0.1))
                        .with_rotation(Quat::from_rotation_z(along.to_angle()))
                        .with_scale(Vec3::splat(CHEVRON_SIZE)),
                ));
            }
        }
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
        Placement::Bubble { radius, .. } => {
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.bubble_fill.clone()),
                Transform::from_scale(Vec3::splat(*radius)),
            ));
            parent.spawn((
                Mesh2d(assets.unit_ring.clone()),
                MeshMaterial2d(assets.bubble_shell.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(*radius)),
            ));
        }
        Placement::SporeFlower { radius, .. } => {
            for petal in 0..PETALS {
                let angle = f32::from(petal) / f32::from(PETALS) * core::f32::consts::TAU;
                let offset = Vec2::from_angle(angle) * (radius * 0.55);
                parent.spawn((
                    Mesh2d(assets.unit_circle.clone()),
                    MeshMaterial2d(assets.petal.clone()),
                    Transform::from_translation(offset.extend(0.0))
                        .with_scale(Vec3::splat(radius * 0.45)),
                ));
            }
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.flower_core.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(radius * 0.3)),
            ));
        }
        Placement::CometStone { radius, .. } => {
            // The flame points down: the direction the stone throws you.
            parent.spawn((
                Mesh2d(assets.down_triangle.clone()),
                MeshMaterial2d(assets.flame.clone()),
                Transform::from_xyz(0.0, -radius * 0.9, -0.1).with_scale(Vec3::new(
                    radius * 0.9,
                    radius * 1.2,
                    1.0,
                )),
            ));
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.stone.clone()),
                Transform::from_scale(Vec3::splat(*radius)),
            ));
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

/// A strip `width` wide along `path`: two vertices per point, offset along
/// the averaged normal of the neighboring segments so joints stay closed.
fn ribbon_mesh(path: &[Vec2], width: f32) -> Mesh {
    // Four lines of vertices across the strip: see-through at both edges,
    // full strength through the middle. A flat-edged band looks like a cut-out
    // plank; this one looks like flowing light.
    let across = [-0.5, -RIBBON_CORE * 0.5, RIBBON_CORE * 0.5, 0.5];
    let alphas = [0.0, 1.0, 1.0, 0.0];
    let mut positions = Vec::with_capacity(path.len() * across.len());
    let mut colors = Vec::with_capacity(path.len() * across.len());
    for (i, &point) in path.iter().enumerate() {
        let before = if i > 0 {
            point - path[i - 1]
        } else {
            Vec2::ZERO
        };
        let after = path.get(i + 1).map_or(Vec2::ZERO, |&next| next - point);
        let tangent =
            (before.normalize_or_zero() + after.normalize_or_zero()).normalize_or(Vec2::X);
        let side = tangent.perp() * width;
        for (offset, alpha) in across.iter().zip(alphas) {
            positions.push((point + side * *offset).extend(0.0).to_array());
            // White: the ribbon's own colour comes from its material, and
            // only the transparency varies across the strip.
            colors.push([1.0, 1.0, 1.0, alpha]);
        }
    }
    // Four lines across, written out so the index arithmetic stays honest.
    let lines: u16 = 4;
    debug_assert_eq!(usize::from(lines), across.len());
    let mut indices = Vec::with_capacity(path.len().saturating_sub(1) * 18);
    // `u16` counting: a ribbon has at most 16 points.
    for (segment, _) in (0_u16..).zip(path.windows(2)) {
        let start = segment * lines;
        for line in 0..lines - 1 {
            let a = start + line;
            let b = a + lines;
            indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U16(indices))
}

/// Small arrowheads every `CHEVRON_SPACING` units along `path`, each
/// Where a ribbon's chevrons sit: their position, the direction they point,
/// and how far along the path they are (which staggers the flow animation).
fn chevrons(path: &[Vec2]) -> Vec<(Vec2, Vec2, f32)> {
    let mut marks = Vec::new();
    let mut travelled = CHEVRON_SPACING / 2.0;
    let mut covered = 0.0;
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = a.distance(b);
        let direction = (b - a).normalize_or_zero();
        while travelled <= length {
            marks.push((a + direction * travelled, direction, covered + travelled));
            travelled += CHEVRON_SPACING;
        }
        travelled -= length;
        covered += length;
    }
    marks
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

/// The colour an object throws off when the astronaut touches it. Each type
/// sparks in its own colour, so a contact is recognisable at a glance.
#[must_use]
pub(crate) fn spark_color(kind: ObjectKind) -> Color {
    match kind {
        ObjectKind::CloudPuff => PUFF_COLOR,
        ObjectKind::WindRibbon => CHEVRON_GLOW,
        ObjectKind::Crystal => CRYSTAL_GLOW,
        ObjectKind::TwinPortals => PORTAL_RIM,
        ObjectKind::GravityOrb => ORB_CORE,
        ObjectKind::Bubble => BUBBLE_SHELL,
        ObjectKind::SporeFlower => PETAL_COLOR,
        ObjectKind::CometStone => FLAME_COLOR,
        ObjectKind::StaticCloud => STATIC_CRACKLE,
    }
}

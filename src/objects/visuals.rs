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
use crate::rng::SplitMix64;
use crate::soft_mesh::{MeshBuilder, arc, soft_polygon, spike, unit_disc};

/// Cloud puffs: three overlapping soft circles (offsets and sizes relative to
/// the radius). Pastel, not glowing: clouds are soft, not light sources.
/// Where a cloud's lobes sit, how big they are, and whether they catch the
/// light. Uneven on purpose: a cloud is not a snowman.
const PUFF_LOBES: [(Vec2, f32, bool); 7] = [
    (Vec2::new(-0.52, -0.18), 0.52, false),
    (Vec2::new(-0.18, -0.30), 0.46, false),
    (Vec2::new(0.30, -0.22), 0.58, false),
    (Vec2::new(0.62, 0.02), 0.38, false),
    (Vec2::new(-0.30, 0.18), 0.60, true),
    (Vec2::new(0.10, 0.30), 0.68, true),
    (Vec2::new(0.44, 0.26), 0.40, true),
];
/// The shards of a crystal: where each sits, its size, its lean, its depth.
const CRYSTAL_SHARDS: [(Vec2, f32, f32, f32); 3] = [
    (Vec2::new(0.0, 0.0), 1.0, 0.0, 0.2),
    (Vec2::new(-0.52, -0.3), 0.55, -0.42, 0.1),
    (Vec2::new(0.46, -0.42), 0.42, 0.55, 0.0),
];
/// Loose spores around a flower, craters on a stone, feelers on a hazard.
const SPORES: u8 = 7;
const CRATERS: u8 = 5;
const TENDRILS: u8 = 9;
const SPORE_SEED: u64 = 0x5350_4F52_4500_0001;
const CRATER_SEED: u64 = 0x4352_4154_4500_0001;
const TENDRIL_SEED: u64 = 0x5445_4E44_0000_0001;
const PUFF_COLOR: Color = Color::srgba(0.72, 0.64, 0.78, 0.75);
/// The side the light falls on.
const PUFF_LIT: Color = Color::srgba(0.98, 0.90, 0.96, 0.85);
/// Wind ribbons: a translucent glowing strip, with brighter chevrons every
/// `CHEVRON_SPACING` units pointing along the flow.
const RIBBON_GLOW: Color = Color::linear_rgba(0.35, 1.5, 1.4, 0.42);
/// The wider, fainter disturbance around the stream itself.
const RIBBON_HAZE: Color = Color::linear_rgba(0.2, 0.9, 0.85, 0.12);
const CHEVRON_GLOW: Color = Color::linear_rgba(0.6, 2.2, 1.9, 0.8);
/// How much of a ribbon's width is at full strength; the rest fades out to
/// its edges.
const RIBBON_CORE: f32 = 0.55;
/// How much of each end of a stream is spent narrowing, and how narrow it
/// gets there: a stream of air has no square ends.
const RIBBON_TAPER: f32 = 0.22;
const RIBBON_WAIST: f32 = 0.45;
const CHEVRON_SPACING: f32 = 115.0;
pub(crate) const CHEVRON_SIZE: f32 = 30.0;
/// Crystals: a tall diamond with a cool glow (slightly above 1.0: a hint of bloom).
const CRYSTAL_GLOW: Color = Color::linear_rgb(0.42, 1.25, 1.7);
/// The one facet catching the light.
const CRYSTAL_FACET: Color = Color::linear_rgba(1.4, 2.4, 2.8, 0.55);
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
const BUBBLE_SHEEN: Color = Color::linear_rgba(2.2, 2.6, 3.0, 0.8);
const BUBBLE_FILL: Color = Color::linear_rgba(0.5, 0.9, 1.3, 0.16);
/// Shell thickness as a fraction of the radius.
const BUBBLE_RIM: f32 = 0.08;
/// Spore flowers: five petals around a bright core, in mischievous magenta.
const PETALS: u8 = 5;
const PETAL_COLOR: Color = Color::linear_rgb(1.4, 0.35, 1.1);
const FLOWER_CORE: Color = Color::linear_rgb(2.4, 1.9, 0.6);
/// Comet stones: a dark stone with a bright flame pointing down (where it throws you).
const STONE_COLOR: Color = Color::srgb(0.26, 0.22, 0.28);
/// Pits in it, a shade darker than the rock.
const CRATER_COLOR: Color = Color::srgba(0.14, 0.11, 0.17, 0.85);
const FLAME_COLOR: Color = Color::linear_rgb(3.2, 1.0, 0.25);
/// The wider, dimmer part of the trail behind it.
const FLAME_HAZE: Color = Color::linear_rgba(2.0, 0.6, 0.2, 0.35);
/// Fixed seeds: a rock has the same lumps every time it's drawn.
const ROCK_SEED: u64 = 0x524F_434B_0000_0001;
/// Dark and brooding, with a faint violet crackle around it (no bloom).
const STATIC_CORE: Color = Color::srgb(0.10, 0.07, 0.16);
const STATIC_CRACKLE: Color = Color::srgba(0.50, 0.34, 0.78, 0.5);
/// The faint bruise of light around the whole thing.
const STATIC_AURA: Color = Color::srgba(0.36, 0.22, 0.58, 0.22);

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
    /// A soft disc of radius 1: cores, glows, craters, spores.
    halo: Handle<Mesh>,
    /// An angular crystal shard, tip up, about one unit tall.
    shard: Handle<Mesh>,
    /// A cratered rock: an irregular outline, nothing like a ball.
    rock: Handle<Mesh>,
    /// A narrow pointed petal, one unit long, pointing up.
    petal_spike: Handle<Mesh>,
    /// A longer, softer version of the same: a tendril.
    tendril: Handle<Mesh>,
    /// Three quarters of a ring: a swirl inside a portal, an orb's disc.
    swirl: Handle<Mesh>,
    /// A short bright stroke: a highlight on glass.
    sheen: Handle<Mesh>,
    puff: Handle<ColorMaterial>,
    puff_lit: Handle<ColorMaterial>,
    crystal_facet: Handle<ColorMaterial>,
    ribbon_haze: Handle<ColorMaterial>,
    bubble_sheen: Handle<ColorMaterial>,
    crater: Handle<ColorMaterial>,
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
    flame_haze: Handle<ColorMaterial>,
    static_core: Handle<ColorMaterial>,
    static_crackle: Handle<ColorMaterial>,
    static_aura: Handle<ColorMaterial>,
}

/// Builds one mesh out of soft shapes, with its colour baked in.
fn shaped(build: impl FnOnce(&mut MeshBuilder), color: Color) -> Mesh {
    let mut builder = MeshBuilder::with_capacity(64);
    let _ = color;
    build(&mut builder);
    builder.into_mesh()
}

/// A rough ring of points: the outline of something that was never made in
/// a factory. `wobble` is how far each corner strays from a circle.
fn rough_outline(corners: u8, wobble: f32, seed: u64) -> Vec<Vec2> {
    let mut rng = SplitMix64::new(seed);
    (0..corners)
        .map(|corner| {
            let angle = f32::from(corner) / f32::from(corners) * core::f32::consts::TAU;
            Vec2::from_angle(angle) * (1.0 - rng.range(0.0, wobble))
        })
        .collect()
}

impl FromWorld for ObjectAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_circle = meshes.add(Circle::new(1.0));
        let unit_ring = meshes.add(Annulus::new(1.0 - BUBBLE_RIM, 1.0));
        let thin_ring = meshes.add(Annulus::new(1.0 - ORB_EDGE, 1.0));
        let white = LinearRgba::WHITE.to_f32_array();
        let halo = meshes.add(unit_disc(24));
        // A shard, not a lozenge: two long faces, an off-centre waist.
        let shard = meshes.add(shaped(
            |builder| {
                soft_polygon(
                    builder,
                    &[
                        Vec2::new(0.0, 1.0),
                        Vec2::new(-0.42, 0.12),
                        Vec2::new(-0.26, -0.78),
                        Vec2::new(0.18, -1.0),
                        Vec2::new(0.40, -0.1),
                    ],
                    white,
                    0.12,
                );
            },
            Color::WHITE,
        ));
        let rock = meshes.add(shaped(
            |builder| soft_polygon(builder, &rough_outline(11, 0.3, ROCK_SEED), white, 0.07),
            Color::WHITE,
        ));
        let petal_spike = meshes.add(shaped(
            |builder| spike(builder, 0.42, white, false),
            Color::WHITE,
        ));
        let tendril = meshes.add(shaped(
            |builder| spike(builder, 0.3, white, true),
            Color::WHITE,
        ));
        let swirl = meshes.add(shaped(
            |builder| arc(builder, 0.3, core::f32::consts::TAU * 0.72, 0.18, white, 24),
            Color::WHITE,
        ));
        let sheen = meshes.add(shaped(
            |builder| arc(builder, 2.1, 0.9, 0.1, white, 12),
            Color::WHITE,
        ));
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
            halo,
            shard,
            rock,
            petal_spike,
            tendril,
            swirl,
            sheen,
            puff: materials.add(PUFF_COLOR),
            puff_lit: materials.add(PUFF_LIT),
            crystal_facet: materials.add(CRYSTAL_FACET),
            ribbon_haze: materials.add(RIBBON_HAZE),
            bubble_sheen: materials.add(BUBBLE_SHEEN),
            crater: materials.add(CRATER_COLOR),
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
            flame_haze: materials.add(FLAME_HAZE),
            static_core: materials.add(STATIC_CORE),
            static_crackle: materials.add(STATIC_CRACKLE),
            static_aura: materials.add(STATIC_AURA),
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
        // Two mouths of one tunnel: a dark throat, and swirls turning inside
        // it at different rates.
        Placement::TwinPortals { a, b, radius } => {
            let middle = placement.center();
            for (end, turn) in [(*a, 0.0), (*b, core::f32::consts::PI)] {
                let local = (end - middle).extend(0.0);
                let size = Vec3::splat(*radius);
                parent.spawn((
                    Mesh2d(assets.halo.clone()),
                    MeshMaterial2d(assets.portal_fill.clone()),
                    Transform::from_translation(local).with_scale(size * 1.5),
                ));
                for (depth, scale, lean) in [(0.1, 0.92, 0.0), (0.2, 0.66, 1.9), (0.3, 0.4, 3.6)] {
                    parent.spawn((
                        Mesh2d(assets.swirl.clone()),
                        MeshMaterial2d(assets.portal_rim.clone()),
                        Transform::from_translation(local + Vec3::Z * depth)
                            .with_rotation(Quat::from_rotation_z(turn + lean))
                            .with_scale(size * scale),
                    ));
                }
            }
        }
        // A well, not a ball: a small fierce core inside a tilted disc of
        // matter falling into it.
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
            // The disc, seen nearly edge-on, crossing in front of the core.
            for (depth, scale, squash) in [(0.15, 2.3, 0.26), (0.35, 1.7, 0.18)] {
                parent.spawn((
                    Mesh2d(assets.swirl.clone()),
                    MeshMaterial2d(assets.orb_field_edge.clone()),
                    Transform::from_xyz(0.0, 0.0, depth)
                        .with_rotation(Quat::from_rotation_z(-0.22))
                        .with_scale(Vec3::new(radius * scale, radius * scale * squash, 1.0)),
                ));
            }
            parent.spawn((
                Mesh2d(assets.halo.clone()),
                MeshMaterial2d(assets.orb_core.clone()),
                Transform::from_xyz(0.0, 0.0, 0.25).with_scale(Vec3::splat(radius * 1.1)),
            ));
        }
        Placement::WindRibbon { path, width } => {
            // Mesh points are relative to the parent, which sits at the center.
            let center = placement.center();
            let local: Vec<Vec2> = path.iter().map(|&point| point - center).collect();
            // A wider, fainter band under the stream: the air it disturbs.
            parent.spawn((
                Mesh2d(meshes.add(ribbon_mesh(&local, *width * 1.7))),
                MeshMaterial2d(assets.ribbon_haze.clone()),
                Transform::from_xyz(0.0, 0.0, -0.1),
            ));
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
        // Not three circles: a drift of lobes of different sizes, lit from
        // above, trailing off to one side.
        Placement::CloudPuff { radius, .. } => {
            for (offset, scale, lit) in PUFF_LOBES {
                let material = if lit {
                    assets.puff_lit.clone()
                } else {
                    assets.puff.clone()
                };
                let depth = if lit { 0.1 } else { 0.0 };
                parent.spawn((
                    Mesh2d(assets.halo.clone()),
                    MeshMaterial2d(material),
                    Transform::from_translation((offset * *radius).extend(depth))
                        .with_scale(Vec3::new(radius * scale * 1.25, radius * scale, 1.0)),
                ));
            }
        }
        // Glass: you see through it, and the light catches one edge.
        Placement::Bubble { radius, .. } => {
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.bubble_fill.clone()),
                Transform::from_scale(Vec3::splat(*radius)),
            ));
            parent.spawn((
                Mesh2d(assets.swirl.clone()),
                MeshMaterial2d(assets.bubble_fill.clone()),
                Transform::from_xyz(0.0, 0.0, 0.05)
                    .with_rotation(Quat::from_rotation_z(0.9))
                    .with_scale(Vec3::splat(radius * 0.62)),
            ));
            parent.spawn((
                Mesh2d(assets.unit_ring.clone()),
                MeshMaterial2d(assets.bubble_shell.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(*radius)),
            ));
            parent.spawn((
                Mesh2d(assets.sheen.clone()),
                MeshMaterial2d(assets.bubble_sheen.clone()),
                Transform::from_xyz(0.0, 0.0, 0.2).with_scale(Vec3::splat(radius * 0.88)),
            ));
        }
        // A seed pod rather than a daisy: narrow spines, a hard bright
        // centre, and spores already loose around it.
        Placement::SporeFlower { radius, .. } => {
            let mut rng = SplitMix64::new(SPORE_SEED);
            for petal in 0..PETALS {
                let angle = f32::from(petal) / f32::from(PETALS) * core::f32::consts::TAU
                    + rng.range(-0.12, 0.12);
                let length = radius * rng.range(0.85, 1.15);
                parent.spawn((
                    Mesh2d(assets.petal_spike.clone()),
                    MeshMaterial2d(assets.petal.clone()),
                    Transform::from_rotation(Quat::from_rotation_z(angle))
                        .with_scale(Vec3::new(length, length, 1.0)),
                ));
            }
            for _ in 0..SPORES {
                let angle = rng.range(0.0, core::f32::consts::TAU);
                let away = Vec2::from_angle(angle) * *radius * rng.range(1.0, 1.5);
                parent.spawn((
                    Mesh2d(assets.halo.clone()),
                    MeshMaterial2d(assets.petal.clone()),
                    Transform::from_translation(away.extend(-0.1))
                        .with_scale(Vec3::splat(radius * rng.range(0.05, 0.11))),
                ));
            }
            parent.spawn((
                Mesh2d(assets.halo.clone()),
                MeshMaterial2d(assets.flower_core.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(radius * 0.34)),
            ));
        }
        // A lump of something old: lopsided, pitted, burning along the edge
        // it is travelling on.
        Placement::CometStone { radius, .. } => {
            parent.spawn((
                Mesh2d(assets.halo.clone()),
                MeshMaterial2d(assets.flame_haze.clone()),
                Transform::from_xyz(0.0, -radius * 1.0, -0.2).with_scale(Vec3::new(
                    radius * 0.45,
                    radius * 1.5,
                    1.0,
                )),
            ));
            parent.spawn((
                Mesh2d(assets.down_triangle.clone()),
                MeshMaterial2d(assets.flame.clone()),
                Transform::from_xyz(0.0, -radius * 0.62, -0.1).with_scale(Vec3::new(
                    radius * 0.42,
                    radius * 0.95,
                    1.0,
                )),
            ));
            parent.spawn((
                Mesh2d(assets.rock.clone()),
                MeshMaterial2d(assets.stone.clone()),
                Transform::from_scale(Vec3::splat(*radius)),
            ));
            // Pits, scattered across the face of it.
            let mut rng = SplitMix64::new(CRATER_SEED);
            for _ in 0..CRATERS {
                let at = Vec2::new(rng.range(-0.5, 0.5), rng.range(-0.35, 0.55)) * *radius;
                parent.spawn((
                    Mesh2d(assets.halo.clone()),
                    MeshMaterial2d(assets.crater.clone()),
                    Transform::from_translation(at.extend(0.1))
                        .with_scale(Vec3::splat(radius * rng.range(0.1, 0.22))),
                ));
            }
        }
        // A cluster, not a lozenge: one long shard with two smaller ones
        // grown off it, and a facet catching the light.
        Placement::Crystal { size, .. } => {
            for (offset, scale, lean, depth) in CRYSTAL_SHARDS {
                parent.spawn((
                    Mesh2d(assets.shard.clone()),
                    MeshMaterial2d(assets.crystal.clone()),
                    Transform::from_translation((offset * *size).extend(depth))
                        .with_rotation(Quat::from_rotation_z(lean))
                        .with_scale(Vec3::new(size * scale, size * scale, 1.0)),
                ));
            }
            parent.spawn((
                Mesh2d(assets.petal_spike.clone()),
                MeshMaterial2d(assets.crystal_facet.clone()),
                Transform::from_xyz(-size * 0.1, -size * 0.3, 0.3)
                    .with_rotation(Quat::from_rotation_z(0.08))
                    .with_scale(Vec3::new(size * 0.5, size * 1.1, 1.0)),
            ));
        }
        // The one thing in the sky that wants something from you: a hole
        // with feelers.
        Placement::StaticCloud { radius, .. } => {
            // The bruise of light it sits in, wide and faint.
            parent.spawn((
                Mesh2d(assets.halo.clone()),
                MeshMaterial2d(assets.static_aura.clone()),
                Transform::from_xyz(0.0, 0.0, -0.1).with_scale(Vec3::splat(radius * 2.1)),
            ));
            let mut rng = SplitMix64::new(TENDRIL_SEED);
            for tendril in 0..TENDRILS {
                let angle = f32::from(tendril) / f32::from(TENDRILS) * core::f32::consts::TAU
                    + rng.range(-0.25, 0.25);
                // Long enough to read as reaching for you.
                let reach = radius * rng.range(1.5, 2.2);
                parent.spawn((
                    Mesh2d(assets.tendril.clone()),
                    MeshMaterial2d(assets.static_crackle.clone()),
                    Transform::from_rotation(Quat::from_rotation_z(angle)).with_scale(Vec3::new(
                        reach * 0.28,
                        reach,
                        1.0,
                    )),
                ));
            }
            parent.spawn((
                Mesh2d(assets.unit_circle.clone()),
                MeshMaterial2d(assets.static_core.clone()),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(radius * 0.82)),
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
    // How far along the path each point is, so the stream can narrow and
    // fade towards both ends instead of being cut off square.
    let mut travelled = vec![0.0_f32; path.len()];
    for index in 1..path.len() {
        travelled[index] = travelled[index - 1] + path[index].distance(path[index - 1]);
    }
    let total = travelled.last().copied().unwrap_or(1.0).max(1e-3);
    for (i, &point) in path.iter().enumerate() {
        let before = if i > 0 {
            point - path[i - 1]
        } else {
            Vec2::ZERO
        };
        let after = path.get(i + 1).map_or(Vec2::ZERO, |&next| next - point);
        let tangent =
            (before.normalize_or_zero() + after.normalize_or_zero()).normalize_or(Vec2::X);
        // Narrow and fade over the first and last fifth of the stream.
        let along = travelled[i] / total;
        let ends = (along / RIBBON_TAPER)
            .min((1.0 - along) / RIBBON_TAPER)
            .clamp(0.0, 1.0);
        let eased = ends * ends * (3.0 - 2.0 * ends);
        let side = tangent.perp() * width * (RIBBON_WAIST + (1.0 - RIBBON_WAIST) * eased);
        for (offset, alpha) in across.iter().zip(alphas) {
            positions.push((point + side * *offset).extend(0.0).to_array());
            // White: the ribbon's own colour comes from its material, and
            // only the transparency varies across the strip.
            colors.push([1.0, 1.0, 1.0, alpha * (0.35 + 0.65 * eased)]);
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

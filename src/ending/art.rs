//! Everything the cartoon draws, built once from a `Planet`.
//!
//! Colours are baked into the vertices, the way the rest of the game's art is
//! (`soft_mesh.rs`), so the whole planet shares one material. Every prop is
//! built with its origin where it meets the ground, so the entity's transform
//! is just the place it stands.

use bevy::prelude::*;

use super::planet::{
    GROUND_WIDTH, GroundKind, Palette, Planet, Prop, PropKind, SAMPLES, Shade, sample_x,
};
use crate::config::ASTRONAUT_RADIUS;
use crate::level::LEVEL_HEIGHT;
use crate::soft_mesh::{MeshBuilder, gradient_mesh, soft_disc, soft_polygon};

/// The ground is drawn down to here, far under the playfield, so no sky can
/// show beneath it however tall the window is.
const GROUND_BOTTOM: f32 = -LEVEL_HEIGHT - 2_000.0;
/// And out to here either side, for the same reason on a wide window. The
/// camera shows more scenery the wider the window gets (Section 5).
const GROUND_REACH: f32 = 5_000.0;
/// Corners on a round prop. They are small on screen; a dozen is plenty.
const SEGMENTS: usize = 14;
/// How far the soft blobs along the skyline reach, in world units.
const SKYLINE: Vec2 = Vec2::new(30.0, 22.0);
/// One astronaut radius, which is how every size here is written.
const R: f32 = ASTRONAUT_RADIUS;

/// A planet's shade as a colour.
pub(super) fn color(shade: Shade) -> Color {
    Color::hsl(shade.hue, shade.saturation, shade.lightness)
}

/// A shade as the linear colour a vertex carries.
fn ink(shade: Shade, alpha: f32) -> [f32; 4] {
    let [red, green, blue, _] = color(shade).to_linear().to_f32_array();
    [red, green, blue, alpha]
}

/// A tapered blade from `base` to `tip`, fading out at the tip: a stem, a
/// reed, a petal.
fn blade(builder: &mut MeshBuilder, base: Vec2, tip: Vec2, width: f32, color: [f32; 4]) {
    let along = (tip - base).normalize_or_zero();
    let across = Vec2::new(-along.y, along.x) * (width / 2.0);
    let faded = [color[0], color[1], color[2], 0.0];
    let left = builder.vertex(base - across, color);
    let right = builder.vertex(base + across, color);
    let point = builder.vertex(tip, faded);
    builder.triangle(left, right, point);
}

/// A four-cornered patch, for anything built out of straight edges.
fn slab(builder: &mut MeshBuilder, center: Vec2, size: Vec2, color: [f32; 4]) {
    let half = size / 2.0;
    let corners = [
        builder.vertex(center + Vec2::new(-half.x, half.y), color),
        builder.vertex(center + Vec2::new(-half.x, -half.y), color),
        builder.vertex(center + Vec2::new(half.x, -half.y), color),
        builder.vertex(center + Vec2::new(half.x, half.y), color),
    ];
    builder.quad(corners);
}

/// The sky: four bands of the planet's hue. A 1×1 quad, scaled into place by
/// whoever spawns it.
pub(super) fn sky_mesh(palette: &Palette) -> Mesh {
    gradient_mesh(&palette.sky.map(color))
}

/// The ground: one column per height sample, darkening with depth, and a run
/// of soft blobs along the skyline so the horizon is painted rather than cut.
pub(super) fn ground_mesh(planet: &Planet) -> Mesh {
    let near = ink(planet.palette.ground, 1.0);
    let far = ink(planet.palette.ground_far, 1.0);
    let mut builder = MeshBuilder::with_capacity(SAMPLES * 6);
    for index in 0..SAMPLES - 1 {
        let left = sample_x(index);
        let right = sample_x(index + 1);
        let corners = [
            builder.vertex(Vec2::new(left, planet.height_at(left)), near),
            builder.vertex(Vec2::new(left, GROUND_BOTTOM), far),
            builder.vertex(Vec2::new(right, GROUND_BOTTOM), far),
            builder.vertex(Vec2::new(right, planet.height_at(right)), near),
        ];
        builder.quad(corners);
    }
    // Skirts, so a window of any shape still sees ground to both edges.
    for (edge, inward) in [(-GROUND_REACH, 0_usize), (GROUND_REACH, SAMPLES - 1)] {
        let inner = sample_x(inward);
        let height = planet.height_at(inner);
        let corners = [
            builder.vertex(Vec2::new(edge, height), near),
            builder.vertex(Vec2::new(edge, GROUND_BOTTOM), far),
            builder.vertex(Vec2::new(inner, GROUND_BOTTOM), far),
            builder.vertex(Vec2::new(inner, height), near),
        ];
        builder.quad(corners);
    }
    for index in 0..SAMPLES {
        let x = sample_x(index);
        soft_disc(
            &mut builder,
            Vec2::new(x, planet.height_at(x)),
            SKYLINE,
            ink(planet.palette.ground, 0.3),
            10,
        );
    }
    builder.into_mesh()
}

/// The water, if this planet has any: its own surface over the ground it cut
/// into, with a lighter band along the top.
pub(super) fn water_mesh(planet: &Planet) -> Option<Mesh> {
    let span = planet
        .spans
        .iter()
        .find(|span| span.kind == GroundKind::Water)?;
    let surface = ink(planet.palette.water, 0.8);
    let deep = ink(planet.palette.water, 0.95);
    let mut builder = MeshBuilder::with_capacity(SAMPLES * 4);
    for index in 0..SAMPLES - 1 {
        let left = sample_x(index);
        let right = sample_x(index + 1);
        if right < span.from || left > span.to {
            continue;
        }
        let corners = [
            builder.vertex(Vec2::new(left, planet.height_at(left)), surface),
            builder.vertex(Vec2::new(left, GROUND_BOTTOM), deep),
            builder.vertex(Vec2::new(right, GROUND_BOTTOM), deep),
            builder.vertex(Vec2::new(right, planet.height_at(right)), surface),
        ];
        builder.quad(corners);
        // The light that sits on top of water, where it meets the air.
        soft_disc(
            &mut builder,
            Vec2::new(left, planet.height_at(left)),
            Vec2::new(GROUND_WIDTH / 60.0, 6.0),
            ink(planet.palette.accent, 0.25),
            8,
        );
    }
    Some(builder.into_mesh())
}

/// One prop, built from its base upwards.
pub(super) fn prop_mesh(prop: &Prop, palette: &Palette) -> Mesh {
    let mut rng = crate::rng::SplitMix64::new(prop.seed);
    let mut builder = MeshBuilder::with_capacity(64);
    let body = ink(palette.prop, 1.0);
    let bright = ink(palette.accent, 1.0);
    let dark = ink(palette.ground_far, 1.0);
    let size = prop.size;
    match prop.kind {
        // A stem with a wide, slightly squashed cap, and freckles on it.
        PropKind::Mushroom => {
            let tall = size * 1.3;
            blade(
                &mut builder,
                Vec2::ZERO,
                Vec2::new(rng.range(-0.08, 0.08) * size, tall * 1.05),
                size * 0.22,
                ink(palette.prop, 0.9),
            );
            let cap = Vec2::new(size / 2.0, size * 0.3);
            soft_disc(&mut builder, Vec2::new(0.0, tall), cap, body, SEGMENTS);
            for _ in 0..3 {
                soft_disc(
                    &mut builder,
                    Vec2::new(
                        rng.range(-0.3, 0.3) * size,
                        tall + rng.range(0.0, 0.12) * size,
                    ),
                    Vec2::splat(size * 0.06),
                    bright,
                    8,
                );
            }
        }
        // A bunch hanging off a stalk, every grape a shade of its own.
        PropKind::GrapeCluster => {
            let tall = size * 1.6;
            blade(
                &mut builder,
                Vec2::ZERO,
                Vec2::new(0.0, tall),
                size * 0.12,
                ink(palette.prop, 0.8),
            );
            let grapes = 7 + (prop.seed % 7) as usize;
            for grape in 0..grapes {
                #[expect(clippy::cast_precision_loss, reason = "under fourteen")]
                let down = grape as f32 / grapes as f32;
                let at = Vec2::new(
                    rng.range(-0.3, 0.3) * size,
                    tall - down * size * 0.9 - size * 0.1,
                );
                let shade = Shade {
                    lightness: (palette.prop.lightness + rng.range(-0.06, 0.06)).clamp(0.05, 0.9),
                    ..palette.prop
                };
                soft_disc(
                    &mut builder,
                    at,
                    Vec2::splat(size * 0.16),
                    ink(shade, 1.0),
                    10,
                );
            }
        }
        // Five petals around a bright middle, on a thin stem.
        PropKind::Flower => {
            let tall = size * 1.8;
            let head = Vec2::new(0.0, tall);
            blade(
                &mut builder,
                Vec2::ZERO,
                head,
                size * 0.12,
                ink(palette.prop, 0.85),
            );
            for petal in 0..5 {
                #[expect(clippy::cast_precision_loss, reason = "five petals")]
                let angle = petal as f32 / 5.0 * core::f32::consts::TAU + rng.range(0.0, 0.4);
                let tip = head + Vec2::from_angle(angle) * size * 0.8;
                blade(&mut builder, head, tip, size * 0.5, body);
            }
            soft_disc(&mut builder, head, Vec2::splat(size * 0.3), bright, 10);
        }
        // A dome sitting on the ground: points around the base, so the fan
        // centre is where the rock meets the dirt.
        PropKind::Boulder => {
            let points: Vec<Vec2> = (0..7)
                .map(|corner| {
                    #[expect(clippy::cast_precision_loss, reason = "seven corners")]
                    let along = corner as f32 / 6.0;
                    let angle = along * core::f32::consts::PI;
                    let reach = size * rng.range(0.85, 1.15);
                    Vec2::new(angle.cos() * reach, angle.sin() * reach * 0.72)
                })
                .rev()
                .collect();
            soft_polygon(&mut builder, &points, body, 0.22);
            soft_disc(
                &mut builder,
                Vec2::new(-size * 0.25, size * 0.3),
                Vec2::splat(size * 0.2),
                ink(palette.ground, 0.5),
                8,
            );
        }
        // A crack in the ground. The steam itself is particles.
        PropKind::SteamVent => {
            soft_disc(
                &mut builder,
                Vec2::new(0.0, size * 0.06),
                Vec2::new(size * 0.6, size * 0.14),
                dark,
                SEGMENTS,
            );
            soft_disc(
                &mut builder,
                Vec2::new(0.0, size * 0.1),
                Vec2::new(size * 0.3, size * 0.08),
                ink(palette.accent, 0.35),
                8,
            );
        }
        // Three blades leaning apart.
        PropKind::Reed => {
            for reed in 0..3 {
                #[expect(clippy::cast_precision_loss, reason = "three reeds")]
                let across = (reed as f32 - 1.0) * size * 0.12;
                let tip = Vec2::new(across * 3.0 + rng.range(-0.1, 0.1) * size, size);
                blade(
                    &mut builder,
                    Vec2::new(across, 0.0),
                    tip,
                    R * 0.1,
                    ink(palette.prop, 0.95),
                );
            }
        }
    }
    builder.into_mesh()
}

/// The pack's body, before the parts that stick out of it.
pub(super) const PACK_BODY: Vec2 = Vec2::new(R * 4.6, R * 4.2);
/// How far the pack reaches either side of where it stands, every part of it
/// counted. Whatever places it leaves this much room.
pub(super) const PACK_REACH: f32 = R * 3.0;
/// How far the cloth reaches from the pole.
pub(super) const CLOTH_WIDTH: f32 = R * 1.4;
/// How far the cloth hangs below it.
const CLOTH_DROP: f32 = R * 1.05;

/// The pack: far more stuff than any suit could have held, built from its
/// base up so it can be scaled out of nothing.
///
/// A box reads as a box, so the silhouette carries the handle, the feet and
/// the aerial, and the face is broken up by a seam, a band and a hatch.
pub(super) fn pack_mesh(palette: &Palette) -> Mesh {
    let body = ink(palette.ground, 1.0);
    let trim = ink(palette.prop, 1.0);
    let dark = ink(palette.ground_far, 1.0);
    let lamp = ink(palette.accent, 1.0);
    let size = PACK_BODY;
    let mut builder = MeshBuilder::with_capacity(96);
    // The feet first, so the body sits over them.
    for side in [-1.0, 1.0] {
        slab(
            &mut builder,
            Vec2::new(side * (PACK_REACH - R * 0.5), R * 0.25),
            Vec2::new(R, R * 0.5),
            dark,
        );
    }
    slab(&mut builder, Vec2::new(0.0, size.y / 2.0), size, body);
    // A seam down it, and a band across it.
    slab(
        &mut builder,
        Vec2::new(0.0, size.y * 0.52),
        Vec2::new(R * 0.14, size.y * 0.86),
        dark,
    );
    slab(
        &mut builder,
        Vec2::new(0.0, size.y * 0.62),
        Vec2::new(size.x, R * 0.5),
        trim,
    );
    // The hatch the flag comes out of, and the latch holding it shut.
    soft_disc(
        &mut builder,
        Vec2::new(0.0, size.y * 0.87),
        Vec2::splat(R * 0.6),
        trim,
        SEGMENTS,
    );
    slab(
        &mut builder,
        Vec2::new(0.0, size.y * 0.87),
        Vec2::new(R * 0.9, R * 0.14),
        dark,
    );
    // Both lamps together on one side: apart, they read as a pair of eyes.
    for step in [0.0, 1.0] {
        soft_disc(
            &mut builder,
            Vec2::new(-size.x * 0.3 + step * R * 0.75, size.y * 0.28),
            Vec2::splat(R * 0.3),
            lamp,
            12,
        );
    }
    // A handle over the top...
    for side in [-1.0, 1.0] {
        slab(
            &mut builder,
            Vec2::new(side * R, size.y + R * 0.3),
            Vec2::new(R * 0.18, R * 0.6),
            dark,
        );
    }
    slab(
        &mut builder,
        Vec2::new(0.0, size.y + R * 0.55),
        Vec2::new(R * 2.2, R * 0.25),
        dark,
    );
    // ...and an aerial with a light on the end of it.
    slab(
        &mut builder,
        Vec2::new(size.x * 0.37, size.y + R * 0.85),
        Vec2::new(R * 0.1, R * 1.7),
        dark,
    );
    soft_disc(
        &mut builder,
        Vec2::new(size.x * 0.37, size.y + R * 1.7),
        Vec2::splat(R * 0.22),
        lamp,
        10,
    );
    builder.into_mesh()
}

/// The flag: a pole and nothing else. The cloth is its own entity, because it
/// waves.
pub(super) fn flag_mesh(palette: &Palette) -> Mesh {
    // Nearly white, whatever colours the planet picked: the pole has to read
    // against a dark hillside as well as against the sky.
    let pole = ink(
        Shade {
            saturation: 0.12,
            lightness: 0.9,
            ..palette.accent
        },
        1.0,
    );
    let mut builder = MeshBuilder::with_capacity(20);
    slab(
        &mut builder,
        Vec2::new(0.0, FLAG_TOP / 2.0),
        Vec2::new(R * 0.16, FLAG_TOP),
        pole,
    );
    // A ball on top, which is what makes a pole look finished.
    soft_disc(
        &mut builder,
        Vec2::new(0.0, FLAG_TOP),
        Vec2::splat(R * 0.26),
        pole,
        12,
    );
    builder.into_mesh()
}

/// The cloth, built hanging from the top of the pole at the origin so it can
/// be swung from there. Its far edge is notched, so even this small the
/// shape says flag.
pub(super) fn cloth_mesh(palette: &Palette) -> Mesh {
    let bright = ink(
        Shade {
            saturation: 0.85,
            lightness: 0.86,
            ..palette.accent
        },
        1.0,
    );
    let fold = ink(
        Shade {
            lightness: 0.5,
            ..palette.accent
        },
        1.0,
    );
    let mut builder = MeshBuilder::with_capacity(12);
    let near_top = builder.vertex(Vec2::ZERO, bright);
    let near_bottom = builder.vertex(Vec2::new(0.0, -CLOTH_DROP), bright);
    let far_top = builder.vertex(Vec2::new(CLOTH_WIDTH, 0.0), bright);
    let far_bottom = builder.vertex(Vec2::new(CLOTH_WIDTH, -CLOTH_DROP), fold);
    let notch = builder.vertex(Vec2::new(CLOTH_WIDTH * 0.72, -CLOTH_DROP * 0.5), bright);
    builder.triangle(near_top, near_bottom, notch);
    builder.triangle(near_top, notch, far_top);
    builder.triangle(near_bottom, far_bottom, notch);
    // A darker band along the bottom, so the cloth has a fold in it.
    slab(
        &mut builder,
        Vec2::new(CLOTH_WIDTH / 2.0, -CLOTH_DROP + R * 0.1),
        Vec2::new(CLOTH_WIDTH, R * 0.2),
        fold,
    );
    builder.into_mesh()
}

/// Where the top of the pole is, so the cloth and the star can hang there.
pub(super) const FLAG_TOP: f32 = R * 3.2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ASTRONAUT_RADIUS;
    use crate::ending::planet::{GroundKind, Planet, Prop, PropKind};
    use crate::level::{LEVEL_HEIGHT, LEVEL_WIDTH};
    use bevy::math::Vec2;
    use bevy::mesh::VertexAttributeValues;

    fn points(mesh: &Mesh) -> Vec<[f32; 3]> {
        let Some(VertexAttributeValues::Float32x3(points)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("the mesh has no vertices");
        };
        points.clone()
    }

    fn width(mesh: &Mesh) -> f32 {
        let points = points(mesh);
        let low = points.iter().map(|point| point[0]).fold(f32::MAX, f32::min);
        let high = points.iter().map(|point| point[0]).fold(f32::MIN, f32::max);
        high - low
    }

    fn colors(mesh: &Mesh) -> Vec<[f32; 4]> {
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("the mesh has no colours");
        };
        colors.clone()
    }

    /// The brightest solid colour in a mesh, as the light it sends out. Rim
    /// vertices fade to nothing, so they are weighted by their own alpha.
    fn brightest(mesh: &Mesh) -> f32 {
        colors(mesh)
            .iter()
            .map(|[red, green, blue, alpha]| {
                (0.2126 * red + 0.7152 * green + 0.0722 * blue) * alpha
            })
            .fold(0.0, f32::max)
    }

    fn height(mesh: &Mesh) -> f32 {
        let points = points(mesh);
        let low = points.iter().map(|point| point[1]).fold(f32::MAX, f32::min);
        let high = points.iter().map(|point| point[1]).fold(f32::MIN, f32::max);
        high - low
    }

    #[test]
    fn the_ground_reaches_past_both_edges_of_any_window() {
        let planet = Planet::generate(3);
        let points = points(&ground_mesh(&planet));
        let left = points.iter().map(|point| point[0]).fold(f32::MAX, f32::min);
        let right = points.iter().map(|point| point[0]).fold(f32::MIN, f32::max);
        assert!(left <= -LEVEL_WIDTH / 2.0, "the ground stops at {left}");
        assert!(right >= LEVEL_WIDTH / 2.0, "the ground stops at {right}");
        // Down to the bottom of the playfield, so no sky shows under the hill.
        let bottom = points.iter().map(|point| point[1]).fold(f32::MAX, f32::min);
        assert!(bottom <= -LEVEL_HEIGHT, "the ground floats at {bottom}");
    }

    #[test]
    fn every_prop_is_something_to_look_at() {
        let palette = Planet::generate(11).palette;
        for kind in [
            PropKind::Mushroom,
            PropKind::GrapeCluster,
            PropKind::Flower,
            PropKind::Boulder,
            PropKind::SteamVent,
            PropKind::Reed,
        ] {
            let prop = Prop {
                kind,
                at: Vec2::ZERO,
                size: 60.0,
                seed: 9,
            };
            let mesh = prop_mesh(&prop, &palette);
            assert!(mesh.count_vertices() >= 3, "{kind:?} is empty");
            assert!(
                mesh.indices().is_some_and(|indices| indices.len() >= 3),
                "{kind:?} has no faces"
            );
        }
    }

    /// The flag is the point of the whole cartoon, so it has to be read from
    /// across the room: taller than the astronaut, with a cloth on it.
    #[test]
    fn the_flag_is_tall_enough_to_see() {
        let palette = Planet::generate(5).palette;
        let flag = flag_mesh(&palette);
        let cloth = cloth_mesh(&palette);
        assert!(
            height(&flag) > ASTRONAUT_RADIUS * 3.0,
            "the pole is only {} tall",
            height(&flag)
        );
        assert!(
            width(&cloth) >= CLOTH_WIDTH,
            "the cloth is only {} wide",
            width(&cloth)
        );
    }

    /// Whatever planet it lands on, the flag is brighter than the ground it
    /// is planted in. A planet picks its own colours, so this is checked
    /// across many of them.
    #[test]
    fn the_flag_stands_out_against_the_ground_it_is_planted_in() {
        for seed in 0..500 {
            let planet = Planet::generate(seed);
            let ground = brightest(&ground_mesh(&planet));
            let cloth = brightest(&cloth_mesh(&planet.palette));
            let pole = brightest(&flag_mesh(&planet.palette));
            assert!(
                cloth > ground * 2.0,
                "seed {seed}: cloth {cloth} against ground {ground}"
            );
            assert!(
                pole > ground * 2.0,
                "seed {seed}: pole {pole} against ground {ground}"
            );
        }
    }

    /// A rectangle reads as a rectangle. The cloth is cut away on its far
    /// edge, which is what makes the shape a flag.
    #[test]
    fn the_cloth_is_cut_like_a_flag() {
        let cloth = cloth_mesh(&Planet::generate(5).palette);
        let points = points(&cloth);
        let far = points.iter().map(|point| point[0]).fold(f32::MIN, f32::max);
        let top = points.iter().map(|point| point[1]).fold(f32::MIN, f32::max);
        let bottom = points.iter().map(|point| point[1]).fold(f32::MAX, f32::min);
        let inside = points.iter().any(|point| {
            point[0] > far * 0.5
                && point[0] < far * 0.95
                && point[1] > bottom + (top - bottom) * 0.15
                && point[1] < top - (top - bottom) * 0.15
        });
        assert!(inside, "the far edge is a straight line");
    }

    /// The pack is a bag of stuff, not a box: there are parts above it and
    /// parts standing out either side of it.
    #[test]
    fn the_pack_is_not_a_plain_box() {
        let pack = pack_mesh(&Planet::generate(5).palette);
        assert!(
            height(&pack) > PACK_BODY.y,
            "nothing stands above the body: {} against {}",
            height(&pack),
            PACK_BODY.y
        );
        assert!(
            width(&pack) > PACK_BODY.x,
            "nothing stands out either side: {} against {}",
            width(&pack),
            PACK_BODY.x
        );
        assert!(
            width(&pack) <= PACK_REACH * 2.0,
            "it reaches further than anything placing it expects"
        );
    }

    #[test]
    fn the_pack_towers_over_the_astronaut_and_the_flag_does_not() {
        let palette = Planet::generate(5).palette;
        let pack = pack_mesh(&palette);
        let flag = flag_mesh(&palette);
        let cloth = cloth_mesh(&palette);
        // The flag is the pole and what hangs off it, measured together.
        let flag_width = width(&flag) + width(&cloth);
        assert!(
            height(&pack) >= ASTRONAUT_RADIUS * 3.0,
            "the pack is only {} tall",
            height(&pack)
        );
        // The joke is the bulk: the pack stands half again as tall as the
        // flag and three times as wide.
        assert!(
            height(&pack) > height(&flag) * 1.5,
            "{} against {}",
            height(&pack),
            height(&flag)
        );
        assert!(
            width(&pack) > flag_width * 3.0,
            "{} against {flag_width}",
            width(&pack)
        );
    }

    #[test]
    fn water_only_exists_where_there_is_water() {
        let dry = (0..2_000).map(Planet::generate).find(|planet| {
            planet
                .spans
                .iter()
                .all(|span| span.kind != GroundKind::Water)
        });
        assert!(
            dry.is_some_and(|planet| water_mesh(&planet).is_none()),
            "a planet without water should have no water to draw"
        );
        let wet = (0..2_000).map(Planet::generate).find(|planet| {
            planet
                .spans
                .iter()
                .any(|span| span.kind == GroundKind::Water)
        });
        assert!(wet.is_some_and(|planet| water_mesh(&planet).is_some()));
    }
}

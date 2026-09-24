//! Meshes for the soft, glowing shapes the game draws in code.
//!
//! Every shape fades out at its rim instead of ending in a hard edge: that
//! soft falloff is what makes shapes built from triangles look painted rather
//! than cut out. Colours are baked into the vertices, so many shapes can share
//! one material and one draw call.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// How far a streak leans from vertical, and how many blobs make one up.
const STREAK_LEAN: f32 = 0.35;
const STREAK_BLOBS: u8 = 5;
const STREAK_SEGMENTS: usize = 7;
/// A curtain of light is a stack of this many blobs. They overlap and add up,
/// so each one alone is faint.
const CURTAIN_BLOBS: u8 = 9;
const CURTAIN_SEGMENTS: usize = 12;
const CURTAIN_BLEND: f32 = 0.45;

/// Vertices and triangles being built up.
#[derive(Default)]
pub(crate) struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    pub(crate) fn with_capacity(vertices: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertices),
            colors: Vec::with_capacity(vertices),
            indices: Vec::with_capacity(vertices * 3),
        }
    }

    pub(crate) fn vertex(&mut self, at: Vec2, color: [f32; 4]) -> u32 {
        self.positions.push([at.x, at.y, 0.0]);
        self.colors.push(color);
        // A tile holds a few thousand vertices at most.
        #[expect(clippy::cast_possible_truncation, reason = "far below u32::MAX")]
        let index = self.positions.len() as u32 - 1;
        index
    }

    pub(crate) fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }

    /// A four-cornered patch, corners given anticlockwise.
    pub(crate) fn quad(&mut self, corners: [u32; 4]) {
        self.triangle(corners[0], corners[1], corners[2]);
        self.triangle(corners[0], corners[2], corners[3]);
    }

    pub(crate) fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// A round mark: solid in the middle, fading to nothing at its rim. This soft
/// edge is what makes shapes drawn in code look painted rather than cut out.
pub(crate) fn soft_disc(
    builder: &mut MeshBuilder,
    at: Vec2,
    radius: Vec2,
    color: [f32; 4],
    segments: usize,
) {
    let rim_color = [color[0], color[1], color[2], 0.0];
    let center = builder.vertex(at, color);
    let mut first = 0;
    let mut previous = 0;
    for step in 0..segments {
        // `segments` is a small constant.
        #[expect(clippy::cast_precision_loss, reason = "tiny segment count")]
        let angle = step as f32 / segments as f32 * core::f32::consts::TAU;
        let point = builder.vertex(at + Vec2::new(angle.cos(), angle.sin()) * radius, rim_color);
        if step == 0 {
            first = point;
        } else {
            builder.triangle(center, previous, point);
        }
        previous = point;
    }
    builder.triangle(center, previous, first);
}

/// A tall column of light: a stack of overlapping soft blobs, brightest in
/// the middle and thinning out at both ends.
///
/// Built from blobs rather than one tall quad on purpose: a quad with
/// transparent corners interpolates along its triangles' diagonals, which
/// shows up as hard creases across something this big.
pub(crate) fn curtain(builder: &mut MeshBuilder, at: Vec2, size: Vec2, color: [f32; 4]) {
    for step in 0..CURTAIN_BLOBS {
        // -1 at the bottom, 0 in the middle, 1 at the top.
        let along = f32::from(step) / f32::from(CURTAIN_BLOBS - 1) * 2.0 - 1.0;
        // Full strength in the middle, fading away at both ends.
        let strength = (1.0 - along * along).max(0.0);
        let blob = [
            color[0],
            color[1],
            color[2],
            color[3] * strength * CURTAIN_BLEND,
        ];
        soft_disc(
            builder,
            at + Vec2::Y * along * size.y / 2.0,
            Vec2::new(size.x / 2.0, size.y / f32::from(CURTAIN_BLOBS) * 1.4),
            blob,
            CURTAIN_SEGMENTS,
        );
    }
}

/// A thin leaning line that fades out at both ends: a meteor, a glint.
pub(crate) fn streak(builder: &mut MeshBuilder, at: Vec2, size: Vec2, color: [f32; 4]) {
    let along = Vec2::new(STREAK_LEAN, 1.0).normalize();
    for step in 0..STREAK_BLOBS {
        let position = f32::from(step) / f32::from(STREAK_BLOBS - 1) * 2.0 - 1.0;
        // Brightest at the head, trailing off behind it.
        let strength = (1.0 + position) / 2.0;
        let blob = [color[0], color[1], color[2], color[3] * strength];
        soft_disc(
            builder,
            at + along * position * size.y / 2.0,
            Vec2::splat(size.x),
            blob,
            STREAK_SEGMENTS,
        );
    }
}

/// A 1×1 quad centered on the origin, split into horizontal bands with one
/// colour per stop (first stop at the top). The GPU blends between rows.
pub(crate) fn gradient_mesh(stops: &[Color]) -> Mesh {
    let mut builder = MeshBuilder::with_capacity(stops.len() * 2);
    for (index, color) in stops.iter().enumerate() {
        // `stops` is a handful of colours, so the conversion is exact.
        #[expect(clippy::cast_precision_loss, reason = "tiny stop count")]
        let y = 0.5 - index as f32 / (stops.len() - 1).max(1) as f32;
        let linear = color.to_linear().to_f32_array();
        builder.vertex(Vec2::new(-0.5, y), linear);
        builder.vertex(Vec2::new(0.5, y), linear);
    }
    // `stops.len()` is small, and there is one band between each pair.
    #[expect(clippy::cast_possible_truncation, reason = "tiny stop count")]
    for band in 0..stops.len().saturating_sub(1) as u32 {
        let top = band * 2;
        builder.quad([top, top + 2, top + 3, top + 1]);
    }
    builder.into_mesh()
}

/// One soft disc of radius 1, white, for anything that wants to be tinted by
/// its material (sparks, glows). Scale it to the size you need.
pub(crate) fn unit_disc(segments: usize) -> Mesh {
    let mut builder = MeshBuilder::with_capacity(segments + 1);
    soft_disc(
        &mut builder,
        Vec2::ZERO,
        Vec2::ONE,
        LinearRgba::WHITE.to_f32_array(),
        segments,
    );
    builder.into_mesh()
}

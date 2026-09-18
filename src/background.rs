//! Sky background: a soft vertical gradient that always fills the screen,
//! plus parallax layers of soft dots that make the fall feel like motion.
//!
//! Placeholder visuals until Milestone 7.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::camera::{CameraSystems, MainCamera};
use crate::rng::SplitMix64;

/// Gradient color stops from the top of the screen to the bottom (deep space).
const SKY_STOPS: [Color; 3] = [
    Color::srgb(0.03, 0.02, 0.10),
    Color::srgb(0.10, 0.06, 0.24),
    Color::srgb(0.22, 0.12, 0.36),
];

/// Distance behind the camera, so the sky is drawn under everything else.
const SKY_DEPTH: f32 = -900.0;

/// One parallax layer of dots.
struct LayerSpec {
    /// How fast the layer scrolls compared with the level (1.0 = moves with the
    /// level). Smaller feels further away.
    parallax: f32,
    dots: usize,
    radius: f32,
    color: Color,
    depth: f32,
}

/// Far to near. Colors are dim (below 1.0) so the background never blooms
/// and never competes with gameplay objects.
const LAYERS: [LayerSpec; 3] = [
    LayerSpec {
        parallax: 0.12,
        dots: 140,
        radius: 2.0,
        color: Color::srgba(0.75, 0.75, 1.0, 0.55),
        depth: -800.0,
    },
    LayerSpec {
        parallax: 0.3,
        dots: 70,
        radius: 3.5,
        color: Color::srgba(0.7, 0.6, 1.0, 0.35),
        depth: -700.0,
    },
    LayerSpec {
        parallax: 0.6,
        dots: 30,
        radius: 7.0,
        color: Color::srgba(0.8, 0.6, 1.0, 0.18),
        depth: -600.0,
    },
];

/// Each layer is a repeating tile this tall, drawn twice (stacked) so it always
/// covers the view, even in a tall portrait window (up to ~3400 units).
const TILE_HEIGHT: f32 = 3600.0;
/// Dots spread wide enough for ultrawide screens.
const TILE_HALF_WIDTH: f32 = 2000.0;
/// Fixed seed: the same starfield every run.
const STAR_SEED: u64 = 0x5747_4152_4452_4F50;

/// The full-screen gradient quad. Its mesh is 1×1, scaled to the visible area.
#[derive(Component)]
struct Sky;

/// A parallax layer's parent entity; its dots are children.
#[derive(Component)]
struct ParallaxLayer {
    parallax: f32,
}

/// Draws the sky gradient and parallax layers.
pub(crate) struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, (spawn_sky, spawn_parallax_layers))
            .add_systems(Update, scroll_parallax.after(CameraSystems::Follow))
            // Projection area only changes on window resize, so this runs rarely.
            .add_systems(
                PostUpdate,
                fit_sky_to_view.after(bevy::camera::CameraUpdateSystems),
            );
    }
}

/// Runs after `Startup` so the camera already exists; the sky is its child and
/// therefore moves with it.
fn spawn_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    camera: Single<Entity, With<MainCamera>>,
) {
    let sky = commands
        .spawn((
            Sky,
            Mesh2d(meshes.add(gradient_mesh(&SKY_STOPS))),
            // White material: the vertex colors provide the gradient.
            MeshMaterial2d(materials.add(ColorMaterial::default())),
            Transform::from_xyz(0.0, 0.0, SKY_DEPTH),
        ))
        .id();
    commands.entity(*camera).add_child(sky);
}

/// Spawns each layer as a parent with two stacked copies of its dot tile.
/// Every dot in a layer shares one mesh and one material, so Bevy draws each
/// layer in a single batch.
fn spawn_parallax_layers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = SplitMix64::new(STAR_SEED);
    for spec in &LAYERS {
        let mesh = meshes.add(Circle::new(spec.radius));
        let material = materials.add(spec.color);
        commands
            .spawn((
                ParallaxLayer {
                    parallax: spec.parallax,
                },
                Transform::from_xyz(0.0, 0.0, spec.depth),
                Visibility::default(),
            ))
            .with_children(|layer| {
                for _ in 0..spec.dots {
                    let x = rng.range(-TILE_HALF_WIDTH, TILE_HALF_WIDTH);
                    let y = rng.range(0.0, TILE_HEIGHT);
                    // Same dot in both copies: the tile repeats seamlessly.
                    for copy_offset in [0.0, -TILE_HEIGHT] {
                        layer.spawn((
                            Mesh2d(mesh.clone()),
                            MeshMaterial2d(material.clone()),
                            Transform::from_xyz(x, y + copy_offset, 0.0),
                        ));
                    }
                }
            });
    }
}

/// Moves each layer so it scrolls at its parallax speed, wrapping the tile.
/// Only the few layer parents move; their dots follow as children.
fn scroll_parallax(
    camera: Single<&Transform, (With<MainCamera>, Changed<Transform>)>,
    mut layers: Query<(&ParallaxLayer, &mut Transform), Without<MainCamera>>,
) {
    let camera_y = camera.translation.y;
    for (layer, mut transform) in &mut layers {
        transform.translation.y = layer_origin(camera_y, layer.parallax);
    }
}

/// Where a layer's tile origin goes so that it appears to scroll by
/// `camera_y * parallax`, while its two copies (covering `origin - TILE_HEIGHT
/// ..origin + TILE_HEIGHT`) always span the view around `camera_y`.
fn layer_origin(camera_y: f32, parallax: f32) -> f32 {
    let scroll = camera_y * parallax;
    let wrapped = (-scroll + TILE_HEIGHT / 2.0).rem_euclid(TILE_HEIGHT) - TILE_HEIGHT / 2.0;
    camera_y + wrapped
}

/// Scales the sky to exactly cover what the camera sees.
fn fit_sky_to_view(
    camera: Single<&Projection, (With<MainCamera>, Changed<Projection>)>,
    mut sky: Single<&mut Transform, With<Sky>>,
) {
    if let Projection::Orthographic(ortho) = *camera {
        let size = ortho.area.size();
        sky.scale = size.extend(1.0);
    }
}

/// A 1×1 quad centered on the origin, split into horizontal bands with one
/// color per stop (first stop at the top). The GPU blends colors between rows.
fn gradient_mesh(stops: &[Color]) -> Mesh {
    let rows = stops.len();
    let mut positions = Vec::with_capacity(rows * 2);
    let mut colors = Vec::with_capacity(rows * 2);
    for (i, color) in stops.iter().enumerate() {
        // `rows` is a handful of stops, so the conversion to f32 is exact.
        #[expect(clippy::cast_precision_loss, reason = "tiny row count")]
        let y = 0.5 - i as f32 / (rows - 1).max(1) as f32;
        let linear = color.to_linear().to_f32_array();
        positions.extend([[-0.5, y, 0.0], [0.5, y, 0.0]]);
        colors.extend([linear, linear]);
    }
    let mut indices = Vec::with_capacity(rows.saturating_sub(1) * 6);
    // One band between each pair of neighboring stops.
    for (band, _) in (0u16..).zip(stops.windows(2)) {
        let top = band * 2;
        // Two counter-clockwise triangles per band.
        indices.extend([top, top + 2, top + 1, top + 1, top + 2, top + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U16(indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_tiles_always_cover_the_view() {
        // The two copies span origin - TILE_HEIGHT .. origin + TILE_HEIGHT.
        let tallest_half_view = 1710.0;
        for camera_y in [0.0, -123.4, -5000.0, -10_799.0, -60_000.0] {
            for spec in &LAYERS {
                let origin = layer_origin(camera_y, spec.parallax);
                assert!(origin - TILE_HEIGHT <= camera_y - tallest_half_view);
                assert!(origin + TILE_HEIGHT >= camera_y + tallest_half_view);
            }
        }
    }

    #[test]
    fn layers_scroll_at_their_parallax_speed() {
        // Away from a wrap point, moving the camera by d moves the layer's
        // content on screen by d * parallax (relative to a fixed world point).
        let parallax = 0.3;
        let a = layer_origin(-100.0, parallax) - (-100.0);
        let b = layer_origin(-110.0, parallax) - (-110.0);
        assert!(((b - a) - 10.0 * parallax).abs() < 1e-3);
    }
}

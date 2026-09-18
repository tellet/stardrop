//! Sky background: a soft vertical gradient that always fills the screen,
//! plus a glowing test circle to check that HDR and bloom work.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::camera::MainCamera;

/// Gradient color stops from the top of the screen to the bottom (deep space).
const SKY_STOPS: [Color; 3] = [
    Color::srgb(0.03, 0.02, 0.10),
    Color::srgb(0.10, 0.06, 0.24),
    Color::srgb(0.22, 0.12, 0.36),
];

/// Distance behind the camera, so the sky is drawn under everything else.
const SKY_DEPTH: f32 = -900.0;

/// HDR color with channels above 1.0: bright enough for bloom to pick up.
const TEST_GLOW: Color = Color::linear_rgb(1.0, 0.55, 3.2);
const TEST_CIRCLE_RADIUS: f32 = 60.0;

/// The full-screen gradient quad. Its mesh is 1×1, scaled to the visible area.
#[derive(Component)]
struct Sky;

/// Draws the sky gradient and the test circle.
pub(crate) struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_sky)
            .add_systems(Startup, spawn_test_circle)
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

fn spawn_test_circle(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(TEST_CIRCLE_RADIUS))),
        MeshMaterial2d(materials.add(TEST_GLOW)),
    ));
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

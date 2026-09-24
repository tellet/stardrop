//! The sky behind the playfield: a colour gradient, three to five layers of
//! drifting scenery, and the dimmed surround that frames the playfield.
//!
//! Every layer is one baked mesh: all its stars (or clouds, curtains, streaks)
//! live in a single vertex buffer with their colours baked in, so a whole
//! layer costs one entity and one draw call. The meshes are built once per
//! theme and kept, so moving between levels never rebuilds them.
//!
//! The camera never moves, so the layers drift on their own.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::sprite_render::AlphaMode2d;

use crate::camera::{MainCamera, PLAYFIELD, PLAYFIELD_CENTER};
use crate::level::{ActiveLevel, Theme};
use crate::rng::SplitMix64;
use crate::settings::Settings;
use crate::soft_mesh::{MeshBuilder, curtain, gradient_mesh, soft_disc, streak};
use crate::theme::{LayerSpec, MENU_THEME, Mark, ThemeArt, art};

/// Distance behind the camera, so the sky is drawn under everything else.
const SKY_DEPTH: f32 = -900.0;
/// Each layer is a repeating tile this tall, drawn twice (stacked) so it always
/// covers the view: the playfield is 1920 tall, and windows narrower than 9:16
/// show up to ~3400.
const TILE_HEIGHT: f32 = 3600.0;
/// Marks spread wide enough for ultrawide screens (32:9 shows ~6800 across).
const TILE_HALF_WIDTH: f32 = 3500.0;
/// Upward drift of a layer with `parallax` 1.0, in units per second. Calm:
/// the nearest layer takes about ten seconds to cross the playfield.
const DRIFT_SPEED: f32 = 300.0;
/// Seconds for one full sideways sway.
const SWAY_PERIOD: f32 = 26.0;
/// Deep enough to cover any window shape beside the playfield.
const VEIL_SIZE: Vec2 = Vec2::new(4000.0, 8000.0);
const EDGE_WIDTH: f32 = 3.0;
/// Veils sit above the scenery layers, below everything in play.
const VEIL_DEPTH: f32 = -550.0;
const EDGE_DEPTH: f32 = -540.0;
/// Fixed seed: the same sky every run.
const SKY_SEED: u64 = 0x5747_4152_4452_4F50;
/// Segments around a soft disc. Small marks need fewer.
const STAR_SEGMENTS: usize = 6;
const PUFF_SEGMENTS: usize = 14;
/// Clouds are wider than they are tall.
const PUFF_SQUASH: f32 = 0.62;

/// The sky the game is currently showing.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub(crate) struct CurrentTheme(pub Theme);

impl Default for CurrentTheme {
    fn default() -> Self {
        Self(MENU_THEME)
    }
}

/// The full-screen gradient quad. Its mesh is 1×1, scaled to the visible area.
#[derive(Component)]
struct Sky;

/// Anything rebuilt when the theme changes.
#[derive(Component)]
struct BackgroundPiece;

/// A layer's parent entity; its two tile copies are children.
#[derive(Component)]
struct ParallaxLayer {
    parallax: f32,
    sway: f32,
    /// Keeps layers from swaying in step with each other.
    phase: f32,
}

/// Meshes and materials for the themes visited so far. Built on first use and
/// kept: there are only ten, and a level change must not rebuild anything.
#[derive(Resource, Default)]
struct SkyAssets {
    /// One material for everything whose colour is baked into its vertices.
    tint: Handle<ColorMaterial>,
    unit_square: Handle<Mesh>,
    built: HashMap<Theme, ThemeAssets>,
}

/// One theme's built meshes and materials.
struct ThemeAssets {
    gradient: Handle<Mesh>,
    layers: Vec<Handle<Mesh>>,
    veil: Handle<ColorMaterial>,
    edge: Handle<ColorMaterial>,
}

/// Draws the sky gradient, drifting layers and playfield frame.
pub(crate) struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTheme>()
            .init_resource::<SkyAssets>()
            .add_systems(Startup, prepare_shared_assets)
            // After `Startup`, so the camera the sky hangs from exists.
            .add_systems(PostStartup, rebuild_background)
            .add_systems(
                Update,
                (follow_level_theme, rebuild_on_theme_change).chain(),
            )
            .add_systems(Update, drift_layers)
            // Projection area only changes on window resize, so this runs rarely.
            .add_systems(
                PostUpdate,
                fit_sky_to_view.after(bevy::camera::CameraUpdateSystems),
            );
    }
}

fn prepare_shared_assets(
    mut assets: ResMut<SkyAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    assets.tint = materials.add(ColorMaterial {
        // White: the mesh's own vertex colours provide every colour. Blending
        // is explicit because those colours carry the transparency.
        color: Color::WHITE,
        alpha_mode: AlphaMode2d::Blend,
        ..default()
    });
    assets.unit_square = meshes.add(Rectangle::new(1.0, 1.0));
}

/// The sky follows the level being played, and falls back to the menu sky
/// when there's no level (menus, the journal, between levels).
fn follow_level_theme(level: Option<Res<ActiveLevel>>, mut current: ResMut<CurrentTheme>) {
    let wanted = CurrentTheme(level.map_or(MENU_THEME, |level| level.theme));
    current.set_if_neq(wanted);
}

fn rebuild_on_theme_change(
    theme: Res<CurrentTheme>,
    commands: Commands,
    assets: ResMut<SkyAssets>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    old: Query<Entity, With<BackgroundPiece>>,
    camera: Single<Entity, With<MainCamera>>,
) {
    if !theme.is_changed() || theme.is_added() {
        return;
    }
    rebuild_background(theme, commands, assets, meshes, materials, old, camera);
}

/// Clears the old sky and builds the current theme's, reusing any meshes and
/// materials already built for it.
fn rebuild_background(
    theme: Res<CurrentTheme>,
    mut commands: Commands,
    mut assets: ResMut<SkyAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    old: Query<Entity, With<BackgroundPiece>>,
    camera: Single<Entity, With<MainCamera>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let art = art(**theme);
    build_theme_assets(**theme, art, &mut assets, &mut meshes, &mut materials);
    let Some(built) = assets.built.get(&theme.0) else {
        return;
    };

    // Anything the sky quad doesn't reach (a window shape the view doesn't
    // fill exactly) shows this instead of Bevy's grey.
    commands.insert_resource(ClearColor(art.sky[art.sky.len() - 1]));
    let sky = commands
        .spawn((
            Sky,
            BackgroundPiece,
            Mesh2d(built.gradient.clone()),
            MeshMaterial2d(assets.tint.clone()),
            Transform::from_xyz(0.0, 0.0, SKY_DEPTH),
        ))
        .id();
    commands.entity(*camera).add_child(sky);

    for (index, (spec, mesh)) in art.layers.iter().zip(&built.layers).enumerate() {
        // A whole number of turns apart, so no two layers sway together.
        #[expect(clippy::cast_precision_loss, reason = "at most five layers")]
        let phase = index as f32 * 1.7;
        commands
            .spawn((
                BackgroundPiece,
                ParallaxLayer {
                    parallax: spec.parallax,
                    sway: spec.sway,
                    phase,
                },
                Transform::from_xyz(0.0, PLAYFIELD_CENTER.y, spec.depth),
                Visibility::default(),
            ))
            .with_children(|layer| {
                // Two copies of the tile, stacked, so the layer always covers
                // the view however far it has drifted.
                for offset in [0.0, -TILE_HEIGHT] {
                    layer.spawn((
                        Mesh2d(mesh.clone()),
                        MeshMaterial2d(assets.tint.clone()),
                        Transform::from_xyz(0.0, offset, 0.0),
                    ));
                }
            });
    }

    spawn_playfield_frame(&mut commands, &assets.unit_square, built);
}

/// Builds a theme's meshes and materials the first time it's shown.
fn build_theme_assets(
    theme: Theme,
    art: &ThemeArt,
    assets: &mut SkyAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    if assets.built.contains_key(&theme) {
        return;
    }
    // Every theme lays its marks out from the same seed, so the sky is the
    // same every run and every visit.
    let mut rng = SplitMix64::new(SKY_SEED);
    let layers = art
        .layers
        .iter()
        .map(|spec| meshes.add(tile_mesh(spec, &mut rng)))
        .collect();
    assets.built.insert(
        theme,
        ThemeAssets {
            gradient: meshes.add(gradient_mesh(&art.sky)),
            layers,
            veil: materials.add(art.veil),
            edge: materials.add(art.edge),
        },
    );
}

/// Dims everything beside the playfield and marks its side edges.
fn spawn_playfield_frame(commands: &mut Commands, unit_square: &Handle<Mesh>, built: &ThemeAssets) {
    let half_width = PLAYFIELD.x / 2.0;
    for side in [-1.0_f32, 1.0] {
        commands.spawn((
            BackgroundPiece,
            Mesh2d(unit_square.clone()),
            MeshMaterial2d(built.veil.clone()),
            Transform::from_xyz(
                side * (half_width + VEIL_SIZE.x / 2.0),
                PLAYFIELD_CENTER.y,
                VEIL_DEPTH,
            )
            .with_scale(VEIL_SIZE.extend(1.0)),
        ));
        commands.spawn((
            BackgroundPiece,
            Mesh2d(unit_square.clone()),
            MeshMaterial2d(built.edge.clone()),
            Transform::from_xyz(side * half_width, PLAYFIELD_CENTER.y, EDGE_DEPTH)
                .with_scale(Vec3::new(EDGE_WIDTH, PLAYFIELD.y, 1.0)),
        ));
    }
}

/// Drifts each layer upward at its own speed and sways it gently sideways.
/// Only the few layer parents move; their tiles follow as children.
fn drift_layers(
    time: Res<Time>,
    settings: Res<Settings>,
    mut drifted: Local<f32>,
    mut swayed: Local<f32>,
    mut layers: Query<(&ParallaxLayer, &mut Transform)>,
) {
    // Accumulated rather than read from the clock, so changing "reduce
    // motion" slows the drift instead of jumping it.
    let scale = settings.motion_scale();
    *drifted += time.delta_secs() * DRIFT_SPEED * scale;
    *swayed += time.delta_secs() * scale;
    let (drifted, swayed) = (*drifted, *swayed);
    for (layer, mut transform) in &mut layers {
        transform.translation.y = layer_origin(PLAYFIELD_CENTER.y, drifted * layer.parallax);
        transform.translation.x =
            layer.sway * (swayed * core::f32::consts::TAU / SWAY_PERIOD + layer.phase).sin();
    }
}

/// Where a layer's tile origin goes when its content has drifted `scroll`
/// units upward, so that its two copies (covering `origin - TILE_HEIGHT ..
/// origin + TILE_HEIGHT`) always span the view around `center_y`.
fn layer_origin(center_y: f32, scroll: f32) -> f32 {
    let wrapped = (scroll + TILE_HEIGHT / 2.0).rem_euclid(TILE_HEIGHT) - TILE_HEIGHT / 2.0;
    center_y + wrapped
}

/// Scales the sky to exactly cover what the camera sees. Cheap enough to
/// check every frame (one camera, one sky), and it has to be: the sky is
/// rebuilt whenever the theme changes, not only when the window resizes.
fn fit_sky_to_view(
    camera: Single<&Projection, With<MainCamera>>,
    sky: Option<Single<&mut Transform, With<Sky>>>,
) {
    let (Projection::Orthographic(ortho), Some(mut sky)) = (*camera, sky) else {
        return;
    };
    let wanted = ortho.area.size().extend(1.0);
    if sky.scale != wanted {
        sky.scale = wanted;
    }
}

/// Every mark of one layer, baked into a single mesh tile.
fn tile_mesh(spec: &LayerSpec, rng: &mut SplitMix64) -> Mesh {
    let mut builder = MeshBuilder::with_capacity(spec.count * (PUFF_SEGMENTS + 1));
    let color = spec.color.to_linear().to_f32_array();
    for _ in 0..spec.count {
        let at = Vec2::new(
            rng.range(-TILE_HALF_WIDTH, TILE_HALF_WIDTH),
            rng.range(0.0, TILE_HEIGHT),
        );
        // A little variation in size, so a layer never looks stamped out.
        let scale = rng.range(0.7, 1.3);
        match spec.mark {
            Mark::Star => soft_disc(
                &mut builder,
                at,
                Vec2::splat(spec.size.x * scale),
                color,
                STAR_SEGMENTS,
            ),
            Mark::Puff => soft_disc(
                &mut builder,
                at,
                Vec2::new(spec.size.x * scale, spec.size.y * scale * PUFF_SQUASH),
                color,
                PUFF_SEGMENTS,
            ),
            Mark::Curtain => curtain(&mut builder, at, spec.size * scale, color),
            Mark::Streak => streak(&mut builder, at, spec.size * scale, color),
        }
    }
    builder.into_mesh()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_tiles_always_cover_the_view() {
        // The two copies span origin - TILE_HEIGHT .. origin + TILE_HEIGHT.
        let tallest_half_view = 1710.0;
        for scroll in [0.0, 123.4, 1799.0, 1801.0, 5000.0, 60_000.0] {
            let origin = layer_origin(PLAYFIELD_CENTER.y, scroll);
            assert!(origin - TILE_HEIGHT <= PLAYFIELD_CENTER.y - tallest_half_view);
            assert!(origin + TILE_HEIGHT >= PLAYFIELD_CENTER.y + tallest_half_view);
        }
    }

    #[test]
    fn layers_drift_upward_smoothly() {
        // Away from a wrap point, drifting 10 more units moves the tile up by 10.
        let a = layer_origin(PLAYFIELD_CENTER.y, 100.0);
        let b = layer_origin(PLAYFIELD_CENTER.y, 110.0);
        assert!(((b - a) - 10.0).abs() < 1e-3);
    }

    /// Every theme's layers must bake into meshes the renderer can draw:
    /// whole triangles, indices in range, and marks inside the tile.
    #[test]
    fn every_theme_bakes_into_drawable_tiles() {
        for theme in Theme::ALL {
            let mut rng = SplitMix64::new(SKY_SEED);
            for spec in art(theme).layers {
                let mesh = tile_mesh(spec, &mut rng);
                let vertices = mesh.count_vertices();
                assert!(vertices > 0, "{theme:?}: an empty layer");
                let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
                    panic!("{theme:?}: a layer has no triangles");
                };
                assert_eq!(indices.len() % 3, 0, "{theme:?}: a half triangle");
                // Vertex counts are far below u32::MAX.
                #[expect(clippy::cast_possible_truncation, reason = "small meshes")]
                let count = vertices as u32;
                assert!(
                    indices.iter().all(|index| *index < count),
                    "{theme:?}: a triangle points at a vertex that isn't there"
                );
            }
        }
    }
}

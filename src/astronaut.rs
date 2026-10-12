//! The astronaut: spawning it and turning player input into steering.

use bevy::input::gamepad::{AxisSettings, GamepadSettings};
use bevy::prelude::*;

use crate::config::GameConfig;
use crate::level::{ActiveLevel, LevelPhase};
use crate::physics::movement::AirInput;
use crate::physics::{AirControl, Motion};
use crate::settings::Settings;
use crate::soft_mesh::unit_disc;

/// The suit: pale, lit from above, bright enough for a little bloom.
const SUIT_COLOR: Color = Color::linear_rgb(1.5, 1.45, 1.6);
/// The shaded underside of the suit and the pack behind it.
const SUIT_SHADE: Color = Color::linear_rgb(0.42, 0.40, 0.52);
/// The life-support pack: darker still, and squarer.
const PACK_COLOR: Color = Color::linear_rgb(0.30, 0.28, 0.40);
/// The lamp on the shoulder, which is what you actually see from far away.
const LAMP_COLOR: Color = Color::linear_rgba(3.0, 2.3, 1.2, 1.0);
/// The soft halo around them: dimmer and much wider than the body.
const HALO_COLOR: Color = Color::linear_rgba(1.6, 1.1, 0.7, 0.30);
const HALO_RADIUS: f32 = 3.0;
/// The visor: deep sky blue, so the figure always has a readable "front".
const VISOR_COLOR: Color = Color::linear_rgb(0.05, 0.14, 0.30);
/// A thin bright line along the top of the visor.
const VISOR_RIM: Color = Color::linear_rgba(0.9, 1.8, 2.4, 0.9);
/// The figure, in fractions of the collision radius. A helmet that fills
/// the whole circle reads as a ball; this one sits on shoulders, and the
/// visor is a window in it rather than most of its face.
const HELMET_SIZE: Vec2 = Vec2::new(0.74, 0.74);
const HELMET_RISE: f32 = 0.34;
const VISOR_SIZE: Vec2 = Vec2::new(0.52, 0.38);
const VISOR_OFFSET: f32 = 0.40;
/// The body under the helmet, and the shoulders either side of it.
const BODY_SIZE: Vec2 = Vec2::new(0.92, 0.72);
const BODY_DROP: f32 = 0.46;
const SHOULDER: Vec2 = Vec2::new(0.68, 0.30);
const SHOULDER_SIZE: f32 = 0.38;
/// The flash across the visor when the astronaut touches something.
const VISOR_FLASH_COLOR: Color = Color::linear_rgba(3.0, 3.0, 2.6, 0.9);

/// How far the astronaut leans at full sideways speed.
const MAX_LEAN: f32 = 0.42;
/// How quickly the lean catches up, per second. Low enough to lag behind the
/// movement a little, which is what makes it read as weight.
const LEAN_RATE: f32 = 5.0;
/// Drawn in front of the background layers.
const ASTRONAUT_DEPTH: f32 = 10.0;
/// Stick positions closer to center than this count as zero. 15 % is the
/// common default: worn sticks rarely rest exactly at center.
const STICK_DEAD_ZONE: f32 = 0.15;

/// Ordering labels for systems that depend on this frame's input.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum AstronautSystems {
    /// Reads keyboard and gamepad into `AirControl`.
    Input,
}

/// The bubble shown around the astronaut while one is carrying it.
#[derive(Component)]
pub(crate) struct CaptureBubble;

const CAPTURE_COLOR: Color = Color::linear_rgba(0.9, 1.6, 2.0, 0.5);
const CAPTURE_GAP: f32 = 16.0;

/// The shimmer shown around the astronaut while spores have swapped the
/// controls: a soft magenta ring, hidden the rest of the time.
#[derive(Component)]
pub(crate) struct Shimmer;

const SHIMMER_COLOR: Color = Color::linear_rgba(1.6, 0.4, 1.4, 0.55);
const SHIMMER_THICKNESS: f32 = 7.0;

/// Marks the player's astronaut.
#[derive(Component)]
pub struct Astronaut;

/// The bright flash over the visor, shown for a moment on contact.
#[derive(Component)]
pub(crate) struct VisorFlash;

/// How far the astronaut is leaning right now.
#[derive(Component, Debug, Default)]
pub(crate) struct Tumble(f32);

/// Spawns the astronaut and reads its controls.
pub(crate) struct AstronautPlugin;

impl Plugin for AstronautPlugin {
    fn build(&self, app: &mut App) {
        // The astronaut appears once, when the first valid level has loaded (it
        // needs the start point). Not `OnEnter(Falling)`: respawns and retries
        // return to Falling too, and would each spawn another astronaut.
        app.add_systems(OnExit(LevelPhase::Loading), spawn_astronaut)
            .add_systems(
                Update,
                (configure_stick_dead_zone, read_air_control)
                    .chain()
                    .in_set(AstronautSystems::Input),
            )
            .add_systems(Update, (tumble_with_movement, show_only_in_a_level));
    }
}

fn spawn_astronaut(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    config: Res<GameConfig>,
    level: Res<ActiveLevel>,
) {
    let figure = spawn_figure(
        &mut commands,
        meshes,
        materials,
        config.astronaut_radius,
        level.start,
        (Astronaut, Tumble::default(), Motion::at_rest(level.start)),
    );
    let _ = figure;
}

/// Draws the astronaut: the one place the figure is described, so the
/// journal, the golden-image check and the game all show the same person.
///
/// `extra` is whatever makes this one *the* astronaut rather than a picture
/// of one.
pub fn spawn_figure(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    radius: f32,
    at: Vec2,
    extra: impl Bundle,
) -> Entity {
    let ring = meshes.add(Annulus::new(
        radius + SHIMMER_THICKNESS,
        radius + SHIMMER_THICKNESS * 2.0,
    ));
    let disc = meshes.add(Circle::new(1.0));
    let glow = meshes.add(unit_disc(24));
    // A small person in a suit: shoulders, a body, a domed helmet with a
    // window in it, and a lamp. At 28 units across, every part has to earn
    // its place — anything smaller than a fifth of that is a smudge.
    commands
        .spawn((
            extra,
            Transform::from_translation(at.extend(ASTRONAUT_DEPTH)),
            Visibility::default(),
        ))
        // The light they give off, behind everything else.
        .with_child((
            Mesh2d(glow.clone()),
            MeshMaterial2d(materials.add(HALO_COLOR)),
            Transform::from_xyz(0.0, 0.0, -1.0).with_scale(Vec3::splat(radius * HALO_RADIUS)),
        ))
        // The pack, just showing behind one shoulder.
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(PACK_COLOR)),
            Transform::from_xyz(-radius * 0.22, -radius * 0.52, -0.3).with_scale(Vec3::new(
                radius * 0.66,
                radius * 0.52,
                1.0,
            )),
        ))
        // Shoulders, which are what make it a person and not a bottle.
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(SUIT_SHADE)),
            Transform::from_xyz(-radius * SHOULDER.x, -radius * SHOULDER.y, -0.15)
                .with_scale(Vec3::splat(radius * SHOULDER_SIZE)),
        ))
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(SUIT_SHADE)),
            Transform::from_xyz(radius * SHOULDER.x, -radius * SHOULDER.y, -0.15)
                .with_scale(Vec3::splat(radius * SHOULDER_SIZE)),
        ))
        // The body.
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(SUIT_COLOR)),
            Transform::from_xyz(0.0, -radius * BODY_DROP, -0.1)
                .with_scale((BODY_SIZE * radius).extend(1.0)),
        ))
        // The helmet, and the window in it.
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(SUIT_COLOR)),
            Transform::from_xyz(0.0, radius * HELMET_RISE, 0.0)
                .with_scale((HELMET_SIZE * radius).extend(1.0)),
        ))
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(VISOR_COLOR)),
            Transform::from_xyz(0.0, radius * VISOR_OFFSET, 0.1)
                .with_scale((VISOR_SIZE * radius).extend(1.0)),
        ))
        // The sky, caught along the top of the glass.
        .with_child((
            Mesh2d(disc.clone()),
            MeshMaterial2d(materials.add(VISOR_RIM)),
            Transform::from_xyz(-radius * 0.08, radius * 0.50, 0.2).with_scale(Vec3::new(
                radius * 0.24,
                radius * 0.07,
                1.0,
            )),
        ))
        // The shoulder lamp: the one part visible right across a level.
        .with_child((
            Mesh2d(glow),
            MeshMaterial2d(materials.add(LAMP_COLOR)),
            Transform::from_xyz(radius * 0.66, -radius * 0.3, 0.3)
                .with_scale(Vec3::splat(radius * 0.22)),
        ))
        .with_child((
            VisorFlash,
            Mesh2d(disc),
            MeshMaterial2d(materials.add(VISOR_FLASH_COLOR)),
            Transform::from_xyz(0.0, radius * VISOR_OFFSET, 0.4)
                .with_scale((VISOR_SIZE * radius).extend(1.0)),
            Visibility::Hidden,
        ))
        .with_child((
            Shimmer,
            Mesh2d(ring),
            MeshMaterial2d(materials.add(SHIMMER_COLOR)),
            Visibility::Hidden,
        ))
        .with_child((
            CaptureBubble,
            Mesh2d(meshes.add(Annulus::new(
                radius + CAPTURE_GAP,
                radius + CAPTURE_GAP + 4.0,
            ))),
            MeshMaterial2d(materials.add(CAPTURE_COLOR)),
            Visibility::Hidden,
        ))
        .id()
}

/// The astronaut is only on screen while a level is: menus unload the level,
/// and an astronaut hanging in a menu looks like a mistake.
fn show_only_in_a_level(
    level: Option<Res<ActiveLevel>>,
    mut astronaut: Query<&mut Visibility, With<Astronaut>>,
) {
    let wanted = if level.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut astronaut {
        visibility.set_if_neq(wanted);
    }
}

/// How far the astronaut leans when drifting sideways at `velocity_x`, given
/// the fastest they can steer. They lean into the drift, and never past
/// [`MAX_LEAN`] however fast a bounce or a dive throws them.
#[must_use]
pub(crate) fn lean_angle(velocity_x: f32, max_speed: f32) -> f32 {
    -(velocity_x / max_speed).clamp(-1.0, 1.0) * MAX_LEAN
}

/// How big the visor flash is with `left` of its life to go (1 at the start,
/// 0 at the end): it opens instantly and eases away.
#[must_use]
pub(crate) fn visor_flash_scale(left: f32) -> Vec2 {
    let left = left.clamp(0.0, 1.0);
    VISOR_SIZE * (left * left * (3.0 - 2.0 * left))
}

/// Eases `current` towards `target`; frame-rate independent.
#[must_use]
pub(crate) fn approach(current: f32, target: f32, rate: f32, delta: f32) -> f32 {
    current + (target - current) * (1.0 - (-rate * delta).exp())
}

/// Leans the astronaut into their sideways drift. Drawing (not physics), so
/// it runs every frame and eases rather than snapping.
fn tumble_with_movement(
    time: Res<Time>,
    settings: Res<Settings>,
    config: Res<GameConfig>,
    astronaut: Option<Single<(&Motion, &mut Tumble, &mut Transform), With<Astronaut>>>,
) {
    let Some(astronaut) = astronaut else {
        return;
    };
    let (motion, mut tumble, mut transform) = astronaut.into_inner();
    let target = lean_angle(motion.body().velocity.x, config.movement.max_air_speed)
        * settings.motion_scale();
    tumble.0 = approach(tumble.0, target, LEAN_RATE, time.delta_secs());
    transform.rotation = Quat::from_rotation_z(tumble.0);
}

/// Each gamepad is an entity with its own settings; apply our dead zone when
/// one connects. `Added<Gamepad>` means this only touches newly connected pads.
fn configure_stick_dead_zone(mut new_pads: Query<&mut GamepadSettings, Added<Gamepad>>) {
    for mut settings in &mut new_pads {
        // `AxisSettings::new` checks its arguments and returns a `Result`, so a
        // bad constant is reported instead of crashing the game.
        match AxisSettings::new(-1.0, -STICK_DEAD_ZONE, STICK_DEAD_ZONE, 1.0, 0.01) {
            Ok(axis) => settings.default_axis_settings = axis,
            Err(error) => warn!("Keeping default stick dead zone: {error}"),
        }
    }
}

/// Samples keyboard and the first connected gamepad into [`AirControl`].
fn read_air_control(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut control: ResMut<AirControl>,
) {
    let left = keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let stick_x = gamepads.iter().next().map_or(0.0, |pad| pad.left_stick().x);
    let input = AirInput::from_controls(left, right, stick_x);
    // Only write when it changes, so change detection stays meaningful.
    control.set_if_neq(AirControl(input));
}

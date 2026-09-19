//! The astronaut: spawning it and turning player input into steering.

use bevy::input::gamepad::{AxisSettings, GamepadSettings};
use bevy::prelude::*;

use crate::config::GameConfig;
use crate::level::{ActiveLevel, LevelPhase};
use crate::physics::movement::AirInput;
use crate::physics::{AirControl, Motion};

/// Placeholder look until Milestone 7: a warm glow, bright enough for bloom.
const ASTRONAUT_GLOW: Color = Color::linear_rgb(2.2, 1.6, 0.9);
/// Drawn in front of the background layers.
const ASTRONAUT_DEPTH: f32 = 10.0;
/// Stick positions closer to center than this count as zero. 15 % is the
/// common default: worn sticks rarely rest exactly at center.
const STICK_DEAD_ZONE: f32 = 0.15;

/// Marks the player's astronaut.
#[derive(Component)]
pub(crate) struct Astronaut;

/// Spawns the astronaut and reads its controls.
pub(crate) struct AstronautPlugin;

impl Plugin for AstronautPlugin {
    fn build(&self, app: &mut App) {
        // The astronaut appears once the level has loaded (it needs the start point).
        app.add_systems(OnEnter(LevelPhase::Falling), spawn_astronaut)
            .add_systems(
                Update,
                (configure_stick_dead_zone, read_air_control).chain(),
            );
    }
}

fn spawn_astronaut(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    config: Res<GameConfig>,
    level: Res<ActiveLevel>,
) {
    commands.spawn((
        Astronaut,
        Motion::at_rest(level.start),
        Mesh2d(meshes.add(Circle::new(config.astronaut_radius))),
        MeshMaterial2d(materials.add(ASTRONAUT_GLOW)),
        Transform::from_translation(level.start.extend(ASTRONAUT_DEPTH)),
    ));
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

//! F2 (with `--features dev`): draws what the game actually collides with.
//!
//! Outlines show each object's visible size, hitboxes show what really
//! counts (hazards are smaller than they look), plus the astronaut, beacon
//! and stardust pickup circles.

use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::config::GameConfig;
use crate::level::{ActiveLevel, MOTE_RADIUS};
use crate::objects::ObjectShape;
use crate::physics::Motion;

const OUTLINE: Color = Color::srgba(1.0, 1.0, 1.0, 0.35);
const HITBOX: Color = Color::srgb(1.0, 0.35, 0.35);
const PLAYER: Color = Color::srgb(0.4, 1.0, 0.5);
const GOAL: Color = Color::srgb(0.4, 0.9, 1.0);

/// Whether F2 shapes are shown.
#[derive(Resource, Debug, Default)]
struct ShowShapes(bool);

/// F2 debug shapes.
pub(super) struct ShapesPlugin;

impl Plugin for ShapesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShowShapes>().add_systems(
            Update,
            (
                toggle,
                draw.run_if(|show: Res<ShowShapes>| show.0)
                    .run_if(resource_exists::<ActiveLevel>),
            ),
        );
    }
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, mut show: ResMut<ShowShapes>) {
    if keys.just_pressed(KeyCode::F2) {
        show.0 = !show.0;
    }
}

fn draw(
    mut gizmos: Gizmos,
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    astronaut: Query<&Motion, With<Astronaut>>,
) {
    for placement in &level.objects {
        draw_shape(&mut gizmos, placement.outline(), OUTLINE);
        draw_shape(&mut gizmos, placement.hitbox(&config.objects), HITBOX);
    }
    gizmos.rect_2d(level.beacon.center, level.beacon.size, GOAL);
    for &mote in &level.stardust {
        gizmos.circle_2d(mote, MOTE_RADIUS, GOAL);
    }
    for motion in &astronaut {
        gizmos.circle_2d(motion.body().position, config.astronaut_radius, PLAYER);
    }
}

/// Circles as circles; paths as their centerline plus the reach around each
/// point (the capsule ends).
fn draw_shape(gizmos: &mut Gizmos, shape: ObjectShape, color: Color) {
    match shape {
        ObjectShape::Circle(circle) => {
            gizmos.circle_2d(circle.center, circle.radius, color);
        }
        ObjectShape::Pair(ends) => {
            for end in ends {
                gizmos.circle_2d(end.center, end.radius, color);
            }
        }
        ObjectShape::Path(path) => {
            gizmos.linestrip_2d(path.points.iter().copied(), color);
            for &point in path.points {
                gizmos.circle_2d(point, path.half_width, color);
            }
        }
    }
}

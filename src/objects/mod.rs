//! Objects in the sky: placing them from the level, detecting contact and
//! applying what they do.
//!
//! Every object of a type behaves identically (the consistency rule): the
//! placement comes from the level, the behavior only from the type's tuning
//! in `GameConfig`. An effect fires when the astronaut *enters* an object,
//! not every frame it stays inside.

use bevy::prelude::*;

mod behavior;
mod kind;
mod visuals;

pub use behavior::{Outcome, on_enter, while_inside};
pub use kind::{
    BubbleConfig, CloudPuffConfig, CometStoneConfig, CrystalConfig, GravityOrbConfig,
    MAX_RIBBON_POINTS, ObjectKind, ObjectShape, ObjectsConfig, Placement, SporeFlowerConfig,
    StaticCloudConfig, TwinPortalsConfig, WindRibbonConfig,
};
use visuals::ObjectAssets;

use crate::astronaut::Astronaut;
use crate::attempt::{AttemptFailed, AttemptRestarted, fail};
use crate::config::GameConfig;
use crate::level::{ActiveLevel, LevelPhase, LevelSystems, still_falling};
use crate::physics::Motion;
use crate::physics::movement::{Body, Drift};

/// Sent when the astronaut starts touching an object.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ObjectTouched {
    /// The object's type.
    pub kind: ObjectKind,
    /// The object that was touched.
    pub entity: Entity,
}

/// An object entity: which of the level's objects it shows.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ObjectIndex(pub usize);

/// Whether the astronaut touched this object in the last physics step, so
/// effects fire on entering, once.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
struct Contact(bool);

/// Whether a one-use object (a bubble) has been spent this attempt.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
struct Used(bool);

/// Objects: spawning, contact and effects.
pub(crate) struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ObjectTouched>()
            .add_systems(Startup, |mut commands: Commands| {
                commands.init_resource::<ObjectAssets>();
            })
            .add_systems(
                FixedUpdate,
                touch_objects
                    .in_set(LevelSystems::Rules)
                    .run_if(still_falling),
            )
            .add_systems(
                Update,
                (
                    spawn_objects.run_if(resource_exists_and_changed::<ActiveLevel>),
                    forget_contacts.run_if(on_message::<AttemptRestarted>),
                ),
            );
    }
}

/// Replaces the object entities whenever the level (re)loads.
fn spawn_objects(
    mut commands: Commands,
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    assets: Res<ObjectAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    old: Query<Entity, With<ObjectIndex>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    for (index, placement) in level.objects.iter().enumerate() {
        commands
            .spawn((
                ObjectIndex(index),
                Contact::default(),
                Used::default(),
                crate::effects::ContactPulse::default(),
                visuals::transform_for(placement),
                Visibility::default(),
            ))
            .with_children(|parent| {
                visuals::spawn_look(parent, &assets, &mut meshes, &config.objects, placement);
            });
    }
}

/// Finds objects the astronaut touches and applies their effects: instant
/// ones on entering, continuous ones (wind) on every step inside.
///
/// A plain loop over the level's objects: one screen holds a few dozen at
/// most, which a loop checks in well under a microsecond (see `benches/`).
fn touch_objects(
    time: Res<Time>,
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    // One mutable handle for reading *and* writing: asking Bevy for `&Motion`
    // and `&mut Motion` of the same entity in one system is a conflict it
    // refuses at startup (the same rule as Rust's borrowing).
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut objects: Query<(
        Entity,
        &ObjectIndex,
        &mut Contact,
        &mut Used,
        &mut Visibility,
    )>,
    mut touched: MessageWriter<ObjectTouched>,
    mut failed: MessageWriter<AttemptFailed>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    for (entity, index, mut contact, mut used, mut visibility) in &mut objects {
        if used.0 {
            continue;
        }
        // Re-read each time: an earlier object this step may have launched us.
        let body = astronaut.body();
        // `get` instead of `[]`: after a level edit the list may be shorter.
        let Some(placement) = level.objects.get(index.0) else {
            continue;
        };
        let touching = placement.touches(&config.objects, body.position, config.astronaut_radius);
        let was_touching = contact.0;
        if touching != was_touching {
            contact.0 = touching;
        }
        if !touching {
            continue;
        }
        let kind = placement.kind();
        // Inside a bubble, only hazards reach the astronaut: nothing else
        // pushes, pulls or carries it (DECISIONS.md M4-2).
        if body.effects.held_by_bubble() && kind != ObjectKind::StaticCloud {
            continue;
        }
        // Instant effects fire on entering only: touching now, not before.
        if !was_touching {
            touched.write(ObjectTouched { kind, entity });
            match on_enter(placement, &config.objects, &config.movement, body) {
                Outcome::Nothing => {}
                // A jump, not a move: no smeared line across the screen.
                Outcome::Teleported(changed) => astronaut.teleport(changed),
                Outcome::Changed(changed) => {
                    astronaut.set_body(changed);
                    // A bubble is spent for the rest of the attempt.
                    if kind == ObjectKind::Bubble {
                        used.0 = true;
                        *visibility = Visibility::Hidden;
                    }
                }
                Outcome::FailAttempt => {
                    fail(&mut failed, &mut next_phase);
                    return;
                }
            }
        }
        // Continuous effects act on every step inside. The astronaut coasts
        // afterwards, keeping the speed the wind gave it.
        let body = astronaut.body();
        if let Some(velocity) = while_inside(placement, &config.objects, body, time.delta_secs()) {
            astronaut.set_body(Body {
                velocity,
                drift: Drift::Coasting,
                ..body
            });
        }
    }
}

/// A fresh attempt starts with no contacts and every one-use object back.
fn forget_contacts(mut objects: Query<(&mut Contact, &mut Used, &mut Visibility)>) {
    for (mut contact, mut used, mut visibility) in &mut objects {
        contact.set_if_neq(Contact(false));
        used.set_if_neq(Used(false));
        visibility.set_if_neq(Visibility::Inherited);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use core::time::Duration;

    use bevy::asset::AssetPlugin;

    use super::*;
    use crate::attempt::AttemptPlugin;
    use crate::effects::EffectsPlugin;
    use crate::level::Level;
    use crate::level::tests::play_app_with;

    /// A falling app with objects (and the assets they need), astronaut at `start`.
    pub(crate) fn object_app(level: Level, start: Vec2) -> App {
        let mut app = play_app_with(level, start, Duration::from_secs_f64(1.0 / 60.0));
        app.add_plugins(AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins((AttemptPlugin, ObjectsPlugin, EffectsPlugin));
        app
    }

    #[derive(Resource, Default)]
    struct Touches(Vec<ObjectKind>);

    fn record(mut read: MessageReader<ObjectTouched>, mut touches: ResMut<Touches>) {
        touches.0.extend(read.read().map(|touch| touch.kind));
    }

    #[test]
    fn falling_into_a_static_cloud_ends_the_attempt_after_one_touch() {
        let mut level = Level::for_tests();
        level.objects = vec![Placement::StaticCloud {
            center: Vec2::new(0.0, -900.0),
            radius: 80.0,
        }];
        let mut app = object_app(level, Vec2::new(0.0, -600.0));
        app.init_resource::<Touches>().add_systems(Update, record);
        let mut frames = 0;
        while *app.world().resource::<State<LevelPhase>>().get() == LevelPhase::Falling {
            app.update();
            frames += 1;
            assert!(frames < 120, "never hit the cloud");
        }
        assert_eq!(
            *app.world().resource::<State<LevelPhase>>().get(),
            LevelPhase::Restarting
        );
        assert_eq!(
            app.world().resource::<Touches>().0,
            vec![ObjectKind::StaticCloud]
        );
    }

    #[test]
    fn near_miss_outside_the_forgiving_hitbox_is_safe() {
        // Passes through the outer 20 % of the cloud's visible edge.
        let mut level = Level::for_tests();
        let radius = 100.0;
        let astronaut = GameConfig::default().astronaut_radius;
        let offset = radius * 0.8 + astronaut + 2.0;
        level.objects = vec![Placement::StaticCloud {
            center: Vec2::new(offset, -900.0),
            radius,
        }];
        let mut app = object_app(level, Vec2::new(0.0, -600.0));
        for _ in 0..90 {
            app.update();
        }
        assert_eq!(
            *app.world().resource::<State<LevelPhase>>().get(),
            LevelPhase::Falling,
            "visibly grazed the cloud but stayed safe"
        );
    }

    fn astronaut_velocity(app: &mut App) -> Vec2 {
        let world = app.world_mut();
        let body = world.query::<&Motion>().single(world).map(Motion::body);
        body.unwrap_or_default().velocity
    }

    #[test]
    fn falling_onto_a_cloud_puff_bounces_up_once() {
        let mut level = Level::for_tests();
        level.objects = vec![Placement::CloudPuff {
            center: Vec2::new(0.0, -900.0),
            radius: 80.0,
        }];
        let mut app = object_app(level, Vec2::new(0.0, -700.0));
        app.init_resource::<Touches>().add_systems(Update, record);
        let mut rose = false;
        for _ in 0..60 {
            app.update();
            rose |= astronaut_velocity(&mut app).y > 0.0;
        }
        assert!(rose, "the cloud bounced the astronaut upward");
        assert_eq!(
            app.world().resource::<Touches>().0,
            vec![ObjectKind::CloudPuff],
            "one touch per contact"
        );
        assert_eq!(
            *app.world().resource::<State<LevelPhase>>().get(),
            LevelPhase::Falling
        );
    }

    #[test]
    fn a_crystal_reverses_sideways_motion() {
        let mut level = Level::for_tests();
        level.objects = vec![Placement::Crystal {
            center: Vec2::new(0.0, -900.0),
            size: 50.0,
        }];
        // Just left of the crystal, falling straight: it pushes us left.
        let mut app = object_app(level, Vec2::new(-20.0, -800.0));
        for _ in 0..30 {
            app.update();
        }
        assert!(astronaut_velocity(&mut app).x < -300.0);
    }

    #[test]
    fn a_wind_ribbon_carries_the_astronaut_and_lets_it_coast_on() {
        let mut level = Level::for_tests();
        // A horizontal ribbon flowing right, across the astronaut's fall.
        level.objects = vec![Placement::WindRibbon {
            path: vec![Vec2::new(-200.0, -800.0), Vec2::new(200.0, -800.0)],
            width: 120.0,
        }];
        let mut app = object_app(level, Vec2::new(-150.0, -650.0));
        let mut fastest_sideways: f32 = 0.0;
        for _ in 0..90 {
            app.update();
            fastest_sideways = fastest_sideways.max(astronaut_velocity(&mut app).x);
        }
        assert!(
            fastest_sideways > 250.0,
            "carried right ({fastest_sideways})"
        );
        // Long after leaving the ribbon, still drifting right (coasting).
        assert!(astronaut_velocity(&mut app).x > 50.0);
    }

    /// Drops the astronaut from `start` and runs `frames` frames.
    fn drop_for(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    #[test]
    fn a_bubble_catches_the_astronaut_then_pops_once_per_attempt() {
        let mut level = Level::for_tests();
        level.objects = vec![Placement::Bubble {
            center: Vec2::new(0.0, -700.0),
            radius: 70.0,
        }];
        let mut app = object_app(level, Vec2::new(0.0, -500.0));
        app.init_resource::<Touches>().add_systems(Update, record);
        let mut rose = false;
        for _ in 0..60 {
            app.update();
            rose |= astronaut_velocity(&mut app).y > 0.0;
        }
        assert!(rose, "the bubble carried the astronaut upward");
        assert_eq!(
            app.world().resource::<Touches>().0,
            vec![ObjectKind::Bubble]
        );
        // It's spent: falling back through it does nothing this attempt.
        drop_for(&mut app, 180);
        assert_eq!(
            app.world().resource::<Touches>().0.len(),
            1,
            "one use per attempt"
        );
    }

    #[test]
    fn inside_a_bubble_only_hazards_reach_the_astronaut() {
        let mut level = Level::for_tests();
        level.objects = vec![
            Placement::Bubble {
                center: Vec2::new(0.0, -700.0),
                radius: 70.0,
            },
            // On the way up, inside the bubble: normally it would reverse us.
            Placement::Crystal {
                center: Vec2::new(0.0, -600.0),
                size: 60.0,
            },
        ];
        // Starting inside the bubble: captured on the first step.
        let mut app = object_app(level, Vec2::new(0.0, -700.0));
        app.init_resource::<Touches>().add_systems(Update, record);
        // 100 frames at 60 FPS: still well inside the 2.5 s capture.
        drop_for(&mut app, 100);
        let touches = &app.world().resource::<Touches>().0;
        assert_eq!(
            touches,
            &vec![ObjectKind::Bubble],
            "the crystal is ignored while held"
        );
    }

    #[test]
    fn a_hazard_still_ends_the_attempt_inside_a_bubble() {
        let mut level = Level::for_tests();
        level.objects = vec![
            Placement::Bubble {
                center: Vec2::new(0.0, -700.0),
                radius: 70.0,
            },
            Placement::StaticCloud {
                center: Vec2::new(0.0, -560.0),
                radius: 90.0,
            },
        ];
        let mut app = object_app(level, Vec2::new(0.0, -660.0));
        let mut frames = 0;
        while *app.world().resource::<State<LevelPhase>>().get() == LevelPhase::Falling {
            app.update();
            frames += 1;
            assert!(frames < 200, "the bubble never carried us into the hazard");
        }
        assert_eq!(
            *app.world().resource::<State<LevelPhase>>().get(),
            LevelPhase::Restarting
        );
    }

    #[test]
    fn falling_into_a_portal_comes_out_of_the_other_end_once() {
        let mut level = Level::for_tests();
        let (a, b) = (Vec2::new(0.0, -700.0), Vec2::new(300.0, -1300.0));
        level.objects = vec![Placement::TwinPortals { a, b, radius: 60.0 }];
        let mut app = object_app(level, Vec2::new(0.0, -500.0));
        app.init_resource::<Touches>().add_systems(Update, record);
        // ~1.5 s: long enough to fall the 200 units to the near end.
        drop_for(&mut app, 90);
        let world = app.world_mut();
        let body = world.query::<&Motion>().single(world).map(Motion::body);
        let position = body.unwrap_or_default().position;
        // Out of the far end, then falling on from there.
        assert!(
            (position.x - b.x).abs() < 50.0,
            "came out at the far end: {position}"
        );
        assert!(position.y < b.y, "and kept falling: {position}");
        assert_eq!(
            app.world().resource::<Touches>().0,
            vec![ObjectKind::TwinPortals]
        );
    }
}

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

pub use behavior::{Outcome, on_enter};
pub use kind::{
    CloudPuffConfig, CrystalConfig, ObjectKind, ObjectsConfig, Placement, StaticCloudConfig,
};
use visuals::ObjectAssets;

use crate::astronaut::Astronaut;
use crate::attempt::{AttemptFailed, AttemptRestarted, fail};
use crate::config::GameConfig;
use crate::level::{ActiveLevel, LevelPhase, LevelSystems, still_falling};
use crate::physics::Motion;

/// Sent when the astronaut starts touching an object.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ObjectTouched {
    /// The object's type.
    // Tests read it already; the game will from M5 on.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "read by the journal (M5), sounds (M8) and effects (M7)"
        )
    )]
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
    assets: Res<ObjectAssets>,
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
                crate::effects::ContactPulse::default(),
                visuals::transform_for(placement),
                Visibility::default(),
            ))
            .with_children(|parent| visuals::spawn_look(parent, &assets, placement));
    }
}

/// Finds objects the astronaut just started touching and applies their effect.
///
/// A plain loop over the level's objects: one screen holds a few dozen at
/// most, which a loop checks in well under a microsecond (see `benches/`).
fn touch_objects(
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    // One mutable handle for reading *and* writing: asking Bevy for `&Motion`
    // and `&mut Motion` of the same entity in one system is a conflict it
    // refuses at startup (the same rule as Rust's borrowing).
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut objects: Query<(Entity, &ObjectIndex, &mut Contact)>,
    mut touched: MessageWriter<ObjectTouched>,
    mut failed: MessageWriter<AttemptFailed>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    for (entity, index, mut contact) in &mut objects {
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
        // Effects fire on entering only: touching now, not touching before.
        if !touching || was_touching {
            continue;
        }
        let kind = placement.kind();
        touched.write(ObjectTouched { kind, entity });
        match on_enter(kind, placement.center(), &config.objects, body) {
            Outcome::Launch(velocity) => {
                astronaut.set_body(body.launched(velocity, &config.movement));
            }
            Outcome::FailAttempt => fail(&mut failed, &mut next_phase),
        }
    }
}

/// A fresh attempt starts without any contacts.
fn forget_contacts(mut contacts: Query<&mut Contact>) {
    for mut contact in &mut contacts {
        contact.set_if_neq(Contact(false));
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
}

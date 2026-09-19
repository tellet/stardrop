//! Stardust pickup and run tracking.
//!
//! Everything here belongs to one attempt: a fresh attempt (after a failure,
//! a retry or a level reload) brings the stardust back and resets the time.
//! The time only counts while falling, in fixed physics steps, so it's exact
//! and the same at any frame rate.

use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::attempt::AttemptRestarted;
use crate::collision::circles_overlap;
use crate::config::GameConfig;
use crate::level::{ActiveLevel, LevelSystems, MOTE_RADIUS, StardustMote, still_falling};
use crate::physics::Motion;
use crate::scoring::RunStats;

/// The current attempt's time and stardust.
#[derive(Resource, Debug, Default, Clone, Copy, Deref, DerefMut)]
pub(crate) struct CurrentRun(pub RunStats);

/// Sent when a stardust mote is picked up.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct StardustCollected {
    /// Index in the level's stardust list.
    #[expect(
        dead_code,
        reason = "read by the stardust twinkle (M8) and effects (M7)"
    )]
    pub index: usize,
}

/// Stardust and run tracking.
pub(crate) struct CollectiblesPlugin;

impl Plugin for CollectiblesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentRun>()
            .add_message::<StardustCollected>()
            .add_systems(
                FixedUpdate,
                (
                    count_run_time.run_if(still_falling),
                    collect_stardust
                        .in_set(LevelSystems::Rules)
                        .run_if(still_falling),
                ),
            )
            .add_systems(
                Update,
                (
                    start_new_run.run_if(on_message::<AttemptRestarted>),
                    show_motes,
                )
                    .chain(),
            );
    }
}

fn count_run_time(time: Res<Time>, mut run: ResMut<CurrentRun>) {
    run.elapsed += time.delta_secs();
}

/// Only a handful of motes per level (at most 64), so checking each one is
/// cheaper than any lookup structure would be.
fn collect_stardust(
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    astronaut: Single<&Motion, With<Astronaut>>,
    mut run: ResMut<CurrentRun>,
    mut collected: MessageWriter<StardustCollected>,
) {
    let position = astronaut.body().position;
    for (index, &mote) in level.stardust.iter().enumerate() {
        let touching = circles_overlap(position, config.astronaut_radius, mote, MOTE_RADIUS);
        // `bypass_change_detection` for the read-only check, so the resource
        // is only marked changed when a mote is really collected.
        if touching && !run.bypass_change_detection().stardust.contains(index) {
            run.stardust.insert(index);
            collected.write(StardustCollected { index });
        }
    }
}

/// A fresh attempt starts a fresh run.
fn start_new_run(mut run: ResMut<CurrentRun>) {
    *run = CurrentRun::default();
}

/// Hides collected motes. Runs its loop only when the run changes or motes
/// are (re)spawned, not every frame.
fn show_motes(run: Res<CurrentRun>, mut motes: Query<(Ref<StardustMote>, &mut Visibility)>) {
    for (mote, mut visibility) in &mut motes {
        if run.is_changed() || mote.is_added() {
            let wanted = if run.stardust.contains(mote.0) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            visibility.set_if_neq(wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::attempt::AttemptPlugin;
    use crate::level::tests::play_app_with;
    use crate::level::{Level, LevelPhase};

    fn app_with_stardust(start: Vec2, frame: Duration) -> App {
        // A mote straight below the start, on the astronaut's path.
        let mut level = Level::for_tests();
        level.stardust = vec![start - Vec2::new(0.0, 400.0), Vec2::new(400.0, -1500.0)];
        let mut app = play_app_with(level, start, frame);
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_plugins((AttemptPlugin, CollectiblesPlugin));
        app
    }

    fn run(app: &App) -> RunStats {
        app.world().resource::<CurrentRun>().0
    }

    /// In the real game the level loads a few frames after startup; nothing
    /// may require it before then. (This crashed the game once.)
    #[test]
    fn runs_before_any_level_has_loaded() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin))
            .init_state::<LevelPhase>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins((AttemptPlugin, CollectiblesPlugin));
        for _ in 0..3 {
            app.update();
        }
    }

    #[test]
    fn falling_through_a_mote_collects_it_once() {
        let mut app = app_with_stardust(Vec2::new(0.0, -200.0), Duration::from_millis(16));
        for _ in 0..120 {
            app.update();
        }
        let stats = run(&app);
        assert_eq!(stats.stardust.count(), 1);
        assert!(stats.stardust.contains(0));
    }

    #[test]
    fn run_time_is_exact_in_physics_steps() {
        let mut app = app_with_stardust(Vec2::new(0.0, -200.0), Duration::from_millis(50));
        for _ in 0..20 {
            app.update();
        }
        // 20 frames of 50 ms; Bevy's first frame has zero delta, so 0.95 s pass.
        let elapsed = run(&app).elapsed;
        let steps = elapsed * 120.0;
        assert!(
            (steps - steps.round()).abs() < 1e-3,
            "{elapsed} s isn't whole physics steps"
        );
        assert!(
            (elapsed - 0.95).abs() <= 1.0 / 120.0 + 1e-4,
            "elapsed {elapsed}"
        );
    }

    #[test]
    fn retry_starts_a_new_run() {
        let mut app = app_with_stardust(Vec2::new(0.0, -200.0), Duration::from_millis(16));
        for _ in 0..120 {
            app.update();
        }
        assert_eq!(run(&app).stardust.count(), 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        let stats = run(&app);
        assert_eq!(stats.stardust.count(), 0);
        assert!(stats.elapsed < 0.1);
    }
}

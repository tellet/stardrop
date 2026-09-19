//! One attempt at a level: waiting at the start, falling, and starting over.
//!
//! - `Ready`: the astronaut floats at the start with a gentle bob while the
//!   player studies the level. The first left/right press starts the fall.
//! - Missing the beacon fails the attempt: a soft fade (under one second)
//!   covers the screen, and a fresh attempt begins at the start.
//! - R starts a fresh attempt instantly. So does a new version of the level
//!   file (hot reload), which shows the edit from the start.
//!
//! The pure decisions (bob offset, fade opacity) are plain functions with unit
//! tests; the systems only apply them.

use core::f32::consts::TAU;

use bevy::prelude::*;

use crate::astronaut::{Astronaut, AstronautSystems};
use crate::config::GameConfig;
use crate::level::{ActiveLevel, LevelPhase, LevelSystems, still_falling};
use crate::physics::movement::{AirInput, Body};
use crate::physics::{AirControl, Motion};

/// Failed-attempt fade timing, in seconds: out, a short hold while hidden,
/// then in.
const FADE_OUT: f32 = 0.3;
const FADE_HOLD: f32 = 0.1;
const FADE_IN: f32 = 0.3;
const FADE_TOTAL: f32 = FADE_OUT + FADE_HOLD + FADE_IN;
// Checked when compiling: the brief requires restarts to take under one second.
const _: () = assert!(FADE_TOTAL < 1.0, "restart fade must be under one second");
/// The attempt restarts while the screen is fully covered.
const RESTART_AT: f32 = FADE_OUT + FADE_HOLD / 2.0;
/// Deep night blue rather than black: softer, and it matches the sky.
const FADE_COLOR: Color = Color::srgb(0.02, 0.015, 0.06);

/// The waiting bob: how far up and down (world units) and how slowly.
const BOB_AMPLITUDE: f32 = 6.0;
const BOB_PERIOD: f32 = 2.4;

/// Sent when an attempt fails (the astronaut missed the beacon).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct AttemptFailed;

/// Sent when a fresh attempt begins: after a failure, a retry (R) or a level
/// reload. Everything tracked per attempt resets.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct AttemptRestarted;

/// Progress of the current failed-attempt fade.
#[derive(Resource, Debug, Default)]
struct RestartFade {
    elapsed: f32,
    restarted: bool,
}

/// Seconds spent waiting in `Ready` this attempt (drives the bob).
#[derive(Resource, Debug, Default)]
struct ReadyWait(f32);

/// The full-screen overlay used for the fade.
#[derive(Component)]
struct FadeOverlay;

/// Attempts: ready, fail, restart, retry.
pub(crate) struct AttemptPlugin;

impl Plugin for AttemptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RestartFade>()
            .init_resource::<ReadyWait>()
            .add_message::<AttemptFailed>()
            .add_message::<AttemptRestarted>()
            .add_systems(Startup, spawn_fade_overlay)
            .add_systems(
                FixedUpdate,
                fall_out_of_level
                    .in_set(LevelSystems::Rules)
                    .run_if(still_falling),
            )
            .add_systems(OnEnter(LevelPhase::Ready), reset_wait)
            .add_systems(OnEnter(LevelPhase::Restarting), start_fade)
            .add_systems(OnExit(LevelPhase::Restarting), clear_fade)
            .add_systems(
                Update,
                (
                    (bob_at_start, drop_on_first_input)
                        .chain()
                        .after(AstronautSystems::Input)
                        .run_if(in_state(LevelPhase::Ready)),
                    run_fade.run_if(in_state(LevelPhase::Restarting)),
                    (
                        retry.run_if(not(in_state(LevelPhase::Loading))),
                        restart_on_level_change.run_if(resource_exists_and_changed::<ActiveLevel>),
                    )
                        .before(bob_at_start),
                ),
            );
    }
}

/// Vertical offset of the waiting bob, `seconds` into the wait. A sine wave:
/// smooth everywhere, never a snap.
#[must_use]
pub(crate) fn bob_offset(seconds: f32) -> f32 {
    BOB_AMPLITUDE * (seconds * TAU / BOB_PERIOD).sin()
}

/// How opaque the fade overlay is (0..=1), `elapsed` seconds into a restart.
/// Eased with smoothstep in both directions; never a linear snap.
#[must_use]
pub(crate) fn fade_alpha(elapsed: f32) -> f32 {
    let t = if elapsed < FADE_OUT {
        elapsed / FADE_OUT
    } else if elapsed < FADE_OUT + FADE_HOLD {
        1.0
    } else {
        1.0 - (elapsed - FADE_OUT - FADE_HOLD) / FADE_IN
    };
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Puts the astronaut back at the start, at rest, and announces a fresh attempt.
fn restart(
    motion: &mut Motion,
    level: &ActiveLevel,
    waited: &mut ReadyWait,
    restarted: &mut MessageWriter<AttemptRestarted>,
) {
    motion.teleport(Body::at_rest(level.start));
    // Also covers a retry while already Ready, where `OnEnter(Ready)` won't run.
    waited.0 = 0.0;
    restarted.write(AttemptRestarted);
}

/// Floats the astronaut up and down around the start. Moves the physics body
/// itself, so the fall later begins exactly where the astronaut is drawn.
fn bob_at_start(
    time: Res<Time>,
    level: Res<ActiveLevel>,
    mut waited: ResMut<ReadyWait>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
) {
    waited.0 += time.delta_secs();
    let position = level.start + Vec2::Y * bob_offset(waited.0);
    astronaut.teleport(Body::at_rest(position));
}

/// Each wait starts the bob at zero, so it begins exactly at the start point
/// instead of jumping to the middle of a swing.
fn reset_wait(mut waited: ResMut<ReadyWait>) {
    waited.0 = 0.0;
}

/// The first left/right press of an attempt starts the fall.
fn drop_on_first_input(control: Res<AirControl>, mut next_phase: ResMut<NextState<LevelPhase>>) {
    if control.0 != AirInput::NONE {
        next_phase.set(LevelPhase::Falling);
    }
}

/// Missing the beacon (falling out of the bottom) fails the attempt.
fn fall_out_of_level(
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    astronaut: Single<&Motion, With<Astronaut>>,
    mut failed: MessageWriter<AttemptFailed>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    if astronaut.body().position.y < level.bottom() - config.astronaut_radius {
        failed.write(AttemptFailed);
        next_phase.set(LevelPhase::Restarting);
    }
}

fn spawn_fade_overlay(mut commands: Commands) {
    commands.spawn((
        FadeOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(FADE_COLOR.with_alpha(0.0)),
        // Above all other UI so the fade covers everything.
        GlobalZIndex(10),
    ));
}

fn start_fade(mut fade: ResMut<RestartFade>) {
    *fade = RestartFade::default();
}

/// Fades out, restarts the attempt while the screen is covered, fades back in.
fn run_fade(
    time: Res<Time>,
    level: Res<ActiveLevel>,
    mut fade: ResMut<RestartFade>,
    mut overlay: Single<&mut BackgroundColor, With<FadeOverlay>>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut waited: ResMut<ReadyWait>,
    mut restarted: MessageWriter<AttemptRestarted>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    fade.elapsed += time.delta_secs();
    overlay.0.set_alpha(fade_alpha(fade.elapsed));
    if !fade.restarted && fade.elapsed >= RESTART_AT {
        restart(&mut astronaut, &level, &mut waited, &mut restarted);
        fade.restarted = true;
    }
    if fade.elapsed >= FADE_TOTAL {
        next_phase.set(LevelPhase::Ready);
    }
}

/// Also runs when a retry interrupts the fade: never leave the screen covered.
fn clear_fade(mut overlay: Single<&mut BackgroundColor, With<FadeOverlay>>) {
    overlay.0.set_alpha(0.0);
}

/// R: a fresh attempt, instantly. Reuses every existing entity (nothing is
/// loaded or spawned), which keeps it fast and leak-free.
fn retry(
    keys: Res<ButtonInput<KeyCode>>,
    level: Res<ActiveLevel>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut waited: ResMut<ReadyWait>,
    mut restarted: MessageWriter<AttemptRestarted>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        restart(&mut astronaut, &level, &mut waited, &mut restarted);
        // NextState's own `set_if_neq`: when already Ready, don't re-run the
        // phase's enter/exit systems. The `*` reaches through `ResMut` to
        // `NextState`; without it Rust picks the unrelated resource method
        // of the same name.
        (*next_phase).set_if_neq(LevelPhase::Ready);
    }
}

/// A new version of the level file starts a fresh attempt. The first load
/// is skipped (`is_added`): the astronaut already starts at the start then.
fn restart_on_level_change(
    level: Res<ActiveLevel>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut waited: ResMut<ReadyWait>,
    mut restarted: MessageWriter<AttemptRestarted>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    if level.is_added() {
        return;
    }
    restart(&mut astronaut, &level, &mut waited, &mut restarted);
    (*next_phase).set_if_neq(LevelPhase::Ready);
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::level::Level;
    use crate::level::tests::play_app;

    #[test]
    fn bob_starts_at_the_start_point_and_stays_small() {
        assert!(bob_offset(0.0).abs() < 1e-6);
        for ms in 0..5000_u16 {
            assert!(bob_offset(f32::from(ms) / 1000.0).abs() <= BOB_AMPLITUDE + 1e-4);
        }
    }

    #[test]
    fn fade_covers_the_screen_when_the_attempt_restarts() {
        assert!(fade_alpha(0.0).abs() < 1e-6);
        assert!((fade_alpha(RESTART_AT) - 1.0).abs() < 1e-6);
        assert!(fade_alpha(FADE_TOTAL).abs() < 1e-6);
    }

    #[test]
    fn fade_never_jumps() {
        let mut previous = fade_alpha(0.0);
        for ms in 1..=700_u16 {
            let alpha = fade_alpha(f32::from(ms) / 1000.0);
            assert!((alpha - previous).abs() < 0.01, "jump at {ms} ms");
            previous = alpha;
        }
    }

    /// An app at 60 FPS with attempts and input, astronaut at `start`, falling.
    pub(crate) fn attempt_app(start: Vec2) -> App {
        let mut app = play_app(start, Duration::from_secs_f64(1.0 / 60.0));
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(AttemptPlugin);
        app
    }

    fn astronaut_body(app: &mut App) -> Body {
        let world = app.world_mut();
        let body = world.query::<&Motion>().single(world).map(Motion::body);
        body.unwrap_or_default()
    }

    fn phase(app: &App) -> LevelPhase {
        *app.world().resource::<State<LevelPhase>>().get()
    }

    fn steer(app: &mut App, input: AirInput) {
        app.world_mut().resource_mut::<AirControl>().0 = input;
    }

    #[test]
    fn missing_the_beacon_restarts_the_attempt_within_a_second() {
        let level = Level::for_tests();
        // Beside the beacon, about to fall out.
        let mut app = attempt_app(Vec2::new(400.0, level.bottom() + 100.0));
        let mut frames = 0;
        while phase(&app) != LevelPhase::Restarting {
            app.update();
            frames += 1;
            assert!(frames < 300, "never fell out");
        }
        let mut restart_frames = 0;
        while phase(&app) == LevelPhase::Restarting {
            app.update();
            restart_frames += 1;
        }
        assert!(
            restart_frames <= 60,
            "restart took {restart_frames} frames at 60 FPS"
        );
        assert_eq!(phase(&app), LevelPhase::Ready);
        let body = astronaut_body(&mut app);
        assert!(body.position.distance(level.start) <= BOB_AMPLITUDE + 1e-3);
    }

    #[test]
    fn ready_waits_until_the_first_steering_input() {
        let level = Level::for_tests();
        let mut app = attempt_app(level.start);
        app.world_mut()
            .resource_mut::<NextState<LevelPhase>>()
            .set(LevelPhase::Ready);
        // Five seconds of waiting: still at the start, never falling.
        for _ in 0..300 {
            app.update();
        }
        assert_eq!(phase(&app), LevelPhase::Ready);
        assert!(astronaut_body(&mut app).position.distance(level.start) <= BOB_AMPLITUDE + 1e-3);
        steer(&mut app, AirInput::RIGHT);
        app.update();
        app.update();
        assert_eq!(phase(&app), LevelPhase::Falling);
    }

    #[test]
    fn retry_starts_a_fresh_attempt_at_the_start() {
        let level = Level::for_tests();
        let mut app = attempt_app(Vec2::new(0.0, -1000.0));
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(phase(&app), LevelPhase::Ready);
        assert!(astronaut_body(&mut app).position.distance(level.start) <= BOB_AMPLITUDE + 1e-3);
    }

    #[test]
    fn a_new_version_of_the_level_starts_a_fresh_attempt() {
        let mut app = attempt_app(Vec2::new(0.0, -1000.0));
        app.update();
        assert_eq!(
            phase(&app),
            LevelPhase::Falling,
            "the first load doesn't restart"
        );
        // Simulate a hot reload that moves the start.
        let new_start = Vec2::new(-300.0, -150.0);
        app.world_mut().resource_mut::<ActiveLevel>().0.start = new_start;
        app.update();
        app.update();
        assert_eq!(phase(&app), LevelPhase::Ready);
        assert!(astronaut_body(&mut app).position.distance(new_start) <= BOB_AMPLITUDE + 1e-3);
    }
}

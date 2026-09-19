//! Checkpoints, respawning and retrying.
//!
//! - Falling past a ring's depth activates it (anywhere across the level).
//! - Falling out of the bottom fades back to the last active checkpoint (or
//!   the start) in under a second, and the fall resumes from rest.
//! - R restarts the whole level instantly.
//! - F3 (debug builds only) jumps to the next checkpoint.
//!
//! The pure decisions (which ring was passed, how opaque the fade is) are
//! plain functions with unit tests; the systems only apply them.

use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::config::GameConfig;
use crate::level::{
    ActiveLevel, CheckpointRing, Level, LevelPhase, LevelSystems, SceneryAssets, still_falling,
};
use crate::physics::Motion;
use crate::physics::movement::Body;

/// Respawn fade timing, in seconds: out, a short hold while hidden, then in.
/// 0.7 s in total, within the brief's "under one second".
const FADE_OUT: f32 = 0.3;
const FADE_HOLD: f32 = 0.1;
const FADE_IN: f32 = 0.3;
const FADE_TOTAL: f32 = FADE_OUT + FADE_HOLD + FADE_IN;
// Checked when compiling: the brief requires respawns to take under one second.
const _: () = assert!(FADE_TOTAL < 1.0, "respawn fade must be under one second");
/// The astronaut is moved while the screen is fully covered.
const RESPAWN_AT: f32 = FADE_OUT + FADE_HOLD / 2.0;
/// Deep night blue rather than black: softer, and it matches the sky.
const FADE_COLOR: Color = Color::srgb(0.02, 0.015, 0.06);

/// Which checkpoint the astronaut returns to. `None` means the level's start:
/// no checkpoint yet.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct CheckpointProgress {
    /// Index into the level's checkpoint list.
    pub active: Option<usize>,
}

impl CheckpointProgress {
    /// Where to respawn. If the level was edited and the checkpoint no longer
    /// exists, `get` returns `None` and we fall back to the start.
    fn respawn_point(self, level: &Level) -> Vec2 {
        self.active
            .and_then(|index| level.checkpoints.get(index).copied())
            .unwrap_or(level.start)
    }
}

/// Sent when a checkpoint is activated.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct CheckpointReached {
    /// Index in the level's checkpoint list.
    #[expect(
        dead_code,
        reason = "read by the checkpoint chime (M8) and effects (M7)"
    )]
    pub index: usize,
}

/// Sent when the astronaut has been put back at a checkpoint (screen covered).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct Respawn;

/// Sent when the player restarts the whole level with R.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct RunRestarted;

/// Progress of the current respawn fade.
#[derive(Resource, Debug, Default)]
struct RespawnFade {
    elapsed: f32,
    moved: bool,
}

/// The full-screen overlay used for the respawn fade.
#[derive(Component)]
struct FadeOverlay;

/// Checkpoints, respawn and retry.
pub(crate) struct CheckpointPlugin;

impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointProgress>()
            .init_resource::<RespawnFade>()
            .add_message::<CheckpointReached>()
            .add_message::<Respawn>()
            .add_message::<RunRestarted>()
            .add_systems(Startup, spawn_fade_overlay)
            .add_systems(
                FixedUpdate,
                (activate_checkpoints, fall_out_of_level)
                    .chain()
                    .in_set(LevelSystems::Rules)
                    .run_if(still_falling),
            )
            .add_systems(OnEnter(LevelPhase::Respawning), start_fade)
            .add_systems(OnExit(LevelPhase::Respawning), clear_fade)
            .add_systems(
                Update,
                (
                    run_fade.run_if(in_state(LevelPhase::Respawning)),
                    retry.run_if(not(in_state(LevelPhase::Loading))),
                    show_active_rings,
                ),
            );
        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            jump_to_next_checkpoint.run_if(in_state(LevelPhase::Falling)),
        );
    }
}

/// The deepest checkpoint at or above `y`, if it's further along than `active`.
///
/// "At or above" means the astronaut has fallen past its depth. Checkpoints
/// are listed top to bottom (validation guarantees it), so the last match
/// from the start of the list is the deepest.
#[must_use]
pub(crate) fn newly_passed(checkpoints: &[Vec2], active: Option<usize>, y: f32) -> Option<usize> {
    let deepest = checkpoints.iter().rposition(|ring| y <= ring.y)?;
    // `Option` compares with `None` below every `Some`, so this also covers
    // "no checkpoint yet".
    (Some(deepest) > active).then_some(deepest)
}

/// How opaque the fade overlay is (0..=1), `elapsed` seconds into a respawn.
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

fn activate_checkpoints(
    level: Res<ActiveLevel>,
    astronaut: Single<&Motion, With<Astronaut>>,
    mut progress: ResMut<CheckpointProgress>,
    mut reached: MessageWriter<CheckpointReached>,
) {
    let y = astronaut.body().position.y;
    if let Some(index) = newly_passed(&level.checkpoints, progress.active, y) {
        progress.active = Some(index);
        reached.write(CheckpointReached { index });
    }
}

/// Missing the beacon (falling below the level) starts a respawn.
fn fall_out_of_level(
    level: Res<ActiveLevel>,
    config: Res<GameConfig>,
    astronaut: Single<&Motion, With<Astronaut>>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    if astronaut.body().position.y < level.bottom() - config.astronaut_radius {
        next_phase.set(LevelPhase::Respawning);
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
        // Above the level's text so the fade covers everything.
        GlobalZIndex(10),
    ));
}

fn start_fade(mut fade: ResMut<RespawnFade>) {
    *fade = RespawnFade::default();
}

/// Fades out, moves the astronaut while the screen is covered, fades back in.
fn run_fade(
    time: Res<Time>,
    level: Res<ActiveLevel>,
    progress: Res<CheckpointProgress>,
    mut fade: ResMut<RespawnFade>,
    mut overlay: Single<&mut BackgroundColor, With<FadeOverlay>>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut respawned: MessageWriter<Respawn>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    fade.elapsed += time.delta_secs();
    overlay.0.set_alpha(fade_alpha(fade.elapsed));
    if !fade.moved && fade.elapsed >= RESPAWN_AT {
        astronaut.teleport(Body::at_rest(progress.respawn_point(&level)));
        fade.moved = true;
        respawned.write(Respawn);
    }
    if fade.elapsed >= FADE_TOTAL {
        next_phase.set(LevelPhase::Falling);
    }
}

/// Also runs when a retry interrupts a respawn: never leave the screen covered.
fn clear_fade(mut overlay: Single<&mut BackgroundColor, With<FadeOverlay>>) {
    overlay.0.set_alpha(0.0);
}

/// R: restart the whole level from the top, instantly. Reuses every existing
/// entity (nothing is loaded or spawned), which keeps it fast and leak-free.
fn retry(
    keys: Res<ButtonInput<KeyCode>>,
    level: Res<ActiveLevel>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut progress: ResMut<CheckpointProgress>,
    mut restarted: MessageWriter<RunRestarted>,
    mut next_phase: ResMut<NextState<LevelPhase>>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    astronaut.teleport(Body::at_rest(level.start));
    *progress = CheckpointProgress::default();
    restarted.write(RunRestarted);
    // NextState's own `set_if_neq`: when already falling, don't re-run the
    // phase's enter/exit systems. The `*` reaches through `ResMut` to
    // `NextState`; without it Rust picks the unrelated resource method of the
    // same name.
    (*next_phase).set_if_neq(LevelPhase::Falling);
}

/// Colors each ring by whether it's the active checkpoint. Runs only when the
/// progress changes or rings are (re)spawned, not every frame.
fn show_active_rings(
    progress: Res<CheckpointProgress>,
    scenery: Option<Res<SceneryAssets>>,
    mut rings: Query<(Ref<CheckpointRing>, &mut MeshMaterial2d<ColorMaterial>)>,
) {
    let Some(scenery) = scenery else {
        return;
    };
    for (ring, mut material) in &mut rings {
        if progress.is_changed() || ring.is_added() {
            material.0 = scenery.ring_material(progress.active == Some(ring.0));
        }
    }
}

/// Debug builds: F3 jumps to (and activates) the next checkpoint.
#[cfg(debug_assertions)]
fn jump_to_next_checkpoint(
    keys: Res<ButtonInput<KeyCode>>,
    level: Res<ActiveLevel>,
    mut astronaut: Single<&mut Motion, With<Astronaut>>,
    mut progress: ResMut<CheckpointProgress>,
    mut reached: MessageWriter<CheckpointReached>,
) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    let next = progress.active.map_or(0, |index| index + 1);
    if let Some(&ring) = level.checkpoints.get(next) {
        astronaut.teleport(Body::at_rest(ring));
        progress.active = Some(next);
        reached.write(CheckpointReached { index: next });
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::level::tests::play_app;

    const RINGS: [Vec2; 2] = [Vec2::new(0.0, -3600.0), Vec2::new(-260.0, -7200.0)];

    #[test]
    fn passing_a_rings_depth_activates_it_anywhere_across() {
        assert_eq!(newly_passed(&RINGS, None, -3000.0), None);
        assert_eq!(newly_passed(&RINGS, None, -3600.0), Some(0));
        assert_eq!(newly_passed(&RINGS, None, -5000.0), Some(0));
        // Skipping past both at once activates the deeper one.
        assert_eq!(newly_passed(&RINGS, None, -8000.0), Some(1));
    }

    #[test]
    fn a_checkpoint_is_reported_only_once() {
        assert_eq!(newly_passed(&RINGS, Some(0), -5000.0), None);
        assert_eq!(newly_passed(&RINGS, Some(1), -9000.0), None);
    }

    #[test]
    fn respawn_point_falls_back_to_start() {
        let level = Level::for_tests();
        assert_eq!(
            CheckpointProgress::default().respawn_point(&level),
            level.start
        );
        let gone = CheckpointProgress { active: Some(99) };
        assert_eq!(
            gone.respawn_point(&level),
            level.start,
            "checkpoint removed by an edit"
        );
        let second = CheckpointProgress { active: Some(1) };
        assert_eq!(second.respawn_point(&level), level.checkpoints[1]);
    }

    #[test]
    fn fade_covers_the_screen_when_the_astronaut_moves() {
        assert!(fade_alpha(0.0).abs() < 1e-6);
        assert!((fade_alpha(RESPAWN_AT) - 1.0).abs() < 1e-6);
        assert!(fade_alpha(FADE_TOTAL).abs() < 1e-6);
    }

    #[test]
    fn fade_never_jumps() {
        // Consecutive 1 ms samples never differ by more than a smooth curve would.
        let mut previous = fade_alpha(0.0);
        for ms in 1..=700_u16 {
            let alpha = fade_alpha(f32::from(ms) / 1000.0);
            assert!((alpha - previous).abs() < 0.01, "jump at {ms} ms");
            previous = alpha;
        }
    }

    /// An app at 60 FPS with checkpoints and input, astronaut at `start`.
    fn checkpoint_app(start: Vec2) -> App {
        let mut app = play_app(start, Duration::from_secs_f64(1.0 / 60.0));
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(CheckpointPlugin);
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

    #[test]
    fn missing_the_beacon_returns_to_the_last_checkpoint_within_a_second() {
        let level = Level::for_tests();
        // Past both checkpoints, beside the beacon, about to fall out.
        let mut app = checkpoint_app(Vec2::new(800.0, level.bottom() + 100.0));
        let mut frames = 0;
        while phase(&app) != LevelPhase::Respawning {
            app.update();
            frames += 1;
            assert!(frames < 300, "never fell out");
        }
        let mut respawn_frames = 0;
        let mut placed = None;
        while phase(&app) == LevelPhase::Respawning {
            app.update();
            respawn_frames += 1;
            // Right after the move, while the screen is still covered.
            if placed.is_none() && app.world().resource::<RespawnFade>().moved {
                placed = Some(astronaut_body(&mut app));
            }
        }
        assert!(
            respawn_frames <= 60,
            "respawn took {respawn_frames} frames at 60 FPS"
        );
        let placed = placed.unwrap_or_default();
        assert!(placed.position.distance(level.checkpoints[1]) < 1e-3);
        assert_eq!(placed.velocity, Vec2::ZERO, "the fall resumes from rest");
    }

    #[test]
    fn retry_restarts_from_the_top_and_forgets_checkpoints() {
        let level = Level::for_tests();
        let mut app = checkpoint_app(Vec2::new(0.0, -8000.0));
        app.update();
        app.update();
        assert_eq!(app.world().resource::<CheckpointProgress>().active, Some(1));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();

        assert_eq!(app.world().resource::<CheckpointProgress>().active, None);
        assert!(astronaut_body(&mut app).position.y > level.start.y - 50.0);
        assert_eq!(phase(&app), LevelPhase::Falling);
    }
}

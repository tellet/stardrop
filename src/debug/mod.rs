//! Debug tools. F1 toggles a performance overlay (available in every build).

use core::fmt::Write as _;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::astronaut::Astronaut;
use crate::collectibles::CurrentRun;
use crate::physics::Motion;

pub mod stats;

/// Frame-time history kept by Bevy: ~4 s at 60 FPS, the window for "average" and "worst".
const FRAME_HISTORY: usize = 240;

/// How often the overlay text refreshes. Faster is unreadable and costs layout work.
const REFRESH_SECONDS: f32 = 0.25;

const OVERLAY_FONT_SIZE: f32 = 16.0;
const OVERLAY_MARGIN: Val = Val::Px(8.0);

/// Whether the F1 overlay is showing.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum OverlayState {
    #[default]
    Hidden,
    Shown,
}

/// The overlay's text node.
#[derive(Component)]
struct OverlayText;

/// The F1 debug overlay.
pub(crate) struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::new(FRAME_HISTORY))
            .init_state::<OverlayState>()
            .add_systems(Update, toggle_overlay)
            .add_systems(OnEnter(OverlayState::Shown), spawn_overlay)
            .add_systems(
                Update,
                refresh_overlay.run_if(in_state(OverlayState::Shown)),
            );
    }
}

fn toggle_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<OverlayState>>,
    mut next: ResMut<NextState<OverlayState>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        next.set(match state.get() {
            OverlayState::Hidden => OverlayState::Shown,
            OverlayState::Shown => OverlayState::Hidden,
        });
    }
}

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        OverlayText,
        // Despawned automatically when the overlay state is left.
        DespawnOnExit(OverlayState::Shown),
        Text::new("…"),
        TextFont::from_font_size(OVERLAY_FONT_SIZE),
        Node {
            position_type: PositionType::Absolute,
            top: OVERLAY_MARGIN,
            left: OVERLAY_MARGIN,
            ..default()
        },
    ));
}

/// Rewrites the overlay text a few times per second. Writes into the existing
/// `String`, so after the first refresh no memory is allocated.
fn refresh_overlay(
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
    diagnostics: Res<DiagnosticsStore>,
    astronaut: Query<&Motion, With<Astronaut>>,
    run: Res<CurrentRun>,
    mut text: Single<&mut Text, With<OverlayText>>,
) {
    let timer =
        timer.get_or_insert_with(|| Timer::from_seconds(REFRESH_SECONDS, TimerMode::Repeating));
    if !timer.tick(time.delta()).just_finished() {
        return;
    }
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(bevy::diagnostic::Diagnostic::smoothed)
        .unwrap_or(0.0);
    let summary = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|frame_time| stats::summarize(frame_time.values().copied()));

    let buffer = &mut text.0;
    buffer.clear();
    // Writing to a `String` cannot fail, so the `fmt::Result`s are safely ignored.
    let _ = write!(buffer, "FPS {fps:.0}");
    if let Some(s) = summary {
        let _ = write!(
            buffer,
            "\nframe ms  now {:.2}  avg {:.2}  worst {:.2}",
            s.current_ms, s.average_ms, s.worst_ms
        );
    }
    if let Some(motion) = astronaut.iter().next() {
        let velocity = motion.body().velocity;
        let _ = write!(
            buffer,
            "\nvelocity  x {:.0}  y {:.0}",
            velocity.x, velocity.y
        );
    }
    let _ = write!(
        buffer,
        "\nrun  {:.1} s  stardust {}",
        run.elapsed,
        run.stardust.count()
    );
}

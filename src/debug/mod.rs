//! Debug tools. F1 toggles a performance overlay (available in every build).
//!
//! The overlay also switches the app between low-power and continuous updates:
//! frame-time numbers are meaningless while the app deliberately sleeps.

use core::fmt::Write as _;
use core::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};

pub mod stats;

/// Frame-time history kept by Bevy: ~4 s at 60 FPS, the window for "average" and "worst".
const FRAME_HISTORY: usize = 240;

/// How often the overlay text refreshes. Faster is unreadable and costs layout work.
const REFRESH_SECONDS: f32 = 0.25;

/// While nothing animates, wake up at most this often (or on input).
const LOW_POWER_WAIT: Duration = Duration::from_secs(1);

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

/// Debug overlay and the power mode it controls.
pub(crate) struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::new(FRAME_HISTORY))
            .insert_resource(low_power_settings())
            .init_state::<OverlayState>()
            .add_systems(Update, toggle_overlay)
            .add_systems(
                OnEnter(OverlayState::Shown),
                (spawn_overlay, use_continuous_updates),
            )
            .add_systems(OnExit(OverlayState::Shown), use_low_power_updates)
            .add_systems(
                Update,
                refresh_overlay.run_if(in_state(OverlayState::Shown)),
            );
    }
}

fn low_power_settings() -> WinitSettings {
    WinitSettings {
        focused_mode: UpdateMode::reactive_low_power(LOW_POWER_WAIT),
        unfocused_mode: UpdateMode::reactive_low_power(LOW_POWER_WAIT),
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

fn use_continuous_updates(mut settings: ResMut<WinitSettings>) {
    *settings = WinitSettings::continuous();
}

fn use_low_power_updates(mut settings: ResMut<WinitSettings>) {
    *settings = low_power_settings();
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
}

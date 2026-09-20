//! The screens the game can be on, and the power mode each one uses.

use core::time::Duration;

use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};

/// Which screen the game is showing (Section 8).
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum AppState {
    /// Title screen.
    #[default]
    MainMenu,
    /// Choosing a level.
    LevelSelect,
    /// The field journal.
    Journal,
    /// Playing a level.
    Playing,
    /// Paused during play.
    Paused,
    /// The results of a finished level.
    LevelComplete,
}

impl AppState {
    /// Whether this screen animates constantly. Everything else only redraws
    /// when something happens, which keeps laptops quiet (Section 7.1).
    fn needs_continuous_updates(self) -> bool {
        matches!(self, Self::Playing)
    }
}

/// While a still screen is showing, wake up at most this often (or on input).
const IDLE_WAIT: Duration = Duration::from_millis(250);

/// App states and their power modes.
pub(crate) struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Update, update_power_mode.run_if(state_changed::<AppState>));
    }
}

fn update_power_mode(state: Res<State<AppState>>, mut settings: ResMut<WinitSettings>) {
    *settings = if state.get().needs_continuous_updates() {
        WinitSettings::game()
    } else {
        WinitSettings {
            focused_mode: UpdateMode::reactive(IDLE_WAIT),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),
        }
    };
}

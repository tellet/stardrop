//! Player settings: sound levels, fullscreen, graphics quality and reduce
//! motion. Stored next to the save file and applied as soon as they change.

use bevy::prelude::*;
use bevy::window::{Monitor, MonitorSelection, PrimaryMonitor, WindowMode};
use serde::{Deserialize, Serialize};

use crate::save::{SaveFolder, read_file, write_file};

const SETTINGS_FILE: &str = "settings.ron";
/// What this build writes. See `save.rs` for how versions are handled.
pub const SETTINGS_VERSION: u32 = 1;
/// The playfield's shape: the window opens the same way round.
const PORTRAIT: f32 = 9.0 / 16.0;
/// How tall the window opens, if the monitor is big enough for it.
const PREFERRED_HEIGHT: f32 = 1200.0;
/// Otherwise it takes this much of the monitor's height.
const SCREEN_SHARE: f32 = 0.85;
/// A resized window is only written to the settings file once it has been
/// left alone this long, so dragging an edge doesn't write it hundreds of times.
const RESIZE_SETTLE: f32 = 1.0;
/// Volumes step through these values (percent) when a row is activated.
const VOLUME_STEPS: [u8; 5] = [0, 25, 50, 75, 100];

/// How much visual effort the game puts in (Section 7.2's graphics quality).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Quality {
    /// Weaker machines: no bloom.
    Plain,
    /// Everything on.
    #[default]
    Full,
}

impl Quality {
    /// The next setting, wrapping around.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Plain => Self::Full,
            Self::Full => Self::Plain,
        }
    }

    /// What to call it on screen.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Full => "full",
        }
    }
}

/// Everything the player can change. Volumes are percentages, and each one
/// sets the level of its own audio channel (`audio.rs`).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
// Unknown fields are allowed, for the same reason as the save file: a
// settings file from a later build must still open here.
#[serde(default)]
pub struct Settings {
    /// Which build's format this is (`0`: written before versioning).
    pub version: u32,
    /// Overall loudness.
    pub master_volume: u8,
    /// Music loudness.
    pub music_volume: u8,
    /// Sound effect loudness.
    pub sfx_volume: u8,
    /// Background ambience loudness.
    pub ambience_volume: u8,
    /// Fullscreen instead of a window.
    pub fullscreen: bool,
    /// Graphics quality.
    pub quality: Quality,
    /// Calms background motion and screen effects.
    pub reduce_motion: bool,
    /// The window size the player last left behind, in logical pixels.
    /// `None` until they resize it: the game picks one that fits the monitor.
    pub window_size: Option<(f32, f32)>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            master_volume: 100,
            music_volume: 80,
            sfx_volume: 100,
            ambience_volume: 80,
            fullscreen: false,
            quality: Quality::Full,
            reduce_motion: false,
            window_size: None,
        }
    }
}

impl Settings {
    /// Reads settings from RON text.
    ///
    /// # Errors
    /// Returns the RON error if the text isn't valid settings.
    pub fn parse(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(text)
    }

    /// The settings as RON text.
    #[must_use]
    pub fn to_text(self) -> String {
        ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default())
            .unwrap_or_else(|_| "()".to_owned())
    }

    /// How much background motion to keep (1.0 normally, much less when
    /// motion is reduced).
    #[must_use]
    pub fn motion_scale(self) -> f32 {
        if self.reduce_motion { 0.25 } else { 1.0 }
    }
}

/// The next volume step above `percent`, wrapping back to the quietest.
#[must_use]
pub fn next_volume(percent: u8) -> u8 {
    VOLUME_STEPS
        .iter()
        .copied()
        .find(|step| *step > percent)
        .unwrap_or(VOLUME_STEPS[0])
}

/// Settings: loading, applying and saving.
pub(crate) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Settings>()
            .add_systems(Startup, load_settings)
            .add_systems(
                Update,
                (apply_window_mode, store_settings).run_if(resource_exists_and_changed::<Settings>),
            )
            .add_systems(Update, (size_window_once, remember_window_size));
    }
}

/// Settings that can't be read are ignored, leaving the defaults in place.
fn load_settings(folder: Res<SaveFolder>, mut settings: ResMut<Settings>) {
    let Some(folder) = folder.0.as_deref() else {
        return;
    };
    match read_file(folder, SETTINGS_FILE) {
        Ok(Some(text)) => match Settings::parse(&text) {
            Ok(loaded) => {
                *settings = Settings {
                    version: SETTINGS_VERSION,
                    ..loaded
                };
            }
            Err(error) => warn!("settings unreadable ({error}); using the defaults"),
        },
        Ok(None) => {}
        Err(error) => warn!("settings unreadable ({error}); using the defaults"),
    }
}

fn store_settings(folder: Res<SaveFolder>, settings: Res<Settings>) {
    let Some(folder) = folder.0.as_deref() else {
        return;
    };
    if let Err(error) = write_file(folder, SETTINGS_FILE, &settings.to_text()) {
        warn!("could not save settings: {error}");
    }
}

/// Sizes the window the first time the monitor is known: the size the player
/// left behind, or a portrait window that fits their screen.
///
/// Waiting for the monitor is why this isn't part of the window's opening
/// settings: the monitor's size isn't known until the window exists.
fn size_window_once(
    mut done: Local<bool>,
    settings: Res<Settings>,
    monitor: Query<&Monitor, With<PrimaryMonitor>>,
    mut window: Single<&mut Window>,
) {
    let Ok(monitor) = monitor.single() else {
        return;
    };
    if *done {
        return;
    }
    *done = true;
    // Physical pixels over the scale factor: what the player sees as "size".
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "screen sizes are small numbers"
    )]
    let monitor_height = monitor.physical_height as f32 / monitor.scale_factor as f32;
    let (width, height) = settings.window_size.unwrap_or_else(|| {
        let height = PREFERRED_HEIGHT.min(monitor_height * SCREEN_SHARE);
        (height * PORTRAIT, height)
    });
    window.resolution.set(width, height);
    window.position.center(MonitorSelection::Primary);
}

/// Remembers a window the player resized, once they've stopped dragging.
fn remember_window_size(
    time: Res<Time>,
    window: Single<&Window>,
    mut settings: ResMut<Settings>,
    mut steady: Local<f32>,
    mut last: Local<Vec2>,
) {
    if window.mode != WindowMode::Windowed {
        return;
    }
    let size = window.resolution.size();
    if size.distance_squared(*last) > 0.25 {
        *last = size;
        *steady = 0.0;
        return;
    }
    if settings.window_size == Some((size.x, size.y)) {
        return;
    }
    *steady += time.delta_secs();
    if *steady >= RESIZE_SETTLE {
        settings.window_size = Some((size.x, size.y));
    }
}

fn apply_window_mode(settings: Res<Settings>, mut window: Single<&mut Window>) {
    let wanted = if settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    if window.mode != wanted {
        window.mode = wanted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volumes_step_and_wrap() {
        assert_eq!(next_volume(0), 25);
        assert_eq!(next_volume(75), 100);
        assert_eq!(next_volume(100), 0, "wraps back to silent");
        // An odd value from a hand-edited file still moves somewhere sensible.
        assert_eq!(next_volume(30), 50);
    }

    #[test]
    fn settings_round_trip_through_text() {
        let settings = Settings {
            master_volume: 50,
            fullscreen: true,
            quality: Quality::Plain,
            reduce_motion: true,
            ..Settings::default()
        };
        assert_eq!(Settings::parse(&settings.to_text()), Ok(settings));
    }

    #[test]
    fn reduce_motion_calms_the_background() {
        assert!((Settings::default().motion_scale() - 1.0).abs() < 1e-6);
        let calm = Settings {
            reduce_motion: true,
            ..Settings::default()
        };
        assert!(calm.motion_scale() < 0.5);
    }
}

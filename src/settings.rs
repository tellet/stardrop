//! Player settings: sound levels, fullscreen, graphics quality and reduce
//! motion. Stored next to the save file and applied as soon as they change.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode};
use serde::{Deserialize, Serialize};

use crate::save::{SaveFolder, read_file, write_file};

const SETTINGS_FILE: &str = "settings.ron";
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

/// Everything the player can change. Volumes are percentages; audio arrives
/// in Milestone 8 and will read them.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 100,
            music_volume: 80,
            sfx_volume: 100,
            ambience_volume: 80,
            fullscreen: false,
            quality: Quality::Full,
            reduce_motion: false,
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
                (apply_window_mode, apply_quality, store_settings)
                    .run_if(resource_exists_and_changed::<Settings>),
            );
    }
}

/// Settings that can't be read are ignored, leaving the defaults in place.
fn load_settings(folder: Res<SaveFolder>, mut settings: ResMut<Settings>) {
    let Some(folder) = folder.0.as_deref() else {
        return;
    };
    match read_file(folder, SETTINGS_FILE) {
        Ok(Some(text)) => match Settings::parse(&text) {
            Ok(loaded) => *settings = loaded,
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

/// Bloom is the expensive part of our look, so "plain" turns it off.
fn apply_quality(
    settings: Res<Settings>,
    mut commands: Commands,
    cameras: Query<Entity, With<Camera2d>>,
) {
    for camera in &cameras {
        let mut entity = commands.entity(camera);
        match settings.quality {
            Quality::Full => {
                entity.insert(crate::camera::bloom());
            }
            Quality::Plain => {
                entity.remove::<bevy::post_process::bloom::Bloom>();
            }
        }
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

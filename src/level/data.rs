//! Level files: their on-disk format and the validated, ready-to-play [`Level`].
//!
//! Plain Rust, no ECS. Parsing and validation are pure, so they're unit-tested
//! with deliberately broken level text, and `cargo test` checks every shipped
//! level. See `docs/LEVELS.md` for the format with an annotated example.
//!
//! File coordinates are designer-friendly: `x` from the level's left edge
//! (0 to [`LEVEL_WIDTH`]) and `depth` downward from the top. [`Level`] converts
//! them to world coordinates: x = 0 at the center, y = 0 at the top, y
//! negative going down.

use bevy::math::Vec2;
use serde::Deserialize;

use super::validate::{self, LevelError};
use crate::physics::movement::Walls;

/// Every level is exactly this wide, in world units, and always fully visible.
pub const LEVEL_WIDTH: f32 = 1920.0;

/// A position in a level file: `(x, depth)`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(super) struct FilePoint(pub f32, pub f32);

/// The beacon as written in a level file.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileBeacon {
    /// Center of the landing pad.
    pub center: FilePoint,
    /// Width and height of the landing pad.
    pub size: (f32, f32),
}

/// A level file exactly as written on disk, before validation.
///
/// `deny_unknown_fields` makes any unexpected field an error. That enforces
/// the consistency rule (Section 3.3): a level can place things, but it has
/// no way to change how they behave.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LevelFile {
    pub name: String,
    pub height: f32,
    pub theme: Theme,
    pub start: FilePoint,
    pub beacon: FileBeacon,
    #[serde(default)]
    pub checkpoints: Vec<FilePoint>,
    #[serde(default)]
    pub stardust: Vec<FilePoint>,
    pub target_time: f32,
}

/// The altitude theme: which palette and scenery a level uses (Section 4).
/// Visuals arrive in Milestone 7; for now every theme looks like deep space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Theme {
    /// Starfield and distant nebula.
    DeepSpace,
    /// Glowing space clouds.
    SpaceClouds,
    /// Upper atmosphere, aurora curtains.
    UpperAtmosphere,
    /// Aurora with floating crystal fields.
    CrystalAurora,
    /// High clouds, distant lightning glow.
    HighClouds,
    /// Above the cloud sea, pastel dawn.
    CloudSea,
    /// Sunset layer, falling meteor streaks.
    Sunset,
    /// Warm evening sky, floating blossoms.
    WarmEvening,
    /// Twilight, first stars and planet lights below.
    Twilight,
    /// Night-to-dawn over the landscape.
    Homecoming,
}

/// The landing pad, in world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beacon {
    /// Center of the pad.
    pub center: Vec2,
    /// Full width and height of the pad.
    pub size: Vec2,
}

/// A validated level in world coordinates. The only way to get one is
/// [`Level::parse`], so every `Level` in the game is known to be valid.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    /// Shown when the level starts.
    pub name: String,
    /// World units from the top (y = 0) to the bottom (y = -height).
    pub height: f32,
    /// Palette and scenery.
    pub theme: Theme,
    /// Where the astronaut starts, at rest.
    pub start: Vec2,
    /// The goal at the bottom.
    pub beacon: Beacon,
    /// Checkpoint ring centers, top to bottom. Passing a ring's height activates it.
    pub checkpoints: Vec<Vec2>,
    /// Stardust mote positions.
    pub stardust: Vec<Vec2>,
    /// Seconds for the "fast run" star.
    pub target_time: f32,
}

impl Level {
    /// Reads and validates a level file. Any problem comes back as a
    /// [`LevelError`] describing what's wrong and where. It never panics.
    ///
    /// # Errors
    /// Returns a [`LevelError`] if the text isn't valid RON for our format or
    /// the level breaks a rule (see [`LevelError`]).
    pub fn parse(text: &str) -> Result<Self, LevelError> {
        // `?` hands any error straight back to our caller. `From` (in
        // `validate.rs`) converts the RON error into our own `LevelError`.
        let file: LevelFile = ron::from_str(text)?;
        validate::check(&file)?;
        Ok(Self::from_file(file))
    }

    /// Lowest y of the level.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        -self.height
    }

    /// Sideways limits for the center of a circle of `radius`.
    #[must_use]
    pub fn walls(&self, radius: f32) -> Walls {
        let half = LEVEL_WIDTH / 2.0 - radius;
        Walls {
            left: -half,
            right: half,
        }
    }

    /// Whether `point` (world coordinates) is within the level's edges.
    #[must_use]
    pub fn contains(&self, point: Vec2) -> bool {
        point.x.abs() <= LEVEL_WIDTH / 2.0 && (self.bottom()..=0.0).contains(&point.y)
    }

    /// Converts already-validated file data to world coordinates.
    fn from_file(file: LevelFile) -> Self {
        Self {
            start: to_world(file.start),
            beacon: Beacon {
                center: to_world(file.beacon.center),
                size: Vec2::new(file.beacon.size.0, file.beacon.size.1),
            },
            checkpoints: file.checkpoints.into_iter().map(to_world).collect(),
            stardust: file.stardust.into_iter().map(to_world).collect(),
            name: file.name,
            height: file.height,
            theme: file.theme,
            target_time: file.target_time,
        }
    }
}

#[cfg(test)]
impl Level {
    /// A plain ten-screen level for tests elsewhere in the crate.
    pub(crate) fn for_tests() -> Self {
        let text = r#"(
            name: "Tests",
            height: 10800,
            theme: DeepSpace,
            start: (960, 300),
            beacon: (center: (960, 10770), size: (360, 40)),
            target_time: 30.0,
        )"#;
        match Self::parse(text) {
            Ok(level) => level,
            Err(error) => panic!("test level is invalid: {error}"),
        }
    }
}

/// File `(x from left, depth down)` → world `(x from center, y up)`.
fn to_world(point: FilePoint) -> Vec2 {
    Vec2::new(point.0 - LEVEL_WIDTH / 2.0, -point.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"(
        name: "Test",
        height: 5400,
        theme: DeepSpace,
        start: (960, 300),
        beacon: (center: (960, 5380), size: (320, 40)),
        checkpoints: [(960, 2000), (500, 4000)],
        stardust: [(200, 1000)],
        target_time: 20.0,
    )"#;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "conversion only negates and shifts by exact values"
    )]
    fn converts_file_coordinates_to_world() {
        let level = Level::parse(VALID).unwrap();
        assert_eq!(level.start, Vec2::new(0.0, -300.0));
        assert_eq!(level.checkpoints[1], Vec2::new(-460.0, -4000.0));
        assert_eq!(level.beacon.center, Vec2::new(0.0, -5380.0));
        assert_eq!(level.bottom(), -5400.0);
    }

    #[test]
    fn optional_lists_may_be_left_out() {
        let text = VALID
            .replace("checkpoints: [(960, 2000), (500, 4000)],", "")
            .replace("stardust: [(200, 1000)],", "");
        let level = Level::parse(&text).unwrap();
        assert!(level.checkpoints.is_empty() && level.stardust.is_empty());
    }

    /// `cargo test` loads and validates every shipped level (Section 4).
    #[test]
    fn every_shipped_level_is_valid() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/levels");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if !path.to_string_lossy().ends_with(".level.ron") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            if let Err(error) = Level::parse(&text) {
                panic!("{}: {error}", path.display());
            }
            checked += 1;
        }
        assert!(checked > 0, "no levels found in {dir}");
    }
}

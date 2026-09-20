//! Saved progress: which levels are finished, best times, stars, stardust
//! and journal discoveries.
//!
//! The file is written **atomically**: a temporary file is written and then
//! renamed over the real one, so a crash mid-write can't leave half a file.
//! The previous save is kept as a backup, and a file that can't be read is
//! moved aside rather than overwritten, so progress is never lost silently
//! (Section 9, Milestone 5).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::collectibles::CurrentRun;
use crate::journal::ObjectDiscovered;
use crate::level::{ActiveLevel, LevelCompleted, Levels, SelectedLevel};
use crate::objects::{ObjectKind, ObjectTouched};
use crate::scoring::Stars;

/// Folder inside the OS's data directory.
const FOLDER: &str = "stardrop";
const SAVE_FILE: &str = "save.ron";
const BACKUP_FILE: &str = "save.backup.ron";
const BROKEN_FILE: &str = "save.unreadable.ron";
const TEMP_FILE: &str = "save.writing.ron";

/// The best results for one level.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LevelRecord {
    /// Whether the beacon has ever been reached.
    pub completed: bool,
    /// Fastest landing, in seconds.
    pub best_time: Option<f32>,
    /// Most stardust collected in one attempt.
    pub most_stardust: usize,
    /// Best star for landing.
    pub star_landed: bool,
    /// Best star for collecting everything in one attempt.
    pub star_all_stardust: bool,
    /// Best star for beating the target time.
    pub star_fast: bool,
}

impl LevelRecord {
    /// The stars earned so far on this level.
    #[must_use]
    pub fn stars(&self) -> Stars {
        Stars {
            landed: self.star_landed,
            all_stardust: self.star_all_stardust,
            fast: self.star_fast,
        }
    }

    /// Folds one landing into the record, keeping the best of everything.
    fn add(&mut self, time: f32, stardust: usize, stars: Stars) {
        self.completed = true;
        self.best_time = Some(self.best_time.map_or(time, |best| best.min(time)));
        self.most_stardust = self.most_stardust.max(stardust);
        let stars = self.stars().merged(stars);
        self.star_landed = stars.landed;
        self.star_all_stardust = stars.all_stardust;
        self.star_fast = stars.fast;
    }
}

/// Everything the game remembers between sessions.
///
/// Levels are keyed by file name, not by position, so reordering or inserting
/// levels never moves someone's progress to the wrong level.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SaveData {
    /// Results per level file.
    pub levels: BTreeMap<String, LevelRecord>,
    /// Object types discovered, by name (stable across versions).
    pub discovered: Vec<String>,
}

impl SaveData {
    /// Reads save data from RON text.
    ///
    /// # Errors
    /// Returns the RON error if the text isn't valid save data.
    pub fn parse(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(text)
    }

    /// The save data as RON text.
    #[must_use]
    pub fn to_text(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .unwrap_or_else(|_| "()".to_owned())
    }

    /// Records a landing on `level_file`.
    pub fn record_landing(&mut self, level_file: &str, time: f32, stardust: usize, stars: Stars) {
        self.levels
            .entry(level_file.to_owned())
            .or_default()
            .add(time, stardust, stars);
    }

    /// The record for a level, or an empty one.
    #[must_use]
    pub fn level(&self, level_file: &str) -> LevelRecord {
        self.levels.get(level_file).copied().unwrap_or_default()
    }

    /// Whether the level at `index` can be played: the first one always, and
    /// after that only once the level before it is finished.
    #[must_use]
    pub fn is_unlocked(&self, index: usize, order: &[String]) -> bool {
        match index.checked_sub(1) {
            None => true,
            Some(previous) => order
                .get(previous)
                .is_some_and(|file| self.level(file).completed),
        }
    }

    /// Notes that an object type has been seen. Returns `true` the first time.
    pub fn discover(&mut self, kind: ObjectKind) -> bool {
        let name = format!("{kind:?}");
        if self.discovered.contains(&name) {
            return false;
        }
        self.discovered.push(name);
        true
    }

    /// Whether this object type has been discovered.
    #[must_use]
    pub fn has_discovered(&self, kind: ObjectKind) -> bool {
        self.discovered.contains(&format!("{kind:?}"))
    }
}

/// Where the save file lives: the OS's data folder, e.g.
/// `~/.local/share/stardrop/` on Linux. `None` if the OS won't tell us.
#[must_use]
pub fn save_folder() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(FOLDER))
}

/// Loads the save file, falling back to the backup, then to a fresh start.
///
/// A file that exists but can't be read is renamed to `save.unreadable.ron`
/// instead of being overwritten, so nothing is lost silently.
#[must_use]
pub fn load(folder: &Path) -> SaveData {
    match read_valid(&folder.join(SAVE_FILE)) {
        Ok(Some(data)) => return data,
        Ok(None) => {}
        Err(error) => {
            warn!("save file unreadable ({error}); trying the backup");
            let broken = folder.join(BROKEN_FILE);
            if let Err(error) = fs::rename(folder.join(SAVE_FILE), &broken) {
                warn!("could not set the unreadable save aside: {error}");
            }
        }
    }
    match read_valid(&folder.join(BACKUP_FILE)) {
        Ok(Some(data)) => {
            info!("recovered progress from the backup save");
            data
        }
        Ok(None) => SaveData::default(),
        Err(error) => {
            warn!("backup save unreadable ({error}); starting fresh");
            SaveData::default()
        }
    }
}

/// `Ok(None)` when the file simply isn't there yet.
fn read_valid(path: &Path) -> Result<Option<SaveData>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    SaveData::parse(&text).map(Some).map_err(|e| e.to_string())
}

/// Reads a text file from the save folder, if it exists.
///
/// # Errors
/// Returns a message if the file exists but can't be read.
pub fn read_file(folder: &Path, name: &str) -> Result<Option<String>, String> {
    match fs::read_to_string(folder.join(name)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Writes `text` to `name` in the save folder, atomically: into a temporary
/// file first, then renamed into place.
///
/// # Errors
/// Returns a message if the folder or file can't be written.
pub fn write_file(folder: &Path, name: &str, text: &str) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let temp = folder.join(format!("{name}.writing"));
    fs::write(&temp, text).map_err(|error| format!("{}: {error}", temp.display()))?;
    let target = folder.join(name);
    fs::rename(&temp, &target).map_err(|error| format!("{}: {error}", target.display()))
}

/// Writes the save file atomically, keeping the previous one as a backup.
///
/// # Errors
/// Returns a message if the folder or file can't be written.
pub fn store(folder: &Path, data: &SaveData) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let temp = folder.join(TEMP_FILE);
    let target = folder.join(SAVE_FILE);
    fs::write(&temp, data.to_text()).map_err(|error| format!("{}: {error}", temp.display()))?;
    // Keep the last good save; ignore the error when there isn't one yet.
    let _ = fs::rename(&target, folder.join(BACKUP_FILE));
    // Rename is atomic: readers see either the old file or the new one.
    fs::rename(&temp, &target).map_err(|error| format!("{}: {error}", target.display()))
}

/// The player's progress, as loaded from (and written back to) disk.
#[derive(Resource, Debug, Default, Deref, DerefMut)]
pub(crate) struct Progress(pub SaveData);

/// Where the save file goes. `None` if the OS wouldn't say, in which case the
/// game still plays, it just can't remember anything.
#[derive(Resource, Debug)]
pub(crate) struct SaveFolder(pub Option<PathBuf>);

/// Loading, recording and writing progress.
pub(crate) struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        let folder = save_folder();
        let progress = folder.as_deref().map(load).unwrap_or_default();
        match folder.as_deref() {
            Some(folder) => info!("progress is saved in {}", folder.display()),
            None => warn!("no data folder on this system: progress won't be saved"),
        }
        app.insert_resource(SaveFolder(folder))
            .insert_resource(Progress(progress))
            .add_systems(
                Update,
                (
                    record_landing.run_if(on_message::<LevelCompleted>),
                    record_discoveries.run_if(on_message::<ObjectTouched>),
                ),
            )
            // Written when leaving play and when quitting, not on every change.
            .add_systems(OnEnter(AppState::LevelSelect), write_progress)
            .add_systems(OnEnter(AppState::MainMenu), write_progress)
            .add_systems(OnEnter(AppState::LevelComplete), write_progress)
            .add_systems(Update, write_progress.run_if(on_message::<AppExit>));
    }
}

/// Folds a finished level into the saved records.
fn record_landing(
    mut progress: ResMut<Progress>,
    run: Res<CurrentRun>,
    level: Res<ActiveLevel>,
    levels: Option<Res<Levels>>,
    selected: Res<SelectedLevel>,
) {
    let Some(entry) = levels.as_ref().and_then(|levels| levels.entry(*selected)) else {
        return;
    };
    let stars = Stars::earned(run.0, level.stardust.len(), level.target_time);
    progress.record_landing(&entry.file, run.elapsed, run.stardust.count(), stars);
}

/// Remembers every object type the astronaut has touched (for the journal).
fn record_discoveries(
    mut touched: MessageReader<ObjectTouched>,
    mut progress: ResMut<Progress>,
    mut discovered: MessageWriter<ObjectDiscovered>,
) {
    for touch in touched.read() {
        if progress.discover(touch.kind) {
            discovered.write(ObjectDiscovered(touch.kind));
        }
    }
}

fn write_progress(folder: Res<SaveFolder>, progress: Res<Progress>) {
    let Some(folder) = folder.0.as_deref() else {
        return;
    };
    if let Err(error) = store(folder, &progress.0) {
        warn!("could not save progress: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::{RunStats, StardustSet};

    /// A fresh empty folder under `target/`, so tests never touch real saves.
    fn temp_folder(name: &str) -> PathBuf {
        let folder =
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/test-saves")).join(name);
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn landing(time: f32, collected: usize) -> (f32, usize, Stars) {
        let mut stardust = StardustSet::default();
        for index in 0..collected {
            stardust.insert(index);
        }
        let run = RunStats {
            elapsed: time,
            stardust,
        };
        (time, collected, Stars::earned(run, 3, 20.0))
    }

    #[test]
    fn records_keep_the_best_of_every_attempt() {
        let mut save = SaveData::default();
        let (time, dust, stars) = landing(30.0, 3);
        save.record_landing("01.level.ron", time, dust, stars);
        let (time, dust, stars) = landing(12.0, 1);
        save.record_landing("01.level.ron", time, dust, stars);

        let record = save.level("01.level.ron");
        assert!(record.completed);
        assert_eq!(record.best_time, Some(12.0), "the fastest time");
        assert_eq!(record.most_stardust, 3, "the best collection");
        assert_eq!(record.stars().count(), 3, "stars from both attempts");
    }

    #[test]
    fn levels_unlock_one_after_another() {
        let order: Vec<String> = ["a.ron", "b.ron", "c.ron"].map(str::to_owned).to_vec();
        let mut save = SaveData::default();
        assert!(
            save.is_unlocked(0, &order),
            "the first level is always open"
        );
        assert!(!save.is_unlocked(1, &order));
        let (time, dust, stars) = landing(10.0, 3);
        save.record_landing("a.ron", time, dust, stars);
        assert!(save.is_unlocked(1, &order));
        assert!(!save.is_unlocked(2, &order));
    }

    #[test]
    fn discoveries_are_remembered_once() {
        let mut save = SaveData::default();
        assert!(save.discover(ObjectKind::CloudPuff));
        assert!(
            !save.discover(ObjectKind::CloudPuff),
            "only new the first time"
        );
        assert!(save.has_discovered(ObjectKind::CloudPuff));
        assert!(!save.has_discovered(ObjectKind::Crystal));
    }

    #[test]
    fn saving_and_loading_round_trips() {
        let folder = temp_folder("round-trip");
        let mut save = SaveData::default();
        let (time, dust, stars) = landing(14.5, 2);
        save.record_landing("01.level.ron", time, dust, stars);
        save.discover(ObjectKind::WindRibbon);
        store(&folder, &save).unwrap();
        assert_eq!(load(&folder), save);
    }

    #[test]
    fn a_missing_save_starts_fresh() {
        let folder = temp_folder("missing");
        assert_eq!(load(&folder), SaveData::default());
    }

    #[test]
    fn a_broken_save_falls_back_to_the_backup_and_keeps_the_broken_file() {
        let folder = temp_folder("broken");
        let mut save = SaveData::default();
        let (time, dust, stars) = landing(9.0, 3);
        save.record_landing("01.level.ron", time, dust, stars);
        store(&folder, &save).unwrap();
        // A second save turns the first into the backup.
        store(&folder, &save).unwrap();
        fs::write(folder.join(SAVE_FILE), "this is not a save file").unwrap();

        assert_eq!(load(&folder), save, "recovered from the backup");
        assert!(
            folder.join(BROKEN_FILE).exists(),
            "the unreadable file is kept, not overwritten"
        );
    }

    #[test]
    fn both_files_broken_starts_fresh_without_crashing() {
        let folder = temp_folder("both-broken");
        fs::write(folder.join(SAVE_FILE), "nonsense").unwrap();
        fs::write(folder.join(BACKUP_FILE), "also nonsense").unwrap();
        assert_eq!(load(&folder), SaveData::default());
    }

    #[test]
    fn unknown_fields_are_rejected_rather_than_silently_dropped() {
        assert!(SaveData::parse("(levels: {}, discovered: [], mystery: 1)").is_err());
    }
}

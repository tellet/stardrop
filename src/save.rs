//! Saved progress: which levels are finished, best times, stars, stardust
//! and journal discoveries.
//!
//! The file is written **atomically**: a temporary file is written and then
//! renamed over the real one, so a crash mid-write can't leave half a file.
//! The previous save is kept as a backup, and a file that can't be read is
//! moved aside rather than overwritten, so progress is never lost silently
//! (Section 9, Milestone 5).
//!
//! All of that sits on three operations (read, write, rename a named file in
//! the save folder), which the `files` module provides twice: real files on
//! the desktop, the browser's own storage on the web.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::collectibles::CurrentRun;
use crate::journal::ObjectDiscovered;
use crate::level::{ActiveLevel, LevelCompleted, Levels, SelectedLevel};
use crate::objects::{ObjectKind, ObjectTouched};
use crate::scoring::Stars;

/// Folder inside the OS's data directory, and the prefix every key in a
/// browser's storage starts with.
const FOLDER: &str = "stardrop";
const SAVE_FILE: &str = "save.ron";
const BACKUP_FILE: &str = "save.backup.ron";
const BROKEN_FILE: &str = "save.unreadable.ron";
const TEMP_FILE: &str = "save.writing.ron";

/// The best results for one level.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
// Part of the save file, so it follows the same rule: a record from a
// later build opens here, carrying what this build understands.
#[serde(default)]
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
    pub star_phrase: bool,
}

impl LevelRecord {
    /// The stars earned so far on this level.
    #[must_use]
    pub fn stars(&self) -> Stars {
        Stars {
            landed: self.star_landed,
            all_stardust: self.star_all_stardust,
            phrase: self.star_phrase,
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
        self.star_phrase = stars.phrase;
    }
}

/// What this build writes. Raise it when the meaning of a field changes,
/// and teach [`SaveData::migrated`] how to bring the old one forward.
pub const SAVE_VERSION: u32 = 2;

/// Everything the game remembers between sessions.
///
/// Levels are keyed by file name, not by position, so reordering or inserting
/// levels never moves someone's progress to the wrong level.
///
/// Unknown fields are *allowed* here, unlike in a level file: a save written
/// by a later build must still open in this one, carrying what it can. The
/// fields that build added are lost when this one writes the file back, which
/// is the price of opening it at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    /// Which build's format this is. A file with no version at all was
    /// written before versioning existed, and reads as 0.
    #[serde(default = "before_versioning")]
    pub version: u32,
    /// Results per level file.
    pub levels: BTreeMap<String, LevelRecord>,
    /// Object types discovered, by name (stable across versions).
    pub discovered: Vec<String>,
}

/// What a file with no version field in it counts as.
const fn before_versioning() -> u32 {
    0
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            levels: BTreeMap::new(),
            discovered: Vec::new(),
        }
    }
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

    /// Brings a save from an older build up to date, and stamps it.
    ///
    /// Nothing has had to change yet: version 0 (before versioning) and
    /// version 1 hold the same fields. The path exists so that the first
    /// time something *does* change, there is somewhere to put it.
    #[must_use]
    pub fn migrated(mut self) -> Self {
        // Version 1 kept a "fast run" star, which version 2 replaced with
        // the phrase star (docs/DESIGN.md M10-1). They mean different things,
        // so the old one is dropped rather than carried across: a star has
        // to have been earned at the thing it is for.
        if self.version < 2 {
            for record in self.levels.values_mut() {
                record.star_phrase = false;
            }
        }
        if self.version > SAVE_VERSION {
            warn!(
                "this save was written by a later version of the game ({} > {SAVE_VERSION}); \
                 opening it anyway, and anything newer in it will be dropped",
                self.version
            );
        }
        self.version = SAVE_VERSION;
        self
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

    /// Whether every level has been landed. An empty list is not a finished
    /// game: it means the levels never loaded.
    #[must_use]
    pub fn finished_everything(&self, order: &[String]) -> bool {
        !order.is_empty() && order.iter().all(|file| self.level(file).completed)
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
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn save_folder() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(FOLDER))
}

/// A browser has no folders: the save "folder" is the prefix every key in the
/// browser's own storage starts with.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn save_folder() -> Option<PathBuf> {
    Some(PathBuf::from(FOLDER))
}

/// The key one save file goes under in a browser's storage.
#[cfg_attr(
    not(target_arch = "wasm32"),
    allow(dead_code, reason = "only the browser build stores files by key")
)]
#[must_use]
fn storage_key(folder: &Path, name: &str) -> String {
    format!("{}/{name}", folder.display())
}

/// Files in a folder, on a machine that has both.
#[cfg(not(target_arch = "wasm32"))]
mod files {
    use std::fs;
    use std::io::ErrorKind;
    use std::path::Path;

    /// `Ok(None)` when the file simply isn't there yet.
    pub(super) fn read(folder: &Path, name: &str) -> Result<Option<String>, String> {
        match fs::read_to_string(folder.join(name)) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Creates the folder if this is the first write.
    pub(super) fn write(folder: &Path, name: &str, text: &str) -> Result<(), String> {
        fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        let path = folder.join(name);
        fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Moves `from` onto `to`. A missing `from` is no error: there was
    /// nothing to move. Rename is atomic, so a reader sees one file or the
    /// other, never half of either.
    pub(super) fn rename(folder: &Path, from: &str, to: &str) -> Result<(), String> {
        let target = folder.join(to);
        match fs::rename(folder.join(from), &target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("{}: {error}", target.display())),
        }
    }

    /// Where to tell the player their progress is kept.
    pub(super) fn describe(folder: &Path) -> String {
        folder.display().to_string()
    }
}

/// The browser's own storage, one key per file. It survives a reload and a
/// closed tab; clearing the site's data wipes it.
#[cfg(target_arch = "wasm32")]
mod files {
    use std::path::Path;

    use super::storage_key;

    /// Private browsing refuses storage altogether, and then the game plays
    /// without remembering anything.
    fn storage() -> Result<web_sys::Storage, String> {
        web_sys::window()
            .ok_or_else(|| "no browser window".to_owned())?
            .local_storage()
            .map_err(|_| "the browser refused its storage".to_owned())?
            .ok_or_else(|| "this browser has no storage".to_owned())
    }

    pub(super) fn read(folder: &Path, name: &str) -> Result<Option<String>, String> {
        storage()?
            .get_item(&storage_key(folder, name))
            .map_err(|_| format!("could not read {name}"))
    }

    /// An error here is usually the storage quota, a few megabytes, which a
    /// save of a few kilobytes never reaches.
    pub(super) fn write(folder: &Path, name: &str, text: &str) -> Result<(), String> {
        storage()?
            .set_item(&storage_key(folder, name), text)
            .map_err(|_| format!("could not write {name}"))
    }

    /// Copies `from` onto `to` and drops `from`. A missing `from` is no
    /// error: there was nothing to move.
    pub(super) fn rename(folder: &Path, from: &str, to: &str) -> Result<(), String> {
        let storage = storage()?;
        let Some(text) = storage
            .get_item(&storage_key(folder, from))
            .map_err(|_| format!("could not read {from}"))?
        else {
            return Ok(());
        };
        storage
            .set_item(&storage_key(folder, to), &text)
            .map_err(|_| format!("could not write {to}"))?;
        storage
            .remove_item(&storage_key(folder, from))
            .map_err(|_| format!("could not drop {from}"))
    }

    /// Where to tell the player their progress is kept.
    pub(super) fn describe(_folder: &Path) -> String {
        "the browser's storage".to_owned()
    }
}

/// Loads the save file, falling back to the backup, then to a fresh start.
///
/// A file that exists but can't be read is renamed to `save.unreadable.ron`
/// instead of being overwritten, so nothing is lost silently.
#[must_use]
pub fn load(folder: &Path) -> SaveData {
    match read_valid(folder, SAVE_FILE) {
        Ok(Some(data)) => return data,
        Ok(None) => {}
        Err(error) => {
            warn!("save file unreadable ({error}); trying the backup");
            if let Err(error) = files::rename(folder, SAVE_FILE, BROKEN_FILE) {
                warn!("could not set the unreadable save aside: {error}");
            }
        }
    }
    match read_valid(folder, BACKUP_FILE) {
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
fn read_valid(folder: &Path, name: &str) -> Result<Option<SaveData>, String> {
    let Some(text) = files::read(folder, name)? else {
        return Ok(None);
    };
    SaveData::parse(&text)
        .map(|data| Some(data.migrated()))
        .map_err(|error| error.to_string())
}

/// Reads a text file from the save folder, if it exists.
///
/// # Errors
/// Returns a message if the file exists but can't be read.
pub fn read_file(folder: &Path, name: &str) -> Result<Option<String>, String> {
    files::read(folder, name)
}

/// Writes `text` to `name` in the save folder, atomically: into a temporary
/// file first, then renamed into place.
///
/// # Errors
/// Returns a message if the folder or file can't be written.
pub fn write_file(folder: &Path, name: &str, text: &str) -> Result<(), String> {
    let temp = format!("{name}.writing");
    files::write(folder, &temp, text)?;
    files::rename(folder, &temp, name)
}

/// Writes the save file atomically, keeping the previous one as a backup.
///
/// # Errors
/// Returns a message if the folder or file can't be written.
pub fn store(folder: &Path, data: &SaveData) -> Result<(), String> {
    // Written beside the old save, so a failed write leaves it in place.
    files::write(folder, TEMP_FILE, &data.to_text())?;
    // Keep the last good save; there isn't one the first time.
    let _ = files::rename(folder, SAVE_FILE, BACKUP_FILE);
    files::rename(folder, TEMP_FILE, SAVE_FILE)
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
            Some(folder) => info!("progress is saved in {}", files::describe(folder)),
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
    let stars = Stars::earned(run.0, level.stardust.len(), level.phrase);
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
    use std::fs;

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

    #[test]
    fn a_browser_key_names_the_folder_and_the_file() {
        let folder = PathBuf::from("stardrop");
        assert_eq!(storage_key(&folder, SAVE_FILE), "stardrop/save.ron");
        assert_eq!(
            storage_key(&folder, "settings.ron"),
            "stardrop/settings.ron"
        );
    }

    fn landing(time: f32, collected: usize) -> (f32, usize, Stars) {
        let mut stardust = StardustSet::default();
        for index in 0..collected {
            stardust.insert(index);
        }
        let run = RunStats {
            elapsed: time,
            stardust,
            touches: 5,
        };
        (time, collected, Stars::earned(run, 3, 5))
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
    fn finishing_the_last_level_finishes_everything() {
        let order: Vec<String> = ["a.ron", "b.ron"].map(str::to_owned).to_vec();
        let mut save = SaveData::default();
        assert!(!save.finished_everything(&order));
        let (time, dust, stars) = landing(10.0, 3);
        save.record_landing("a.ron", time, dust, stars);
        assert!(
            !save.finished_everything(&order),
            "one of two is not everything"
        );
        save.record_landing("b.ron", time, dust, stars);
        assert!(save.finished_everything(&order));
        assert!(
            !SaveData::default().finished_everything(&[]),
            "no levels at all is not a finished game"
        );
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

    /// A save written before versioning must still open, with its stars.
    #[test]
    fn a_save_from_before_versioning_still_opens() {
        let old = "(levels: {\"01.ron\": (completed: true, best_time: Some(4.0), \
                   most_stardust: 3, star_landed: true)}, discovered: [])";
        let raw = SaveData::parse(old).expect("an old save is still valid");
        assert_eq!(raw.version, 0, "a file with no version reads as 0");
        let data = raw.migrated();
        assert_eq!(data.version, SAVE_VERSION, "it gets stamped on the way in");
        assert!(data.level("01.ron").completed, "and keeps what it knew");
    }

    /// A save from a later build must open too, carrying what this build
    /// understands. Refusing it would throw away someone's progress.
    #[test]
    fn a_save_from_a_later_build_opens_with_what_we_understand() {
        let newer = "(version: 99, levels: {}, discovered: [\"CloudPuff\"], mystery: 1)";
        let data = SaveData::parse(newer)
            .expect("a later save is not an error")
            .migrated();
        assert_eq!(data.version, SAVE_VERSION);
        assert!(data.has_discovered(ObjectKind::CloudPuff));
    }

    #[test]
    fn a_saved_file_says_which_version_wrote_it() {
        let text = SaveData::default().to_text();
        assert!(text.contains(&format!("version: {SAVE_VERSION}")), "{text}");
    }
}

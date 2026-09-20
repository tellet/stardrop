//! The list of levels the game offers, in order, read from
//! `assets/levels/levels.list.ron`.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::Deserialize;

use super::validate::LevelError;

/// One entry in the level list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelEntry {
    /// Shown in the level select.
    pub name: String,
    /// File name inside `assets/levels/`.
    pub file: String,
}

/// Every level, in the order they're played.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelList {
    /// The levels, first to last.
    pub levels: Vec<LevelEntry>,
}

impl LevelList {
    /// Reads and checks the level list.
    ///
    /// # Errors
    /// Returns a [`LevelError`] if the text isn't valid RON for this format,
    /// the list is empty, or two entries share a file.
    pub fn parse(text: &str) -> Result<Self, LevelError> {
        let list: Self = ron::from_str(text)?;
        if list.levels.is_empty() {
            return Err(LevelError::EmptyLevelList);
        }
        for (index, entry) in list.levels.iter().enumerate() {
            if entry.name.trim().is_empty() || entry.file.trim().is_empty() {
                return Err(LevelError::BadLevelEntry(index));
            }
            if list.levels[..index]
                .iter()
                .any(|other| other.file == entry.file)
            {
                return Err(LevelError::DuplicateLevelFile(index));
            }
        }
        Ok(list)
    }
}

/// The level list as loaded, valid or not (like [`super::loader::LevelAsset`]).
#[derive(Asset, TypePath, Debug)]
pub(crate) struct LevelListAsset(pub Result<LevelList, LevelError>);

/// Reads `levels.ron`.
#[derive(Default, TypePath)]
pub(crate) struct LevelListLoader;

impl AssetLoader for LevelListLoader {
    type Asset = LevelListAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<LevelListAsset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let list = match core::str::from_utf8(&bytes) {
            Ok(text) => LevelList::parse(text),
            Err(_) => Err(LevelError::NotText),
        };
        Ok(LevelListAsset(list))
    }

    fn extensions(&self) -> &[&str] {
        // Bevy matches everything after the first dot, so this can't be
        // plain ".ron": that would collide with the level loader.
        &["list.ron"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"(
        levels: [
            (name: "First Light", file: "01_first_light.level.ron"),
            (name: "Soft Landing", file: "02_soft_landing.level.ron"),
        ],
    )"#;

    #[test]
    fn reads_the_levels_in_order() {
        let list = LevelList::parse(VALID).unwrap();
        assert_eq!(list.levels.len(), 2);
        assert_eq!(list.levels[0].name, "First Light");
    }

    #[test]
    fn empty_or_broken_lists_are_rejected() {
        assert_eq!(
            LevelList::parse("(levels: [])"),
            Err(LevelError::EmptyLevelList)
        );
        let no_name = VALID.replace(r#""First Light""#, r#""  ""#);
        assert_eq!(
            LevelList::parse(&no_name),
            Err(LevelError::BadLevelEntry(0))
        );
        let twice = VALID.replace("02_soft_landing.level.ron", "01_first_light.level.ron");
        assert_eq!(
            LevelList::parse(&twice),
            Err(LevelError::DuplicateLevelFile(1))
        );
    }

    /// The shipped list must name files that exist and are valid levels.
    #[test]
    fn every_listed_level_exists_and_is_valid() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/levels");
        let text = std::fs::read_to_string(format!("{dir}/levels.list.ron")).unwrap();
        let list = LevelList::parse(&text).unwrap();
        assert_eq!(list.levels.len(), 10, "the game has ten levels");
        for entry in &list.levels {
            let path = format!("{dir}/{}", entry.file);
            let level = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", entry.file));
            if let Err(error) = super::super::Level::parse(&level) {
                panic!("{}: {error}", entry.file);
            }
        }
    }
}

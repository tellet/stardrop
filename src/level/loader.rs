//! Loads `.level.ron` files through Bevy's asset system, which gives us
//! hot reloading for free (with the `dev` feature).

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;

use super::data::Level;
use super::validate::LevelError;

/// A level file as loaded: either a valid [`Level`] or what's wrong with it.
///
/// A broken file still "loads" successfully, holding its error. That turns a
/// bad level into ordinary data the game can show on screen, and during hot
/// reload the previous version keeps playing until the file is fixed.
#[derive(Asset, TypePath, Debug)]
pub(crate) struct LevelAsset(pub Result<Level, LevelError>);

/// Reads level files from `assets/levels/`.
#[derive(Default, TypePath)]
pub(crate) struct LevelLoader;

impl AssetLoader for LevelLoader {
    type Asset = LevelAsset;
    type Settings = ();
    /// Only I/O can fail the load itself; level problems are part of the asset.
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<LevelAsset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let level = match core::str::from_utf8(&bytes) {
            Ok(text) => Level::parse(text),
            Err(_) => Err(LevelError::NotText),
        };
        Ok(LevelAsset(level))
    }

    fn extensions(&self) -> &[&str] {
        &["level.ron"]
    }
}

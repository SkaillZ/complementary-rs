use std::{
    borrow::{Borrow, Cow},
    collections::{BTreeSet, HashSet},
    error::Error,
    ffi::OsString,
    fmt::Display,
    fs,
    hash::Hash,
    io,
    path::{Path, PathBuf},
};

use crate::tilemap::{Tilemap, TilemapLoadError};

pub fn get_all_levels() -> Result<Vec<String>, io::Error> {
    let map_file_entries = fs::read_dir("assets/maps")?;

    let mut levels = Vec::new();
    for entry in map_file_entries {
        let path = entry?.path();

        if matches!(path.extension().and_then(|ext| ext.to_str()), Some("cmtm")) {
            if let Some(name_without_extension) = path.file_stem() {
                levels.push(name_without_extension.to_string_lossy().into_owned());
            }
        }
    }

    levels.sort();
    Ok(levels)
}

#[derive(Default)]
pub struct Level {
    pub tilemap: Tilemap,
}

impl Level {
    pub fn load<'a, T: AsRef<str> + ?Sized>(name: &'a T) -> Result<Level, LevelLoadError> {
        let tilemap_path: PathBuf = ["assets", "maps", &format!("{}.cmtm", name.as_ref())]
            .iter()
            .collect();
        let tilemap = Tilemap::load_from_file(tilemap_path)?;
        Ok(Level { tilemap })
    }
}

#[derive(Debug)]
pub enum LevelLoadError {
    Tilemap(TilemapLoadError),
}

impl From<TilemapLoadError> for LevelLoadError {
    fn from(inner: TilemapLoadError) -> Self {
        LevelLoadError::Tilemap(inner)
    }
}

impl Display for LevelLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LevelLoadError::Tilemap(err) => err.fmt(f),
        }
    }
}
impl Error for LevelLoadError {}

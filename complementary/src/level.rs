use std::{fs, io, path::PathBuf, collections::HashMap};

use log::debug;

use crate::{
    objects::{ObjectSet, ObjectSetLoadError},
    tilemap::{Tilemap, TilemapLoadError, TilemapRenderer},
};

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

pub struct Level {
    pub tilemap: Tilemap,
    pub objects: ObjectSet,
    pub state: LevelState,

    pub tilemap_renderer: TilemapRenderer,
}

pub struct LevelState {
    keys_by_group: HashMap<i32, CollectedKeys>
}

#[derive(Default, Copy, Clone)]
pub struct CollectedKeys {
    total_key_count: usize,
    collected_key_count: usize,
}

impl Level {
    pub fn load<'a, T: AsRef<str> + ?Sized>(
        device: &'a wgpu::Device,
        name: &'a T,
    ) -> Result<Level, LevelLoadError> {
        let tilemap_path: PathBuf = ["assets", "maps", &format!("{}.cmtm", name.as_ref())]
            .iter()
            .collect();
        let object_map_path = tilemap_path.with_extension("json");
        debug!("Loaded level: {}", &object_map_path.display());
        let tilemap = Tilemap::load_from_file(tilemap_path)?;
        let mut objects = ObjectSet::load_from_file(object_map_path, &device)?;

        let mut keys_by_group: HashMap<i32, CollectedKeys> = HashMap::new();
        for key in &mut objects.objects.keys {
            let entry = keys_by_group.entry(key.group()).or_default();
            entry.total_key_count += 1;
        }

        let state = LevelState { keys_by_group };

        let tilemap_renderer = TilemapRenderer::new(device, &tilemap);
        Ok(Level {
            tilemap,
            objects,
            state,
            tilemap_renderer,
        })
    }
}

impl LevelState {
    pub fn add_collected_key(&mut self, group: i32) {
        self.keys_by_group.entry(group).or_default().collected_key_count += 1;
    }

    pub fn key_collected_percentage(&self, group: i32) -> f32 {
        let entry = self.keys_by_group.get(&group).expect("Invalid key group");
        if entry.total_key_count == 0 {
            1.0
        } else {
            entry.collected_key_count as f32 / entry.total_key_count as f32
        }
    }

    pub fn all_keys_collected(&self, group: i32) -> bool {
        let entry = self.keys_by_group.get(&group).expect("Invalid key group");
        entry.collected_key_count >= entry.total_key_count
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LevelLoadError {
    #[error("failed to load tilemap: {0}")]
    Tilemap(#[from] TilemapLoadError),
    #[error("failed to load objects: {0}")]
    ObjectSet(#[from] ObjectSetLoadError),
}

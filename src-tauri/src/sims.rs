//! The client's `sims` folder: the war3map.lua of each Smashcraft version it has
//! played (`<version>.lua`, from MapSims) and each version's viewer simulation
//! (`<version>.js`, from SharedReplays). This module is the only place that
//! names the folder.

use std::path::{Path, PathBuf};

use crate::files;

#[derive(Clone)]
pub struct SimStore {
    dir: PathBuf,
}

impl SimStore {
    pub fn new(data: &Path) -> Self {
        Self { dir: data.join("sims") }
    }

    /// The path of the file `name` in the folder.
    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn write(&self, name: &str, text: &str) -> Result<(), String> {
        files::write_atomic(&self.file(name), text)
    }

    pub fn read(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.file(name)).ok()
    }

    /// The names without `suffix` of the files that end in it, sorted; none when the folder can't be read.
    pub fn stems(&self, suffix: &str) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut stems: Vec<String> = entries.flatten().filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(suffix).map(str::to_owned)).collect();
        stems.sort();
        stems
    }
}

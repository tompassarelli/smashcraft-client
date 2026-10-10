//! Replays (smashcraft:docs/design/client.md, "Full-match replays"): the
//! manifests and parts the map writes into each CustomMapData folder, the
//! joined replays the client keeps (opened from a file or saved to share),
//! and the simulation of every version the client has played, which watching
//! a replay needs. The pages join, check and play them
//! (smashcraft-client:ui/src/replays.ts); this side only reads and keeps files.

use std::path::{Path, PathBuf};

use crate::files::{self, TextFile};

const PREFIX: &str = "smashcraft-replay-";
const SUFFIX: &str = ".txt";

/// The serial a manifest's name `smashcraft-replay-<serial>.txt` carries; a part's name has none.
pub fn manifest_serial(name: &str) -> Option<u32> {
    files::serial_of(name, PREFIX, SUFFIX)?.parse().ok()
}

pub fn part_name(serial: u32, part: u32) -> String {
    format!("{PREFIX}{serial}-{part}{SUFFIX}")
}

/// Every manifest in `folders`, and every `.txt` file in `kept`; missing or unreadable ones are skipped.
pub fn read_replay_files(folders: &[String], kept: &Path) -> Vec<TextFile> {
    let mut found = Vec::new();
    let mut read = |folder: &Path, wanted: &dyn Fn(&str) -> bool| {
        let Ok(entries) = std::fs::read_dir(folder) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !wanted(&name) {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else { continue };
            found.push(TextFile {
                folder: folder.to_string_lossy().into_owned(),
                name,
                text: String::from_utf8_lossy(&bytes).into_owned(),
                modified: files::modified_ms(std::fs::metadata(entry.path())).unwrap_or(0),
            });
        }
    };
    for folder in folders {
        read(Path::new(folder), &|name| manifest_serial(name).is_some());
    }
    read(kept, &|name| name.ends_with(SUFFIX));
    found.sort_by(|a, b| (&a.folder, &a.name).cmp(&(&b.folder, &b.name)));
    found
}

/// The texts of a manifest's parts 1 to `parts` beside it, in order.
pub fn read_parts(folder: &str, serial: u32, parts: u32) -> Result<Vec<String>, String> {
    (1..=parts)
        .map(|part| {
            let path = Path::new(folder).join(part_name(serial, part));
            std::fs::read(&path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).map_err(|_| format!("{} is missing", path.display()))
        })
        .collect()
}

/// A file name the client may keep: letters, digits, `-`, `_` and `.`, ending in `suffix`, no path.
fn safe_name(name: &str, suffix: &str) -> Result<(), String> {
    let plain = name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.');
    if plain && name.ends_with(suffix) && !name.starts_with('.') && name.len() <= 120 {
        Ok(())
    } else {
        Err(format!("{name} isn't a name the client keeps"))
    }
}

/// The client's own folders: kept replays and the simulation of each version it has played.
pub struct Kept {
    pub replays: PathBuf,
    sims: PathBuf,
}

impl Kept {
    pub fn new(data: &Path) -> Self {
        Self { replays: data.join("replays"), sims: data.join("sims") }
    }

    /// Keeps a joined replay; its path.
    pub fn keep_replay(&self, name: &str, text: &str) -> Result<String, String> {
        safe_name(name, SUFFIX)?;
        let path = self.replays.join(name);
        files::write_atomic(&path, text)?;
        Ok(path.to_string_lossy().into_owned())
    }

    /// Keeps a version's simulation unless it is already kept.
    pub fn keep_sim(&self, version: &str, code: &str) -> Result<(), String> {
        let name = format!("{version}.js");
        safe_name(&name, ".js")?;
        if self.sims.join(&name).is_file() {
            return Ok(());
        }
        files::write_atomic(&self.sims.join(name), code)
    }

    /// A kept version's simulation.
    pub fn sim(&self, version: &str) -> Option<String> {
        let name = format!("{version}.js");
        safe_name(&name, ".js").ok()?;
        std::fs::read_to_string(self.sims.join(name)).ok()
    }

    /// Every version kept, sorted.
    pub fn sims(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.sims) else { return Vec::new() };
        let mut versions: Vec<String> =
            entries.flatten().filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".js").map(str::to_owned)).collect();
        versions.sort();
        versions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smashcraft-replays-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_manifests_beside_parts_and_every_kept_replay() {
        let folder = temp("folder");
        let kept = Kept::new(&temp("data"));
        for name in ["smashcraft-replay-3.txt", "smashcraft-replay-3-1.txt", "smashcraft-replay-3-2.txt", "smashcraft-match-3.txt"] {
            std::fs::write(folder.join(name), name).unwrap();
        }
        kept.keep_replay("shared.txt", "joined").unwrap();
        let files = read_replay_files(&[folder.to_string_lossy().into_owned(), "/missing".into()], &kept.replays);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"smashcraft-replay-3.txt") && names.contains(&"shared.txt"));
        assert_eq!(read_parts(&folder.to_string_lossy(), 3, 2).unwrap(), ["smashcraft-replay-3-1.txt", "smashcraft-replay-3-2.txt"]);
        assert!(read_parts(&folder.to_string_lossy(), 3, 3).unwrap_err().contains("smashcraft-replay-3-3.txt"));
        assert_eq!(manifest_serial("smashcraft-replay-3-1.txt"), None);
        assert_eq!(manifest_serial("smashcraft-replay-12.txt"), Some(12));
    }

    #[test]
    fn keeps_each_version_once_and_refuses_paths() {
        let kept = Kept::new(&temp("sims"));
        assert_eq!(kept.sim("abc123"), None);
        kept.keep_sim("abc123", "first").unwrap();
        kept.keep_sim("abc123", "second").unwrap();
        assert_eq!(kept.sim("abc123").as_deref(), Some("first"));
        assert_eq!(kept.sims(), ["abc123"]);
        assert!(kept.keep_sim("../escape", "x").is_err());
        assert!(kept.keep_replay("../escape.txt", "x").is_err());
        assert!(kept.keep_replay("a/b.txt", "x").is_err());
    }
}

//! Match records the map writes into each configured CustomMapData folder
//! (smashcraft:docs/design/client.md, "Match records"), and the client's own
//! history store. The pages parse, ingest and count them
//! (smashcraft:client/ui/src/records.ts); this side only reads the folders and
//! keeps the store as JSON in the client's data folder.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const PREFIX: &str = "smashcraft-match-";
const SUFFIX: &str = ".txt";

/// One record file as it is on disk.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordFile {
    pub folder: String,
    pub name: String,
    pub text: String,
    /// When the file was last written, in milliseconds since 1970; 0 when unknown.
    pub modified: u64,
}

/// Whether `name` is a match record's file name, `smashcraft-match-<serial>.txt`.
fn is_record_name(name: &str) -> bool {
    name.strip_prefix(PREFIX)
        .and_then(|rest| rest.strip_suffix(SUFFIX))
        .is_some_and(|serial| !serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit()))
}

/// Every record file in `folders`; a missing or unreadable folder or file is skipped.
pub fn read_record_files(folders: &[String]) -> Vec<RecordFile> {
    let mut files = Vec::new();
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(folder) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !is_record_name(&name) {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else { continue };
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |since| since.as_millis() as u64);
            files.push(RecordFile { folder: folder.clone(), name, text: String::from_utf8_lossy(&bytes).into_owned(), modified });
        }
    }
    files.sort_by(|a, b| (&a.folder, &a.name).cmp(&(&b.folder, &b.name)));
    files
}

/// Warcraft III's CustomMapData under the player's Documents, when it exists.
pub fn default_folders() -> Vec<String> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Vec::new();
    };
    let folder = PathBuf::from(home).join("Documents").join("Warcraft III").join("CustomMapData");
    if folder.is_dir() { vec![folder.to_string_lossy().into_owned()] } else { Vec::new() }
}

/// The history store: the pages' JSON, kept as written.
pub struct History {
    path: PathBuf,
}

impl History {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("history.json") }
    }

    /// The stored JSON, or `null` before the first save.
    pub fn load(&self) -> serde_json::Value {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(serde_json::Value::Null)
    }

    pub fn save(&self, value: &serde_json::Value) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_record_files_from_each_folder_and_skips_missing_ones() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/test/fixtures/records");
        let folders: Vec<String> = ["a", "b", "missing"].iter().map(|f| fixtures.join(f).to_string_lossy().into_owned()).collect();
        let files = read_record_files(&folders);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            ["smashcraft-match-1.txt", "smashcraft-match-2.txt", "smashcraft-match-3.txt", "smashcraft-match-4.txt", "smashcraft-match-1.txt", "smashcraft-match-7.txt"]
        );
        assert!(files.iter().all(|f| f.text.starts_with("function PreloadFiles") && f.modified > 0));
        assert!(!is_record_name("smashcraft-match-index.pld"));
        assert!(!is_record_name("smashcraft-match-.txt"));
    }

    #[test]
    fn history_round_trips_and_is_null_before_the_first_save() {
        let dir = std::env::temp_dir().join(format!("smashcraft-history-{}", std::process::id()));
        let history = History::new(&dir);
        assert_eq!(history.load(), serde_json::Value::Null);
        let value = serde_json::json!({ "version": 1, "records": [{ "id": "x" }] });
        history.save(&value).unwrap();
        assert_eq!(history.load(), value);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

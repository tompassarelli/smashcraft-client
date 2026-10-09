//! Match records the map writes into each configured CustomMapData folder
//! (smashcraft:docs/design/client.md, "Match records"), and the client's own
//! history store. The pages parse, ingest and count them
//! (smashcraft-client:ui/src/records.ts); this side only reads the folders and
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

/// `relative`'s `/`-separated parts below `base`, with this system's separator.
fn below(base: &Path, relative: &str) -> PathBuf {
    relative.split('/').fold(base.to_path_buf(), |path, part| path.join(part))
}

/// Warcraft III's CustomMapData below a Documents folder.
fn custom_map_data(documents: &Path) -> PathBuf {
    below(documents, "Warcraft III/CustomMapData")
}

/// The subfolders of `dir`, sorted; none when it can't be read.
fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

/// Every Wine user's Documents in a Wine prefix.
fn wine_documents(prefix: &Path) -> Vec<PathBuf> {
    subfolders(&below(prefix, "drive_c/users")).into_iter().map(|user| user.join("Documents")).collect()
}

/// Warcraft III's CustomMapData folders that exist: under `home`'s Documents
/// (Windows, macOS), then in Wine prefixes on Linux: every Steam/Proton
/// prefix, ~/.wine and `wine_prefix`. Only reads; nothing is written.
pub fn find_folders(home: &Path, wine_prefix: Option<&Path>) -> Vec<String> {
    let mut documents = vec![home.join("Documents")];
    for prefix in subfolders(&below(home, ".local/share/Steam/steamapps/compatdata")) {
        documents.extend(wine_documents(&prefix.join("pfx")));
    }
    documents.extend(wine_documents(&home.join(".wine")));
    if let Some(prefix) = wine_prefix {
        documents.extend(wine_documents(prefix));
    }
    let mut folders: Vec<String> = Vec::new();
    for folder in documents.iter().map(|d| custom_map_data(d)).filter(|f| f.is_dir()) {
        let folder = folder.to_string_lossy().into_owned();
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    folders
}

/// The record folders found on this computer, for a player who hasn't set them.
pub fn default_folders() -> Vec<String> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else {
        return Vec::new();
    };
    let wine_prefix = std::env::var_os("WINEPREFIX").map(PathBuf::from);
    find_folders(Path::new(&home), wine_prefix.as_deref())
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
    fn finds_custom_map_data_in_documents_steam_prefixes_wine_and_wineprefix() {
        let home = std::env::temp_dir().join(format!("smashcraft-find-{}", std::process::id()));
        let other = home.join("elsewhere");
        let made = [
            below(&home, "Documents/Warcraft III/CustomMapData"),
            below(&home, ".local/share/Steam/steamapps/compatdata/3516115571/pfx/drive_c/users/steamuser/Documents/Warcraft III/CustomMapData"),
            below(&home, ".wine/drive_c/users/tom/Documents/Warcraft III/CustomMapData"),
            below(&other, "drive_c/users/tom/Documents/Warcraft III/CustomMapData"),
        ];
        for folder in &made {
            std::fs::create_dir_all(folder).unwrap();
        }
        // A Proton prefix without Warcraft III, and one whose Documents has no CustomMapData.
        std::fs::create_dir_all(below(&home, ".local/share/Steam/steamapps/compatdata/228980/pfx/drive_c/users/steamuser/Documents")).unwrap();
        std::fs::create_dir_all(below(&home, ".local/share/Steam/steamapps/compatdata/1/pfx/drive_c/users/steamuser/Documents/Warcraft III")).unwrap();
        let expected: Vec<String> = made.iter().map(|f| f.to_string_lossy().into_owned()).collect();
        assert_eq!(find_folders(&home, Some(&other)), expected);
        assert_eq!(find_folders(&home, None), expected[..3].to_vec());
        // The same prefix named twice is listed once.
        assert_eq!(find_folders(&home, Some(&home.join(".wine"))), expected[..3].to_vec());
        std::fs::remove_dir_all(home).unwrap();
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

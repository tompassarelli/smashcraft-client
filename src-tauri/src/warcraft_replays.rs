//! Warcraft's own replays of Smashcraft games (#159). Warcraft keeps only
//! `LastReplay.w3g`, rewritten at the end of every game, in each account's
//! `Replays` folder. The client copies it into its library once per game
//! (the same content is kept once), links the copy to the match records
//! written during that game, and puts a copy back into Warcraft's
//! `Replays` folder for the player to watch from Warcraft's Replays menu.
//! A game is a whole lobby session, every rematch included; the client's own
//! per-match replays are smashcraft:client/src-tauri/src/replays.rs.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LAST_REPLAY: &str = "LastReplay.w3g";
/// Allowed between a record's write time and the game's start or end.
const SLACK_MS: u64 = 10_000;
/// A game's length when the replay's header doesn't say.
const UNKNOWN_LENGTH_MS: u64 = 60 * 60 * 1000;

/// One Warcraft game the library keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WarcraftGame {
    /// The file in the library.
    pub file: String,
    /// SHA-256 of its content, in hex.
    pub hash: String,
    /// When the game started and ended, in milliseconds since 1970.
    pub started: u64,
    pub ended: u64,
    /// The CustomMapData folder its match records are in.
    pub folder: String,
    /// The Warcraft `Replays` folder it was copied from.
    pub replays: String,
    /// The match record files written during the game.
    pub records: Vec<String>,
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
}

fn modified(path: &Path) -> Option<u64> {
    std::fs::metadata(path).and_then(|meta| meta.modified()).ok().map(millis)
}

/// Warcraft's `Replays` folders beside a CustomMapData folder: each Battle.net
/// account's, then the offline one, those that exist.
pub fn replay_folders(custom_map_data: &str) -> Vec<PathBuf> {
    let Some(warcraft) = Path::new(custom_map_data).parent() else { return Vec::new() };
    let mut accounts: Vec<PathBuf> = std::fs::read_dir(warcraft.join("BattleNet"))
        .map(|entries| entries.flatten().map(|entry| entry.path().join("Replays")).collect())
        .unwrap_or_default();
    accounts.sort();
    accounts.push(warcraft.join("Replays"));
    accounts.into_iter().filter(|dir| dir.is_dir()).collect()
}

/// The game's length a `.w3g` header records (subheader version 1), in milliseconds.
pub fn length_ms(bytes: &[u8]) -> Option<u64> {
    if !bytes.starts_with(b"Warcraft III recorded game\x1a\0") || bytes.len() < 0x40 {
        return None;
    }
    if &bytes[0x30..0x34] != b"PX3W" && &bytes[0x30..0x34] != b"W3XP" {
        return None;
    }
    Some(u32::from_le_bytes(bytes[0x3c..0x40].try_into().ok()?) as u64)
}

/// `YYYYMMDD-HHMMSS` in UTC.
fn stamp(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rest / 3600, rest / 60 % 60, rest % 60)
}

/// Match record files in `folder` written between `from` and `to`.
fn records_between(folder: &str, from: u64, to: u64) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_prefix("smashcraft-match-")
                .and_then(|rest| rest.strip_suffix(".txt"))
                .is_some_and(|serial| !serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit()))
        })
        .filter(|entry| modified(&entry.path()).is_some_and(|at| (from..=to).contains(&at)))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The client's library of Warcraft games: the copies and `games.json`.
pub struct Library {
    dir: PathBuf,
}

impl Library {
    pub fn new(data: &Path) -> Self {
        Self { dir: data.join("warcraft-replays") }
    }

    fn index(&self) -> PathBuf {
        self.dir.join("games.json")
    }

    /// Every game kept, oldest first.
    pub fn games(&self) -> Vec<WarcraftGame> {
        std::fs::read_to_string(self.index()).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    /// Keeps the `LastReplay.w3g` in `replays` when it is a game not yet kept
    /// whose time holds a match record from `folder`; the game kept, if any.
    pub fn keep_last(&self, folder: &str, replays: &Path) -> Result<Option<WarcraftGame>, String> {
        let source = replays.join(LAST_REPLAY);
        let Ok(bytes) = std::fs::read(&source) else { return Ok(None) };
        let Some(ended) = modified(&source) else { return Ok(None) };
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let mut games = self.games();
        if games.iter().any(|game| game.hash == hash) {
            return Ok(None);
        }
        let started = ended.saturating_sub(length_ms(&bytes).unwrap_or(UNKNOWN_LENGTH_MS));
        let records = records_between(folder, started.saturating_sub(SLACK_MS), ended + SLACK_MS);
        if records.is_empty() {
            return Ok(None);
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let file = format!("warcraft-{}-{}.w3g", stamp(ended), &hash[..12]);
        std::fs::write(self.dir.join(&file), &bytes).map_err(|e| e.to_string())?;
        let game = WarcraftGame {
            file,
            hash,
            started,
            ended,
            folder: folder.to_owned(),
            replays: replays.to_string_lossy().into_owned(),
            records,
        };
        games.push(game.clone());
        let tmp = self.dir.join("games.json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&games).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.index()).map_err(|e| e.to_string())?;
        Ok(Some(game))
    }

    /// Copies a kept game into the Warcraft `Replays` folder it came from as
    /// `<name>.w3g`, where Warcraft's Replays menu lists it; its path.
    pub fn put_in_warcraft(&self, file: &str, name: &str) -> Result<String, String> {
        let game = self.games().into_iter().find(|game| game.file == file).ok_or("This replay is no longer in the library.")?;
        let plain = |c: char| c.is_ascii_alphanumeric() || " -_.".contains(c);
        if name.is_empty() || name.len() > 80 || name.starts_with('.') || !name.chars().all(plain) {
            return Err(format!("{name} isn't a replay name Warcraft III can show"));
        }
        let target = Path::new(&game.replays).join(format!("{name}.w3g"));
        std::fs::copy(self.dir.join(&game.file), &target).map_err(|e| e.to_string())?;
        Ok(target.to_string_lossy().into_owned())
    }
}

/// Keeps each finished game while the client runs: every 5 s, each
/// `LastReplay.w3g` whose write time or size changed, once it has rested 3 s.
pub fn watch(library: Library, folders: impl Fn() -> Vec<String>) {
    let mut seen: HashMap<PathBuf, (u64, u64)> = HashMap::new();
    loop {
        for folder in folders() {
            for replays in replay_folders(&folder) {
                let source = replays.join(LAST_REPLAY);
                let Ok(meta) = std::fs::metadata(&source) else { continue };
                let Some(at) = meta.modified().ok().map(millis) else { continue };
                if millis(SystemTime::now()).saturating_sub(at) < 3000 || seen.get(&source) == Some(&(at, meta.len())) {
                    continue;
                }
                if library.keep_last(&folder, &replays).is_ok() {
                    seen.insert(source, (at, meta.len()));
                }
            }
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    const MINUTE: u64 = 60_000;

    /// A `.w3g` header recording a game of `length` ms, then `body`.
    fn w3g(length: u64, body: &[u8]) -> Vec<u8> {
        let mut bytes = b"Warcraft III recorded game\x1a\0".to_vec();
        bytes.resize(0x30, 0);
        bytes.extend_from_slice(b"PX3W");
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&(length as u32).to_le_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    fn write_at(path: &Path, bytes: &[u8], at: u64) {
        std::fs::write(path, bytes).unwrap();
        File::options().write(true).open(path).unwrap().set_modified(UNIX_EPOCH + Duration::from_millis(at)).unwrap();
    }

    fn warcraft(name: &str) -> (PathBuf, String, PathBuf) {
        let root = std::env::temp_dir().join(format!("smashcraft-w3g-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let docs = root.join("Documents").join("Warcraft III");
        let folder = docs.join("CustomMapData");
        let replays = docs.join("BattleNet").join("1524008679").join("Replays");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::create_dir_all(&replays).unwrap();
        (root, folder.to_string_lossy().into_owned(), replays)
    }

    #[test]
    fn keeps_each_game_once_linked_to_the_records_written_during_it() {
        let (root, folder, replays) = warcraft("keep");
        let library = Library::new(&root.join("data"));
        let end = 1_791_340_378_000; // 2026-10-07 02:32:58 UTC
        let records = Path::new(&folder);
        write_at(&records.join("smashcraft-match-1.txt"), b"earlier game", end - 30 * MINUTE);
        write_at(&records.join("smashcraft-match-2.txt"), b"first match", end - 9 * MINUTE);
        write_at(&records.join("smashcraft-match-3.txt"), b"rematch", end - 2_000);
        write_at(&records.join("smashcraft-match-index.pld"), b"index", end - 2_000);
        write_at(&replays.join(LAST_REPLAY), &w3g(10 * MINUTE, b"game one"), end);

        assert_eq!(replay_folders(&folder), [replays.clone()]);
        let game = library.keep_last(&folder, &replays).unwrap().unwrap();
        assert!(game.file.starts_with("warcraft-20261007-023258-") && game.file.ends_with(".w3g"));
        assert_eq!(game.records, ["smashcraft-match-2.txt", "smashcraft-match-3.txt"]);
        assert_eq!((game.started, game.ended), (end - 10 * MINUTE, end));
        assert_eq!(std::fs::read(root.join("data/warcraft-replays").join(&game.file)).unwrap(), w3g(10 * MINUTE, b"game one"));

        // The same game seen again, even rewritten later, is not kept twice.
        assert_eq!(library.keep_last(&folder, &replays).unwrap(), None);
        write_at(&replays.join(LAST_REPLAY), &w3g(10 * MINUTE, b"game one"), end + MINUTE);
        assert_eq!(library.keep_last(&folder, &replays).unwrap(), None);

        // A game with no Smashcraft match in it is not kept.
        write_at(&replays.join(LAST_REPLAY), &w3g(5 * MINUTE, b"melee"), end + 40 * MINUTE);
        assert_eq!(library.keep_last(&folder, &replays).unwrap(), None);

        // The next Smashcraft game is.
        write_at(&records.join("smashcraft-match-4.txt"), b"next", end + 58 * MINUTE);
        write_at(&replays.join(LAST_REPLAY), &w3g(4 * MINUTE, b"game two"), end + 59 * MINUTE);
        let next = library.keep_last(&folder, &replays).unwrap().unwrap();
        assert_eq!(next.records, ["smashcraft-match-4.txt"]);
        assert_eq!(library.games(), [game.clone(), next]);

        let put = library.put_in_warcraft(&game.file, "Smashcraft 2026-10-07 10.32").unwrap();
        assert_eq!(std::fs::read(&put).unwrap(), w3g(10 * MINUTE, b"game one"));
        assert_eq!(Path::new(&put).parent().unwrap(), replays);
        assert!(library.put_in_warcraft(&game.file, "../escape").is_err());
        assert!(library.put_in_warcraft("missing.w3g", "x").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_the_length_from_a_w3g_header_only() {
        assert_eq!(length_ms(&w3g(70_250, b"")), Some(70_250));
        assert_eq!(length_ms(b"not a replay"), None);
        assert_eq!(stamp(0), "19700101-000000");
    }
}

//! Playing a replay in the simulation of the map that recorded it. A replay
//! names its version; the map of that version is in the player's Warcraft III
//! Maps folder (its own builds, or Download for a map someone hosted). The
//! client reads the map's war3map.lua (mpq.rs), keeps it under that version,
//! and runs it in 32-bit Lua (lua32.rs), Warcraft's number model, with the
//! viewer's two modules (viewer.lua from the client kit,
//! smashcraft:docs/client-interface.md) added to the map's own modules. The
//! viewer's functions take and give text: the page's scenes as JSON.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::lua32::Lua;

/// Loads a map's bundle without starting the map, adds the viewer's modules
/// and calls the viewer driver. war3map.lua holds the bundle as a long string
/// passed to load; the bundle ends by requiring its entry module, which this
/// replaces with returning its own require and modules.
const GLUE: &str = include_str!("mapsim.lua");

/// The version stamped into a map's war3map.lua (smashcraft:ts/src/game/shell/sourceVersion.ts):
/// the first quoted 12 hex digits in its sourceVersion module.
pub fn stamped_version(war3: &str) -> Option<String> {
    let module = war3.find("[\"game.shell.sourceVersion\"] = function")?;
    let text = &war3[module..war3.len().min(module + 2000)];
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(13)).find_map(|at| {
        let candidate = &bytes[at..at + 14];
        let hex = candidate[1..13].iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b));
        (candidate[0] == b'"' && candidate[13] == b'"' && hex).then(|| text[at + 1..at + 13].to_owned())
    })
}

/// The Maps folder beside each CustomMapData folder.
pub fn maps_folders(record_folders: &[String]) -> Vec<PathBuf> {
    record_folders.iter().filter_map(|folder| Path::new(folder).parent().map(|parent| parent.join("Maps"))).filter(|maps| maps.is_dir()).collect()
}

/// Every Smashcraft map under `folder`, a few levels deep: Download keeps hosted maps in their own folders.
fn smashcraft_maps(folder: &Path, depth: u32, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if path.is_dir() && depth > 0 {
            smashcraft_maps(&path, depth - 1, found);
        } else if name.contains("smashcraft") && (name.ends_with(".w3x") || name.ends_with(".w3m")) {
            found.push(path);
        }
    }
}

fn stamp_of_file(path: &Path) -> (u64, u64) {
    let meta = std::fs::metadata(path);
    let len = meta.as_ref().map_or(0, |meta| meta.len());
    (crate::files::modified_ms(meta).unwrap_or(0), len)
}

fn war3map_lua(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    crate::mpq::read_file_from(&mut file, "war3map.lua").map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// A map file's modified time and length when read, and the version it holds.
type Scanned = ((u64, u64), Option<String>);

struct Session {
    lua: Lua,
}

/// The maps found so far and the viewers open.
pub struct MapSims {
    /// Where a version's war3map.lua is kept once found.
    kept: PathBuf,
    /// Each map read: its modified time and length, and the version it holds.
    scanned: Mutex<HashMap<PathBuf, Scanned>>,
    sessions: Mutex<HashMap<u32, Mutex<Session>>>,
    next: Mutex<u32>,
}

impl MapSims {
    pub fn new(data: &Path) -> Self {
        Self { kept: data.join("sims"), scanned: Mutex::default(), sessions: Mutex::default(), next: Mutex::new(1) }
    }

    fn kept_path(&self, version: &str) -> Option<PathBuf> {
        let plain = version.len() == 12 && version.bytes().all(|b| b.is_ascii_hexdigit());
        plain.then(|| self.kept.join(format!("{version}.lua")))
    }

    /// Reads every Smashcraft map in `folders` not read since it last changed, keeping each version's war3map.lua.
    pub fn scan(&self, folders: &[PathBuf]) -> Vec<String> {
        let mut maps = Vec::new();
        for folder in folders {
            smashcraft_maps(folder, 3, &mut maps);
        }
        let mut scanned = self.scanned.lock().unwrap();
        for path in maps {
            let stamp = stamp_of_file(&path);
            if scanned.get(&path).is_some_and(|(seen, _)| *seen == stamp) {
                continue;
            }
            let version = war3map_lua(&path).ok().and_then(|war3| {
                let version = stamped_version(&war3)?;
                let kept = self.kept_path(&version)?;
                if !kept.is_file() {
                    let _ = std::fs::create_dir_all(&self.kept);
                    let _ = std::fs::write(&kept, &war3);
                }
                Some(version)
            });
            scanned.insert(path, (stamp, version));
        }
        let mut versions: Vec<String> = scanned.values().filter_map(|(_, v)| v.clone()).collect();
        if let Ok(entries) = std::fs::read_dir(&self.kept) {
            versions.extend(entries.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".lua").map(str::to_owned)));
        }
        versions.sort();
        versions.dedup();
        versions
    }

    /// Opens `replay` in the map of `version` with the viewer's modules: its id and the driver's answer.
    pub fn open(&self, folders: &[PathBuf], version: &str, replay: &str, viewer: &str) -> Result<(u32, String), String> {
        let kept = self.kept_path(version).ok_or("not a version")?;
        if !kept.is_file() {
            self.scan(folders);
        }
        let war3 = std::fs::read_to_string(&kept).map_err(|_| format!("no map of version {version} was found"))?;
        let lua = Lua::new()?;
        lua.run(GLUE, "glue")?;
        lua.call(c"smashcraft_load_map", &[&war3])?;
        lua.call(c"smashcraft_add_viewer", &[viewer])?;
        let opened = lua.call(c"smashcraft_open", &[replay])?;
        let mut next = self.next.lock().unwrap();
        let id = *next;
        *next += 1;
        self.sessions.lock().unwrap().insert(id, Mutex::new(Session { lua }));
        Ok((id, opened))
    }

    /// Runs `frames` more frames, or with `seek` shows the state after frame `frames`.
    pub fn step(&self, id: u32, seek: bool, frames: i64) -> Result<String, String> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(&id).ok_or("that replay isn't open")?.lock().unwrap();
        let arg = frames.to_string();
        session.lua.call(if seek { c"smashcraft_seek" } else { c"smashcraft_advance" }, &[&arg])
    }

    pub fn close(&self, id: u32) {
        self.sessions.lock().unwrap().remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_stamp_in_either_form_and_none_in_an_unstamped_map() {
        let stamped = "x\n[\"game.shell.sourceVersion\"] = function(...) \nlocal ____exports = {}\nlocal SOURCE_STAMP = \"93760e0bffd6\"\n";
        assert_eq!(stamped_version(stamped).as_deref(), Some("93760e0bffd6"));
        let visitor = "[\"game.shell.sourceVersion\"] = function(...) \n____exports.DEVELOPMENT_SOURCE = \"development\"\nreturn type(\"2e03d50e5b39\") == \"string\"";
        assert_eq!(stamped_version(visitor).as_deref(), Some("2e03d50e5b39"));
        let unstamped = "[\"game.shell.sourceVersion\"] = function(...) \nlocal SOURCE_STAMP = \"%%SOURCE%%%%\"\n";
        assert_eq!(stamped_version(unstamped), None);
        assert_eq!(stamped_version("no module"), None);
    }

    #[test]
    fn plays_a_replay_in_a_map_bundle_with_the_viewer_modules_added() {
        // A stand-in map: a bundle whose simulation module counts frames, and a viewer driver reading it.
        let war3 = r#"function main() end
smashcraftTs = assert(load([=[
local ____modules = {}
local function require(file, ...) return ____modules[file](file) end
____modules = {
["game.sim.counter"] = function(...) local ____exports = {}; function ____exports.double(n) return n * 2 end; return ____exports end,
["game.shell.sourceVersion"] = function(...) local SOURCE_STAMP = "0123456789ab"; return {} end,
}
local ____entry = require("platform.playableMain", ...)
return ____entry
]=], "=map"))()
"#;
        let viewer = r#"return {
["game.replay.viewerDriver"] = function(...)
  local counter = require("game.sim.counter")
  local frame = 0
  return {
    open = function(text) return '{"first":0,"last":' .. #text .. '}' end,
    advance = function(n) frame = frame + n; return '{"frame":' .. counter.double(frame) .. '}' end,
    seek = function(n) frame = n; return '{"frame":' .. frame .. '}' end,
  }
end,
}"#;
        let dir = std::env::temp_dir().join(format!("smashcraft-mapsim-{}", std::process::id()));
        let maps = dir.join("Maps/Download");
        std::fs::create_dir_all(&maps).unwrap();
        let sims = MapSims::new(&dir.join("data"));
        std::fs::create_dir_all(dir.join("data/sims")).unwrap();
        std::fs::write(dir.join("data/sims/0123456789ab.lua"), war3).unwrap();
        assert_eq!(stamped_version(war3).as_deref(), Some("0123456789ab"));
        let (id, opened) = sims.open(&[dir.join("Maps")], "0123456789ab", "abcd", viewer).unwrap();
        assert_eq!(opened, r#"{"first":0,"last":4}"#);
        assert_eq!(sims.step(id, false, 3).unwrap(), r#"{"frame":6}"#);
        assert_eq!(sims.step(id, true, 10).unwrap(), r#"{"frame":10}"#);
        sims.close(id);
        assert!(sims.step(id, false, 1).is_err());
        assert!(sims.open(&[dir.join("Maps")], "ffffffffffff", "abcd", viewer).unwrap_err().contains("no map of version ffffffffffff"));
        assert_eq!(maps_folders(&[dir.join("CustomMapData").to_string_lossy().into_owned()]), vec![dir.join("Maps")]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// #141: the client kit's recorded replay plays in its stand-in map (the viewer's Lua bundle) in 32-bit Lua.
    #[test]
    fn plays_the_kit_replay_in_a_map_bundle_with_the_kit_viewer() {
        let kit = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/kit");
        let read = |name: &str| std::fs::read_to_string(kit.join(name)).unwrap_or_else(|_| panic!("no {name} in ui/kit: run bun scripts/kit.ts"));
        let lua = Lua::new().unwrap();
        lua.run(GLUE, "glue").unwrap();
        lua.call(c"smashcraft_load_map", &[&read("fixtures/map.lua")]).unwrap();
        lua.call(c"smashcraft_add_viewer", &[&read("viewer.lua")]).unwrap();
        let opened = lua.call(c"smashcraft_open", &[&read("fixtures/tape-replay.txt")]).unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&opened).unwrap(), serde_json::json!({ "first": 0, "last": 700, "frame": 0 }));
        let advanced: serde_json::Value = serde_json::from_str(&lua.call(c"smashcraft_advance", &["450"]).unwrap()).unwrap();
        assert_eq!((advanced["frame"].as_i64(), advanced["ended"].as_bool(), advanced["scene"]["fighters"].as_array().map(Vec::len)), (Some(450), Some(false), Some(2)));
        assert!(!advanced["scene"]["fighters"][0]["parts"].as_array().unwrap().is_empty());
        let sought: serde_json::Value = serde_json::from_str(&lua.call(c"smashcraft_seek", &["120"]).unwrap()).unwrap();
        assert_eq!((sought["frame"].as_i64(), sought["scene"]["frame"].as_i64()), (Some(120), Some(120)));
        let ended: serde_json::Value = serde_json::from_str(&lua.call(c"smashcraft_advance", &["1000"]).unwrap()).unwrap();
        assert_eq!((ended["frame"].as_i64(), ended["ended"].as_bool()), (Some(700), Some(true)));
    }
}

//! Play: Warcraft III's `-launch -loadfile` on the current map (smashcraft:docs/client-interface.md, "Maps").

use serde::Serialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

const MAPS_SUBFOLDER: &str = "00-Smashcraft";
const RETAIL_EXE: &str = "Warcraft III/_retail_/x86_64/Warcraft III.exe";

pub fn map_version(name: &str) -> Option<(u32, u32, u32)> {
    let rest = name.strip_prefix("Smashcraft ")?.strip_suffix(".w3x")?;
    let mut parts = rest.split('.').map(|part| part.parse::<u32>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

pub fn current_map(maps_folders: &[PathBuf]) -> Option<PathBuf> {
    maps_folders
        .iter()
        .filter_map(|maps| std::fs::read_dir(maps.join(MAPS_SUBFOLDER)).ok())
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| Some((map_version(&entry.file_name().to_string_lossy())?, entry.path())))
        .filter(|(_, path)| path.is_file())
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, path)| path)
}

#[derive(Debug, PartialEq)]
pub struct Launch {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub prefix: Option<PathBuf>,
}

fn wine_prefix(path: &Path) -> Option<PathBuf> {
    path.ancestors().find(|dir| dir.file_name().is_some_and(|name| name == "drive_c")).and_then(Path::parent).map(Path::to_path_buf)
}

pub fn launch_for(map: &Path, program: Option<OsString>, windows: bool) -> Option<Launch> {
    let args = |map: OsString| vec!["-launch".into(), "-loadfile".into(), map];
    if let Some(program) = program.filter(|p| !p.is_empty()) {
        return Some(Launch { program, args: args(map.into()), prefix: None });
    }
    if windows {
        let exe = ["C:\\Program Files (x86)", "C:\\Program Files"].iter().map(|dir| Path::new(dir).join(RETAIL_EXE)).find(|exe| exe.is_file())?;
        let mut all = args(map.into());
        all.insert(0, exe.into());
        let program = all.remove(0);
        return Some(Launch { program, args: all, prefix: None });
    }
    let prefix = wine_prefix(map)?;
    let exe = ["Program Files (x86)", "Program Files"].iter().map(|dir| prefix.join("drive_c").join(dir).join(RETAIL_EXE)).find(|exe| exe.is_file())?;
    let windows_path = |path: &Path| OsString::from(format!("Z:{}", path.to_string_lossy().replace('/', "\\")));
    let mut all = args(windows_path(map));
    all.insert(0, exe.into_os_string());
    let wine = std::env::var_os("WINE").unwrap_or_else(|| "wine".into());
    Some(Launch { program: wine, args: all, prefix: Some(prefix) })
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlayState {
    pub available: bool,
    pub running: bool,
    pub lines: Vec<String>,
    pub finished: Option<bool>,
}

#[derive(Default)]
pub struct Play {
    state: Mutex<PlayState>,
}

impl Play {
    pub fn state(&self, maps_folders: &[PathBuf]) -> PlayState {
        let mut state = self.state.lock().unwrap().clone();
        state.available = current_map(maps_folders).is_some();
        state
    }

    pub fn start(&'static self, maps_folders: &[PathBuf], changed: impl Fn(PlayState) + Send + Sync + 'static) -> Result<(), String> {
        let map = current_map(maps_folders).ok_or("No Smashcraft map is in Warcraft III's Maps/00-Smashcraft folder yet.")?;
        let name = map.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut lines = vec![format!("Map: {name}")];
        let started = match launch_for(&map, std::env::var_os("WARCRAFT_III"), cfg!(windows)) {
            None => {
                lines.push("Warcraft III wasn't found. Set WARCRAFT_III to the program that starts it.".to_owned());
                false
            }
            Some(launch) => {
                lines.push("Starting Warcraft III…".to_owned());
                let mut command = Command::new(&launch.program);
                command.args(&launch.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
                if let Some(prefix) = &launch.prefix {
                    command.env("WINEPREFIX", prefix);
                }
                match command.spawn() {
                    Ok(mut child) => {
                        std::thread::spawn(move || {
                            let _ = child.wait();
                        });
                        lines.push("Warcraft III is starting the match.".to_owned());
                        true
                    }
                    Err(e) => {
                        lines.push(format!("Warcraft III couldn't start: {e}"));
                        false
                    }
                }
            }
        };
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            *state = PlayState { available: true, running: false, lines, finished: Some(started) };
            state.clone()
        };
        changed(snapshot);
        if started { Ok(()) } else { Err("Play couldn't start Warcraft III.".to_owned()) }
    }
}

pub fn tools_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("SMASHCRAFT_TS")?);
    dir.join("package.json").exists().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_map_is_the_highest_release_in_00_smashcraft_per_the_client_interface() {
        assert_eq!(map_version("Smashcraft 0.0.41.w3x"), Some((0, 0, 41)));
        assert_eq!(map_version("Smashcraft 0.0.41 test 2.w3x"), None);
        assert_eq!(map_version("Other 0.0.41.w3x"), None);
        let dir = std::env::temp_dir().join(format!("smashcraft-play-{}", std::process::id()));
        let folder = dir.join("drive_c/users/me/Documents/Warcraft III/Maps");
        std::fs::create_dir_all(folder.join("00-Smashcraft/tests")).unwrap();
        std::fs::create_dir_all(dir.join("drive_c/Program Files (x86)/Warcraft III/_retail_/x86_64")).unwrap();
        std::fs::write(dir.join("drive_c/Program Files (x86)").join(RETAIL_EXE), "").unwrap();
        for name in ["Smashcraft 0.0.9.w3x", "Smashcraft 0.0.10.w3x", "tests/Smashcraft 0.0.11.w3x", "Smashcraft 0.0.12 test 1.w3x"] {
            std::fs::write(folder.join("00-Smashcraft").join(name), "").unwrap();
        }
        let map = current_map(std::slice::from_ref(&folder)).unwrap();
        assert_eq!(map, folder.join("00-Smashcraft/Smashcraft 0.0.10.w3x"));
        let launch = launch_for(&map, None, false).unwrap();
        assert_eq!(launch.prefix.as_deref(), Some(dir.as_path()));
        assert_eq!(launch.args[1..3], [OsString::from("-launch"), OsString::from("-loadfile")]);
        assert!(launch.args[3].to_string_lossy().starts_with("Z:\\") && launch.args[3].to_string_lossy().ends_with("\\00-Smashcraft\\Smashcraft 0.0.10.w3x"));
        let given = launch_for(&map, Some("wc3".into()), false).unwrap();
        assert_eq!((given.program, given.args.len(), given.prefix), ("wc3".into(), 3, None));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

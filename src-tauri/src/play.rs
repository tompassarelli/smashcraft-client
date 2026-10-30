//! Play: gets the player from the Play button to a match, by the route their system has (smashcraft:docs/client-interface.md, "Maps").

use serde::Serialize;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

const MAPS_SUBFOLDER: &str = "00-Smashcraft";
const RETAIL_EXE: &str = "Warcraft III/_retail_/x86_64/Warcraft III.exe";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapVersion {
    Release(u32, u32, u32),
    Dev(u32),
}

pub fn map_version(name: &str) -> Option<MapVersion> {
    let rest = name.strip_prefix("Smashcraft ")?.strip_suffix(".w3x")?;
    if let Some((line, build)) = rest.split_once("-dev.") {
        let mut line = line.split('.').map(|part| part.parse::<u32>().ok());
        let (_, _) = (line.next()??, line.next()??);
        return line.next().is_none().then_some(MapVersion::Dev(build.parse().ok()?));
    }
    let mut parts = rest.split('.').map(|part| part.parse::<u32>().ok());
    let version = MapVersion::Release(parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

fn map_title(map: &Path) -> Option<String> {
    map.file_stem().map(|stem| stem.to_string_lossy().into_owned())
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

pub const IN_MATCH: &str = "You're in a match: switch to the Warcraft III window";
const COULDNT_START: &str = "Play couldn't start Warcraft III.";
const SHORTCUT_NEXT: &str = "Battle.net is starting. Sign in, start Warcraft III from it and press Play again. To have Play host the match too, set SMASHCRAFT_TS to the ts folder of a Smashcraft checkout.";
/// How long the output of a finished run has to drain when something else still holds its pipes open.
const DRAIN: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq)]
pub struct Launch {
    pub program: OsString,
    pub args: Vec<OsString>,
}

/// The direct launch: the given program, else Warcraft III's own executable on Windows.
pub fn launch_for(map: &Path, program: Option<OsString>, windows: bool) -> Option<Launch> {
    let args = |map: OsString| vec!["-launch".into(), "-loadfile".into(), map];
    if let Some(program) = program.filter(|p| !p.is_empty()) {
        return Some(Launch { program, args: args(map.into()) });
    }
    if !windows {
        return None;
    }
    let exe = ["C:\\Program Files (x86)", "C:\\Program Files"].iter().map(|dir| Path::new(dir).join(RETAIL_EXE)).find(|exe| exe.is_file())?;
    Some(Launch { program: exe.into_os_string(), args: args(map.into()) })
}

/// `bun wisp play` in Smashcraft's tools, which handles Battle.net, the doctor checks and hosting.
#[derive(Debug, PartialEq)]
pub struct Run {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub dir: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum Route {
    /// Warcraft III is already in a match: start nothing.
    InMatch,
    Direct(Launch),
    Wisp(Run),
    /// Linux without Smashcraft's tools: Steam's Battle.net shortcut.
    Shortcut,
    NotFound,
}

pub struct Setup {
    pub program: Option<OsString>,
    pub tools: Option<PathBuf>,
    pub bun: OsString,
    pub windows: bool,
    pub in_match: bool,
}

impl Setup {
    pub fn from_env(in_match: bool) -> Self {
        Self {
            program: std::env::var_os("WARCRAFT_III").filter(|p| !p.is_empty()),
            tools: tools_dir(),
            bun: std::env::var_os("BUN").unwrap_or_else(|| "bun".into()),
            windows: cfg!(windows),
            in_match,
        }
    }
}

pub fn route_for(map: &Path, setup: &Setup) -> Route {
    if setup.in_match {
        return Route::InMatch;
    }
    if setup.program.is_some() || setup.windows {
        return launch_for(map, setup.program.clone(), setup.windows).map_or(Route::NotFound, Route::Direct);
    }
    match &setup.tools {
        Some(dir) => Route::Wisp(Run { program: setup.bun.clone(), args: vec!["wisp".into(), "play".into()], dir: dir.clone() }),
        None => Route::Shortcut,
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlayState {
    pub available: bool,
    pub running: bool,
    pub lines: Vec<String>,
    /// The last press ended well (true), failed (false), or only said something (none).
    pub finished: Option<bool>,
}

type Changed = Arc<dyn Fn(PlayState) + Send + Sync>;

#[derive(Default)]
pub struct Play {
    state: Mutex<PlayState>,
    warcraft: Mutex<Option<u32>>,
}

impl Play {
    pub fn state(&self, maps_folders: &[PathBuf]) -> PlayState {
        let mut state = self.state.lock().unwrap().clone();
        state.available = current_map(maps_folders).is_some();
        state
    }

    /// The Warcraft III that Play started, while it runs.
    pub fn warcraft_pid(&self) -> Option<u32> {
        *self.warcraft.lock().unwrap()
    }

    pub fn start(&'static self, maps_folders: &[PathBuf], in_match: bool, changed: impl Fn(PlayState) + Send + Sync + 'static) -> Result<(), String> {
        let map = current_map(maps_folders).ok_or("No Smashcraft map is in Warcraft III's Maps/00-Smashcraft folder yet.")?;
        let route = route_for(&map, &Setup::from_env(in_match));
        self.go(&map, route, crate::launch::window::raise_warcraft, Arc::new(changed))
    }

    fn go(&'static self, map: &Path, route: Route, raise: impl FnOnce() -> Result<(), String>, changed: Changed) -> Result<(), String> {
        if self.state.lock().unwrap().running {
            return Err("Play is already getting you into a match.".to_owned());
        }
        match route {
            Route::Wisp(run) => self.follow(run, changed),
            Route::InMatch => {
                let mut lines = vec![IN_MATCH.to_owned()];
                if let Err(e) = raise() {
                    lines.push(format!("Couldn't bring it forward: {e}"));
                }
                self.finish(lines, None, &changed);
                Ok(())
            }
            Route::NotFound => {
                self.finish(vec!["Warcraft III wasn't found. Set WARCRAFT_III to the program that starts it.".to_owned()], Some(false), &changed);
                Err(COULDNT_START.to_owned())
            }
            Route::Shortcut => {
                let mut lines = vec![format!("Starting the Steam shortcut \"{}\"…", crate::launch::steam::SHORTCUT)];
                let started = crate::launch::steam::start_shortcut();
                lines.push(started.clone().map_or_else(|e| format!("Steam couldn't start it: {e}"), |()| SHORTCUT_NEXT.to_owned()));
                self.finish(lines, Some(started.is_ok()), &changed);
                started.map_err(|_| COULDNT_START.to_owned())
            }
            Route::Direct(launch) => {
                let mut lines = vec![format!("Map: {}", map_title(map).unwrap_or_default()), "Starting Warcraft III…".to_owned()];
                let spawned = Command::new(&launch.program).args(&launch.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
                match spawned {
                    Ok(mut child) => {
                        *self.warcraft.lock().unwrap() = Some(child.id());
                        std::thread::spawn(move || {
                            let _ = child.wait();
                            *self.warcraft.lock().unwrap() = None;
                        });
                        lines.push("Warcraft III is starting the match.".to_owned());
                        self.finish(lines, Some(true), &changed);
                        Ok(())
                    }
                    Err(e) => {
                        lines.push(format!("Warcraft III couldn't start: {}: {e}", launch.program.to_string_lossy()));
                        self.finish(lines, Some(false), &changed);
                        Err(COULDNT_START.to_owned())
                    }
                }
            }
        }
    }

    fn finish(&self, lines: Vec<String>, finished: Option<bool>, changed: &Changed) {
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            *state = PlayState { available: true, running: false, lines, finished };
            state.clone()
        };
        changed(snapshot);
    }

    fn say(&self, line: String) -> PlayState {
        let mut state = self.state.lock().unwrap();
        state.lines.push(line);
        state.clone()
    }

    /// Runs `bun wisp play` and shows each line it prints, on either stream, as a step.
    fn follow(&'static self, run: Run, changed: Changed) -> Result<(), String> {
        let spawned = Command::new(&run.program).args(&run.args).current_dir(&run.dir).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                let what = format!("{} {}", run.program.to_string_lossy(), run.args.iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" "));
                self.finish(vec![format!("Couldn't run {what} in {}: {e}", run.dir.display())], Some(false), &changed);
                return Err(COULDNT_START.to_owned());
            }
        };
        *self.state.lock().unwrap() = PlayState { available: true, running: true, lines: Vec::new(), finished: None };
        changed(self.state.lock().unwrap().clone());
        let (done, drained) = mpsc::channel();
        let follow = |stream: Box<dyn Read + Send>| {
            let (changed, done) = (changed.clone(), done.clone());
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    changed(self.say(line));
                }
                let _ = done.send(());
            });
        };
        follow(Box::new(child.stdout.take().expect("piped stdout")));
        follow(Box::new(child.stderr.take().expect("piped stderr")));
        drop(done);
        std::thread::spawn(move || {
            let status = child.wait();
            for _ in 0..2 {
                let _ = drained.recv_timeout(DRAIN);
            }
            let ok = status.as_ref().is_ok_and(|status| status.success());
            let mut lines = self.state.lock().unwrap().lines.clone();
            if !ok {
                lines.push(status.map_or_else(|e| format!("Play stopped: {e}."), |status| format!("Play stopped before the match started ({status}).")));
            }
            self.finish(lines, Some(ok), &changed);
        });
        Ok(())
    }
}

pub fn tools_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("SMASHCRAFT_TS")?);
    dir.join("package.json").exists().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smashcraft-play-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup(tools: Option<PathBuf>, in_match: bool) -> Setup {
        Setup { program: None, tools, bun: "bun".into(), windows: false, in_match }
    }

    #[test]
    fn the_current_map_is_the_highest_release_in_00_smashcraft_per_the_client_interface() {
        assert_eq!(map_version("Smashcraft 0.0.41.w3x"), Some(MapVersion::Release(0, 0, 41)));
        assert_eq!(map_version("Smashcraft 0.0.41 test 2.w3x"), None);
        assert_eq!(map_version("Other 0.0.41.w3x"), None);
        let dir = tmp("maps");
        let folder = dir.join("Maps");
        std::fs::create_dir_all(folder.join("00-Smashcraft/tests")).unwrap();
        for name in ["Smashcraft 0.0.9.w3x", "Smashcraft 0.0.10.w3x", "tests/Smashcraft 0.0.11.w3x", "Smashcraft 0.0.12 test 1.w3x"] {
            std::fs::write(folder.join("00-Smashcraft").join(name), "").unwrap();
        }
        let map = current_map(std::slice::from_ref(&folder)).unwrap();
        assert_eq!(map, folder.join("00-Smashcraft/Smashcraft 0.0.10.w3x"));
        for name in ["Smashcraft 0.3-dev.438.w3x", "Smashcraft 0.3-dev.456.w3x", "Smashcraft 0.3-dev.99.w3x", "Smashcraft 0.3-dev.500 test 2.w3x"] {
            std::fs::write(folder.join("00-Smashcraft").join(name), "").unwrap();
        }
        assert_eq!(current_map(std::slice::from_ref(&folder)).unwrap(), folder.join("00-Smashcraft/Smashcraft 0.3-dev.456.w3x"));
        let given = launch_for(&map, Some("wc3".into()), false).unwrap();
        assert_eq!((given.program, given.args.len()), ("wc3".into(), 3));
        assert_eq!(launch_for(&map, None, false), None, "no wine route on Linux");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn linux_plays_through_bun_wisp_play_in_the_tools_folder_or_steams_shortcut_and_never_wine() {
        let tools = tmp("tools");
        let map = Path::new("/maps/00-Smashcraft/Smashcraft 0.3-dev.470.w3x");
        let Route::Wisp(run) = route_for(map, &setup(Some(tools.clone()), false)) else { panic!("expected the wisp route") };
        assert_eq!((run.program, run.args, run.dir), ("bun".into(), vec![OsString::from("wisp"), "play".into()], tools.clone()));
        assert_eq!(route_for(map, &setup(None, false)), Route::Shortcut);
        let direct = Setup { program: Some("wc3".into()), ..setup(Some(tools.clone()), false) };
        assert!(matches!(route_for(map, &direct), Route::Direct(Launch { program, .. }) if program == "wc3"));
        assert_eq!(route_for(map, &Setup { windows: true, ..setup(None, false) }), Route::NotFound, "no Warcraft III install in this sandbox");
        std::fs::remove_dir_all(tools).unwrap();
    }

    fn collect() -> (Changed, mpsc::Receiver<PlayState>) {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        (Arc::new(move |state| drop(tx.lock().unwrap().send(state))), rx)
    }

    fn wait_finished(rx: &mpsc::Receiver<PlayState>) -> PlayState {
        loop {
            let state = rx.recv_timeout(Duration::from_secs(10)).expect("the run finishes");
            if !state.running && state.finished.is_some() {
                return state;
            }
        }
    }

    #[test]
    fn warcraft_already_in_a_match_starts_nothing_says_so_and_raises_its_window() {
        let play: &'static Play = Box::leak(Box::default());
        let (changed, rx) = collect();
        let raised = std::cell::Cell::new(0);
        let tools = tmp("in-match");
        let map = Path::new("/maps/Smashcraft 0.3-dev.470.w3x");
        let route = route_for(map, &setup(Some(tools.clone()), true));
        assert_eq!(route, Route::InMatch);
        play.go(
            map,
            route,
            || {
                raised.set(raised.get() + 1);
                Ok(())
            },
            changed.clone(),
        )
        .unwrap();
        let state = rx.recv().unwrap();
        assert_eq!((state.lines, state.running, state.finished, raised.get()), (vec![IN_MATCH.to_owned()], false, None, 1));
        assert_eq!(play.warcraft_pid(), None);
        play.go(map, Route::InMatch, || Err("there is no Warcraft III window".to_owned()), changed).unwrap();
        assert_eq!(rx.recv().unwrap().lines, [IN_MATCH, "Couldn't bring it forward: there is no Warcraft III window"]);
        std::fs::remove_dir_all(tools).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_launch_keeps_every_step_line_and_the_error_and_a_missing_program_shows_the_os_error() {
        let play: &'static Play = Box::leak(Box::default());
        let (changed, rx) = collect();
        let map = Path::new("/maps/Smashcraft 0.3-dev.470.w3x");
        let script = "echo '1/7 Wine prefix: free'; echo '2/7 Battle.net: starting the Steam shortcut' ; echo 'Battle.net needs its owner to sign in' >&2; exit 3";
        let run = Run { program: "sh".into(), args: vec!["-c".into(), script.into()], dir: std::env::temp_dir() };
        play.go(map, Route::Wisp(run), || Ok(()), changed.clone()).unwrap();
        let done = wait_finished(&rx);
        assert_eq!(done.finished, Some(false));
        let steps: Vec<&str> = done.lines.iter().map(String::as_str).filter(|l| !l.starts_with("Battle.net needs")).collect();
        assert_eq!(&steps[..2], ["1/7 Wine prefix: free", "2/7 Battle.net: starting the Steam shortcut"]);
        assert!(done.lines.contains(&"Battle.net needs its owner to sign in".to_owned()));
        assert!(done.lines.last().unwrap().contains("exit status: 3"), "{:?}", done.lines);

        let missing = Run { program: "/no/such/bun".into(), args: vec!["wisp".into(), "play".into()], dir: std::env::temp_dir() };
        assert_eq!(play.go(map, Route::Wisp(missing), || Ok(()), changed), Err(COULDNT_START.to_owned()));
        let failed = wait_finished(&rx);
        assert!(failed.lines[0].contains("/no/such/bun wisp play") && failed.lines[0].contains("No such file or directory"), "{:?}", failed.lines);
    }
}

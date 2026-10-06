//! The Play button: runs `bun wisp play` from a Smashcraft checkout and streams
//! its one-line steps. For now this needs the developer checkout; a packaged
//! game launch replaces it later.

use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// `SMASHCRAFT_TS`, else ~/code/smashcraft/main/ts when it exists.
pub fn play_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("SMASHCRAFT_TS") {
        return Some(dir.into());
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let dir = PathBuf::from(home).join("code/smashcraft/main/ts");
    dir.join("package.json").exists().then_some(dir)
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PlayState {
    /// Play can run on this computer.
    pub available: bool,
    pub running: bool,
    /// The steps printed so far in this run.
    pub lines: Vec<String>,
    /// The last run finished: true when it reached a match.
    pub finished: Option<bool>,
}

#[derive(Default)]
pub struct Play {
    state: Mutex<PlayState>,
}

impl Play {
    pub fn state(&self) -> PlayState {
        let mut state = self.state.lock().unwrap().clone();
        state.available = play_dir().is_some();
        state
    }

    /// Starts a run unless one is going. `changed` is called after each line and at the end.
    pub fn start(&'static self, changed: impl Fn(PlayState) + Send + Sync + 'static) -> Result<(), String> {
        let dir = play_dir().ok_or("Play isn't set up on this computer yet.")?;
        {
            let mut state = self.state.lock().unwrap();
            if state.running {
                return Ok(());
            }
            *state = PlayState { available: true, running: true, ..PlayState::default() };
        }
        let bun = std::env::var_os("BUN").unwrap_or_else(|| "bun".into());
        let child = Command::new(bun)
            .args(["wisp", "play"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(e) => {
                let mut state = self.state.lock().unwrap();
                state.running = false;
                state.finished = Some(false);
                state.lines.push("Play couldn't start.".to_owned());
                return Err(e.to_string());
            }
        };
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let changed = std::sync::Arc::new(changed);
        let push = {
            let changed = changed.clone();
            move |line: String| {
                let snapshot = {
                    let mut state = self.state.lock().unwrap();
                    state.lines.push(line);
                    state.clone()
                };
                changed(snapshot);
            }
        };
        let err_push = push.clone();
        let errors = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                err_push(line);
            }
        });
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                push(line);
            }
            let _ = errors.join();
            let ok = child.wait().map(|status| status.success()).unwrap_or(false);
            let snapshot = {
                let mut state = self.state.lock().unwrap();
                state.running = false;
                state.finished = Some(ok);
                state.clone()
            };
            changed(snapshot);
        });
        Ok(())
    }
}

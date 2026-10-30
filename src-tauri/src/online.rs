//! Direct play (#142): runs `bun wisp online setup|host|join CODE` in Smashcraft's
//! tools, `SMASHCRAFT_TS` (smashcraft:docs/client-interface.md, "Commands"), and
//! Wisp's standalone window through its relay, `bun wisp play --standalone --host
//! --relay nearest` and `--join CODE` (smashcraft:docs/commands/online.md). The
//! command's output lines are written for players and shown on the Online
//! page; its error output goes to the client's log, except the lines a run
//! asks to show. "Start now" sends a running host a `start` line.

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Setup,
    Host,
    Join,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OnlineState {
    /// Online play can run on this computer.
    pub available: bool,
    pub running: bool,
    pub mode: Option<Mode>,
    /// The lines printed so far in this run.
    pub lines: Vec<String>,
    /// The last run finished: true when it reached a match (or finished setting up).
    pub finished: Option<bool>,
}

/// The command's arguments after `bun`. `repair` writes the menu page again first.
pub fn online_args(mode: Mode, code: Option<&str>, repair: bool) -> Vec<String> {
    let mut args = vec!["wisp".to_owned(), "online".to_owned()];
    match mode {
        Mode::Setup => args.push("setup".to_owned()),
        Mode::Host => args.push("host".to_owned()),
        Mode::Join => {
            args.push("join".to_owned());
            args.push(code.unwrap_or_default().trim().to_owned());
        }
    }
    if repair && mode != Mode::Setup {
        args.push("--repair".to_owned());
    }
    args
}

/// The standalone window's arguments after `bun`: a host opens a room on the nearest relay region, a joiner takes the region's code.
pub fn standalone_args(mode: Mode, code: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = ["wisp", "play", "--standalone"].map(str::to_owned).into();
    match mode {
        Mode::Host => args.extend(["--host", "--relay", "nearest"].map(str::to_owned)),
        Mode::Join => args.extend(["--join".to_owned(), code.unwrap_or_default().trim().to_owned()]),
        Mode::Setup => {}
    }
    args
}

/// The standalone run's error-output lines worth showing: the round trip to the relay, printed before the match.
pub fn shows_relay_round_trip(line: &str) -> bool {
    line.starts_with("net: hosting through ") || line.starts_with("net: joining through ")
}

/// How long a cancelled run has to end on SIGTERM; Wisp's window run ignores it while it waits on the relay.
#[cfg(unix)]
const CANCEL_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

#[cfg(unix)]
fn signal_group(group: u32, signal: i32) -> bool {
    i32::try_from(group).is_ok_and(|group| unsafe { libc::kill(-group, signal) } == 0)
}

#[derive(Default)]
pub struct Online {
    state: Mutex<OnlineState>,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
}

impl Online {
    pub fn state(&self) -> OnlineState {
        let mut state = self.state.lock().unwrap().clone();
        state.available = crate::play::tools_dir().is_some();
        state
    }

    /// Starts `program args` in `dir` unless a run is going. `changed` is called after each line and at the end;
    /// error-output lines for which `shown` is true are listed with the output's.
    pub fn start(
        self: &Arc<Self>,
        mode: Mode,
        program: OsString,
        args: Vec<String>,
        dir: &Path,
        log: Option<PathBuf>,
        shown: fn(&str) -> bool,
        changed: impl Fn(OnlineState) + Send + Sync + 'static,
    ) -> Result<(), String> {
        let changed = Arc::new(changed);
        {
            let mut state = self.state.lock().unwrap();
            if state.running {
                return Err("A game is already being set up.".to_owned());
            }
            *state = OnlineState { available: true, running: true, mode: Some(mode), ..OnlineState::default() };
        }
        let mut command = Command::new(program);
        command.args(&args).current_dir(dir).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        let spawned = command.spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                let mut state = self.state.lock().unwrap();
                state.running = false;
                state.finished = Some(false);
                state.lines.push("Online play couldn't start.".to_owned());
                return Err(e.to_string());
            }
        };
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        *self.stdin.lock().unwrap() = child.stdin.take();
        *self.child.lock().unwrap() = Some(child);
        let errors = self.follow_errors(stderr, log, shown, changed.clone());
        let online = self.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                changed(online.say(line));
            }
            let _ = errors.join();
            let ok = online.child.lock().unwrap().take().and_then(|mut child| child.wait().ok()).is_some_and(|status| status.success());
            *online.stdin.lock().unwrap() = None;
            let snapshot = {
                let mut state = online.state.lock().unwrap();
                state.running = false;
                state.finished = Some(ok);
                state.clone()
            };
            changed(snapshot);
        });
        Ok(())
    }

    fn say(&self, line: String) -> OnlineState {
        let mut state = self.state.lock().unwrap();
        state.lines.push(line);
        state.clone()
    }

    /// Writes the run's error output to `log`, and lists the lines `shown` accepts with the output's.
    fn follow_errors(
        self: &Arc<Self>,
        stderr: ChildStderr,
        log: Option<PathBuf>,
        shown: fn(&str) -> bool,
        changed: Arc<impl Fn(OnlineState) + Send + Sync + 'static>,
    ) -> JoinHandle<()> {
        let online = self.clone();
        std::thread::spawn(move || {
            let mut file = log.and_then(|path| std::fs::OpenOptions::new().create(true).append(true).open(path).ok());
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(file) = file.as_mut() {
                    let _ = writeln!(file, "{line}");
                }
                if shown(&line) {
                    changed(online.say(line));
                }
            }
        })
    }

    /// Asks a waiting host to start once their opponent has joined.
    pub fn start_now(&self) -> bool {
        match self.stdin.lock().unwrap().as_mut() {
            Some(stdin) => stdin.write_all(b"start\n").and_then(|_| stdin.flush()).is_ok(),
            None => false,
        }
    }

    /// Stops the current run and what it started, such as the standalone window; a hosted game stays open in Warcraft III until the player leaves it.
    pub fn cancel(self: &Arc<Self>) {
        let Some(pid) = self.child.lock().unwrap().as_ref().map(Child::id) else { return };
        #[cfg(unix)]
        if signal_group(pid, libc::SIGTERM) {
            let run = self.clone();
            std::thread::spawn(move || {
                std::thread::sleep(CANCEL_GRACE);
                if run.child.lock().unwrap().as_ref().is_some_and(|child| child.id() == pid) {
                    signal_group(pid, libc::SIGKILL);
                }
            });
            return;
        }
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn arguments_follow_the_wisp_online_cli_setup_host_repair_and_join_code() {
        assert_eq!(online_args(Mode::Setup, None, true), ["wisp", "online", "setup"]);
        assert_eq!(online_args(Mode::Host, None, true), ["wisp", "online", "host", "--repair"]);
        assert_eq!(online_args(Mode::Join, Some(" abcd-efgh "), false), ["wisp", "online", "join", "abcd-efgh"]);
    }

    #[test]
    fn standalone_arguments_host_through_the_nearest_relay_region_and_join_by_its_code() {
        assert_eq!(standalone_args(Mode::Host, None), ["wisp", "play", "--standalone", "--host", "--relay", "nearest"]);
        assert_eq!(standalone_args(Mode::Join, Some(" sgp-7K2M-QX9D-R3TB-4 ")), ["wisp", "play", "--standalone", "--join", "sgp-7K2M-QX9D-R3TB-4"]);
    }

    #[test]
    fn only_the_relay_round_trip_lines_of_wisps_error_output_are_shown() {
        assert!(shows_relay_round_trip("net: hosting through sgp, round trip to it 183 ms"));
        assert!(shows_relay_round_trip("net: joining through sgp, round trip to it 40 ms"));
        assert!(!shows_relay_round_trip("net: relay regions"));
        assert!(!shows_relay_round_trip("net: {\"turns\":3}"));
    }

    fn wait_finished(rx: &mpsc::Receiver<OnlineState>) -> OnlineState {
        loop {
            let state = rx.recv_timeout(Duration::from_secs(10)).expect("the run finishes");
            if !state.running {
                return state;
            }
        }
    }

    /// A stand-in host: prints its code, then waits for `start` on stdin as `wisp online host` does.
    #[cfg(unix)]
    #[test]
    fn start_now_reaches_a_waiting_host_and_only_output_lines_are_shown() {
        let online = Arc::new(Online::default());
        let log = std::env::temp_dir().join(format!("smashcraft-online-{}.log", std::process::id()));
        let (tx, rx) = mpsc::channel();
        let script = "echo 'Join code: ABCD-EFGH'; echo detail >&2; read line; echo \"got $line\"; echo 'In the match'";
        online
            .start(Mode::Host, "sh".into(), vec!["-c".into(), script.into()], &std::env::temp_dir(), Some(log.clone()), |_| false, move |state| {
                let _ = tx.send(state);
            })
            .unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap().lines, ["Join code: ABCD-EFGH"]);
        assert!(online.start(Mode::Join, "sh".into(), vec![], &std::env::temp_dir(), None, |_| false, |_| {}).is_err(), "one run at a time");
        assert!(online.start_now());
        let done = wait_finished(&rx);
        assert_eq!(done.lines, ["Join code: ABCD-EFGH", "got start", "In the match"]);
        assert_eq!(done.finished, Some(true));
        assert_eq!(done.mode, Some(Mode::Host));
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "detail\n");
        let _ = std::fs::remove_file(log);
        assert!(!online.start_now(), "nothing to start once the run ended");
    }

    #[cfg(unix)]
    #[test]
    fn error_output_lines_the_run_asks_for_are_listed_among_the_output() {
        let online = Arc::new(Online::default());
        let (tx, rx) = mpsc::channel();
        let script = "echo 'net: hosting through sgp, round trip to it 183 ms' >&2; echo 'Smashcraft: join code sgp-7K2M-QX9D-R3TB-4'; echo detail >&2";
        online
            .start(Mode::Host, "sh".into(), vec!["-c".into(), script.into()], &std::env::temp_dir(), None, shows_relay_round_trip, move |state| {
                let _ = tx.send(state);
            })
            .unwrap();
        let done = wait_finished(&rx);
        assert_eq!(done.lines.len(), 2);
        assert!(done.lines.contains(&"net: hosting through sgp, round trip to it 183 ms".to_owned()));
        assert!(done.lines.contains(&"Smashcraft: join code sgp-7K2M-QX9D-R3TB-4".to_owned()));
    }

    #[cfg(unix)]
    #[test]
    fn a_cancelled_run_finishes_as_failed() {
        let online = Arc::new(Online::default());
        let (tx, rx) = mpsc::channel();
        online
            .start(Mode::Join, "sh".into(), vec!["-c".into(), "echo waiting; exec sleep 30".into()], &std::env::temp_dir(), None, |_| false, move |state| {
                let _ = tx.send(state);
            })
            .unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap().lines, ["waiting"]);
        online.cancel();
        assert_eq!(wait_finished(&rx).finished, Some(false));
    }
}

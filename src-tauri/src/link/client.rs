use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use padbridge_model::{ClientMessage, InputView, ServiceMessage, Snapshot};

use crate::env::Env;

use super::supervisor::{Action, ControllerState, Supervisor};

/// Where status goes: the window and tray in the app, a recorder in tests.
pub trait Sink: Send + Sync + 'static {
    fn status(&self, state: &ControllerState);
    fn input(&self, input: &InputView);
}

/// Launches the service; the app's spawns the bundled binary.
pub trait Starter: Send + Sync + 'static {
    fn start(&self) -> Result<(), String>;
}

/// The bundled service: `WC3_CONTROLLER_SERVICE`, else `padbridge` beside
/// this executable, else `padbridge` on the PATH, with Smashcraft's
/// plug-in ([`plugin_program`]). Started detached so it keeps serving after the
/// window closes.
pub struct BundledService {
    pub log: Option<PathBuf>,
    pub env: Env,
}

pub fn service_program(env: &Env) -> PathBuf {
    if let Some(path) = &env.controller_service {
        return path.clone();
    }
    let name = if cfg!(windows) { "padbridge.exe" } else { "padbridge" };
    env.exe_dir().map(|dir| dir.join(name)).filter(|path| path.exists()).unwrap_or_else(|| name.into())
}

/// Smashcraft's plug-in (smashcraft:controller): `wc3-journal` beside this
/// executable, else the one `bun wisp controller` installs; none leaves the
/// service on Any map.
pub fn plugin_program(env: &Env) -> Option<PathBuf> {
    let beside = env.exe_dir().map(|dir| dir.join("wc3-journal"));
    let installed = env.home.as_ref().map(|home| home.join(".local/share/smashcraft-build-inputs/controller/wc3-journal"));
    [beside, installed].into_iter().flatten().find(|path| path.exists())
}

impl Starter for BundledService {
    fn start(&self) -> Result<(), String> {
        let program = service_program(&self.env);
        let stderr = match &self.log {
            Some(path) => std::fs::OpenOptions::new().create(true).append(true).open(path).map(Stdio::from).unwrap_or_else(|_| Stdio::null()),
            None => Stdio::null(),
        };
        let mut child = Command::new(&program)
            .arg("--service")
            .args(plugin_program(&self.env).map(|plugin| vec!["--plugin".into(), plugin.into_os_string()]).unwrap_or_default())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .map_err(|e| format!("starting {}: {e}", program.display()))?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}

/// ControllerLink between the link thread and the app's commands.
pub struct ControllerLink {
    pub supervisor: Mutex<Supervisor>,
    pub snapshot: Mutex<Snapshot>,
    writer: Mutex<Option<TcpStream>>,
    /// Sent on every new connection, so a restarted service gets the player's choices.
    reconnect_messages: Mutex<Vec<ClientMessage>>,
    stop: AtomicBool,
}

impl ControllerLink {
    pub fn new(wanted: bool) -> Arc<Self> {
        Arc::new(Self {
            supervisor: Mutex::new(Supervisor::new(wanted)),
            snapshot: Mutex::new(Snapshot::default()),
            writer: Mutex::new(None),
            reconnect_messages: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        })
    }

    /// Warcraft III is running and the map reports a match in progress.
    pub fn in_match(&self) -> bool {
        let snapshot = self.snapshot.lock().unwrap();
        snapshot.game.is_some() && snapshot.session.as_ref().is_some_and(|session| session.phase == padbridge_model::Phase::Match)
    }

    pub fn state(&self) -> ControllerState {
        ControllerState::new(&self.supervisor.lock().unwrap(), &self.snapshot.lock().unwrap())
    }

    /// Sends a message to the service; false when none is connected.
    pub fn send(&self, message: &ClientMessage) -> bool {
        let mut writer = self.writer.lock().unwrap();
        match writer.as_mut() {
            Some(stream) => stream.write_all(message.line().as_bytes()).is_ok(),
            None => false,
        }
    }

    pub fn set_reconnect_messages(&self, messages: Vec<ClientMessage>) {
        *self.reconnect_messages.lock().unwrap() = messages;
    }

    #[cfg(test)]
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(stream) = self.writer.lock().unwrap().take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

/// Runs the link until a test stops it: connect, follow status, reconnect,
/// and start the service when it is wanted and nothing answers.
pub fn run(shared: Arc<ControllerLink>, port: u16, sink: Arc<dyn Sink>, starter: Arc<dyn Starter>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let publish = |shared: &ControllerLink| sink.status(&shared.state());
    let mut last_link = None;
    while !shared.stop.load(Ordering::SeqCst) {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                *shared.writer.lock().unwrap() = stream.try_clone().ok();
                for message in shared.reconnect_messages.lock().unwrap().clone() {
                    shared.send(&message);
                }
                shared.supervisor.lock().unwrap().connected();
                publish(&shared);
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    match ServiceMessage::parse(&line) {
                        Ok(ServiceMessage::Status(snapshot)) => {
                            *shared.snapshot.lock().unwrap() = snapshot;
                            publish(&shared);
                        }
                        Ok(ServiceMessage::Input(input)) => sink.input(&input),
                        Err(_) => {}
                    }
                }
                shared.writer.lock().unwrap().take();
                *shared.snapshot.lock().unwrap() = Snapshot::default();
                last_link = None;
            }
            Err(_) => {
                let action = shared.supervisor.lock().unwrap().unreachable(Instant::now());
                if action == Action::Start && starter.start().is_err() {
                    shared.supervisor.lock().unwrap().start_failed();
                }
                let state = shared.state();
                if last_link != Some((state.link, state.view.problem.is_some(), state.wanted)) {
                    last_link = Some((state.link, state.view.problem.is_some(), state.wanted));
                    sink.status(&state);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use padbridge_model::{self as model, Game, Light, Link, Pad, Phase, Profile, ProfileChoice, Session};
    use std::net::TcpListener;
    use std::sync::mpsc;

    struct Recorder(Mutex<mpsc::Sender<ControllerState>>, Mutex<mpsc::Sender<InputView>>);
    impl Sink for Recorder {
        fn status(&self, state: &ControllerState) {
            let _ = self.0.lock().unwrap().send(state.clone());
        }
        fn input(&self, input: &InputView) {
            let _ = self.1.lock().unwrap().send(*input);
        }
    }

    /// A stand-in service that starts listening only when the client asks for it.
    struct FakeService {
        port: u16,
        started: mpsc::Sender<TcpListener>,
    }
    impl Starter for FakeService {
        fn start(&self) -> Result<(), String> {
            let listener = TcpListener::bind(("127.0.0.1", self.port)).map_err(|e| e.to_string())?;
            self.started.send(listener).map_err(|e| e.to_string())
        }
    }

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
    }

    fn until(rx: &mpsc::Receiver<ControllerState>, pred: impl Fn(&ControllerState) -> bool) -> ControllerState {
        loop {
            let state = rx.recv_timeout(Duration::from_secs(10)).expect("status within 10 s");
            if pred(&state) {
                return state;
            }
        }
    }

    #[test]
    fn turning_on_starts_the_service_once_then_shows_its_status_and_input_and_sends_the_profile() {
        let port = free_port();
        let (status_tx, status_rx) = mpsc::channel();
        let (input_tx, input_rx) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let shared = ControllerLink::new(false);
        let sink = Arc::new(Recorder(Mutex::new(status_tx), Mutex::new(input_tx)));
        let starter = Arc::new(FakeService { port, started: started_tx });
        let link = std::thread::spawn({
            let shared = shared.clone();
            move || run(shared, port, sink, starter)
        });

        let off = until(&status_rx, |s| s.link == Link::Off);
        assert_eq!(off.view.rows[0].text, "Controller support is off");
        assert!(started_rx.try_recv().is_err(), "not started before the player asks");

        shared.supervisor.lock().unwrap().turn_on();
        let listener = started_rx.recv_timeout(Duration::from_secs(10)).expect("service started");
        let (mut conn, _) = listener.accept().unwrap();
        until(&status_rx, |s| s.link == Link::Connected);

        let snapshot = Snapshot {
            pad: Some(Pad { name: "Xbox One S pad".into(), id: "pad".into() }),
            game: Some(Game { pid: 7, window: true }),
            session: Some(Session { map: "Smashcraft".into(), phase: Phase::Match, player: Some(1) }),
            profile: Profile::Map,
            map: Some("Smashcraft".into()),
            choice: ProfileChoice::Auto,
            output: model::Output { running: true, ready: true, focused: true },
            problem: None,
            settings: Default::default(),
        };
        conn.write_all(ServiceMessage::Status(snapshot).line().as_bytes()).unwrap();
        let live = until(&status_rx, |s| s.snapshot.pad.is_some());
        let texts: Vec<_> = live.view.rows.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, ["Xbox One S pad connected", "Running", "In a match as Player 1"]);
        assert_eq!(live.view.overall, Light::Green);

        let mut input = InputView::default();
        input.press(model::Button::A, true);
        conn.write_all(ServiceMessage::Input(input).line().as_bytes()).unwrap();
        assert_eq!(input_rx.recv_timeout(Duration::from_secs(10)).unwrap(), input);

        assert!(shared.send(&ClientMessage::Profile(ProfileChoice::AnyMap)));
        let mut line = String::new();
        BufReader::new(conn.try_clone().unwrap()).read_line(&mut line).unwrap();
        assert_eq!(ClientMessage::parse(&line).unwrap(), ClientMessage::Profile(ProfileChoice::AnyMap));

        drop(conn);
        drop(listener);
        let lost = until(&status_rx, |s| s.link != Link::Connected);
        assert_eq!(lost.snapshot, Snapshot::default(), "no stale status after the service goes away");
        assert!(started_rx.recv_timeout(Duration::from_secs(10)).is_ok(), "a lost service is started again");

        shared.stop();
        link.join().unwrap();
    }

    #[test]
    fn a_service_already_running_is_used_and_never_started_again() {
        let port = free_port();
        let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
        let (status_tx, status_rx) = mpsc::channel();
        let (input_tx, _input_rx) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let shared = ControllerLink::new(true);
        let sink = Arc::new(Recorder(Mutex::new(status_tx), Mutex::new(input_tx)));
        let starter = Arc::new(FakeService { port, started: started_tx });
        let link = std::thread::spawn({
            let shared = shared.clone();
            move || run(shared, port, sink, starter)
        });
        let (conn, _) = listener.accept().unwrap();
        until(&status_rx, |s| s.link == Link::Connected);
        assert!(started_rx.try_recv().is_err());
        shared.stop();
        drop(conn);
        link.join().unwrap();
    }

    fn install_folder(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smashcraft-link-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for file in files {
            std::fs::write(dir.join(file), "").unwrap();
        }
        dir
    }

    #[test]
    fn the_service_is_the_named_one_else_padbridge_beside_this_executable_else_padbridge_on_the_path() {
        let name = if cfg!(windows) { "padbridge.exe" } else { "padbridge" };
        let named = Env { controller_service: Some("/opt/padbridge-test".into()), ..Env::default() };
        assert_eq!(service_program(&named), PathBuf::from("/opt/padbridge-test"));
        let install = install_folder("service", &[name]);
        let beside = Env { current_exe: Some(install.join("smashcraft")), ..Env::default() };
        assert_eq!(service_program(&beside), install.join(name));
        let empty = install_folder("service-empty", &[]);
        let alone = Env { current_exe: Some(empty.join("smashcraft")), ..Env::default() };
        assert_eq!(service_program(&alone), PathBuf::from(name));
        assert_eq!(service_program(&Env::default()), PathBuf::from(name));
        std::fs::remove_dir_all(install).unwrap();
        std::fs::remove_dir_all(empty).unwrap();
    }

    #[test]
    fn the_plug_in_is_beside_this_executable_else_the_one_wisp_installs_in_the_home_folder_else_none() {
        let install = install_folder("plugin", &["wc3-journal"]);
        let beside = Env { current_exe: Some(install.join("smashcraft")), ..Env::default() };
        assert_eq!(plugin_program(&beside), Some(install.join("wc3-journal")));
        let home = install_folder("plugin-home", &[]);
        let installed = home.join(".local/share/smashcraft-build-inputs/controller");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(installed.join("wc3-journal"), "").unwrap();
        let wisp = Env { home: Some(home.clone()), ..Env::default() };
        assert_eq!(plugin_program(&wisp), Some(installed.join("wc3-journal")));
        assert_eq!(plugin_program(&Env::default()), None);
        std::fs::remove_dir_all(install).unwrap();
        std::fs::remove_dir_all(home).unwrap();
    }
}

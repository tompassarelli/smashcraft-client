//! The client's link to the controller service: connect to the running
//! service, or start the bundled one once the player has turned controller
//! support on. The service is single-instance, so the client never runs a
//! second copy beside one that already answers.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use wc3_controller_model::{self as model, ClientMessage, InputView, Light, Link, ServiceMessage, Snapshot, View};

/// The service's local interface (smashcraft:companion/model).
pub const SERVICE_PORT: u16 = 47631;
/// The interface port; `WC3_CONTROLLER_PORT` overrides it for a test service.
pub fn service_port() -> u16 {
    std::env::var("WC3_CONTROLLER_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(SERVICE_PORT)
}

/// How long a started service may take to answer before it is started again.
pub const START_GRACE: Duration = Duration::from_secs(10);
/// Starts tried in a row before the client stops and tells the player.
pub const START_ATTEMPTS: u32 = 3;

pub const START_FAILED: &str = "Controller support couldn't start. Try again, or restart your computer if it keeps happening.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Start,
}

/// Whether to start the service, from what the client has seen. No I/O.
#[derive(Debug, Clone)]
pub struct Supervise {
    /// The player turned controller support on (remembered across runs).
    pub wanted: bool,
    pub link: Link,
    /// The last start attempt gave up.
    pub failed: bool,
    started_at: Option<Instant>,
    attempts: u32,
}

impl Supervise {
    pub fn new(wanted: bool) -> Self {
        Self {
            wanted,
            link: if wanted { Link::Starting } else { Link::Off },
            failed: false,
            started_at: None,
            attempts: 0,
        }
    }

    pub fn connected(&mut self) {
        self.link = Link::Connected;
        self.failed = false;
        self.attempts = 0;
        self.started_at = None;
    }

    /// Nothing answered on the service's interface.
    pub fn unreachable(&mut self, now: Instant) -> Action {
        if !self.wanted || self.failed {
            self.link = Link::Off;
            return Action::None;
        }
        let waiting = self.started_at.is_some_and(|at| now.duration_since(at) < START_GRACE);
        if waiting {
            self.link = Link::Starting;
            return Action::None;
        }
        if self.attempts >= START_ATTEMPTS {
            self.failed = true;
            self.link = Link::Off;
            return Action::None;
        }
        self.attempts += 1;
        self.started_at = Some(now);
        self.link = Link::Starting;
        Action::Start
    }

    /// The service could not even be launched.
    pub fn start_failed(&mut self) {
        self.failed = true;
        self.link = Link::Off;
    }

    /// The player pressed "Turn on controller support".
    pub fn turn_on(&mut self) {
        self.wanted = true;
        self.failed = false;
        self.attempts = 0;
        self.started_at = None;
        if self.link != Link::Connected {
            self.link = Link::Starting;
        }
    }
}

/// What the Controller page and tray show.
#[derive(Debug, Clone, Serialize)]
pub struct ControllerState {
    pub link: Link,
    pub wanted: bool,
    pub view: View,
    pub snapshot: Snapshot,
}

impl ControllerState {
    pub fn new(sup: &Supervise, snapshot: &Snapshot) -> Self {
        let mut view = model::view(sup.link, snapshot);
        if sup.failed {
            view.problem = Some(START_FAILED.to_owned());
            view.overall = Light::Red;
        }
        Self {
            link: sup.link,
            wanted: sup.wanted,
            view,
            snapshot: snapshot.clone(),
        }
    }
}

/// Where status goes: the window and tray in the app, a recorder in tests.
pub trait Sink: Send + Sync + 'static {
    fn status(&self, state: &ControllerState);
    fn input(&self, input: &InputView);
}

/// Launches the service; the app's spawns the bundled binary.
pub trait Starter: Send + Sync + 'static {
    fn start(&self) -> Result<(), String>;
}

/// The bundled service: `WC3_CONTROLLER_SERVICE`, else `wc3-journal` beside
/// this executable, else `wc3-journal` on the PATH. Started detached so it
/// keeps serving after the window closes.
pub struct BundledService {
    pub log: Option<PathBuf>,
}

pub fn service_program() -> PathBuf {
    if let Some(path) = std::env::var_os("WC3_CONTROLLER_SERVICE") {
        return path.into();
    }
    let name = if cfg!(windows) { "wc3-journal.exe" } else { "wc3-journal" };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|path| path.exists())
        .unwrap_or_else(|| name.into())
}

impl Starter for BundledService {
    fn start(&self) -> Result<(), String> {
        let program = service_program();
        let stderr = match &self.log {
            Some(path) => std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map(Stdio::from)
                .unwrap_or_else(|_| Stdio::null()),
            None => Stdio::null(),
        };
        let mut child = Command::new(&program)
            .arg("--service")
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

/// Shared between the link thread and the app's commands.
pub struct Shared {
    pub sup: Mutex<Supervise>,
    pub snapshot: Mutex<Snapshot>,
    writer: Mutex<Option<TcpStream>>,
    /// Sent on every new connection, so a restarted service gets the player's choices.
    greeting: Mutex<Vec<ClientMessage>>,
    stop: AtomicBool,
}

impl Shared {
    pub fn new(wanted: bool) -> Arc<Self> {
        Arc::new(Self {
            sup: Mutex::new(Supervise::new(wanted)),
            snapshot: Mutex::new(Snapshot::default()),
            writer: Mutex::new(None),
            greeting: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        })
    }

    pub fn state(&self) -> ControllerState {
        ControllerState::new(&self.sup.lock().unwrap(), &self.snapshot.lock().unwrap())
    }

    /// Sends a message to the service; false when none is connected.
    pub fn send(&self, message: &ClientMessage) -> bool {
        let mut writer = self.writer.lock().unwrap();
        match writer.as_mut() {
            Some(stream) => stream.write_all(message.line().as_bytes()).is_ok(),
            None => false,
        }
    }

    pub fn greet(&self, messages: Vec<ClientMessage>) {
        *self.greeting.lock().unwrap() = messages;
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(stream) = self.writer.lock().unwrap().take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

/// Runs the link until [`Shared::stop`]: connect, follow status, reconnect,
/// and start the service when it is wanted and nothing answers.
pub fn run(shared: Arc<Shared>, port: u16, sink: Arc<dyn Sink>, starter: Arc<dyn Starter>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let publish = |shared: &Shared| sink.status(&shared.state());
    let mut last_link = None;
    while !shared.stop.load(Ordering::SeqCst) {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(300)) {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                *shared.writer.lock().unwrap() = stream.try_clone().ok();
                for message in shared.greeting.lock().unwrap().clone() {
                    shared.send(&message);
                }
                shared.sup.lock().unwrap().connected();
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
                let action = shared.sup.lock().unwrap().unreachable(Instant::now());
                if action == Action::Start {
                    if let Err(_why) = starter.start() {
                        shared.sup.lock().unwrap().start_failed();
                    }
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
    use std::net::TcpListener;
    use std::sync::mpsc;
    use wc3_controller_model::{Game, Pad, Phase, Profile, ProfileChoice, Session};

    #[test]
    fn off_until_turned_on_and_never_started_when_not_wanted() {
        let mut sup = Supervise::new(false);
        let t = Instant::now();
        assert_eq!(sup.unreachable(t), Action::None);
        assert_eq!(sup.link, Link::Off);
        sup.turn_on();
        assert_eq!(sup.link, Link::Starting);
        assert_eq!(sup.unreachable(t), Action::Start);
    }

    #[test]
    fn a_started_service_gets_a_grace_period_then_retries_then_gives_up() {
        let mut sup = Supervise::new(true);
        let t = Instant::now();
        assert_eq!(sup.unreachable(t), Action::Start);
        assert_eq!(sup.unreachable(t + Duration::from_secs(5)), Action::None);
        assert_eq!(sup.link, Link::Starting);
        assert_eq!(sup.unreachable(t + START_GRACE), Action::Start);
        assert_eq!(sup.unreachable(t + START_GRACE * 2), Action::Start);
        assert_eq!(sup.unreachable(t + START_GRACE * 3), Action::None);
        assert!(sup.failed);
        let state = ControllerState::new(&sup, &Snapshot::default());
        assert_eq!(state.view.problem.as_deref(), Some(START_FAILED));
        assert_eq!(state.view.overall, Light::Red);
        sup.turn_on();
        assert!(!sup.failed);
        assert_eq!(sup.unreachable(t + START_GRACE * 4), Action::Start);
    }

    #[test]
    fn connecting_clears_a_failure_and_resets_attempts() {
        let mut sup = Supervise::new(true);
        sup.start_failed();
        sup.connected();
        assert_eq!(sup.link, Link::Connected);
        assert!(!sup.failed);
        let t = Instant::now();
        assert_eq!(sup.unreachable(t), Action::Start, "a lost service is started again");
    }

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
        let shared = Shared::new(false);
        let sink = Arc::new(Recorder(Mutex::new(status_tx), Mutex::new(input_tx)));
        let starter = Arc::new(FakeService { port, started: started_tx });
        let link = std::thread::spawn({
            let shared = shared.clone();
            move || run(shared, port, sink, starter)
        });

        let off = until(&status_rx, |s| s.link == Link::Off);
        assert_eq!(off.view.rows[0].text, "Controller support is off");
        assert!(started_rx.try_recv().is_err(), "not started before the player asks");

        shared.sup.lock().unwrap().turn_on();
        let listener = started_rx.recv_timeout(Duration::from_secs(10)).expect("service started");
        let (mut conn, _) = listener.accept().unwrap();
        until(&status_rx, |s| s.link == Link::Connected);

        let snapshot = Snapshot {
            pad: Some(Pad { name: "Xbox One S pad".into(), id: "pad".into() }),
            game: Some(Game { pid: 7, window: true }),
            session: Some(Session { map: "Smashcraft".into(), phase: Phase::Match, player: Some(1) }),
            profile: Profile::Smashcraft,
            choice: ProfileChoice::Auto,
            output: model::Output { running: true, ready: true, focused: true },
            problem: None,
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
        let shared = Shared::new(true);
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
}

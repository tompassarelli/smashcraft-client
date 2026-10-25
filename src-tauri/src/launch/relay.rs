//! Host and Join through Smashcraft Servers (#26): the client runs wisp's
//! relay agent (relay-agent's `AgentHost`) against the public relay,
//! `SMASHCRAFT_RELAY` when set (wisp:docs/relay.md, "Where the relay runs,
//! NAT, and who can join"). Only a player who opted in plays this way, and
//! only with the switch this build was made with (lan_switch.rs) on while the
//! match runs.

use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use relay_agent::host::{AgentHost, HostConfig};
use relay_agent::stop::StopHandle;
use relay_agent::Link;
use relay_proto::room::MapFacts;

use super::lan_switch::LanSwitch;
use crate::online::{Mode, OnlineState};

pub const PUBLIC_RELAY: &str = "159.89.205.60:7457";
pub const OPT_IN_FIRST: &str = "Opt in to Smashcraft Servers first.";
pub const NO_WARCRAFT: &str = "Start Warcraft III with Play first, then host or join.";
pub const GAME_NAME: &str = "Smashcraft";
const VERSION: u32 = 10200;
const ANNOUNCE_PORTS: [u16; 8] = [16000, 16001, 16002, 16003, 16004, 16005, 16006, 16007];

pub fn relay_address() -> String {
    std::env::var("SMASHCRAFT_RELAY").unwrap_or_else(|_| PUBLIC_RELAY.to_owned())
}

pub enum Role {
    Host(MapFacts),
    /// The room to join and the SHA-1 of this computer's copy of the map.
    Join { room: u64, sha1: [u8; 20] },
}

/// The code a host gives their opponent: the relay's room id, as relay-agent prints it.
pub fn code(room: u64) -> String {
    format!("{room:016x}")
}

pub fn parse_code(typed: &str) -> Result<u64, String> {
    let digits: String = typed.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    match u64::from_str_radix(&digits, 16) {
        Ok(room) if digits.len() == 16 => Ok(room),
        _ => Err(format!("{} isn't a Smashcraft Servers code: those are 16 letters and digits.", typed.trim())),
    }
}

/// Takes a slot in the role's room and the `AgentHost` config that serves it; a joiner expects its own map.
fn open(relay: SocketAddr, role: Role) -> io::Result<(Link, HostConfig)> {
    let (link, host, expect_map_sha1) = match role {
        Role::Host(map) => (Link::create(relay, 2)?, Some((map, 0)), None),
        Role::Join { room, sha1 } => (Link::join(relay, room)?, None, Some(sha1)),
    };
    let announce_ports = ANNOUNCE_PORTS.to_vec();
    let config = HostConfig { game_name: GAME_NAME.as_bytes().to_vec(), version: VERSION, listen_port: 0, announce_ports, host, expect_map_sha1, trace: None, tuning: Default::default() };
    Ok((link, config))
}

/// What a run needs besides its role: the player's opt-in, the switch, the Warcraft III it serves and the relay.
pub struct Start {
    pub opted_in: bool,
    pub switch: Arc<dyn LanSwitch>,
    pub warcraft_pid: Option<u32>,
    pub relay: String,
}

/// The running host's stop handle, or a Cancel that came before it existed.
#[derive(Default)]
struct Stop {
    handle: Option<StopHandle>,
    cancelled: bool,
}

#[derive(Default)]
pub struct Relay {
    state: Mutex<OnlineState>,
    stop: Mutex<Stop>,
}

fn resolve(relay: &str) -> Result<SocketAddr, String> {
    relay.to_socket_addrs().ok().and_then(|mut addrs| addrs.next()).ok_or(format!("Smashcraft Servers ({relay}) can't be found."))
}

impl Relay {
    pub fn state(&self) -> OnlineState {
        let mut state = self.state.lock().unwrap().clone();
        state.available = true;
        state
    }

    fn say(&self, line: String) -> OnlineState {
        let mut state = self.state.lock().unwrap();
        state.lines.push(line);
        state.clone()
    }

    /// Checks the opt-in and the switch before anything else, then reads `role` and runs the agent on its own thread.
    pub fn start(
        self: &Arc<Self>,
        mode: Mode,
        how: Start,
        role: impl FnOnce() -> Result<Role, String>,
        changed: impl Fn(OnlineState) + Send + Sync + 'static,
    ) -> Result<(), String> {
        if !how.opted_in {
            return Err(OPT_IN_FIRST.to_owned());
        }
        how.switch.available()?;
        let pid = how.warcraft_pid.ok_or(NO_WARCRAFT)?;
        let role = role()?;
        let relay = resolve(&how.relay)?;
        {
            let mut state = self.state.lock().unwrap();
            if state.running {
                return Err("A game is already being set up.".to_owned());
            }
            *state = OnlineState { available: true, running: true, mode: Some(mode), ..OnlineState::default() };
            *self.stop.lock().unwrap() = Stop::default();
        }
        let relay_run = self.clone();
        std::thread::spawn(move || {
            let ok = relay_run.run(relay, role, pid, &*how.switch, &changed);
            let snapshot = {
                let mut state = relay_run.state.lock().unwrap();
                state.running = false;
                state.finished = Some(ok);
                state.clone()
            };
            changed(snapshot);
        });
        Ok(())
    }

    fn run(&self, relay: SocketAddr, role: Role, pid: u32, switch: &dyn LanSwitch, changed: &dyn Fn(OnlineState)) -> bool {
        let mut say = |line: String| changed(self.say(line));
        if let Err(problem) = switch.enable(pid) {
            say(problem);
            return false;
        }
        say("Connecting to Smashcraft Servers".to_owned());
        let ok = match open(relay, role) {
            Err(e) => {
                say(format!("Smashcraft Servers didn't answer: {e}"));
                false
            }
            Ok((link, config)) => self.serve(link, config, &mut say),
        };
        switch.disable();
        ok
    }

    /// Runs `AgentHost` until the match is over or Cancel; true when the match was played.
    fn serve(&self, link: Link, config: HostConfig, say: &mut dyn FnMut(String)) -> bool {
        match config.host {
            Some(_) => say(format!("Join code: {}", code(link.room()))),
            None => say("Found the host's game".to_owned()),
        }
        say(format!("In Warcraft III, join “{GAME_NAME}”"));
        let played = AgentHost::new(config, link, say).and_then(|host| {
            self.keep(host.stop_handle());
            host.run()
        });
        played.map(|summary| summary.turns > 0).unwrap_or_else(|e| {
            say(format!("The game stopped: {e}"));
            false
        })
    }

    fn keep(&self, handle: StopHandle) {
        let mut stop = self.stop.lock().unwrap();
        if stop.cancelled {
            handle.stop();
        }
        stop.handle = Some(handle);
    }

    pub fn cancel(&self) {
        let mut stop = self.stop.lock().unwrap();
        stop.cancelled = true;
        if let Some(handle) = stop.handle.as_ref() {
            handle.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch::lan_switch;
    use relay_proto::room::MapPlayer;
    use std::net::UdpSocket;
    use std::sync::mpsc;
    use std::time::Duration;

    struct Ready;

    impl LanSwitch for Ready {
        fn available(&self) -> Result<(), String> {
            Ok(())
        }
        fn enable(&self, _warcraft_pid: u32) -> Result<(), String> {
            Ok(())
        }
        fn disable(&self) {}
    }

    fn map(sha1: [u8; 20]) -> MapFacts {
        let players = (0..2).map(|id| MapPlayer { id, controller: 1, race: 1 }).collect();
        MapFacts { path: b"Maps\\00-Smashcraft\\Smashcraft 0.0.1.w3x".to_vec(), size: 1024, crc32: 1, sha1, xoro: 2, width: 64, height: 64, layout: 3, forces: vec![3], players }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn local_relay() -> String {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let address = socket.local_addr().unwrap().to_string();
        std::thread::spawn(move || relay::udp::serve(&socket, &mut relay::Server::new(relay::Config::default()), false, 600_000_000));
        address
    }

    fn start(switch: Arc<dyn LanSwitch>, relay: &str, opted_in: bool) -> Start {
        Start { opted_in, switch, warcraft_pid: Some(1), relay: relay.to_owned() }
    }

    fn run(relay: &Arc<Relay>, mode: Mode, how: Start, role: Role) -> mpsc::Receiver<OnlineState> {
        let (tx, rx) = mpsc::channel();
        relay.start(mode, how, || Ok(role), move |state| drop(tx.send(state))).unwrap();
        rx
    }

    /// The first state whose last line is `wanted`, or the run's end, else the last within 10 s.
    fn line(rx: &mpsc::Receiver<OnlineState>, wanted: impl Fn(&str) -> bool) -> OnlineState {
        let mut last = OnlineState::default();
        while let Ok(state) = rx.recv_timeout(Duration::from_secs(10)) {
            if state.lines.last().is_some_and(|l| wanted(l)) || !state.running {
                return state;
            }
            last = state;
        }
        last
    }

    fn hosted(relay: &str) -> (Arc<Relay>, String) {
        let host = Arc::new(Relay::default());
        let hosting = run(&host, Mode::Host, start(Arc::new(Ready), relay, true), Role::Host(map([7; 20])));
        let shown = line(&hosting, |l| l.starts_with("Join code: "));
        let code = shown.lines.last().unwrap().strip_prefix("Join code: ").unwrap().to_owned();
        (host, code)
    }

    #[test]
    fn host_shows_a_code_and_join_takes_it_into_the_hosts_game_and_cancel_stops_it_on_a_wisp_relay() {
        let relay = local_relay();
        let (host, code) = hosted(&relay);
        assert_eq!(code.len(), 16);
        let joiner = Arc::new(Relay::default());
        let joining = run(&joiner, Mode::Join, start(Arc::new(Ready), &relay, true), Role::Join { room: parse_code(&code).unwrap(), sha1: [7; 20] });
        let joined = line(&joining, |l| l.starts_with("room: "));
        assert!(joined.lines.last().unwrap().starts_with("room: p2 of 2, "), "{:?}", joined.lines);
        assert!(joined.running);
        joiner.cancel();
        line(&joining, |_| false);
        let stopped = joiner.state();
        assert!(!stopped.running, "Cancel stops the joiner: {:?}", stopped.lines);
        assert_eq!(stopped.finished, Some(false));
        host.cancel();
    }

    #[test]
    fn a_joiner_whose_map_sha1_differs_from_the_rooms_is_refused_before_the_countdown_with_both_hashes() {
        let relay = local_relay();
        let (host, code) = hosted(&relay);
        let joiner = Arc::new(Relay::default());
        let joining = run(&joiner, Mode::Join, start(Arc::new(Ready), &relay, true), Role::Join { room: parse_code(&code).unwrap(), sha1: [9; 20] });
        let done = line(&joining, |_| false);
        let refusal = format!("The game stopped: map SHA-1 {} differs from the room's {}", hex(&[9; 20]), hex(&[7; 20]));
        assert_eq!(done.lines.last().map(String::as_str), Some(refusal.as_str()), "{:?}", done.lines);
        assert!(!done.running);
        assert_eq!(done.finished, Some(false));
        host.cancel();
    }

    #[test]
    fn without_the_opt_in_or_with_the_public_builds_switch_host_and_join_explain_and_do_nothing_else() {
        let watcher = UdpSocket::bind("127.0.0.1:0").unwrap();
        watcher.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let relay = watcher.local_addr().unwrap().to_string();
        let unavailable = lan_switch::built_in().available().unwrap_err();
        let online = Arc::new(Relay::default());
        for (mode, opted_in, problem) in [(Mode::Host, false, OPT_IN_FIRST), (Mode::Join, false, OPT_IN_FIRST), (Mode::Host, true, &unavailable), (Mode::Join, true, &unavailable)] {
            let read = std::cell::Cell::new(false);
            let result = online.start(mode, start(lan_switch::built_in(), &relay, opted_in), || {
                read.set(true);
                Ok(Role::Join { room: 1, sha1: [0; 20] })
            }, |_| panic!("nothing runs"));
            assert_eq!(result, Err(problem.to_owned()));
            assert!(!read.get(), "neither the map nor the code is read");
        }
        assert!(!unavailable.is_empty());
        let state = online.state();
        assert!(!state.running && state.lines.is_empty() && state.mode.is_none());
        assert!(watcher.recv_from(&mut [0u8; 64]).is_err(), "nothing reached the relay");
    }
}

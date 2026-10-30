use std::time::{Duration, Instant};

use padbridge_model::{self as model, Light, Link, Snapshot, View};
use serde::Serialize;

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
pub struct Supervisor {
    /// The player turned controller support on (remembered across runs).
    pub wanted: bool,
    pub link: Link,
    /// The last start attempt gave up.
    pub failed: bool,
    started_at: Option<Instant>,
    attempts: u32,
}

impl Supervisor {
    pub fn new(wanted: bool) -> Self {
        Self { wanted, link: if wanted { Link::Starting } else { Link::Off }, failed: false, started_at: None, attempts: 0 }
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
    pub fn new(supervisor: &Supervisor, snapshot: &Snapshot) -> Self {
        let mut view = model::view(supervisor.link, snapshot);
        if supervisor.failed {
            view.problem = Some(START_FAILED.to_owned());
            view.overall = Light::Red;
        }
        Self { link: supervisor.link, wanted: supervisor.wanted, view, snapshot: snapshot.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// xorshift64: the same cases on every run.
    fn random(mut seed: u64) -> impl FnMut(u64) -> u64 {
        move |below| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % below
        }
    }

    #[test]
    fn the_supervisor_starts_only_when_wanted_waits_out_each_grace_period_and_gives_up_after_its_attempts_2000_seeded_runs() {
        let mut next = random(0x9e37_79b9_7f4a_7c15);
        for _ in 0..2000 {
            let wanted = next(2) == 0;
            let mut supervisor = Supervisor::new(wanted);
            assert_eq!(supervisor.link, if wanted { Link::Starting } else { Link::Off });
            let base = Instant::now();
            let mut now = base;
            let (mut want, mut failed, mut starts, mut last): (bool, bool, u32, Option<Instant>) = (wanted, false, 0, None);
            for _ in 0..40 {
                match next(10) {
                    0 => {
                        supervisor.turn_on();
                        (want, failed, starts, last) = (true, false, 0, None);
                        assert_ne!(supervisor.link, Link::Off);
                    }
                    1 => {
                        supervisor.connected();
                        (failed, starts, last) = (false, 0, None);
                        assert_eq!(supervisor.link, Link::Connected);
                    }
                    2 => {
                        supervisor.start_failed();
                        failed = true;
                        assert_eq!(supervisor.link, Link::Off);
                    }
                    _ => {
                        now += Duration::from_millis(next(START_GRACE.as_millis() as u64 * 2));
                        let action = supervisor.unreachable(now);
                        let waiting = last.is_some_and(|at| now.duration_since(at) < START_GRACE);
                        if !want || failed {
                            assert_eq!((action, supervisor.link), (Action::None, Link::Off), "never started when not wanted or after giving up");
                        } else if waiting {
                            assert_eq!((action, supervisor.link), (Action::None, Link::Starting), "a started service gets its grace period");
                        } else if starts >= START_ATTEMPTS {
                            assert_eq!((action, supervisor.link), (Action::None, Link::Off), "gives up after its attempts");
                            failed = true;
                        } else {
                            assert_eq!((action, supervisor.link), (Action::Start, Link::Starting), "starts when wanted and not waiting");
                            starts += 1;
                            last = Some(now);
                        }
                    }
                }
                assert_eq!(supervisor.failed, failed);
                let state = ControllerState::new(&supervisor, &Snapshot::default());
                assert_eq!(state.view.problem.as_deref() == Some(START_FAILED), failed);
                if failed {
                    assert_eq!(state.view.overall, Light::Red);
                }
            }
        }
    }
}

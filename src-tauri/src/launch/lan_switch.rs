//! Smashcraft Servers' switch for Warcraft III (Tom, 25 October 2026): the
//! player opts in, and while a relay match runs the switch is on. Its
//! implementation is private and ships compiled through the
//! `smashcraft-servers` feature; the public build has `Unavailable`.

pub trait LanSwitch: Send + Sync {
    /// Whether this build can turn the switch on; the error is written for players.
    fn available(&self) -> Result<(), String>;
    fn enable(&self, warcraft_pid: u32) -> Result<(), String>;
    fn disable(&self);
}

pub const UNAVAILABLE: &str = "This copy of Smashcraft can't play on Smashcraft Servers: it was built without them. Use Battle.net instead.";

pub struct Unavailable;

impl LanSwitch for Unavailable {
    fn available(&self) -> Result<(), String> {
        Err(UNAVAILABLE.to_owned())
    }

    fn enable(&self, _warcraft_pid: u32) -> Result<(), String> {
        self.available()
    }

    fn disable(&self) {}
}

#[cfg(feature = "smashcraft-servers")]
impl LanSwitch for smashcraft_servers::Switch {
    fn available(&self) -> Result<(), String> {
        smashcraft_servers::Switch::available(self)
    }

    fn enable(&self, warcraft_pid: u32) -> Result<(), String> {
        smashcraft_servers::Switch::enable(self, warcraft_pid)
    }

    fn disable(&self) {
        smashcraft_servers::Switch::disable(self)
    }
}

/// The switch this build was made with.
pub fn built_in() -> std::sync::Arc<dyn LanSwitch> {
    #[cfg(feature = "smashcraft-servers")]
    return std::sync::Arc::new(smashcraft_servers::Switch::default());
    #[cfg(not(feature = "smashcraft-servers"))]
    std::sync::Arc::new(Unavailable)
}

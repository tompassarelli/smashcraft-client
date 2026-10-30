//! The client's link to the controller service: connect to the running
//! service, or start the bundled one once the player has turned controller
//! support on. The service is single-instance, so the client never runs a
//! second copy beside one that already answers.

mod client;
mod supervisor;

pub use client::{BundledService, Shared, Sink, run, service_program};
pub use supervisor::ControllerState;

/// The service's local interface (padbridge:model).
pub const SERVICE_PORT: u16 = 47631;
/// The interface port; `WC3_CONTROLLER_PORT` overrides it for a test service.
pub fn service_port() -> u16 {
    std::env::var("WC3_CONTROLLER_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(SERVICE_PORT)
}

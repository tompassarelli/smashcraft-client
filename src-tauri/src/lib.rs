//! The Smashcraft client: a window with routed pages (Controller, History,
//! Stats, Replays and Online) and a tray light. Controller support comes
//! from the Warcraft III Controller service over its local interface; the
//! client starts the bundled service only when none answers.

mod app;
mod autostart;
mod cli;
mod commands;
mod files;
mod instance_lock;
mod launch;
mod link;
mod lua32;
mod mapsim;
mod mpq;
mod online;
mod play;
mod records;
mod replays;
mod settings;
mod sims;
mod sink;
mod state;
mod tray;
mod warcraft_replays;

use tauri_plugin_autostart::MacosLauncher;

pub fn run() {
    if std::env::args().any(|arg| arg == "--check") {
        return cli::check();
    }
    let hidden = std::env::args().any(|arg| arg == "--hidden");
    let instance_lock::Claim::First(instance) = instance_lock::claim(instance_lock::client_port()) else {
        return;
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .setup(move |app| app::setup(app, instance, hidden))
        .on_window_event(app::hide_on_close)
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("Smashcraft failed to start");
}

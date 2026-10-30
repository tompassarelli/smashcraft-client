//! The Smashcraft client: a window with routed pages (Controller, History,
//! Stats, Replays and Online) and a tray light. Controller support comes
//! from the Warcraft III Controller service over its local interface; the
//! client starts the bundled service only when none answers.

pub mod autostart;
pub mod cli;
pub mod files;
pub mod commands;
pub mod game;
pub mod instance_lock;
pub mod launch;
pub mod link;
pub mod lua32;
pub mod mapsim;
pub mod mpq;
pub mod online;
pub mod play;
pub mod records;
pub mod replays;
pub mod settings;
pub mod sink;
pub mod state;
pub mod tray;
pub mod warcraft_replays;

use std::sync::{Arc, Mutex};

use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use link::Shared;
use settings::Store;
use sink::{add_tray, show_input, TauriSink};
use state::{record_folders_of, show_window, AppState};

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
        .setup(move |app| {
            let config = app.path().app_config_dir()?;
            let store = Store::new(&config);
            let history = records::History::new(&app.path().app_data_dir()?);
            let kept = replays::Kept::new(&app.path().app_data_dir()?);
            let mapsims = mapsim::MapSims::new(&app.path().app_data_dir()?);
            let warcraft = warcraft_replays::Library::new(&app.path().app_data_dir()?);
            let settings = store.load();
            let shared = Shared::new(settings.controller_on);
            shared.greet(settings.greeting());

            let initial = shared.state().view.overall;
            add_tray(app, initial)?;

            let log_dir = app.path().app_log_dir().ok().inspect(|dir| {
                let _ = std::fs::create_dir_all(dir);
            });
            let log = log_dir.as_ref().map(|dir| dir.join("controller.log"));
            let pending = Arc::new(Mutex::new(None));
            std::thread::spawn({
                let (app, pending) = (app.handle().clone(), pending.clone());
                move || show_input(app, pending)
            });
            let sink = Arc::new(TauriSink { app: app.handle().clone(), light: Mutex::new(Some(initial)), input: pending });
            let starter = Arc::new(link::BundledService { log });
            std::thread::spawn({
                let shared = shared.clone();
                move || link::run(shared, link::service_port(), sink, starter)
            });
            std::thread::spawn({
                let app = app.handle().clone();
                move || instance_lock::listen(instance, move || show_window(&app))
            });
            app.manage(AppState { link: shared, store, settings: Mutex::new(settings), history, online: Arc::default(), standalone: Arc::default(), relay: Arc::default(), log_dir, kept, mapsims, warcraft });
            std::thread::spawn({
                let (app, library) = (app.handle().clone(), warcraft_replays::Library::new(&app.path().app_data_dir()?));
                move || warcraft_replays::watch(library, move || record_folders_of(&app.state::<AppState>().settings.lock().unwrap()))
            });
            if !hidden {
                show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("Smashcraft failed to start");
}

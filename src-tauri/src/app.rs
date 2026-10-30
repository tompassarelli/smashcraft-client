//! The Tauri setup: the state every command reads, the tray light, the
//! controller link, the single-instance listener and the Warcraft replay
//! watcher. Closing the window hides it; the tray keeps the app running.

use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use tauri::{App, Manager, Window, WindowEvent};

use crate::link::{self, Shared};
use crate::settings::Store;
use crate::sink::{TauriSink, add_tray, show_input};
use crate::state::{AppState, show_window};
use crate::{instance_lock, mapsim, records, replays, warcraft_replays};

pub fn setup(app: &mut App, instance: TcpListener, hidden: bool) -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::new(&app.path().app_config_dir()?);
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
    std::thread::spawn({
        let (app, library) = (app.handle().clone(), warcraft.clone());
        move || warcraft_replays::watch(library, move || app.state::<AppState>().folders())
    });
    app.manage(AppState {
        link: shared,
        store,
        settings: Mutex::new(settings),
        history,
        play: Arc::default(),
        online: Arc::default(),
        standalone: Arc::default(),
        relay: Arc::default(),
        log_dir,
        kept,
        mapsims,
        warcraft,
    });
    if !hidden {
        show_window(app.handle());
    }
    Ok(())
}

pub fn hide_on_close(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    }
}

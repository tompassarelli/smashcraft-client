use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::link::Shared;
use crate::settings::{Settings, Store};
use crate::{mapsim, online, records, replays, warcraft_replays};

pub struct AppState {
    pub link: Arc<Shared>,
    pub store: Store,
    pub settings: Mutex<Settings>,
    pub history: records::History,
    pub online: Arc<online::Online>,
    pub log_dir: Option<std::path::PathBuf>,
    pub kept: replays::Kept,
    pub mapsims: mapsim::MapSims,
    pub warcraft: warcraft_replays::Library,
}

impl AppState {
    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
        let mut settings = self.settings.lock().unwrap();
        change(&mut settings);
        self.store.save(&settings)
    }
}

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn record_folders_of(settings: &Settings) -> Vec<String> {
    settings.record_folders.clone().unwrap_or_else(records::default_folders)
}

pub fn maps_of(state: &AppState) -> Vec<std::path::PathBuf> {
    mapsim::maps_folders(&record_folders_of(&state.settings.lock().unwrap()))
}

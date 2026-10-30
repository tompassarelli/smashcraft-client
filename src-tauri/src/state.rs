use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::link::ControllerLink;
use crate::settings::{Settings, Store};
use crate::{mapsim, online, play, records, replays, warcraft_replays};

pub struct AppState {
    pub link: Arc<ControllerLink>,
    pub store: Store,
    pub settings: Mutex<Settings>,
    pub history: records::History,
    pub play: Arc<play::Play>,
    pub standalone: Arc<online::Online>,
    pub log_dir: Option<std::path::PathBuf>,
    pub kept: replays::SharedReplays,
    pub mapsims: mapsim::MapSims,
    pub warcraft: warcraft_replays::Library,
}

impl AppState {
    /// The record folders the player chose, or the ones found on this computer.
    pub fn folders(&self) -> Vec<String> {
        let settings = self.settings.lock().unwrap();
        settings.custom_map_data_folders.clone().unwrap_or_else(records::default_folders)
    }

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

pub fn maps_of(state: &AppState) -> Vec<std::path::PathBuf> {
    mapsim::maps_folders(&state.folders())
}

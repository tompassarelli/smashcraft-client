use tauri::State;

use crate::state::{AppState, maps_of, record_folders_of};
use crate::warcraft_replays;
use crate::{files, replays};

#[tauri::command]
pub fn read_replays(state: State<AppState>) -> Vec<files::TextFile> {
    let folders = record_folders_of(&state.settings.lock().unwrap());
    replays::read_replay_files(&folders, &state.kept.replays)
}

#[tauri::command]
pub fn read_replay_parts(folder: String, serial: u32, parts: u32) -> Result<Vec<String>, String> {
    replays::read_parts(&folder, serial, parts)
}

#[tauri::command]
pub fn keep_replay(state: State<AppState>, name: String, text: String) -> Result<String, String> {
    state.kept.keep_replay(&name, &text)
}

#[tauri::command]
pub fn keep_sim(state: State<AppState>, version: String, code: String) -> Result<(), String> {
    state.kept.keep_sim(&version, &code)
}

#[tauri::command]
pub fn kept_sim(state: State<AppState>, version: String) -> Option<String> {
    state.kept.sim(&version)
}

/// Versions whose maps are in the Maps folders beside the record folders, or were read before.
#[tauri::command]
pub async fn map_versions(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.mapsims.scan(&maps_of(&state)))
}

/// Opens a joined replay in its version's map: {"id", "opened"} with the viewer driver's answer.
#[tauri::command]
pub async fn map_replay_open(state: State<'_, AppState>, version: String, replay: String, viewer: String) -> Result<String, String> {
    let (id, opened) = state.mapsims.open(&maps_of(&state), &version, &replay, &viewer)?;
    Ok(format!("{{\"id\":{id},\"opened\":{opened}}}"))
}

#[tauri::command]
pub async fn map_replay_step(state: State<'_, AppState>, id: u32, seek: bool, frames: i64) -> Result<String, String> {
    state.mapsims.step(id, seek, frames)
}

#[tauri::command]
pub fn map_replay_close(state: State<AppState>, id: u32) {
    state.mapsims.close(id);
}

#[tauri::command]
pub fn kept_sims(state: State<AppState>) -> Vec<String> {
    state.kept.sims()
}

#[tauri::command]
pub fn warcraft_games(state: State<AppState>) -> Vec<warcraft_replays::WarcraftGame> {
    state.warcraft.games()
}

#[tauri::command]
pub fn watch_in_warcraft(state: State<AppState>, file: String, name: String) -> Result<String, String> {
    state.warcraft.put_in_warcraft(&file, &name)
}

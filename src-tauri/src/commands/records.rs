use tauri::State;

use crate::state::AppState;
use crate::{files, records};

#[tauri::command]
pub fn record_folders(state: State<AppState>) -> Vec<String> {
    state.folders()
}

#[tauri::command]
pub fn set_record_folders(state: State<AppState>, folders: Vec<String>) -> Result<Vec<String>, String> {
    state.update(|s| s.record_folders = Some(folders))?;
    Ok(state.folders())
}

#[tauri::command]
pub fn read_records(state: State<AppState>) -> Vec<files::TextFile> {
    records::read_record_files(&state.folders())
}

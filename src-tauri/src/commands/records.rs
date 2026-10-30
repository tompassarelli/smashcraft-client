use tauri::State;

use crate::{files, records};
use crate::state::{record_folders_of, AppState};

#[tauri::command]
pub fn record_folders(state: State<AppState>) -> Vec<String> {
    record_folders_of(&state.settings.lock().unwrap())
}

#[tauri::command]
pub fn set_record_folders(state: State<AppState>, folders: Vec<String>) -> Result<Vec<String>, String> {
    state.update(|s| s.record_folders = Some(folders))?;
    Ok(record_folders_of(&state.settings.lock().unwrap()))
}

#[tauri::command]
pub fn read_records(state: State<AppState>) -> Vec<files::TextFile> {
    let folders = record_folders_of(&state.settings.lock().unwrap());
    records::read_record_files(&folders)
}

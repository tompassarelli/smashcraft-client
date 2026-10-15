use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn history(state: State<AppState>) -> serde_json::Value {
    state.history.load()
}

#[tauri::command]
pub fn save_history(state: State<AppState>, history: serde_json::Value) -> Result<(), String> {
    state.history.save(&history)
}

use tauri::{AppHandle, Emitter, State};

use crate::state::{AppState, maps_of};
use crate::{online, play};

#[tauri::command]
pub fn play_state(state: State<AppState>) -> play::PlayState {
    state.play.state(&maps_of(&state))
}

#[tauri::command]
pub fn play(app: AppHandle, state: State<AppState>) -> Result<play::PlayState, String> {
    let maps = maps_of(&state);
    state.play.start(&maps, state.link.in_match(), online::tools_dir(), move |snapshot| {
        let _ = app.emit("play", snapshot);
    })?;
    Ok(state.play.state(&maps))
}

pub fn warcraft_pid(state: &AppState) -> Option<u32> {
    state.play.warcraft_pid()
}

use std::sync::LazyLock;

use tauri::{AppHandle, Emitter, State};

use crate::play;
use crate::state::{maps_of, AppState};

static PLAY: LazyLock<play::Play> = LazyLock::new(play::Play::default);

#[tauri::command]
pub fn play_state(state: State<AppState>) -> play::PlayState {
    PLAY.state(&maps_of(&state))
}

#[tauri::command]
pub fn play(app: AppHandle, state: State<AppState>) -> Result<play::PlayState, String> {
    let maps = maps_of(&state);
    PLAY.start(&maps, move |snapshot| {
        let _ = app.emit("play", snapshot);
    })?;
    Ok(PLAY.state(&maps))
}

pub fn warcraft_pid() -> Option<u32> {
    PLAY.warcraft_pid()
}

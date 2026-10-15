use tauri::{AppHandle, Emitter, State};

use crate::online;
use crate::play;
use crate::state::AppState;

#[tauri::command]
pub fn online_state(state: State<AppState>) -> online::OnlineState {
    state.online.state()
}

/// The player's answer to adding Smashcraft's page to Warcraft III's menus: none until asked.
#[tauri::command]
pub fn menu_page_choice(state: State<AppState>) -> Option<bool> {
    state.settings.lock().unwrap().menu_page
}

#[tauri::command]
pub fn decline_menu_page(state: State<AppState>) -> Result<(), String> {
    state.update(|s| s.menu_page = Some(false))
}

fn start_online(app: AppHandle, state: &AppState, mode: online::Mode, code: Option<&str>) -> Result<online::OnlineState, String> {
    let dir = play::tools_dir().ok_or("Online play needs Smashcraft's tools: set SMASHCRAFT_TS to their ts folder.")?;
    let repair = state.settings.lock().unwrap().menu_page == Some(true);
    let bun = std::env::var_os("BUN").unwrap_or_else(|| "bun".into());
    let log = state.log_dir.as_ref().map(|dir| dir.join("online.log"));
    state.online.start(mode, bun, online::online_args(mode, code, repair), &dir, log, move |snapshot| {
        let _ = app.emit("online", snapshot);
    })?;
    Ok(state.online.state())
}

/// The player agreed: remember it and set up Warcraft III's menus.
#[tauri::command]
pub fn online_setup(app: AppHandle, state: State<AppState>) -> Result<online::OnlineState, String> {
    state.update(|s| s.menu_page = Some(true))?;
    start_online(app, &state, online::Mode::Setup, None)
}

#[tauri::command]
pub fn online_host(app: AppHandle, state: State<AppState>) -> Result<online::OnlineState, String> {
    start_online(app, &state, online::Mode::Host, None)
}

#[tauri::command]
pub fn online_join(app: AppHandle, state: State<AppState>, code: String) -> Result<online::OnlineState, String> {
    start_online(app, &state, online::Mode::Join, Some(&code))
}

#[tauri::command]
pub fn online_start_now(state: State<AppState>) -> bool {
    state.online.start_now()
}

#[tauri::command]
pub fn online_cancel(state: State<AppState>) {
    state.online.cancel()
}

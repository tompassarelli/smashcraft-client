use tauri::{AppHandle, Emitter, State};

use crate::commands::play as play_page;
use crate::game::map_facts;
use crate::launch::{lan_switch, relay};
use crate::online;
use crate::play;
use crate::state::{maps_of, AppState};

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

/// The player's answer to Smashcraft Servers: none until they choose.
#[tauri::command]
pub fn smashcraft_servers_choice(state: State<AppState>) -> Option<bool> {
    state.settings.lock().unwrap().smashcraft_servers
}

/// The player opted in on the screen that shows the opt-in text.
#[tauri::command]
pub fn opt_in_smashcraft_servers(state: State<AppState>) -> Result<(), String> {
    state.update(|s| s.smashcraft_servers = Some(true))
}

#[tauri::command]
pub fn relay_state(state: State<AppState>) -> online::OnlineState {
    state.relay.state()
}

fn start_relay(app: AppHandle, state: &AppState, mode: online::Mode, role: impl FnOnce() -> Result<relay::Role, String>) -> Result<online::OnlineState, String> {
    let how = relay::Start {
        opted_in: state.settings.lock().unwrap().smashcraft_servers == Some(true),
        switch: lan_switch::built_in(),
        warcraft_pid: play_page::warcraft_pid(),
        relay: relay::relay_address(),
    };
    state.relay.start(mode, how, role, move |snapshot| {
        let _ = app.emit("relay", snapshot);
    })?;
    Ok(state.relay.state())
}

fn current_map(state: &AppState) -> Result<std::path::PathBuf, String> {
    play::current_map(&maps_of(state)).ok_or("No Smashcraft map is in Warcraft III's Maps/00-Smashcraft folder yet.".to_owned())
}

#[tauri::command]
pub fn relay_host(app: AppHandle, state: State<AppState>) -> Result<online::OnlineState, String> {
    let map = || map_facts::read(&current_map(&state)?).map(relay::Role::Host);
    start_relay(app, &state, online::Mode::Host, map)
}

#[tauri::command]
pub fn relay_join(app: AppHandle, state: State<AppState>, code: String) -> Result<online::OnlineState, String> {
    let role = || {
        let room = relay::parse_code(&code)?;
        let file = std::fs::read(current_map(&state)?).map_err(|e| e.to_string())?;
        Ok(relay::Role::Join { room, sha1: map_facts::sha1(&file) })
    };
    start_relay(app, &state, online::Mode::Join, role)
}

#[tauri::command]
pub fn relay_cancel(state: State<AppState>) {
    state.relay.cancel()
}

use tauri::{AppHandle, Emitter, State};

use crate::online;
use crate::state::AppState;

#[tauri::command]
pub fn standalone_state(state: State<AppState>) -> online::OnlineState {
    state.standalone.state()
}

fn start_standalone(app: AppHandle, state: &AppState, mode: online::Mode, code: Option<&str>) -> Result<online::OnlineState, String> {
    let dir = online::tools_dir().ok_or("Online play needs Smashcraft's tools: set SMASHCRAFT_TS to their ts folder.")?;
    let bun = online::bun();
    let log = state.log_dir.as_ref().map(|dir| dir.join("standalone.log"));
    state.standalone.start(mode, online::Process::new(bun, online::standalone_args(mode, code), dir), log, online::shows_relay_round_trip, move |snapshot| {
        let _ = app.emit("standalone", snapshot);
    })?;
    Ok(state.standalone.state())
}

/// Hosts a match in Wisp's standalone window through the nearest relay region; the run prints the join code.
#[tauri::command]
pub fn standalone_host(app: AppHandle, state: State<AppState>) -> Result<online::OnlineState, String> {
    start_standalone(app, &state, online::Mode::Host, None)
}

#[tauri::command]
pub fn standalone_join(app: AppHandle, state: State<AppState>, code: String) -> Result<online::OnlineState, String> {
    start_standalone(app, &state, online::Mode::Join, Some(&code))
}

#[tauri::command]
pub fn standalone_cancel(state: State<AppState>) {
    state.standalone.cancel()
}

//! The Smashcraft client: a window with routed pages (Controller, History,
//! Stats, Replays and Online) and a tray light. Controller support comes
//! from the Warcraft III Controller service over its local interface; the
//! client starts the bundled service only when none answers.

pub mod autostart;
pub mod link;
pub mod lua32;
pub mod mapsim;
pub mod mpq;
pub mod online;
pub mod play;
pub mod records;
pub mod replays;
pub mod settings;
pub mod single;
pub mod tray;
pub mod warcraft_replays;

use std::sync::{Arc, LazyLock, Mutex};

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use wc3_controller_model::{Binding, ClientMessage, InputView, Light, PadPreset, ProfileChoice};

use link::{ControllerState, Shared};
use settings::{Settings, Store};

const TRAY: &str = "status";

static PLAY: LazyLock<play::Play> = LazyLock::new(play::Play::default);

struct AppState {
    link: Arc<Shared>,
    store: Store,
    settings: Mutex<Settings>,
    history: records::History,
    online: Arc<online::Online>,
    log_dir: Option<std::path::PathBuf>,
    kept: replays::Kept,
    mapsims: mapsim::MapSims,
    warcraft: warcraft_replays::Library,
}

impl AppState {
    fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
        let mut settings = self.settings.lock().unwrap();
        change(&mut settings);
        self.store.save(&settings)
    }
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

struct TauriSink {
    app: AppHandle,
    light: Mutex<Option<Light>>,
    /// The newest pad state not yet shown; the service sends every pad report.
    input: Arc<Mutex<Option<InputView>>>,
}

/// Shows pad input at most once a frame (60 Hz), always ending on the newest state.
fn show_input(app: AppHandle, pending: Arc<Mutex<Option<InputView>>>) {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(16));
        if let Some(input) = pending.lock().unwrap().take() {
            let _ = app.emit("input", input);
        }
    }
}

impl link::Sink for TauriSink {
    fn status(&self, state: &ControllerState) {
        let _ = self.app.emit("controller", state);
        let light = state.view.overall;
        let mut last = self.light.lock().unwrap();
        if *last != Some(light) {
            *last = Some(light);
            if let Some(tray) = self.app.tray_by_id(TRAY) {
                let _ = tray.set_icon(Some(Image::new_owned(tray::light_rgba(light), tray::SIZE, tray::SIZE)));
                let _ = tray.set_tooltip(Some(tray::tooltip(light)));
            }
        }
    }

    fn input(&self, input: &InputView) {
        *self.input.lock().unwrap() = Some(*input);
    }
}

#[tauri::command]
fn controller_state(state: State<AppState>) -> ControllerState {
    state.link.state()
}

#[tauri::command]
fn turn_on_controller(state: State<AppState>) -> Result<ControllerState, String> {
    state.update(|s| s.controller_on = true)?;
    state.link.sup.lock().unwrap().turn_on();
    Ok(state.link.state())
}

#[tauri::command]
fn set_profile(state: State<AppState>, choice: ProfileChoice) -> Result<bool, String> {
    state.update(|s| s.profile = choice)?;
    state.link.greet(state.settings.lock().unwrap().greeting());
    Ok(state.link.send(&ClientMessage::Profile(choice)))
}

#[tauri::command]
fn set_pad_preset(state: State<AppState>, preset: PadPreset) -> Result<bool, String> {
    state.update(|s| s.pad_preset = preset)?;
    state.link.greet(state.settings.lock().unwrap().greeting());
    Ok(state.link.send(&ClientMessage::PadPreset(preset)))
}

#[tauri::command]
fn set_tap_jump(state: State<AppState>, on: bool) -> Result<bool, String> {
    state.update(|s| s.tap_jump = on)?;
    state.link.greet(state.settings.lock().unwrap().greeting());
    Ok(state.link.send(&ClientMessage::TapJump(on)))
}

#[derive(Serialize)]
struct PadPresetBindings {
    preset: PadPreset,
    label: &'static str,
    bindings: Vec<Binding>,
}

#[derive(Serialize)]
struct Bindings {
    smashcraft: Vec<Binding>,
    smashcraft_menus: Vec<Binding>,
    any_map: Vec<Binding>,
    any_map_defaults: Vec<Binding>,
    profile: ProfileChoice,
    pad_preset: PadPreset,
    tap_jump: bool,
    pad_presets: Vec<PadPresetBindings>,
}

#[tauri::command]
fn bindings(state: State<AppState>) -> Bindings {
    let settings = state.settings.lock().unwrap();
    Bindings {
        smashcraft: wc3_controller_model::smashcraft_bindings_for(settings.pad_preset),
        smashcraft_menus: wc3_controller_model::smashcraft_menu_bindings(),
        any_map: if settings.any_map_bindings.is_empty() {
            wc3_controller_model::any_map_bindings()
        } else {
            settings.any_map_bindings.clone()
        },
        any_map_defaults: wc3_controller_model::any_map_bindings(),
        profile: settings.profile,
        pad_preset: settings.pad_preset,
        tap_jump: settings.tap_jump,
        pad_presets: [(PadPreset::Standard, "Standard"), (PadPreset::ZJump, "Z-jump")]
            .into_iter()
            .map(|(preset, label)| PadPresetBindings {
                preset,
                label,
                bindings: wc3_controller_model::smashcraft_bindings_for(preset),
            })
            .collect(),
    }
}

#[tauri::command]
fn set_any_map_bindings(state: State<AppState>, bindings: Vec<Binding>) -> Result<bool, String> {
    state.update(|s| s.any_map_bindings = bindings.clone())?;
    Ok(state.link.send(&ClientMessage::Bindings(bindings)))
}

#[tauri::command]
fn start_with_computer(app: AppHandle) -> bool {
    autostart::is_enabled(&app)
}

#[tauri::command]
fn set_start_with_computer(app: AppHandle, on: bool) -> Result<bool, String> {
    autostart::set(&app, on)
}

fn record_folders_of(settings: &Settings) -> Vec<String> {
    settings.record_folders.clone().unwrap_or_else(records::default_folders)
}

#[tauri::command]
fn record_folders(state: State<AppState>) -> Vec<String> {
    record_folders_of(&state.settings.lock().unwrap())
}

#[tauri::command]
fn set_record_folders(state: State<AppState>, folders: Vec<String>) -> Result<Vec<String>, String> {
    state.update(|s| s.record_folders = Some(folders))?;
    Ok(record_folders_of(&state.settings.lock().unwrap()))
}

#[tauri::command]
fn read_records(state: State<AppState>) -> Vec<records::RecordFile> {
    let folders = record_folders_of(&state.settings.lock().unwrap());
    records::read_record_files(&folders)
}

#[tauri::command]
fn history(state: State<AppState>) -> serde_json::Value {
    state.history.load()
}

#[tauri::command]
fn save_history(state: State<AppState>, history: serde_json::Value) -> Result<(), String> {
    state.history.save(&history)
}

#[tauri::command]
fn read_replays(state: State<AppState>) -> Vec<replays::ReplayFile> {
    let folders = record_folders_of(&state.settings.lock().unwrap());
    replays::read_replay_files(&folders, &state.kept.replays)
}

#[tauri::command]
fn read_replay_parts(folder: String, serial: u32, parts: u32) -> Result<Vec<String>, String> {
    replays::read_parts(&folder, serial, parts)
}

#[tauri::command]
fn keep_replay(state: State<AppState>, name: String, text: String) -> Result<String, String> {
    state.kept.keep_replay(&name, &text)
}

#[tauri::command]
fn keep_sim(state: State<AppState>, version: String, code: String) -> Result<(), String> {
    state.kept.keep_sim(&version, &code)
}

#[tauri::command]
fn kept_sim(state: State<AppState>, version: String) -> Option<String> {
    state.kept.sim(&version)
}

fn maps_of(state: &AppState) -> Vec<std::path::PathBuf> {
    mapsim::maps_folders(&record_folders_of(&state.settings.lock().unwrap()))
}

/// Versions whose maps are in the Maps folders beside the record folders, or were read before.
#[tauri::command]
async fn map_versions(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.mapsims.scan(&maps_of(&state)))
}

/// Opens a joined replay in its version's map: {"id", "opened"} with the viewer driver's answer.
#[tauri::command]
async fn map_replay_open(state: State<'_, AppState>, version: String, replay: String, viewer: String) -> Result<String, String> {
    let (id, opened) = state.mapsims.open(&maps_of(&state), &version, &replay, &viewer)?;
    Ok(format!("{{\"id\":{id},\"opened\":{opened}}}"))
}

#[tauri::command]
async fn map_replay_step(state: State<'_, AppState>, id: u32, seek: bool, frames: i64) -> Result<String, String> {
    state.mapsims.step(id, seek, frames)
}

#[tauri::command]
fn map_replay_close(state: State<AppState>, id: u32) {
    state.mapsims.close(id);
}

#[tauri::command]
fn kept_sims(state: State<AppState>) -> Vec<String> {
    state.kept.sims()
}

#[tauri::command]
fn warcraft_games(state: State<AppState>) -> Vec<warcraft_replays::WarcraftGame> {
    state.warcraft.games()
}

#[tauri::command]
fn watch_in_warcraft(state: State<AppState>, file: String, name: String) -> Result<String, String> {
    state.warcraft.put_in_warcraft(&file, &name)
}

#[tauri::command]
fn play_state() -> play::PlayState {
    PLAY.state()
}

#[tauri::command]
fn play(app: AppHandle) -> Result<play::PlayState, String> {
    PLAY.start(move |state| {
        let _ = app.emit("play", state);
    })?;
    Ok(PLAY.state())
}

#[tauri::command]
fn online_state(state: State<AppState>) -> online::OnlineState {
    state.online.state()
}

/// The player's answer to adding Smashcraft's page to Warcraft III's menus: none until asked.
#[tauri::command]
fn menu_page_choice(state: State<AppState>) -> Option<bool> {
    state.settings.lock().unwrap().menu_page
}

#[tauri::command]
fn decline_menu_page(state: State<AppState>) -> Result<(), String> {
    state.update(|s| s.menu_page = Some(false))
}

fn start_online(app: AppHandle, state: &AppState, mode: online::Mode, code: Option<&str>) -> Result<online::OnlineState, String> {
    let dir = play::play_dir().ok_or("Online play isn't set up on this computer yet.")?;
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
fn online_setup(app: AppHandle, state: State<AppState>) -> Result<online::OnlineState, String> {
    state.update(|s| s.menu_page = Some(true))?;
    start_online(app, &state, online::Mode::Setup, None)
}

#[tauri::command]
fn online_host(app: AppHandle, state: State<AppState>) -> Result<online::OnlineState, String> {
    start_online(app, &state, online::Mode::Host, None)
}

#[tauri::command]
fn online_join(app: AppHandle, state: State<AppState>, code: String) -> Result<online::OnlineState, String> {
    start_online(app, &state, online::Mode::Join, Some(&code))
}

#[tauri::command]
fn online_start_now(state: State<AppState>) -> bool {
    state.online.start_now()
}

#[tauri::command]
fn online_cancel(state: State<AppState>) {
    state.online.cancel()
}

pub fn run() {
    let hidden = std::env::args().any(|arg| arg == "--hidden");
    let single::Claim::First(instance) = single::claim(single::client_port()) else {
        return;
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .setup(move |app| {
            let config = app.path().app_config_dir()?;
            let store = Store::new(&config);
            let history = records::History::new(&app.path().app_data_dir()?);
            let kept = replays::Kept::new(&app.path().app_data_dir()?);
            let mapsims = mapsim::MapSims::new(&app.path().app_data_dir()?);
            let warcraft = warcraft_replays::Library::new(&app.path().app_data_dir()?);
            let settings = store.load();
            let shared = Shared::new(settings.controller_on);
            shared.greet(settings.greeting());

            let open = MenuItem::with_id(app, "open", "Open Smashcraft", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let initial = shared.state().view.overall;
            TrayIconBuilder::with_id(TRAY)
                .icon(Image::new_owned(tray::light_rgba(initial), tray::SIZE, tray::SIZE))
                .tooltip(tray::tooltip(initial))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let log_dir = app.path().app_log_dir().ok().inspect(|dir| {
                let _ = std::fs::create_dir_all(dir);
            });
            let log = log_dir.as_ref().map(|dir| dir.join("controller.log"));
            let pending = Arc::new(Mutex::new(None));
            std::thread::spawn({
                let (app, pending) = (app.handle().clone(), pending.clone());
                move || show_input(app, pending)
            });
            let sink = Arc::new(TauriSink { app: app.handle().clone(), light: Mutex::new(Some(initial)), input: pending });
            let starter = Arc::new(link::BundledService { log });
            std::thread::spawn({
                let shared = shared.clone();
                move || link::run(shared, link::service_port(), sink, starter)
            });
            std::thread::spawn({
                let app = app.handle().clone();
                move || single::listen(instance, move || show_window(&app))
            });
            app.manage(AppState { link: shared, store, settings: Mutex::new(settings), history, online: Arc::default(), log_dir, kept, mapsims, warcraft });
            std::thread::spawn({
                let (app, library) = (app.handle().clone(), warcraft_replays::Library::new(&app.path().app_data_dir()?));
                move || warcraft_replays::watch(library, move || record_folders_of(&app.state::<AppState>().settings.lock().unwrap()))
            });
            if !hidden {
                show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            controller_state,
            turn_on_controller,
            set_profile,
            set_pad_preset,
            set_tap_jump,
            bindings,
            set_any_map_bindings,
            start_with_computer,
            set_start_with_computer,
            play_state,
            play,
            record_folders,
            set_record_folders,
            read_records,
            history,
            save_history,
            online_state,
            menu_page_choice,
            decline_menu_page,
            online_setup,
            online_host,
            online_join,
            online_start_now,
            online_cancel,
            read_replays,
            read_replay_parts,
            keep_replay,
            keep_sim,
            kept_sim,
            kept_sims,
            map_versions,
            map_replay_open,
            map_replay_step,
            map_replay_close,
            warcraft_games,
            watch_in_warcraft,
        ])
        .run(tauri::generate_context!())
        .expect("Smashcraft failed to start");
}

//! The Smashcraft client: a window with routed pages (Controller, History and
//! Stats now; replays and online later) and a tray light. Controller support comes
//! from the Warcraft III Controller service over its local interface; the
//! client starts the bundled service only when none answers.

pub mod autostart;
pub mod link;
pub mod play;
pub mod records;
pub mod settings;
pub mod single;
pub mod tray;

use std::sync::{Arc, LazyLock, Mutex};

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use wc3_controller_model::{Binding, ClientMessage, InputView, Light, ProfileChoice};

use link::{ControllerState, Shared};
use settings::{Settings, Store};

const TRAY: &str = "status";

static PLAY: LazyLock<play::Play> = LazyLock::new(play::Play::default);

struct AppState {
    link: Arc<Shared>,
    store: Store,
    settings: Mutex<Settings>,
    history: records::History,
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
    state.link.greet(vec![ClientMessage::Profile(choice)]);
    Ok(state.link.send(&ClientMessage::Profile(choice)))
}

#[derive(Serialize)]
struct Bindings {
    smashcraft: Vec<Binding>,
    smashcraft_menus: Vec<Binding>,
    any_map: Vec<Binding>,
    any_map_defaults: Vec<Binding>,
    profile: ProfileChoice,
}

#[tauri::command]
fn bindings(state: State<AppState>) -> Bindings {
    let settings = state.settings.lock().unwrap();
    Bindings {
        smashcraft: wc3_controller_model::smashcraft_bindings(),
        smashcraft_menus: wc3_controller_model::smashcraft_menu_bindings(),
        any_map: if settings.any_map_bindings.is_empty() {
            wc3_controller_model::any_map_bindings()
        } else {
            settings.any_map_bindings.clone()
        },
        any_map_defaults: wc3_controller_model::any_map_bindings(),
        profile: settings.profile,
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
            let settings = store.load();
            let shared = Shared::new(settings.controller_on);
            shared.greet(vec![ClientMessage::Profile(settings.profile)]);

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

            let log = app.path().app_log_dir().ok().map(|dir| {
                let _ = std::fs::create_dir_all(&dir);
                dir.join("controller.log")
            });
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
            app.manage(AppState { link: shared, store, settings: Mutex::new(settings), history });
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
        ])
        .run(tauri::generate_context!())
        .expect("Smashcraft failed to start");
}

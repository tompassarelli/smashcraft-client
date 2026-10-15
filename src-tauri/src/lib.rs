//! The Smashcraft client: a window with routed pages (Controller, History,
//! Stats, Replays and Online) and a tray light. Controller support comes
//! from the Warcraft III Controller service over its local interface; the
//! client starts the bundled service only when none answers.

pub mod autostart;
pub mod cli;
pub mod commands;
pub mod instance_lock;
pub mod link;
pub mod lua32;
pub mod mapsim;
pub mod mpq;
pub mod online;
pub mod play;
pub mod records;
pub mod replays;
pub mod settings;
pub mod sink;
pub mod state;
pub mod tray;
pub mod warcraft_replays;

use std::sync::{Arc, Mutex};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use commands::{controller, history, online as online_page, play as play_page, records as records_page, replays as replays_page};
use link::Shared;
use settings::Store;
use sink::{show_input, TauriSink, TRAY};
use state::{record_folders_of, show_window, AppState};

pub fn run() {
    if std::env::args().any(|arg| arg == "--check") {
        return cli::check();
    }
    let hidden = std::env::args().any(|arg| arg == "--hidden");
    let instance_lock::Claim::First(instance) = instance_lock::claim(instance_lock::client_port()) else {
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
                move || instance_lock::listen(instance, move || show_window(&app))
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
            controller::controller_state,
            controller::turn_on_controller,
            controller::set_profile,
            controller::set_pad_preset,
            controller::set_tap_jump,
            controller::set_trigger_shields,
            controller::set_right_stick,
            controller::bindings,
            controller::start_with_computer,
            controller::set_start_with_computer,
            play_page::play_state,
            play_page::play,
            records_page::record_folders,
            records_page::set_record_folders,
            records_page::read_records,
            history::history,
            history::save_history,
            online_page::online_state,
            online_page::menu_page_choice,
            online_page::decline_menu_page,
            online_page::online_setup,
            online_page::online_host,
            online_page::online_join,
            online_page::online_start_now,
            online_page::online_cancel,
            replays_page::read_replays,
            replays_page::read_replay_parts,
            replays_page::keep_replay,
            replays_page::keep_sim,
            replays_page::kept_sim,
            replays_page::kept_sims,
            replays_page::map_versions,
            replays_page::map_replay_open,
            replays_page::map_replay_step,
            replays_page::map_replay_close,
            replays_page::warcraft_games,
            replays_page::watch_in_warcraft,
        ])
        .run(tauri::generate_context!())
        .expect("Smashcraft failed to start");
}

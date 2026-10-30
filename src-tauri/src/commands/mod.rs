pub mod controller;
pub mod history;
pub mod online;
pub mod play;
pub mod records;
pub mod replays;

/// Every command the UI invokes, grouped by page.
pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
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
        play::play_state,
        play::play,
        records::record_folders,
        records::set_record_folders,
        records::read_records,
        history::history,
        history::save_history,
        online::online_state,
        online::menu_page_choice,
        online::decline_menu_page,
        online::online_setup,
        online::online_host,
        online::online_join,
        online::online_start_now,
        online::online_cancel,
        online::smashcraft_servers_choice,
        online::opt_in_smashcraft_servers,
        online::relay_state,
        online::relay_host,
        online::relay_join,
        online::relay_cancel,
        online::standalone_state,
        online::standalone_host,
        online::standalone_join,
        online::standalone_cancel,
        replays::read_replays,
        replays::read_replay_parts,
        replays::keep_replay,
        replays::keep_sim,
        replays::kept_sim,
        replays::kept_sims,
        replays::map_versions,
        replays::map_replay_open,
        replays::map_replay_step,
        replays::map_replay_close,
        replays::warcraft_games,
        replays::watch_in_warcraft,
    ]
}

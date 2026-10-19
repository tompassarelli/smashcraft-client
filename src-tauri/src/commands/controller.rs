use serde::Serialize;
use tauri::{AppHandle, State};
use padbridge_model::{Binding, ClientMessage, PadKind, PadPreset, ProfileChoice, RightStick, TriggerShields};

use crate::autostart;
use crate::link::ControllerState;
use crate::state::AppState;

#[tauri::command]
pub fn controller_state(state: State<AppState>) -> ControllerState {
    state.link.state()
}

#[tauri::command]
pub fn turn_on_controller(state: State<AppState>) -> Result<ControllerState, String> {
    state.update(|s| s.controller_on = true)?;
    state.link.sup.lock().unwrap().turn_on();
    Ok(state.link.state())
}

#[tauri::command]
pub fn set_profile(state: State<AppState>, choice: ProfileChoice) -> Result<bool, String> {
    state.update(|s| s.profile = choice)?;
    state.link.greet(state.settings.lock().unwrap().greeting());
    Ok(state.link.send(&ClientMessage::Profile(choice)))
}

#[tauri::command]
pub fn set_pad_preset(state: State<AppState>, preset: PadPreset) -> Result<bool, String> {
    Ok(state.link.send(&ClientMessage::PadPreset(preset)))
}

#[tauri::command]
pub fn set_tap_jump(state: State<AppState>, on: bool) -> Result<bool, String> {
    Ok(state.link.send(&ClientMessage::TapJump(on)))
}

#[tauri::command]
pub fn set_trigger_shields(state: State<AppState>, triggers: TriggerShields) -> bool {
    state.link.send(&ClientMessage::TriggerShields(triggers))
}

#[tauri::command]
pub fn set_right_stick(state: State<AppState>, mode: RightStick) -> bool {
    state.link.send(&ClientMessage::RightStick(mode))
}

#[derive(Serialize)]
pub struct PadPresetBindings {
    preset: PadPreset,
    label: &'static str,
    bindings: Vec<Binding>,
}

#[derive(Serialize)]
pub struct Bindings {
    smashcraft: Vec<Binding>,
    smashcraft_menus: Vec<Binding>,
    profile: ProfileChoice,
    pad_preset: PadPreset,
    tap_jump: bool,
    triggers: TriggerShields,
    right_stick: RightStick,
    pad_presets: Vec<PadPresetBindings>,
}

#[tauri::command]
pub fn bindings(state: State<AppState>) -> Bindings {
    let settings = state.settings.lock().unwrap();
    let controller = state.link.snapshot.lock().unwrap().settings.clone();
    Bindings {
        smashcraft: controller.bindings(PadKind::default()),
        smashcraft_menus: padbridge_model::menu_pointer_bindings(),
        profile: settings.profile,
        pad_preset: controller.pad_preset,
        tap_jump: controller.tap_jump,
        triggers: controller.triggers,
        right_stick: controller.right_stick,
        pad_presets: [(PadPreset::Melee, "Melee"), (PadPreset::ZJump, "Z-jump"), (PadPreset::Tom, "Tom")]
            .into_iter()
            .map(|(preset, label)| PadPresetBindings {
                preset,
                label,
                bindings: padbridge_model::pad_bindings(preset, controller.triggers, controller.right_stick, PadKind::default()),
            })
            .collect(),
    }
}

#[tauri::command]
pub fn start_with_computer(app: AppHandle) -> bool {
    autostart::is_enabled(&app)
}

#[tauri::command]
pub fn set_start_with_computer(app: AppHandle, on: bool) -> Result<bool, String> {
    autostart::set(&app, on)
}

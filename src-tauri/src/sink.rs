use std::sync::{Arc, Mutex};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter};
use padbridge_model::{InputView, Light};

use crate::link::{self, ControllerState};
use crate::state::show_window;
use crate::tray;

pub const TRAY: &str = "status";

/// The tray light, starting at `initial`, with Open and Quit; a left click opens the window.
pub fn add_tray(app: &tauri::App, initial: Light) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Smashcraft", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
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
    Ok(())
}

pub struct TauriSink {
    pub app: AppHandle,
    pub light: Mutex<Option<Light>>,
    /// The newest pad state not yet shown; the service sends every pad report.
    pub input: Arc<Mutex<Option<InputView>>>,
}

/// Shows pad input at most once a frame (60 Hz), always ending on the newest state.
pub fn show_input(app: AppHandle, pending: Arc<Mutex<Option<InputView>>>) {
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

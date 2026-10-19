use std::sync::{Arc, Mutex};

use tauri::image::Image;
use tauri::{AppHandle, Emitter};
use padbridge_model::{InputView, Light};

use crate::link::{self, ControllerState};
use crate::tray;

pub const TRAY: &str = "status";

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

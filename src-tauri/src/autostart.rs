//! "Start with my computer". Linux writes its own XDG autostart entry so it can
//! name the installed launcher (`SMASHCRAFT_LAUNCHER`), which survives updates;
//! Windows and macOS use the autostart plugin.

use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// What login should start: the launcher when one started us, else this executable.
pub fn launcher() -> PathBuf {
    std::env::var_os("SMASHCRAFT_LAUNCHER")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| "smashcraft".into())
}

pub fn desktop_entry(exec: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Smashcraft\nComment=Controller support for Warcraft III, waiting in the tray\nExec=\"{}\" --hidden\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exec.display()
    )
}

#[cfg(target_os = "linux")]
fn entry_path() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(config.join("autostart/smashcraft.desktop"))
}

#[cfg(target_os = "linux")]
pub fn is_enabled(_app: &AppHandle) -> bool {
    entry_path().is_some_and(|path| path.exists())
}

#[cfg(target_os = "linux")]
pub fn set(_app: &AppHandle, on: bool) -> Result<bool, String> {
    let path = entry_path().ok_or("no home folder")?;
    if on {
        std::fs::create_dir_all(path.parent().expect("autostart folder")).map_err(|e| e.to_string())?;
        std::fs::write(&path, desktop_entry(&launcher())).map_err(|e| e.to_string())?;
    } else if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(path.exists())
}

#[cfg(not(target_os = "linux"))]
pub fn is_enabled(app: &AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
pub fn set(app: &AppHandle, on: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if on { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    launcher.is_enabled().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_starts_the_launcher_hidden_in_the_tray() {
        let entry = desktop_entry(Path::new("/home/p/.local/share/smashcraft-build-inputs/smashcraft-client/smashcraft"));
        assert!(entry.contains("Exec=\"/home/p/.local/share/smashcraft-build-inputs/smashcraft-client/smashcraft\" --hidden\n"));
        assert!(entry.starts_with("[Desktop Entry]\nType=Application\n"));
    }
}

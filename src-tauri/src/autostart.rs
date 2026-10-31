//! "Start with my computer". Linux writes its own XDG autostart entry so it can
//! name the installed launcher (`SMASHCRAFT_LAUNCHER`), which survives updates;
//! Windows and macOS use the autostart plugin.

#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use crate::env::Env;

/// What login should start: the launcher when one started us, else this executable.
#[cfg(target_os = "linux")]
pub fn launcher(env: &Env) -> PathBuf {
    env.launcher.clone().or_else(|| env.current_exe.clone()).unwrap_or_else(|| "smashcraft".into())
}

#[cfg(target_os = "linux")]
pub fn desktop_entry(exec: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Smashcraft\nComment=Controller support for Warcraft III, waiting in the tray\nExec=\"{}\" --hidden\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        exec.display()
    )
}

#[cfg(target_os = "linux")]
fn entry_path(env: &Env) -> Option<PathBuf> {
    let config = env.xdg_config_home.clone().or_else(|| env.home.as_ref().map(|home| home.join(".config")))?;
    Some(config.join("autostart/smashcraft.desktop"))
}

#[cfg(target_os = "linux")]
pub fn is_enabled(_app: &AppHandle, env: &Env) -> bool {
    entry_path(env).is_some_and(|path| path.exists())
}

#[cfg(target_os = "linux")]
pub fn set(_app: &AppHandle, env: &Env, on: bool) -> Result<bool, String> {
    let path = entry_path(env).ok_or("no home folder")?;
    if on {
        std::fs::create_dir_all(path.parent().expect("autostart folder")).map_err(|e| e.to_string())?;
        std::fs::write(&path, desktop_entry(&launcher(env))).map_err(|e| e.to_string())?;
    } else if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(path.exists())
}

#[cfg(not(target_os = "linux"))]
pub fn is_enabled(app: &AppHandle, _env: &Env) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
pub fn set(app: &AppHandle, _env: &Env, on: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if on { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    launcher.is_enabled().map_err(|e| e.to_string())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn the_entry_follows_the_xdg_desktop_entry_spec_and_starts_the_launcher_hidden_in_the_tray() {
        let entry = desktop_entry(Path::new("/home/p/.local/share/smashcraft-build-inputs/smashcraft-client/smashcraft"));
        assert!(entry.contains("Exec=\"/home/p/.local/share/smashcraft-build-inputs/smashcraft-client/smashcraft\" --hidden\n"));
        assert!(entry.starts_with("[Desktop Entry]\nType=Application\n"));
    }
}

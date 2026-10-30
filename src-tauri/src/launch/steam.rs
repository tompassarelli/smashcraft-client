//! Warcraft III's Battle.net shortcut in Steam: where Linux players keep their Warcraft III under Proton.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const SHORTCUT: &str = "Warcraft III (Battle.net)";

const APP_ID_KEY: &[u8] = b"\x02appid\0";
const NAME_KEY: &[u8] = b"\x01AppName\0";

/// The app id Steam gave the shortcut named `name` in its binary `shortcuts.vdf`.
pub fn shortcut_app_id(vdf: &[u8], name: &str) -> Option<u32> {
    let starts: Vec<usize> = vdf.windows(APP_ID_KEY.len()).enumerate().filter(|(_, w)| *w == APP_ID_KEY).map(|(at, _)| at).collect();
    let wanted = [NAME_KEY, name.as_bytes(), b"\0"].concat();
    starts.iter().enumerate().find_map(|(i, &at)| {
        let entry = &vdf[at..starts.get(i + 1).copied().unwrap_or(vdf.len())];
        let id = entry.get(APP_ID_KEY.len()..APP_ID_KEY.len() + 4)?;
        entry.windows(wanted.len()).any(|w| w == wanted.as_slice()).then(|| u32::from_le_bytes(id.try_into().unwrap()))
    })
}

/// Steam's URL that starts a non-Steam shortcut: its app id shifted into the high 32 bits, with 0x02000000 below.
pub fn rungame_url(app_id: u32) -> String {
    format!("steam://rungameid/{}", (u64::from(app_id) << 32) | 0x0200_0000)
}

fn shortcut_files() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return Vec::new() };
    [".local/share/Steam", ".steam/steam"]
        .iter()
        .filter_map(|root| std::fs::read_dir(home.join(root).join("userdata")).ok())
        .flat_map(|users| users.flatten())
        .map(|user| user.path().join("config/shortcuts.vdf"))
        .filter(|file| file.is_file())
        .collect()
}

fn find_app_id(files: &[PathBuf]) -> Option<u32> {
    files.iter().find_map(|file| shortcut_app_id(&std::fs::read(file).ok()?, SHORTCUT))
}

/// Asks Steam to start the Battle.net shortcut.
pub fn start_shortcut() -> Result<(), String> {
    let files = shortcut_files();
    let id = find_app_id(&files).ok_or_else(|| format!("Steam has no \"{SHORTCUT}\" shortcut to start"))?;
    let steam = std::env::var_os("STEAM").unwrap_or_else(|| "steam".into());
    let mut child = Command::new(&steam)
        .arg(rungame_url(id))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{}: {e}", Path::new(&steam).display()))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u32, app_id: u32, name: &str) -> Vec<u8> {
        [format!("\0{index}\0").as_bytes(), APP_ID_KEY, &app_id.to_le_bytes(), NAME_KEY, name.as_bytes(), b"\0\x01Exe\0/bin/true\0\x08"].concat()
    }

    /// Steam's own formula for a non-Steam shortcut, as `wisp play` declares it: app id 3775098022 for this shortcut (wisp:docs/play.md, "Declare it").
    #[test]
    fn the_shortcut_is_found_by_name_among_others_and_started_by_steams_rungameid_url() {
        let vdf = [b"\0shortcuts\0".as_slice(), &entry(0, 11, "Other"), &entry(1, 3_775_098_022, SHORTCUT), b"\x08\x08"].concat();
        assert_eq!(shortcut_app_id(&vdf, SHORTCUT), Some(3_775_098_022));
        assert_eq!(shortcut_app_id(&vdf, "Missing"), None);
        assert_eq!(rungame_url(3_775_098_022), "steam://rungameid/16213922543717842944");
    }
}

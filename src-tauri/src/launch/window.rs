//! Brings the player's running Warcraft III window to the front.

const TITLE: &str = "Warcraft III";

/// The id of the window titled `title` in `niri msg --json windows`, which is a list or an object keyed by id.
#[cfg(unix)]
pub fn niri_window_id(windows: &str, title: &str) -> Option<u64> {
    let all: serde_json::Value = serde_json::from_str(windows).ok()?;
    let mut list: Vec<&serde_json::Value> = match &all {
        serde_json::Value::Array(list) => list.iter().collect(),
        serde_json::Value::Object(map) => map.values().collect(),
        _ => return None,
    };
    list.retain(|window| window["title"] == title);
    list.first().and_then(|window| window["id"].as_u64())
}

#[cfg(unix)]
fn niri(args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("niri").arg("msg").args(args).output().map_err(|e| format!("niri: {e}"))?;
    if !output.status.success() {
        return Err(format!("niri {}: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(unix)]
pub fn raise_warcraft() -> Result<(), String> {
    let id = niri_window_id(&niri(&["--json", "windows"])?, TITLE).ok_or("there is no Warcraft III window")?;
    niri(&["action", "focus-window", "--id", &id.to_string()]).map(drop)
}

#[cfg(windows)]
pub fn raise_warcraft() -> Result<(), String> {
    let script = format!("if (-not (New-Object -ComObject WScript.Shell).AppActivate('{TITLE}')) {{ exit 1 }}");
    let status = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).status().map_err(|e| format!("powershell: {e}"))?;
    if status.success() { Ok(()) } else { Err("there is no Warcraft III window".to_owned()) }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The shape `niri msg --json windows` prints (niri 25 and later list windows in an array).
    #[test]
    fn the_warcraft_window_is_picked_by_title_from_niris_window_list() {
        let list = r#"[{"id":4,"title":"Terminal","app_id":"foot"},{"id":9,"title":"Warcraft III","app_id":"steam_app_0"}]"#;
        assert_eq!(niri_window_id(list, "Warcraft III"), Some(9));
        assert_eq!(niri_window_id(r#"{"4":{"id":4,"title":"Terminal"}}"#, "Warcraft III"), None);
        assert_eq!(niri_window_id("not json", "Warcraft III"), None);
    }
}

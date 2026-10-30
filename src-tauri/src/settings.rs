//! What the client remembers between runs, as JSON in its config folder.

use crate::files;
use padbridge_model::{ClientMessage, ProfileChoice};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The player turned controller support on.
    pub controller_on: bool,
    pub profile: ProfileChoice,
    /// The CustomMapData folders match records are read from; the ones found on this computer until the player sets them.
    #[serde(alias = "record_folders")]
    pub custom_map_data_folders: Option<Vec<String>>,
}

impl Settings {
    pub fn reconnect_messages(&self) -> Vec<ClientMessage> {
        vec![ClientMessage::Profile(self.profile)]
    }
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("settings.json") }
    }

    /// Missing or unreadable settings start from the defaults.
    pub fn load(&self) -> Settings {
        std::fs::read_to_string(&self.path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        files::write_atomic(&self.path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_default_when_missing() {
        let dir = std::env::temp_dir().join(format!("smashcraft-settings-{}", std::process::id()));
        let store = Store::new(&dir);
        assert_eq!(store.load(), Settings::default());
        let settings = Settings { controller_on: true, profile: ProfileChoice::AnyMap, custom_map_data_folders: Some(vec!["/games/CustomMapData".into()]) };
        assert!(matches!(settings.reconnect_messages().as_slice(), [ClientMessage::Profile(ProfileChoice::AnyMap)]));
        store.save(&settings).unwrap();
        assert_eq!(store.load(), settings);
        let before_the_rename: Settings = serde_json::from_str(r#"{"controller_on":false,"record_folders":["/games/CustomMapData"]}"#).unwrap();
        assert_eq!(before_the_rename.custom_map_data_folders, Some(vec!["/games/CustomMapData".into()]));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

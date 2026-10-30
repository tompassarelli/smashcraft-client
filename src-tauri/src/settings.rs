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
    pub record_folders: Option<Vec<String>>,
    /// Whether the player agreed to add Smashcraft's page to Warcraft III's menus (online play); none until asked.
    pub menu_page: Option<bool>,
    /// Whether the player opted in to Smashcraft Servers; none until they choose.
    pub smashcraft_servers: Option<bool>,
}

impl Settings {
    pub fn greeting(&self) -> Vec<ClientMessage> {
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
        let settings = Settings {
            controller_on: true,
            profile: ProfileChoice::AnyMap,
            record_folders: Some(vec!["/games/CustomMapData".into()]),
            menu_page: Some(true),
            smashcraft_servers: Some(true),
        };
        assert!(matches!(settings.greeting().as_slice(), [ClientMessage::Profile(ProfileChoice::AnyMap)]));
        store.save(&settings).unwrap();
        assert_eq!(store.load(), settings);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

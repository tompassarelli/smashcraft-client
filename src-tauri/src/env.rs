//! The process environment the client reads, once, when it starts; everything
//! else takes what it needs from an `Env` it is given, and tests pass a fake.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// The variable that names Smashcraft's tools folder (its ts folder).
pub const TOOLS_VARIABLE: &str = "SMASHCRAFT_TS";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    pub home: Option<PathBuf>,
    pub user_profile: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub wine_prefix: Option<PathBuf>,
    pub current_exe: Option<PathBuf>,
    /// Smashcraft's tools folder.
    pub tools: Option<PathBuf>,
    /// `SMASHCRAFT_LAUNCHER`: the installed launcher that survives updates.
    pub launcher: Option<PathBuf>,
    /// `WC3_CONTROLLER_SERVICE`: the service program to start.
    pub controller_service: Option<PathBuf>,
    pub controller_port: Option<u16>,
    pub client_port: Option<u16>,
    pub warcraft_iii: Option<OsString>,
    pub bun: Option<OsString>,
    pub steam: Option<OsString>,
}

impl Env {
    pub fn read() -> Self {
        Self::from_vars(|name| std::env::var_os(name), std::env::current_exe().ok())
    }

    pub fn from_vars(var: impl Fn(&str) -> Option<OsString>, current_exe: Option<PathBuf>) -> Self {
        let path = |name| var(name).map(PathBuf::from);
        let port = |name| var(name).and_then(|text| text.to_str()?.parse().ok());
        Self {
            home: path("HOME"),
            user_profile: path("USERPROFILE"),
            xdg_config_home: path("XDG_CONFIG_HOME"),
            wine_prefix: path("WINEPREFIX"),
            current_exe,
            tools: path(TOOLS_VARIABLE),
            launcher: path("SMASHCRAFT_LAUNCHER"),
            controller_service: path("WC3_CONTROLLER_SERVICE"),
            controller_port: port("WC3_CONTROLLER_PORT"),
            client_port: port("SMASHCRAFT_CLIENT_PORT"),
            warcraft_iii: var("WARCRAFT_III"),
            bun: var("BUN"),
            steam: var("STEAM"),
        }
    }

    /// The folder holding the running executable's neighbours.
    pub fn exe_dir(&self) -> Option<PathBuf> {
        self.current_exe.as_deref().and_then(|exe| exe.parent()).map(PathBuf::from)
    }

    pub fn bun_program(&self) -> OsString {
        self.bun.clone().unwrap_or_else(|| OsStr::new("bun").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_each_variable_once_and_ports_only_when_they_parse() {
        let vars =
            [("HOME", "/h"), ("USERPROFILE", "C:/u"), ("SMASHCRAFT_TS", "/ts"), ("WC3_CONTROLLER_PORT", "4000"), ("SMASHCRAFT_CLIENT_PORT", "not a port"), ("STEAM", "/bin/steam")];
        let env = Env::from_vars(|name| vars.iter().find(|(key, _)| *key == name).map(|(_, value)| OsString::from(value)), Some("/opt/smashcraft/client".into()));
        assert_eq!(env.home.as_deref(), Some("/h".as_ref()));
        assert_eq!(env.user_profile.as_deref(), Some("C:/u".as_ref()));
        assert_eq!(env.tools.as_deref(), Some("/ts".as_ref()));
        assert_eq!((env.controller_port, env.client_port), (Some(4000), None));
        assert_eq!(env.steam.as_deref(), Some("/bin/steam".as_ref()));
        assert_eq!(env.exe_dir().as_deref(), Some("/opt/smashcraft".as_ref()));
        assert_eq!((env.bun_program(), env.xdg_config_home, env.warcraft_iii), ("bun".into(), None, None));
    }
}

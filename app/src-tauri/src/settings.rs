use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

pub const SETTINGS_FILE: &str = "settings.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    pub user_dir: Option<PathBuf>,
    pub game_dir: Option<PathBuf>,
    pub analysis_lang: Option<String>,
    pub ui_lang: Option<String>,
    pub show_vanilla: bool,
    pub check_updates: bool,
    pub launcher_process_names: Option<Vec<String>>,
    pub update_checked_at: Option<i64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            user_dir: None,
            game_dir: None,
            analysis_lang: None,
            ui_lang: None,
            show_vanilla: false,
            check_updates: true,
            launcher_process_names: None,
            update_checked_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsFile {
    pub settings: Settings,
    pub error: Option<String>,
}

/// Missing file: defaults without an error. Unparseable file: defaults plus the parse error.
pub fn load_settings(dir: &Path) -> SettingsFile {
    let path = dir.join(SETTINGS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return SettingsFile {
                settings: Settings::default(),
                error: None,
            };
        }
        Err(e) => {
            return SettingsFile {
                settings: Settings::default(),
                error: Some(e.to_string()),
            };
        }
    };
    match toml::from_str(&text) {
        Ok(settings) => SettingsFile {
            settings,
            error: None,
        },
        Err(e) => SettingsFile {
            settings: Settings::default(),
            error: Some(e.to_string()),
        },
    }
}

/// An existing file that does not parse is kept as `settings.toml.bak` before being replaced.
pub fn save_settings(dir: &Path, s: &Settings) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(SETTINGS_FILE);
    if let Ok(old) = std::fs::read_to_string(&path)
        && toml::from_str::<Settings>(&old).is_err()
    {
        std::fs::rename(&path, dir.join("settings.toml.bak"))?;
    }
    let text = toml::to_string_pretty(s).map_err(std::io::Error::other)?;
    let tmp = dir.join("settings.toml.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)
}

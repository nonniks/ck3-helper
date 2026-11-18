use crate::dto::{PlaysetRow, SetupView};
use crate::error::{AppError, ErrorCode};
use crate::settings::{Settings, SettingsFile, load_settings, save_settings};
use ck3h_core::launcher::LAUNCHER_DB_FILE;
use ck3h_core::paths::{default_game_dir, default_user_dir, game_language};
use ck3h_core::session::launcher_db;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    config_dir: PathBuf,
    settings: Mutex<Settings>,
    settings_error: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(config_dir: PathBuf) -> Self {
        let SettingsFile { settings, error } = load_settings(&config_dir);
        Self {
            config_dir,
            settings: Mutex::new(settings),
            settings_error: Mutex::new(error),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The configured user folder, else the platform default.
    pub fn user_dir(&self) -> Result<PathBuf, AppError> {
        self.settings()
            .user_dir
            .or_else(default_user_dir)
            .ok_or_else(|| AppError::new(ErrorCode::NoUserDir))
    }

    pub fn setup(&self) -> SetupView {
        let settings = self.settings();
        let user_dir = settings.user_dir.clone().or_else(default_user_dir);
        let game_dir = settings.game_dir.clone().or_else(default_game_dir);
        SetupView {
            launcher_db_found: user_dir
                .as_ref()
                .is_some_and(|d| d.join(LAUNCHER_DB_FILE).is_file()),
            game_language: user_dir.as_deref().and_then(game_language),
            user_dir: user_dir.map(|p| p.display().to_string()),
            game_dir: game_dir.map(|p| p.display().to_string()),
            settings_error: self
                .settings_error
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            settings,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn save_settings(&self, s: Settings) -> Result<SetupView, AppError> {
        save_settings(&self.config_dir, &s).map_err(|e| AppError {
            code: ErrorCode::Io,
            items: Vec::new(),
            detail: e.to_string(),
        })?;
        *self.settings.lock().unwrap_or_else(|e| e.into_inner()) = s;
        *self
            .settings_error
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        Ok(self.setup())
    }

    pub fn list_playsets(&self) -> Result<Vec<PlaysetRow>, AppError> {
        let (_, db) = launcher_db(&self.user_dir()?)?;
        db.playsets()?
            .into_iter()
            .map(|p| {
                let mods = db.playset_mods(&p.id)?;
                Ok(PlaysetRow {
                    enabled: mods.iter().filter(|m| m.enabled).count(),
                    mods: mods.len(),
                    id: p.id,
                    name: p.name,
                    active: p.is_active,
                })
            })
            .collect::<Result<_, ck3h_core::error::CoreError>>()
            .map_err(AppError::from)
    }
}

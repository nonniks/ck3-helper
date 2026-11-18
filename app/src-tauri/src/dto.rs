use crate::settings::Settings;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetupView {
    pub settings: Settings,
    pub settings_error: Option<String>,
    pub user_dir: Option<String>,
    pub game_dir: Option<String>,
    pub game_language: Option<String>,
    pub launcher_db_found: bool,
    pub app_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaysetRow {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub mods: usize,
    pub enabled: usize,
}

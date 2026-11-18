pub mod analysis;
pub mod commands;
pub mod dto;
pub mod error;
pub mod settings;
pub mod state;

use ck3h_core::paths::config_dir;
use state::AppState;
use std::path::PathBuf;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new(
            config_dir().unwrap_or_else(|| PathBuf::from("ck3-helper")),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_setup,
            commands::save_settings,
            commands::list_playsets,
            commands::analyze,
            commands::set_vanilla,
            commands::summary,
            commands::pair_conflicts,
            commands::search_conflicts,
            commands::mod_warnings
        ])
        .run(tauri::generate_context!())
        .expect("error while running CK3 Helper");
}

pub mod analysis;
pub mod commands;
pub mod dto;
pub mod error;
pub mod order;
pub mod rules_file;
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
            commands::mod_warnings,
            commands::rules,
            commands::add_rule,
            commands::mark_intended,
            commands::remove_rule,
            commands::propose,
            commands::launcher_status,
            commands::apply_preview,
            commands::apply
        ])
        .run(tauri::generate_context!())
        .expect("error while running CK3 Helper");
}

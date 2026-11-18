use crate::dto::{PlaysetRow, SetupView};
use crate::error::AppError;
use crate::settings::Settings;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_setup(state: State<'_, AppState>) -> SetupView {
    state.setup()
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SetupView, AppError> {
    state.save_settings(settings)
}

/// `async` so Tauri runs it on its worker pool, not the main thread; the SQLite read is brief.
#[tauri::command]
pub async fn list_playsets(state: State<'_, AppState>) -> Result<Vec<PlaysetRow>, AppError> {
    state.list_playsets()
}

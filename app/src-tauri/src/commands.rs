use crate::dto::{
    AnalysisView, ConflictQuery, ConflictView, IndexWarningView, Page, PlaysetRow, Progress,
    SetupView,
};
use crate::error::{AppError, ErrorCode};
use crate::settings::Settings;
use crate::state::AppState;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

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

/// Runs `f` on the blocking pool; `State<'_>` is not `'static`, so `f` takes the state from
/// its own `AppHandle`.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError {
            code: ErrorCode::Io,
            items: Vec::new(),
            detail: e.to_string(),
        })?
}

#[tauri::command]
pub async fn analyze(
    app: AppHandle,
    playset_id: Option<String>,
    on_progress: Channel<Progress>,
) -> Result<AnalysisView, AppError> {
    blocking(move || {
        app.state::<AppState>()
            .analyze(playset_id.as_deref(), &|p| {
                // A closed window must not stop the analysis.
                let _ = on_progress.send(p);
            })
    })
    .await
}

#[tauri::command]
pub async fn set_vanilla(
    app: AppHandle,
    on: bool,
    on_progress: Channel<Progress>,
) -> Result<AnalysisView, AppError> {
    blocking(move || {
        app.state::<AppState>().set_vanilla(on, &|p| {
            let _ = on_progress.send(p);
        })
    })
    .await
}

/// The light commands are `async` too: off the main thread, they may wait for a running analysis.
#[tauri::command]
pub async fn summary(state: State<'_, AppState>) -> Result<AnalysisView, AppError> {
    state.summary()
}

#[tauri::command]
pub async fn pair_conflicts(
    state: State<'_, AppState>,
    winner_id: Option<String>,
    loser_id: Option<String>,
    offset: usize,
    limit: usize,
) -> Result<Page<ConflictView>, AppError> {
    state.pair_conflicts(winner_id.as_deref(), loser_id.as_deref(), offset, limit)
}

#[tauri::command]
pub async fn search_conflicts(
    state: State<'_, AppState>,
    query: ConflictQuery,
    offset: usize,
    limit: usize,
) -> Result<Page<ConflictView>, AppError> {
    state.search_conflicts(&query, offset, limit)
}

#[tauri::command]
pub async fn mod_warnings(
    state: State<'_, AppState>,
    mod_id: String,
) -> Result<Vec<IndexWarningView>, AppError> {
    state.mod_warnings(&mod_id)
}

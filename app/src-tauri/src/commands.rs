use crate::dto::{
    AnalysisView, ApplyPreviewView, ApplyView, CalibrationView, ConflictQuery, ConflictView,
    IndexWarningView, LauncherStatus, Page, PlaysetRow, Progress, ProposalView, RulesView,
    SetupView, UpdateView,
};
use crate::error::{AppError, ErrorCode};
use crate::settings::Settings;
use crate::state::AppState;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

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
pub async fn get_setup(app: AppHandle) -> Result<SetupView, AppError> {
    blocking(move || Ok(app.state::<AppState>().setup())).await
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> Result<SetupView, AppError> {
    blocking(move || app.state::<AppState>().save_settings(settings)).await
}

#[tauri::command]
pub async fn list_playsets(app: AppHandle) -> Result<Vec<PlaysetRow>, AppError> {
    blocking(move || app.state::<AppState>().list_playsets()).await
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

/// The light commands run on the blocking pool too: they may wait for a running analysis.
#[tauri::command]
pub async fn summary(app: AppHandle) -> Result<AnalysisView, AppError> {
    blocking(move || app.state::<AppState>().summary()).await
}

#[tauri::command]
pub async fn pair_conflicts(
    app: AppHandle,
    winner_id: Option<String>,
    loser_id: Option<String>,
    offset: usize,
    limit: usize,
) -> Result<Page<ConflictView>, AppError> {
    blocking(move || {
        app.state::<AppState>().pair_conflicts(
            winner_id.as_deref(),
            loser_id.as_deref(),
            offset,
            limit,
        )
    })
    .await
}

#[tauri::command]
pub async fn search_conflicts(
    app: AppHandle,
    query: ConflictQuery,
    offset: usize,
    limit: usize,
) -> Result<Page<ConflictView>, AppError> {
    blocking(move || {
        app.state::<AppState>()
            .search_conflicts(&query, offset, limit)
    })
    .await
}

#[tauri::command]
pub async fn mod_warnings(
    app: AppHandle,
    mod_id: String,
) -> Result<Vec<IndexWarningView>, AppError> {
    blocking(move || app.state::<AppState>().mod_warnings(&mod_id)).await
}

#[tauri::command]
pub async fn rules(app: AppHandle) -> Result<RulesView, AppError> {
    blocking(move || Ok(app.state::<AppState>().rules())).await
}

/// The editing commands recompute the report, so they run on the blocking pool like `analyze`.
#[tauri::command]
pub async fn add_rule(
    app: AppHandle,
    before: String,
    after: String,
    note: Option<String>,
) -> Result<RulesView, AppError> {
    blocking(move || app.state::<AppState>().add_rule(before, after, note)).await
}

#[tauri::command]
pub async fn mark_intended(
    app: AppHandle,
    winner_id: String,
    loser_id: String,
    note: String,
) -> Result<RulesView, AppError> {
    blocking(move || {
        app.state::<AppState>()
            .mark_intended(&winner_id, &loser_id, note)
    })
    .await
}

#[tauri::command]
pub async fn remove_rule(app: AppHandle, index: usize) -> Result<RulesView, AppError> {
    blocking(move || app.state::<AppState>().remove_rule(index)).await
}

/// Re-analyses the playset in the proposed order, so it runs on the blocking pool.
#[tauri::command]
pub async fn propose(app: AppHandle) -> Result<ProposalView, AppError> {
    blocking(move || app.state::<AppState>().propose()).await
}

#[tauri::command]
pub async fn launcher_status(app: AppHandle) -> Result<LauncherStatus, AppError> {
    blocking(move || Ok(app.state::<AppState>().launcher_status())).await
}

#[tauri::command]
pub async fn apply_preview(app: AppHandle) -> Result<ApplyPreviewView, AppError> {
    blocking(move || app.state::<AppState>().apply_preview()).await
}

/// The only command that writes the launcher database.
#[tauri::command]
pub async fn apply(app: AppHandle) -> Result<ApplyView, AppError> {
    blocking(move || app.state::<AppState>().apply(chrono::Local::now())).await
}

#[tauri::command]
pub async fn calibrate(
    app: AppHandle,
    on_progress: Channel<Progress>,
) -> Result<CalibrationView, AppError> {
    blocking(move || {
        app.state::<AppState>().calibrate(&|p| {
            let _ = on_progress.send(p);
        })
    })
    .await
}

/// The request runs on the blocking pool; a failed task is the same as no update.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Option<UpdateView> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().check_update(now))
        .await
        .ok()
        .flatten()
}

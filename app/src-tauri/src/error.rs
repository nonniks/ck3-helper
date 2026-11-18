use ck3h_core::error::CoreError;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorCode {
    NoUserDir,
    LauncherDbNotFound,
    Schema,
    PlaysetNotFound,
    NoActivePlayset,
    LauncherRunning,
    ProcessListUnavailable,
    PlaysetChanged,
    NothingToApply,
    Cycle,
    NotGameDir,
    LogNotFound,
    Rules,
    Settings,
    NotAnalyzed,
    Io,
    Db,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppError {
    pub code: ErrorCode,
    pub items: Vec<String>,
    pub detail: String,
}

impl AppError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            items: Vec::new(),
            detail: String::new(),
        }
    }
}

impl From<CoreError> for AppError {
    fn from(e: CoreError) -> Self {
        let detail = e.to_string();
        let (code, items) = match e {
            CoreError::LauncherRunning(n) => (ErrorCode::LauncherRunning, n),
            CoreError::Cycle(n) => (ErrorCode::Cycle, n),
            CoreError::LauncherDbNotFound(p) => (ErrorCode::LauncherDbNotFound, vec![path(&p)]),
            CoreError::NotGameDir(p) => (ErrorCode::NotGameDir, vec![path(&p)]),
            CoreError::LogNotFound(p) => (ErrorCode::LogNotFound, vec![path(&p)]),
            CoreError::Schema { table, column } => {
                (ErrorCode::Schema, vec![format!("{table}.{column}")])
            }
            CoreError::PlaysetNotFound(x) => (ErrorCode::PlaysetNotFound, vec![x]),
            CoreError::NoActivePlayset => (ErrorCode::NoActivePlayset, vec![]),
            CoreError::OrderMismatch => (ErrorCode::PlaysetChanged, vec![]),
            CoreError::NoLauncherNames => (ErrorCode::Settings, vec![]),
            CoreError::ProcessListUnavailable => (ErrorCode::ProcessListUnavailable, vec![]),
            CoreError::Rules(_) => (ErrorCode::Rules, vec![]),
            CoreError::Descriptor(_) | CoreError::Io(_) => (ErrorCode::Io, vec![]),
            CoreError::Db(_) => (ErrorCode::Db, vec![]),
        };
        Self {
            code,
            items,
            detail,
        }
    }
}

fn path(p: &std::path::Path) -> String {
    p.display().to_string()
}

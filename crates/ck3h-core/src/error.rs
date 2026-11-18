use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("unexpected launcher schema: missing {table}.{column}")]
    Schema { table: String, column: String },
    #[error("launcher database not found: {}", .0.display())]
    LauncherDbNotFound(PathBuf),
    #[error("not a CK3 game folder (no common/): {}", .0.display())]
    NotGameDir(PathBuf),
    #[error("log not found: {}", .0.display())]
    LogNotFound(PathBuf),
    #[error("playset not found: {0}")]
    PlaysetNotFound(String),
    #[error("no active playset")]
    NoActivePlayset,
    #[error("invalid descriptor: {0}")]
    Descriptor(String),
    #[error("invalid rules: {0}")]
    Rules(String),
    #[error("dependency cycle: {}", .0.join(" -> "))]
    Cycle(Vec<String>),
    #[error("launcher is running: {}", .0.join(", "))]
    LauncherRunning(Vec<String>),
    #[error("the playset changed since it was analysed; analyse it again")]
    OrderMismatch,
    #[error("no launcher process names given; the launcher-running check cannot be skipped")]
    NoLauncherNames,
    #[error("cannot read the process list to check whether the launcher is running")]
    ProcessListUnavailable,
}

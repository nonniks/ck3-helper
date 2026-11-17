use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("unexpected launcher schema: missing {table}.{column}")]
    Schema { table: String, column: String },
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
    #[error("the playset changed since it was analysed; run `ck3h sort` again")]
    OrderMismatch,
    #[error("no launcher process names given; the launcher-running check cannot be skipped")]
    NoLauncherNames,
}

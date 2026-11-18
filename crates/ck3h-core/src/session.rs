//! Loading a launcher playset (and the game's conflicts log) for analysis; shared by the CLI
//! and the app.

use crate::calibrate::{ParsedLog, parse_conflicts_log};
use crate::error::CoreError;
use crate::index::{ModIndex, index_vanilla};
use crate::launcher::{LAUNCHER_DB_FILE, LauncherDb, Playset};
use crate::paths::{game_dir_if_valid, game_language};
use crate::playset::{ModEntry, load_playset_with_progress};
use std::path::{Path, PathBuf};

pub const CONFLICTS_LOG: &str = "logs/database_conflicts.log";

#[derive(Debug, Clone, PartialEq)]
pub enum PlaysetSelector {
    Active,
    ByName(String),
    ById(String),
}

#[derive(Debug, Clone)]
pub struct Loaded {
    pub db_path: PathBuf,
    pub playset: Playset,
    pub lang: String,
    pub mods: Vec<ModEntry>,
}

/// Opens the launcher database in `user_dir` read-only.
pub fn launcher_db(user_dir: &Path) -> Result<(PathBuf, LauncherDb), CoreError> {
    let db_path = user_dir.join(LAUNCHER_DB_FILE);
    if !db_path.is_file() {
        return Err(CoreError::LauncherDbNotFound(db_path));
    }
    let db = LauncherDb::open_read_only(&db_path)?;
    Ok((db_path, db))
}

/// The explicit language, else the game's, else `l_english`.
pub fn analysis_lang(user_dir: &Path, explicit: Option<&str>) -> String {
    explicit
        .map(str::to_string)
        .or_else(|| game_language(user_dir))
        .unwrap_or_else(|| "l_english".to_string())
}

pub fn load(
    user_dir: &Path,
    selector: &PlaysetSelector,
    lang: &str,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<Loaded, CoreError> {
    let (db_path, db) = launcher_db(user_dir)?;
    let playset = match selector {
        PlaysetSelector::Active => db.active_playset()?,
        PlaysetSelector::ByName(name) => db.playset_by_name(name)?,
        PlaysetSelector::ById(id) => db.playset_by_id(id)?,
    };
    let mods = load_playset_with_progress(&db, &playset, lang, progress)?;
    Ok(Loaded {
        db_path,
        playset,
        lang: lang.to_string(),
        mods,
    })
}

pub fn load_vanilla(game_dir: &Path, lang: &str) -> Result<ModIndex, CoreError> {
    let dir =
        game_dir_if_valid(game_dir).ok_or_else(|| CoreError::NotGameDir(game_dir.to_path_buf()))?;
    index_vanilla(&dir, lang)
}

/// True if the launcher's mod order and enabled flags still match what was loaded.
pub fn playset_unchanged(loaded: &Loaded) -> Result<bool, CoreError> {
    let db = LauncherDb::open_read_only(&loaded.db_path)?;
    let now = db.playset_mods(&loaded.playset.id)?;
    Ok(now.len() == loaded.mods.len()
        && now
            .iter()
            .zip(&loaded.mods)
            .all(|(a, b)| a.mod_id == b.id && a.position == b.position && a.enabled == b.enabled))
}

pub fn read_conflicts_log(user_dir: &Path) -> Result<ParsedLog, CoreError> {
    let path = user_dir.join(CONFLICTS_LOG);
    let bytes = std::fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => CoreError::LogNotFound(path),
        _ => CoreError::Io(e),
    })?;
    let (text, _) = crate::text::decode(&bytes);
    Ok(parse_conflicts_log(&text))
}

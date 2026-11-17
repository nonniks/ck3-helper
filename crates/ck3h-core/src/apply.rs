//! The only writer of the launcher database: saves a sorted COPY of a playset.
//! Existing playsets are never modified; a backup of the database is made first.

use crate::error::CoreError;
use crate::launcher::{READ_COLUMNS, check_columns};
use chrono::{DateTime, Local};
use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

pub const DEFAULT_LAUNCHER_PROCESS_NAMES: &[&str] = &["paradox launcher", "paradox-launcher"];

const WRITE_COLUMNS: &[(&str, &[&str])] = &[
    (
        "playsets",
        &["loadOrder", "createdOn", "updatedOn", "syncState"],
    ),
    ("playsets_dlcs", &["playsetId", "dlcId", "enabled"]),
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApplyResult {
    pub new_playset_id: String,
    pub new_playset_name: String,
    pub backup_path: PathBuf,
}

/// Running processes whose name or executable path contains any of `names` (case-insensitive).
/// Each hit is the process name, or the executable path when only the path matched.
pub fn launcher_processes(names: &[&str]) -> Vec<String> {
    let needles: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    // Fail closed: if the process list cannot be read, report a blocker rather than none.
    let Ok(mut sys) = System::new() else {
        return vec!["<process list unavailable>".to_string()];
    };
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    let mut found = BTreeSet::new();
    for process in sys.processes().values() {
        let name = process.name().to_string_lossy().to_string();
        let name_lower = name.to_lowercase();
        // macOS truncates process names, so the full executable path is matched too.
        let exe = process
            .exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let exe_lower = exe.to_lowercase();
        if needles.iter().any(|n| name_lower.contains(n.as_str())) {
            found.insert(name);
        } else if needles.iter().any(|n| exe_lower.contains(n.as_str())) {
            found.insert(exe);
        }
    }
    found.into_iter().collect()
}

/// Saves `order` (mod ids, first = loaded first) as a new playset "<source> (CK3 Helper)".
pub fn apply_order(
    db_path: &Path,
    source_playset_id: &str,
    order: &[String],
    now: DateTime<Local>,
    launcher_names: &[&str],
) -> Result<ApplyResult, CoreError> {
    if launcher_names.is_empty() {
        return Err(CoreError::NoLauncherNames);
    }
    let running = launcher_processes(launcher_names);
    if !running.is_empty() {
        return Err(CoreError::LauncherRunning(running));
    }

    // No CREATE flag: a wrong path must not create an empty database.
    let mut conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    check_columns(&conn, READ_COLUMNS)?;

    let source_name: String = conn
        .query_row(
            "SELECT name FROM playsets WHERE id = ?1",
            [source_playset_id],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                CoreError::PlaysetNotFound(source_playset_id.to_string())
            }
            other => CoreError::Db(other),
        })?;
    let mut source_mods: Vec<String> = conn
        .prepare("SELECT modId FROM playsets_mods WHERE playsetId = ?1")?
        .query_map([source_playset_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut wanted = order.to_vec();
    source_mods.sort();
    wanted.sort();
    if source_mods != wanted {
        return Err(CoreError::OrderMismatch);
    }

    check_columns(&conn, WRITE_COLUMNS)?;

    let existing: BTreeSet<String> = conn
        .prepare("SELECT name FROM playsets")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let new_name = (1u32..)
        .map(|n| match n {
            1 => format!("{source_name} (CK3 Helper)"),
            n => format!("{source_name} (CK3 Helper {n})"),
        })
        .find(|name| !existing.contains(name))
        .expect("unbounded iterator");

    let backup_path = create_backup(db_path, now)?;

    let new_id = uuid::Uuid::new_v4().to_string();
    let millis = now.timestamp_millis();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO playsets (id, name, isActive, loadOrder, createdOn, updatedOn, syncState)
         VALUES (?1, ?2, 0, 'custom', ?3, ?3, 'NOT_ELIGIBLE')",
        params![new_id, new_name, millis],
    )?;
    {
        let mut select = tx.prepare(
            "SELECT COALESCE(enabled, 1) FROM playsets_mods WHERE playsetId = ?1 AND modId = ?2",
        )?;
        let mut insert = tx.prepare(
            "INSERT INTO playsets_mods (playsetId, modId, enabled, position) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (position, mod_id) in order.iter().enumerate() {
            let enabled: i64 =
                select.query_row(params![source_playset_id, mod_id], |r| r.get(0))?;
            insert.execute(params![new_id, mod_id, enabled, position as i64])?;
        }
    }
    tx.execute(
        "INSERT INTO playsets_dlcs (playsetId, dlcId, enabled)
         SELECT ?1, dlcId, enabled FROM playsets_dlcs WHERE playsetId = ?2",
        params![new_id, source_playset_id],
    )?;
    tx.commit()?;

    Ok(ApplyResult {
        new_playset_id: new_id,
        new_playset_name: new_name,
        backup_path,
    })
}

/// Copies the database next to itself under a name that is guaranteed to be new.
fn create_backup(db_path: &Path, now: DateTime<Local>) -> Result<PathBuf, CoreError> {
    let dir = db_path.parent().unwrap_or(Path::new("."));
    let stamp = now.format("%Y-%m-%d_%H-%M-%S");
    for n in 1u32.. {
        let file_name = match n {
            1 => format!("launcher-v2_ck3helper_{stamp}.sqlite"),
            n => format!("launcher-v2_ck3helper_{stamp}_{n}.sqlite"),
        };
        let path = dir.join(file_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut out) => {
                let copied =
                    File::open(db_path).and_then(|mut src| std::io::copy(&mut src, &mut out));
                if let Err(e) = copied.and_then(|_| out.sync_all()) {
                    drop(out);
                    let _ = std::fs::remove_file(&path); // our own partial file
                    return Err(e.into());
                }
                return Ok(path);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!("backup counter is unbounded")
}

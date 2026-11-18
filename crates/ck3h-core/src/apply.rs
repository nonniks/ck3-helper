//! The only writer of the launcher database: saves a sorted COPY of a playset.
//! Existing playsets are never modified; a backup of the database is made first.

use crate::error::CoreError;
use crate::launcher::{READ_COLUMNS, check_columns};
use chrono::{DateTime, Local};
use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
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

/// One running process, as far as the launcher check needs to know it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe: String,
}

/// Processes whose name or executable path contains any of `names` (case-insensitive).
/// Each hit is the process name, or the executable path when only the path matched.
/// Fail closed: a list that does not contain `own_pid` is incomplete, so it is an error.
pub fn blocking_processes(
    processes: &[ProcessInfo],
    own_pid: u32,
    names: &[&str],
) -> Result<Vec<String>, CoreError> {
    if !processes.iter().any(|p| p.pid == own_pid) {
        return Err(CoreError::ProcessListUnavailable);
    }
    let needles: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    let mut found = BTreeSet::new();
    for process in processes {
        let name_lower = process.name.to_lowercase();
        // macOS truncates process names, so the full executable path is matched too.
        let exe_lower = process.exe.to_lowercase();
        if needles.iter().any(|n| name_lower.contains(n.as_str())) {
            found.insert(process.name.clone());
        } else if needles.iter().any(|n| exe_lower.contains(n.as_str())) {
            found.insert(process.exe.clone());
        }
    }
    Ok(found.into_iter().collect())
}

/// Running launcher processes, read from the operating system.
pub fn launcher_processes(names: &[&str]) -> Result<Vec<String>, CoreError> {
    let own_pid = sysinfo::get_current_pid().map_err(|_| CoreError::ProcessListUnavailable)?;
    let mut sys = System::new().map_err(|_| CoreError::ProcessListUnavailable)?;
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    let processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, process)| ProcessInfo {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().to_string(),
            exe: process
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        })
        .collect();
    blocking_processes(&processes, own_pid.as_u32(), names)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApplyPreview {
    pub new_playset_name: String,
    pub backup_dir: PathBuf,
}

/// What `apply_order` would create, without writing anything.
pub fn preview(db_path: &Path, source_playset_id: &str) -> Result<ApplyPreview, CoreError> {
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    check_columns(&conn, READ_COLUMNS)?;
    let source_name = source_playset_name(&conn, source_playset_id)?;
    Ok(ApplyPreview {
        new_playset_name: copy_name(&conn, &source_name)?,
        backup_dir: db_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
    })
}

fn source_playset_name(conn: &Connection, source_playset_id: &str) -> Result<String, CoreError> {
    conn.query_row(
        "SELECT name FROM playsets WHERE id = ?1",
        [source_playset_id],
        |r| r.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            CoreError::PlaysetNotFound(source_playset_id.to_string())
        }
        other => CoreError::Db(other),
    })
}

/// "<source> (CK3 Helper)", then "(CK3 Helper 2)", ... — the first name not yet taken.
fn copy_name(conn: &Connection, source_name: &str) -> Result<String, CoreError> {
    let existing: BTreeSet<String> = conn
        .prepare("SELECT name FROM playsets")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok((1u32..)
        .map(|n| match n {
            1 => format!("{source_name} (CK3 Helper)"),
            n => format!("{source_name} (CK3 Helper {n})"),
        })
        .find(|name| !existing.contains(name))
        .expect("unbounded iterator"))
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
    let running = launcher_processes(launcher_names)?;
    if !running.is_empty() {
        return Err(CoreError::LauncherRunning(running));
    }

    // No CREATE flag: a wrong path must not create an empty database.
    let mut conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    check_columns(&conn, READ_COLUMNS)?;

    let source_name = source_playset_name(&conn, source_playset_id)?;
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

    let new_name = copy_name(&conn, &source_name)?;

    let backup_path = create_backup(&conn, db_path, now)?;

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

/// Writes a consistent copy of the open database next to it under a name that is guaranteed to be new.
fn create_backup(
    conn: &Connection,
    db_path: &Path,
    now: DateTime<Local>,
) -> Result<PathBuf, CoreError> {
    let dir = db_path.parent().unwrap_or(Path::new("."));
    let stamp = now.format("%Y-%m-%d_%H-%M-%S");
    for n in 1u32.. {
        let file_name = match n {
            1 => format!("launcher-v2_ck3helper_{stamp}.sqlite"),
            n => format!("launcher-v2_ck3helper_{stamp}_{n}.sqlite"),
        };
        let path = dir.join(file_name);
        // Reserve the name; VACUUM INTO accepts an empty file.
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                if let Err(e) = conn.execute("VACUUM INTO ?1", [path.to_string_lossy()]) {
                    let _ = std::fs::remove_file(&path); // our own empty file
                    return Err(e.into());
                }
                // SQLite does not sync VACUUM INTO output. Opened for writing: Windows refuses
                // to flush a read-only handle.
                let synced = OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .and_then(|f| f.sync_all());
                if let Err(e) = synced {
                    let _ = std::fs::remove_file(&path); // our own unsynced copy
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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, name: &str, exe: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: name.into(),
            exe: exe.into(),
        }
    }

    #[test]
    fn process_list_without_own_pid_is_unavailable() {
        let procs = [p(1, "launchd", "/sbin/launchd")];
        assert!(matches!(
            blocking_processes(&procs, 42, &["paradox launcher"]),
            Err(CoreError::ProcessListUnavailable)
        ));
    }

    #[test]
    fn blocking_matches_name_or_exe_without_case() -> Result<(), CoreError> {
        let procs = [
            p(42, "me", ""),
            p(7, "Paradox Launcher", ""),
            p(
                8,
                "Electron",
                "/Applications/Paradox Launcher.app/Contents/MacOS/Electron",
            ),
        ];
        assert_eq!(
            blocking_processes(&procs, 42, &["paradox launcher"])?,
            [
                "/Applications/Paradox Launcher.app/Contents/MacOS/Electron",
                "Paradox Launcher"
            ]
        );
        Ok(())
    }
}

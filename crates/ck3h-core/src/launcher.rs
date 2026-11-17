use crate::error::CoreError;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

pub const LAUNCHER_DB_FILE: &str = "launcher-v2.sqlite";

/// Tables and columns this crate reads from the launcher database.
pub const READ_COLUMNS: &[(&str, &[&str])] = &[
    ("playsets", &["id", "name", "isActive", "isRemoved"]),
    (
        "playsets_mods",
        &["playsetId", "modId", "enabled", "position"],
    ),
    (
        "mods",
        &["id", "displayName", "steamId", "source", "dirPath"],
    ),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Playset {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaysetMod {
    pub mod_id: String,
    pub display_name: String,
    pub steam_id: Option<String>,
    pub source: String,
    pub dir_path: PathBuf,
    pub position: i64,
    pub enabled: bool,
}

/// Verifies with `PRAGMA table_info` that every required column exists.
pub fn check_columns(conn: &Connection, required: &[(&str, &[&str])]) -> Result<(), CoreError> {
    for (table, columns) in required {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{table}\")"))?;
        let present: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<_, _>>()?;
        for column in *columns {
            if !present.iter().any(|c| c == column) {
                return Err(CoreError::Schema {
                    table: (*table).to_string(),
                    column: (*column).to_string(),
                });
            }
        }
    }
    Ok(())
}

pub struct LauncherDb {
    conn: Connection,
}

impl LauncherDb {
    pub fn open_read_only(path: &Path) -> Result<Self, CoreError> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        check_columns(&conn, READ_COLUMNS)?;
        Ok(Self { conn })
    }

    pub fn playsets(&self) -> Result<Vec<Playset>, CoreError> {
        self.query_playsets("WHERE COALESCE(isRemoved, 0) = 0 ORDER BY name, id", [])
    }

    pub fn active_playset(&self) -> Result<Playset, CoreError> {
        self.query_playsets(
            "WHERE COALESCE(isRemoved, 0) = 0 AND COALESCE(isActive, 0) = 1 ORDER BY name, id",
            [],
        )?
        .into_iter()
        .next()
        .ok_or(CoreError::NoActivePlayset)
    }

    pub fn playset_by_name(&self, name: &str) -> Result<Playset, CoreError> {
        self.query_playsets(
            "WHERE COALESCE(isRemoved, 0) = 0 AND name = ?1 ORDER BY id",
            [name],
        )?
        .into_iter()
        .next()
        .ok_or_else(|| CoreError::PlaysetNotFound(name.to_string()))
    }

    pub fn playset_mods(&self, playset_id: &str) -> Result<Vec<PlaysetMod>, CoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, COALESCE(m.displayName, m.name, ''), m.steamId, m.source,
                    COALESCE(m.dirPath, ''), COALESCE(pm.position, 0), COALESCE(pm.enabled, 1)
             FROM playsets_mods pm JOIN mods m ON m.id = pm.modId
             WHERE pm.playsetId = ?1
             ORDER BY COALESCE(pm.position, 0), m.id",
        )?;
        let rows = stmt.query_map([playset_id], |row| {
            Ok(PlaysetMod {
                mod_id: row.get(0)?,
                display_name: row.get(1)?,
                steam_id: row.get(2)?,
                source: row.get(3)?,
                dir_path: PathBuf::from(row.get::<_, String>(4)?),
                position: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    fn query_playsets(
        &self,
        clause: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<Playset>, CoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, name, COALESCE(isActive, 0) FROM playsets {clause}"
        ))?;
        let rows = stmt.query_map(params, |row| {
            Ok(Playset {
                id: row.get(0)?,
                name: row.get(1)?,
                is_active: row.get::<_, i64>(2)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

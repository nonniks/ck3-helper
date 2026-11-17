//! Test helpers: build a launcher DB from the real schema and fake mod folders.
//! Panics on failure; for tests only.

use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SCHEMA: &str = include_str!("../fixtures/launcher_schema.sql");
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn new_id() -> String {
    format!(
        "00000000-0000-0000-0000-{:012}",
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn open_ro(db: &Path) -> Connection {
    Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).expect("open db")
}

pub fn create_launcher_db(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("create db");
    conn.execute_batch(SCHEMA).expect("apply launcher schema");
    conn
}

pub fn add_playset(conn: &Connection, name: &str, active: bool) -> String {
    let id = new_id();
    conn.execute(
        "INSERT INTO playsets (id, name, isActive, createdOn) VALUES (?1, ?2, ?3, '2026-01-01')",
        rusqlite::params![id, name, active],
    )
    .expect("insert playset");
    id
}

pub fn add_mod(
    conn: &Connection,
    display_name: &str,
    dir: &Path,
    steam_id: Option<&str>,
) -> String {
    let id = new_id();
    let source = if steam_id.is_some() { "steam" } else { "local" };
    conn.execute(
        "INSERT INTO mods (id, steamId, displayName, dirPath, status, source)
         VALUES (?1, ?2, ?3, ?4, 'ready_to_play', ?5)",
        rusqlite::params![id, steam_id, display_name, dir.to_string_lossy(), source],
    )
    .expect("insert mod");
    id
}

pub fn add_playset_mod(
    conn: &Connection,
    playset_id: &str,
    mod_id: &str,
    position: i64,
    enabled: bool,
) {
    conn.execute(
        "INSERT INTO playsets_mods (playsetId, modId, enabled, position) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![playset_id, mod_id, enabled, position],
    )
    .expect("insert playset mod");
}

pub fn add_playset_dlc(conn: &Connection, playset_id: &str, dlc_id: &str, enabled: bool) {
    conn.execute(
        "INSERT INTO playsets_dlcs (playsetId, dlcId, enabled) VALUES (?1, ?2, ?3)",
        rusqlite::params![playset_id, dlc_id, enabled],
    )
    .expect("insert playset dlc");
}

/// Creates `root/dir_name` with the given files (paths relative to the mod folder).
pub fn write_mod(root: &Path, dir_name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let dir = root.join(dir_name);
    std::fs::create_dir_all(&dir).expect("create mod dir");
    for (rel, bytes) in files {
        let file = dir.join(rel);
        std::fs::create_dir_all(file.parent().expect("file has parent")).expect("create dirs");
        std::fs::write(file, bytes).expect("write file");
    }
    dir
}

pub fn playset_count(db: &Path) -> usize {
    open_ro(db)
        .query_row("SELECT COUNT(*) FROM playsets", [], |r| r.get::<_, i64>(0))
        .expect("count playsets") as usize
}

pub fn playset_mod_names(db: &Path, playset_id: &str) -> Vec<String> {
    let conn = open_ro(db);
    let mut stmt = conn
        .prepare(
            "SELECT m.displayName FROM playsets_mods pm JOIN mods m ON m.id = pm.modId
             WHERE pm.playsetId = ?1 ORDER BY pm.position",
        )
        .expect("prepare");
    stmt.query_map([playset_id], |r| r.get::<_, String>(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

pub fn playset_dlcs(db: &Path, playset_id: &str) -> Vec<(String, bool)> {
    let conn = open_ro(db);
    let mut stmt = conn
        .prepare("SELECT dlcId, enabled FROM playsets_dlcs WHERE playsetId = ?1 ORDER BY dlcId")
        .expect("prepare");
    stmt.query_map([playset_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? != 0))
    })
    .expect("query")
    .collect::<Result<_, _>>()
    .expect("rows")
}

/// A user dir (`root/user`) with a launcher DB, `pdx_settings.txt` (`l_english`) and the active
/// playset "Main" of five enabled mods living in `root/mods`: Base A, Base B, A+B Compatch,
/// Ghost (no folder on disk) and Broken (unbalanced braces, unknown dependency).
pub fn standard_fixture(root: &Path) -> PathBuf {
    let user = root.join("user");
    std::fs::create_dir_all(&user).expect("create user dir");
    std::fs::write(
        user.join("pdx_settings.txt"),
        "\"language\"={\n\tversion=0\n\tvalue=\"l_english\"\n}\n",
    )
    .expect("write pdx_settings");
    let mods = root.join("mods");
    let conn = create_launcher_db(&user.join("launcher-v2.sqlite"));
    let playset = add_playset(&conn, "Main", true);

    let a = write_mod(
        &mods,
        "base_a",
        &[
            ("descriptor.mod", b"name=\"Base A\"\n"),
            (
                "common/traits/zz_base_a.txt",
                b"shared_trait = { a = 1 }\nonly_a = { }\n",
            ),
            ("events/shared_events.txt", b"namespace = shared\n"),
            (
                "localization/english/base_a_l_english.yml",
                "\u{feff}l_english:\n shared_key:0 \"From A\"\n".as_bytes(),
            ),
        ],
    );
    let b = write_mod(
        &mods,
        "base_b",
        &[
            ("descriptor.mod", b"name=\"Base B\"\n"),
            ("common/traits/00_base_b.txt", b"shared_trait = { b = 2 }\n"),
            ("Events/Shared_Events.txt", b"namespace = shared\n"),
            (
                "localization/english/base_b_l_english.yml",
                "\u{feff}l_english:\n shared_key:0 \"From B\"\n".as_bytes(),
            ),
        ],
    );
    let compat = write_mod(
        &mods,
        "compat_ab",
        &[
            (
                "descriptor.mod",
                b"name=\"A+B Compatch\"\ndependencies={\n\t\"Base A\"\n\t\"Base B\"\n}\n",
            ),
            ("common/traits/zzz_compat.txt", b"compat_trait = { }\n"),
            (
                "localization/english/replace/compat_l_english.yml",
                "\u{feff}l_english:\n replace_key:0 \"Compat\"\n".as_bytes(),
            ),
        ],
    );
    let broken = write_mod(
        &mods,
        "broken",
        &[
            (
                "descriptor.mod",
                b"name=\"Broken\"\ndependencies={\n\t\"Missing Mod\"\n}\n",
            ),
            ("common/traits/bad.txt", b"x = { y = 1"),
        ],
    );
    let ghost = mods.join("ghost");

    for (pos, (name, dir)) in [
        ("Base A", a),
        ("Base B", b),
        ("A+B Compatch", compat),
        ("Ghost", ghost),
        ("Broken", broken),
    ]
    .into_iter()
    .enumerate()
    {
        let id = add_mod(&conn, name, &dir, None);
        add_playset_mod(&conn, &playset, &id, pos as i64, true);
    }
    user
}

#![cfg(feature = "testkit")]

use ck3h_core::error::CoreError;
use ck3h_core::launcher::LauncherDb;
use ck3h_core::testkit::*;
use rusqlite::Connection;
use std::path::PathBuf;

fn setup() -> (tempfile::TempDir, PathBuf, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("launcher-v2.sqlite");
    let conn = create_launcher_db(&path);
    (dir, path, conn)
}

#[test]
fn lists_playsets_and_active() {
    let (_d, path, conn) = setup();
    add_playset(&conn, "Main", true);
    add_playset(&conn, "Other", false);
    let db = LauncherDb::open_read_only(&path).unwrap();
    assert_eq!(db.active_playset().unwrap().name, "Main");
    assert_eq!(db.playsets().unwrap().len(), 2);
    assert_eq!(db.playset_by_name("Other").unwrap().name, "Other");
}

#[test]
fn removed_playsets_hidden() {
    let (_d, path, conn) = setup();
    add_playset(&conn, "Main", true);
    let old = add_playset(&conn, "Old", false);
    conn.execute("UPDATE playsets SET isRemoved = 1 WHERE id = ?1", [&old])
        .unwrap();
    let db = LauncherDb::open_read_only(&path).unwrap();
    assert_eq!(db.playsets().unwrap().len(), 1);
}

#[test]
fn no_active_playset_is_error() {
    let (_d, path, conn) = setup();
    add_playset(&conn, "Main", false);
    let db = LauncherDb::open_read_only(&path).unwrap();
    assert!(matches!(
        db.active_playset(),
        Err(CoreError::NoActivePlayset)
    ));
}

#[test]
fn playset_mods_ordered_by_position() {
    let (d, path, conn) = setup();
    let ps = add_playset(&conn, "Main", true);
    for (name, pos) in [("C", 2), ("A", 0), ("B", 1)] {
        let dir = write_mod(d.path(), name, &[("descriptor.mod", b"name=\"x\"")]);
        let m = add_mod(&conn, name, &dir, None);
        add_playset_mod(&conn, &ps, &m, pos, true);
    }
    let db = LauncherDb::open_read_only(&path).unwrap();
    let names: Vec<String> = db
        .playset_mods(&ps)
        .unwrap()
        .into_iter()
        .map(|m| m.display_name)
        .collect();
    assert_eq!(names, ["A", "B", "C"]);
    assert_eq!(playset_mod_names(&path, &ps), ["A", "B", "C"]);
}

#[test]
fn mod_fields_are_mapped() {
    let (d, path, conn) = setup();
    let ps = add_playset(&conn, "Main", true);
    let dir = d.path().join("w");
    let steam = add_mod(&conn, "S", &dir, Some("123"));
    let local = add_mod(&conn, "L", &dir, None);
    add_playset_mod(&conn, &ps, &steam, 0, true);
    add_playset_mod(&conn, &ps, &local, 1, false);
    let db = LauncherDb::open_read_only(&path).unwrap();
    let mods = db.playset_mods(&ps).unwrap();
    assert_eq!(mods[0].steam_id.as_deref(), Some("123"));
    assert_eq!(mods[0].source, "steam");
    assert!(mods[0].enabled);
    assert_eq!(mods[0].dir_path, dir);
    assert_eq!(mods[1].steam_id, None);
    assert_eq!(mods[1].source, "local");
    assert!(!mods[1].enabled);
}

#[test]
fn playset_by_name_not_found() {
    let (_d, path, _conn) = setup();
    let db = LauncherDb::open_read_only(&path).unwrap();
    assert!(matches!(
        db.playset_by_name("Nope"),
        Err(CoreError::PlaysetNotFound(_))
    ));
}

#[test]
fn schema_drift_reports_missing_column() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("drift.sqlite");
    let conn = Connection::open(&p).unwrap();
    conn.execute_batch(
        "CREATE TABLE playsets (id, name, isActive, isRemoved);
         CREATE TABLE playsets_mods (playsetId, modId, enabled, position);
         CREATE TABLE mods (id, displayName, steamId, source);",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        LauncherDb::open_read_only(&p),
        Err(CoreError::Schema { table, column }) if table == "mods" && column == "dirPath"
    ));
}

#[test]
fn schema_drift_reports_missing_mods_name_column() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("drift.sqlite");
    let conn = Connection::open(&p).unwrap();
    conn.execute_batch(
        "CREATE TABLE playsets (id, name, isActive, isRemoved);
         CREATE TABLE playsets_mods (playsetId, modId, enabled, position);
         CREATE TABLE mods (id, displayName, steamId, source, dirPath);",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        LauncherDb::open_read_only(&p),
        Err(CoreError::Schema { table, column }) if table == "mods" && column == "name"
    ));
}

#[test]
fn dlcs_and_playset_count_helpers() {
    let (_d, path, conn) = setup();
    let ps = add_playset(&conn, "Main", true);
    add_playset_dlc(&conn, &ps, "dlc_a", true);
    add_playset_dlc(&conn, &ps, "dlc_b", false);
    assert_eq!(playset_count(&path), 1);
    assert_eq!(
        playset_dlcs(&path, &ps),
        [("dlc_a".to_string(), true), ("dlc_b".to_string(), false)]
    );
}

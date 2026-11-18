#![cfg(feature = "testkit")]

use chrono::{DateTime, Local, TimeZone};
use ck3h_core::apply::{apply_order, preview};
use ck3h_core::error::CoreError;
use ck3h_core::testkit::*;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

const NONE: &[&str] = &["ck3h-test-no-such-process"];

struct Fx {
    dir: tempfile::TempDir,
    db: PathBuf,
    src: String,
    ids: [String; 3],
}

fn now() -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 11, 17, 12, 30, 45).unwrap()
}

fn setup_with(schema_tweak: Option<&str>) -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("launcher-v2.sqlite");
    let conn = create_launcher_db(&db);
    let src = add_playset(&conn, "Main", true);
    let mut ids = Vec::new();
    for (pos, name) in ["A", "B", "C"].iter().enumerate() {
        let id = add_mod(&conn, name, &dir.path().join(name), None);
        add_playset_mod(&conn, &src, &id, pos as i64, pos != 1);
        ids.push(id);
    }
    add_playset_dlc(&conn, &src, "dlc-1", true);
    add_playset_dlc(&conn, &src, "dlc-2", false);
    if let Some(sql) = schema_tweak {
        conn.execute_batch(sql).unwrap();
    }
    drop(conn);
    let [a, b, c]: [String; 3] = ids.try_into().unwrap();
    Fx {
        dir,
        db,
        src,
        ids: [a, b, c],
    }
}

fn setup() -> Fx {
    setup_with(None)
}

fn backups_in(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("_ck3helper_")
        })
        .count()
}

fn active_is(db: &Path, name: &str) -> bool {
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM playsets WHERE isActive = 1")
        .unwrap();
    let names: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    names == [name]
}

fn order(fx: &Fx, idx: [usize; 3]) -> Vec<String> {
    idx.iter().map(|&i| fx.ids[i].clone()).collect()
}

#[test]
fn creates_sorted_copy_and_backup() {
    let fx = setup();
    let r = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap();
    assert_eq!(playset_count(&r.backup_path), 1);
    assert_eq!(r.new_playset_name, "Main (CK3 Helper)");
    assert!(r.backup_path.exists());
    assert_eq!(
        r.backup_path.file_name().unwrap().to_string_lossy(),
        "launcher-v2_ck3helper_2026-11-17_12-30-45.sqlite"
    );
    assert_eq!(
        playset_mod_names(&fx.db, &r.new_playset_id),
        ["C", "A", "B"]
    );
    // enabled flags follow the mods, not the positions
    let conn = Connection::open_with_flags(&fx.db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (active, load_order, sync, created): (i64, String, String, i64) = conn
        .query_row(
            "SELECT isActive, loadOrder, syncState, createdOn FROM playsets WHERE id = ?1",
            [&r.new_playset_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        (active, load_order.as_str(), sync.as_str()),
        (0, "custom", "NOT_ELIGIBLE")
    );
    assert_eq!(created, now().timestamp_millis());
    let disabled: String = conn
        .query_row(
            "SELECT modId FROM playsets_mods WHERE playsetId = ?1 AND enabled = 0",
            [&r.new_playset_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(disabled, fx.ids[1]);
}

#[test]
fn source_playset_untouched() {
    let fx = setup();
    apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap();
    assert_eq!(playset_mod_names(&fx.db, &fx.src), ["A", "B", "C"]);
    assert!(active_is(&fx.db, "Main"));
}

#[test]
fn second_apply_gets_counter_and_keeps_first_backup() {
    let fx = setup();
    let o = order(&fx, [2, 0, 1]);
    let r1 = apply_order(&fx.db, &fx.src, &o, now(), NONE).unwrap();
    let first = std::fs::read(&r1.backup_path).unwrap();
    let r2 = apply_order(&fx.db, &fx.src, &o, now(), NONE).unwrap();
    assert_eq!(r2.new_playset_name, "Main (CK3 Helper 2)");
    assert_ne!(r1.backup_path, r2.backup_path);
    assert!(r2.backup_path.to_string_lossy().ends_with("_2.sqlite"));
    assert_eq!(std::fs::read(&r1.backup_path).unwrap(), first);
}

#[test]
fn dlc_rows_copied() {
    let fx = setup();
    let r = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap();
    assert_eq!(
        playset_dlcs(&fx.db, &r.new_playset_id),
        playset_dlcs(&fx.db, &fx.src)
    );
    assert_eq!(playset_dlcs(&fx.db, &r.new_playset_id).len(), 2);
}

#[test]
fn wrong_order_writes_nothing() {
    let fx = setup();
    let e = apply_order(&fx.db, &fx.src, &[fx.ids[0].clone()], now(), NONE).unwrap_err();
    assert!(matches!(e, CoreError::OrderMismatch));
    let dup = vec![fx.ids[0].clone(), fx.ids[0].clone(), fx.ids[1].clone()];
    let e = apply_order(&fx.db, &fx.src, &dup, now(), NONE).unwrap_err();
    assert!(matches!(e, CoreError::OrderMismatch));
    assert_eq!(playset_count(&fx.db), 1);
    assert_eq!(backups_in(fx.dir.path()), 0);
}

#[test]
fn schema_drift_writes_nothing() {
    let fx = setup_with(Some("ALTER TABLE playsets DROP COLUMN syncState"));
    let e = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap_err();
    assert!(matches!(e, CoreError::Schema { .. }));
    assert_eq!(playset_count(&fx.db), 1);
    assert_eq!(backups_in(fx.dir.path()), 0);
}

#[test]
fn running_launcher_blocks_apply() {
    let fx = setup();
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_name().unwrap().to_string_lossy().to_string();
    // sysinfo 0.40 takes the process name from the executable path on macOS; keep it short anyway
    let pattern: String = name.chars().take(15).collect();
    let e = apply_order(
        &fx.db,
        &fx.src,
        &order(&fx, [2, 0, 1]),
        now(),
        &[pattern.as_str()],
    )
    .unwrap_err();
    assert!(matches!(e, CoreError::LauncherRunning(_)));
    assert_eq!(playset_count(&fx.db), 1);
    assert_eq!(backups_in(fx.dir.path()), 0);
}

#[test]
fn launcher_is_detected_by_executable_path() {
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_name().unwrap().to_string_lossy().to_string();
    // the parent directory appears in the executable path but not in the process name
    let parent = exe
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(!name.to_lowercase().contains(&parent.to_lowercase()));
    let found = ck3h_core::apply::launcher_processes(&[parent.as_str()]).unwrap();
    assert!(
        found.iter().any(|f| f.contains(&name)),
        "expected the current process in {found:?}"
    );
}

#[test]
fn failing_transaction_rolls_back_and_keeps_backup() {
    let fx = setup_with(Some(
        "CREATE TRIGGER boom BEFORE INSERT ON playsets_dlcs BEGIN SELECT RAISE(ABORT, 'boom'); END;",
    ));
    let e = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap_err();
    assert!(matches!(e, CoreError::Db(_)));
    assert_eq!(playset_count(&fx.db), 1);
    let conn = Connection::open_with_flags(&fx.db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let stray: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM playsets_mods WHERE playsetId <> ?1",
            [&fx.src],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stray, 0);
    assert_eq!(backups_in(fx.dir.path()), 1);
}

#[test]
fn empty_launcher_names_is_refused() {
    let fx = setup();
    let e = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), &[]).unwrap_err();
    assert!(matches!(e, CoreError::NoLauncherNames));
    assert_eq!(playset_count(&fx.db), 1);
    assert_eq!(backups_in(fx.dir.path()), 0);
}

#[test]
fn backup_is_a_consistent_sqlite_copy() {
    let fx = setup();
    let r = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap();
    assert_eq!(playset_count(&r.backup_path), 1);
    assert_eq!(playset_count(&fx.db), 2);
}

#[test]
fn preview_matches_applied_name() {
    let fx = setup();
    let p = preview(&fx.db, &fx.src).unwrap();
    assert_eq!(p.backup_dir, fx.db.parent().unwrap());
    let r = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap();
    assert_eq!(r.new_playset_name, p.new_playset_name);
    assert_eq!(
        preview(&fx.db, &fx.src).unwrap().new_playset_name,
        "Main (CK3 Helper 2)"
    );
    assert_eq!(playset_count(&fx.db), 2);
}

#[test]
fn order_mismatch_text_names_no_command() {
    assert!(!CoreError::OrderMismatch.to_string().contains("ck3h"));
}

#[test]
fn backup_keeps_rows_still_in_the_wal() {
    let fx = setup();
    let conn = Connection::open(&fx.db).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    // committed, but only in the -wal file: a plain copy of the main file loses it
    add_playset(&conn, "Only in WAL", false);

    let r = apply_order(&fx.db, &fx.src, &order(&fx, [2, 0, 1]), now(), NONE).unwrap();

    let backup =
        Connection::open_with_flags(&r.backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let wal_row: i64 = backup
        .query_row(
            "SELECT COUNT(*) FROM playsets WHERE name = 'Only in WAL'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(wal_row, 1);
    let check: String = backup
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");
    drop(conn);
}

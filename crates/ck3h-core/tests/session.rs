#![cfg(feature = "testkit")]

use ck3h_core::error::CoreError;
use ck3h_core::session::*;
use ck3h_core::testkit::*;
use rusqlite::Connection;
use std::sync::Mutex;

fn db_conn(user: &std::path::Path) -> Connection {
    Connection::open(user.join("launcher-v2.sqlite")).unwrap()
}

#[test]
fn loads_active_by_name_and_by_id() {
    let tmp = tempfile::tempdir().unwrap();
    let ud = standard_fixture(tmp.path());
    let active = load(&ud, &PlaysetSelector::Active, "l_english", &|_, _| {}).unwrap();
    let by_id = load(
        &ud,
        &PlaysetSelector::ById(active.playset.id.clone()),
        "l_english",
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(by_id.playset.name, "Main");
    assert_eq!(active.mods.len(), 5);
    let by_name = load(
        &ud,
        &PlaysetSelector::ByName("Main".into()),
        "l_english",
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(by_name.playset, active.playset);
}

#[test]
fn missing_db_is_launcher_db_not_found() {
    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(
        launcher_db(empty.path()),
        Err(CoreError::LauncherDbNotFound(p)) if p.ends_with("launcher-v2.sqlite")
    ));
}

#[test]
fn unknown_id_is_playset_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let ud = standard_fixture(tmp.path());
    assert!(matches!(
        load(
            &ud,
            &PlaysetSelector::ById("nope".into()),
            "l_english",
            &|_, _| {}
        ),
        Err(CoreError::PlaysetNotFound(_))
    ));
}

#[test]
fn progress_reports_every_mod() {
    let tmp = tempfile::tempdir().unwrap();
    let ud = standard_fixture(tmp.path());
    let calls = Mutex::new(Vec::new());
    load(
        &ud,
        &PlaysetSelector::Active,
        "l_english",
        &|done, total| {
            calls.lock().unwrap().push((done, total));
        },
    )
    .unwrap();
    let mut calls = calls.into_inner().unwrap();
    calls.sort();
    assert_eq!(calls, [(1, 5), (2, 5), (3, 5), (4, 5), (5, 5)]);
}

#[test]
fn analysis_lang_prefers_explicit_then_game() {
    let de = tempfile::tempdir().unwrap();
    std::fs::write(
        de.path().join("pdx_settings.txt"),
        "\"language\"={\n\tversion=0\n\tvalue=\"l_german\"\n}\n",
    )
    .unwrap();
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(analysis_lang(de.path(), Some("l_russian")), "l_russian");
    assert_eq!(analysis_lang(de.path(), None), "l_german");
    assert_eq!(analysis_lang(empty.path(), None), "l_english");
}

#[test]
fn load_vanilla_rejects_non_game_dir() {
    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(
        load_vanilla(empty.path(), "l_english"),
        Err(CoreError::NotGameDir(_))
    ));
}

#[test]
fn playset_unchanged_detects_reorder_and_toggle() {
    let tmp = tempfile::tempdir().unwrap();
    let ud = standard_fixture(tmp.path());
    let loaded = load(&ud, &PlaysetSelector::Active, "l_english", &|_, _| {}).unwrap();
    assert!(playset_unchanged(&loaded).unwrap());

    let conn = db_conn(&ud);
    conn.execute(
        "UPDATE playsets_mods SET position = 9 WHERE position = 0",
        [],
    )
    .unwrap();
    assert!(!playset_unchanged(&loaded).unwrap());

    conn.execute(
        "UPDATE playsets_mods SET position = 0 WHERE position = 9",
        [],
    )
    .unwrap();
    assert!(playset_unchanged(&loaded).unwrap());
    conn.execute(
        "UPDATE playsets_mods SET enabled = 0 WHERE position = 1",
        [],
    )
    .unwrap();
    assert!(!playset_unchanged(&loaded).unwrap());
}

#[test]
fn conflicts_log_missing_then_read() {
    let tmp = tempfile::tempdir().unwrap();
    let ud = standard_fixture(tmp.path());
    assert!(matches!(
        read_conflicts_log(&ud),
        Err(CoreError::LogNotFound(_))
    ));
    std::fs::create_dir_all(ud.join("logs")).unwrap();
    std::fs::write(
        ud.join(CONFLICTS_LOG),
        "[01:59:17][W][game_database.h:272]: Overriding entry 'shared_trait' for database 'common/traits' in 'file: common/traits/zz_base_a.txt line: 1'\n",
    )
    .unwrap();
    assert_eq!(read_conflicts_log(&ud).unwrap().overrides.len(), 1);
}

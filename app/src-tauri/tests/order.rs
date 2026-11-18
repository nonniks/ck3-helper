use ck3h_app::dto::{LauncherStatus, MoveReasonView, ProposalView};
use ck3h_app::error::ErrorCode;
use ck3h_app::settings::{Settings, save_settings};
use ck3h_app::state::AppState;
use ck3h_core::testkit::{
    add_mod, add_playset, add_playset_mod, create_launcher_db, playset_count, write_mod,
};
use std::path::{Path, PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    _config: tempfile::TempDir,
    db: PathBuf,
    state: AppState,
}

fn now() -> chrono::DateTime<chrono::Local> {
    chrono::Local::now()
}

/// One playset "Main" with the given mods `(name, dependencies)` in this order, all enabled.
fn setup(mods: &[(&str, &[&str])], launcher_names: &str) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let user = root.path().join("user");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::write(
        user.join("pdx_settings.txt"),
        "\"language\"={\n\tversion=0\n\tvalue=\"l_english\"\n}\n",
    )
    .unwrap();
    let db = user.join("launcher-v2.sqlite");
    let conn = create_launcher_db(&db);
    let playset = add_playset(&conn, "Main", true);
    for (pos, (name, deps)) in mods.iter().enumerate() {
        let deps: String = deps.iter().map(|d| format!("\"{d}\"\n")).collect();
        let descriptor = format!("name=\"{name}\"\ndependencies={{\n{deps}}}\n");
        let dir = write_mod(
            &root.path().join("mods"),
            name,
            &[("descriptor.mod", descriptor.as_bytes())],
        );
        let m = add_mod(&conn, name, &dir, None);
        add_playset_mod(&conn, &playset, &m, pos as i64, true);
    }
    drop(conn);
    let game = root.path().join("game");
    std::fs::create_dir_all(&game).unwrap();
    save_settings(
        config.path(),
        &Settings {
            user_dir: Some(user),
            game_dir: Some(game),
            launcher_process_names: Some(vec![launcher_names.to_string()]),
            ..Settings::default()
        },
    )
    .unwrap();
    let state = AppState::new(config.path().to_path_buf());
    state.analyze(None, &|_| {}).unwrap();
    Fixture {
        _root: root,
        _config: config,
        db,
        state,
    }
}

const NO_PROCESS: &str = "ck3h-test-no-such-process";

fn unsorted() -> Fixture {
    setup(&[("Child", &["Parent"]), ("Parent", &[])], NO_PROCESS)
}

fn row_names(p: &ProposalView) -> Vec<&str> {
    p.rows.iter().map(|r| r.name.as_str()).collect()
}

#[test]
fn proposal_moves_dependent_after_dependency() {
    let f = unsorted();
    let p = f.state.propose().unwrap();
    assert_eq!(row_names(&p), ["Parent", "Child"]);
    assert_eq!(
        p.moves[0].reasons,
        [MoveReasonView::Dependency {
            on: "Parent".into()
        }]
    );
    assert!(!p.already_sorted);
    // Both ends of the broken constraint are moves.
    assert_eq!(p.moves.len(), 2);
    assert!(p.rows.iter().all(|r| r.moved));
}

#[test]
fn mods_that_only_shift_are_not_moved() {
    let f = setup(
        &[
            ("A", &["E"]),
            ("B", &[]),
            ("C", &[]),
            ("D", &[]),
            ("E", &[]),
        ],
        NO_PROCESS,
    );
    let p = f.state.propose().unwrap();
    assert_eq!(row_names(&p), ["B", "C", "D", "E", "A"]);
    let rows: Vec<(&str, bool, usize, usize)> = p
        .rows
        .iter()
        .map(|r| (r.name.as_str(), r.moved, r.from, r.to))
        .collect();
    assert_eq!(
        rows,
        [
            ("B", false, 1, 0),
            ("C", false, 2, 1),
            ("D", false, 3, 2),
            ("E", true, 4, 3),
            ("A", true, 0, 4),
        ]
    );
    let mut moves: Vec<&str> = p.moves.iter().map(|m| m.name.as_str()).collect();
    moves.sort();
    assert_eq!(moves, ["A", "E"]);
}

#[test]
fn sorted_playset_has_nothing_to_apply() {
    let f = setup(&[("Parent", &[]), ("Child", &["Parent"])], NO_PROCESS);
    let p = f.state.propose().unwrap();
    assert!(p.already_sorted);
    assert_eq!(
        f.state.apply(now()).unwrap_err().code,
        ErrorCode::NothingToApply
    );
}

#[test]
fn apply_without_proposal_is_nothing_to_apply() {
    let f = unsorted();
    assert_eq!(
        f.state.apply(now()).unwrap_err().code,
        ErrorCode::NothingToApply
    );
}

#[test]
fn cycle_is_error_with_names() {
    let f = setup(&[("A", &["B"]), ("B", &["A"])], NO_PROCESS);
    let mut e = f.state.propose().unwrap_err();
    assert_eq!(e.code, ErrorCode::Cycle);
    e.items.sort();
    assert_eq!(e.items, ["A", "B"]);
}

#[test]
fn apply_creates_copy_and_backup() {
    let f = unsorted();
    f.state.propose().unwrap();
    assert_eq!(
        f.state.apply_preview().unwrap().new_playset_name,
        "Main (CK3 Helper)"
    );
    let r = f.state.apply(now()).unwrap();
    assert_eq!(r.new_playset_name, "Main (CK3 Helper)");
    assert_eq!(playset_count(&f.db), 2);
    assert!(Path::new(&r.backup_path).exists());
}

#[test]
fn apply_refuses_changed_playset() {
    let f = unsorted();
    f.state.propose().unwrap();
    let conn = rusqlite::Connection::open(&f.db).unwrap();
    conn.execute(
        "UPDATE playsets_mods SET enabled = 0 WHERE position = 0",
        [],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        f.state.apply(now()).unwrap_err().code,
        ErrorCode::PlaysetChanged
    );
    assert_eq!(playset_count(&f.db), 1);
}

#[test]
fn rule_edit_drops_cached_proposal() {
    let f = unsorted();
    f.state.propose().unwrap();
    f.state
        .add_rule("Parent".into(), "Child".into(), None)
        .unwrap();
    assert_eq!(
        f.state.apply(now()).unwrap_err().code,
        ErrorCode::NothingToApply
    );
}

#[test]
fn apply_blocked_by_running_launcher() {
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_name().unwrap().to_string_lossy().to_string();
    let f = setup(&[("Child", &["Parent"]), ("Parent", &[])], &name);
    f.state.propose().unwrap();
    assert_eq!(
        f.state.apply(now()).unwrap_err().code,
        ErrorCode::LauncherRunning
    );
    assert_eq!(playset_count(&f.db), 1);
}

#[test]
fn launcher_status_not_running() {
    let f = unsorted();
    assert_eq!(f.state.launcher_status(), LauncherStatus::NotRunning);
}

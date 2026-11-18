use assert_cmd::Command;
use ck3h_core::testkit::{playset_count, standard_fixture};
use predicates::str::contains;
use std::path::{Path, PathBuf};

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let user = standard_fixture(tmp.path());
    (tmp, user)
}

fn cmd(user: &Path) -> Command {
    let mut c = Command::cargo_bin("ck3h").unwrap();
    // Keep the default rules path (under the config dir) inside the sandbox.
    c.env("HOME", user).env("XDG_CONFIG_HOME", user);
    c.arg("--user-dir").arg(user);
    c
}

#[test]
fn playsets_marks_active() {
    let (_tmp, ud) = fixture();
    cmd(&ud)
        .arg("playsets")
        .assert()
        .success()
        .stdout(contains("* Main (5 mods, 5 enabled)"));
}

#[test]
fn playsets_json_is_valid() {
    let (_tmp, ud) = fixture();
    let out = cmd(&ud).args(["playsets", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v[0]["name"], "Main");
    assert_eq!(v[0]["active"], true);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn conflicts_text_output() {
    let (_tmp, ud) = fixture();
    cmd(&ud)
        .arg("conflicts")
        .assert()
        .success()
        .stdout(contains(
            "[suspicious] file events/shared_events.txt — winner: Base B (lower in playset); losers: Base A\n",
        ))
        .stdout(contains("missing on disk: Ghost"))
        .stdout(contains("dependency missing: Broken -> Missing Mod"))
        .stdout(contains("compatch: A+B Compatch"));
}

#[test]
fn conflicts_json_is_valid() {
    let (_tmp, ud) = fixture();
    let out = cmd(&ud).args(["conflicts", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["conflicts"].as_array().unwrap().len() >= 2);
}

#[test]
fn sort_dry_run_does_not_write() {
    let (_tmp, ud) = fixture();
    cmd(&ud)
        .arg("sort")
        .assert()
        .success()
        .stdout(contains(
            "moves: 0 (the current order already satisfies dependencies and rules)",
        ))
        .stdout(contains("winner changes: 0"));
    assert_eq!(playset_count(&ud.join("launcher-v2.sqlite")), 1);
}

fn swap_rules(dir: &Path) -> PathBuf {
    let rules = dir.join("rules.toml");
    std::fs::write(
        &rules,
        "[[rule]]\nbefore = \"Base B\"\nafter = \"Base A\"\nnote = \"B first\"\n",
    )
    .unwrap();
    rules
}

#[test]
fn sort_reports_moves_from_rules() {
    let (tmp, ud) = fixture();
    let rules = swap_rules(tmp.path());
    cmd(&ud)
        .arg("sort")
        .arg("--rules")
        .arg(&rules)
        .assert()
        .success()
        .stdout(contains("moves: 2\n"))
        .stdout(contains(
            "move Base B: #2 -> #1 (your rule: before Base A — B first)",
        ))
        .stdout(contains(
            "move Base A: #1 -> #2 (your rule: after Base B — B first)",
        ));
}

#[test]
fn sort_apply_pins_the_outcome_for_the_launcher_state() {
    let (tmp, ud) = fixture();
    let rules = swap_rules(tmp.path());
    let launcher_running =
        !ck3h_core::apply::launcher_processes(ck3h_core::apply::DEFAULT_LAUNCHER_PROCESS_NAMES)
            .is_empty();
    let out = cmd(&ud)
        .args(["sort", "--apply", "--rules"])
        .arg(&rules)
        .output()
        .unwrap();
    let db = ud.join("launcher-v2.sqlite");
    if launcher_running {
        assert_eq!(out.status.code(), Some(2));
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.contains("Close the Paradox Launcher first: "),
            "{stderr}"
        );
        assert_eq!(playset_count(&db), 1);
    } else {
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        let line = stdout
            .lines()
            .find(|l| l.starts_with("created playset \"Main (CK3 Helper)\"; backup: "))
            .unwrap_or_else(|| panic!("no created-playset line in {stdout}"));
        let backup = Path::new(line.rsplit("backup: ").next().unwrap());
        assert!(backup.is_file(), "backup missing: {}", backup.display());
        assert_eq!(playset_count(&db), 2);
    }
}

#[test]
fn calibrate_without_game_and_no_default_fails() {
    let (_tmp, ud) = fixture();
    write_log(&ud);
    cmd(&ud).arg("calibrate").assert().code(1).stderr(contains(
        "error: calibrate needs the game folder: pass --game <path to game/>",
    ));
}

#[test]
fn calibrate_rejects_invalid_game_folder() {
    let (tmp, ud) = fixture();
    write_log(&ud);
    let bad = tmp.path().join("not_a_game");
    std::fs::create_dir_all(&bad).unwrap();
    cmd(&ud)
        .arg("calibrate")
        .arg("--game")
        .arg(&bad)
        .assert()
        .code(1)
        .stderr(contains("error: not a CK3 game folder (no common/): "))
        .stderr(contains("not_a_game"));
}

#[test]
fn calibrate_prints_single_definition_lines() {
    let (tmp, ud) = fixture();
    // only_a has one definition in our playset: the log claims an override we cannot see.
    std::fs::create_dir_all(ud.join("logs")).unwrap();
    std::fs::write(
        ud.join("logs/database_conflicts.log"),
        "[01:59:17][W][game_database.h:272]: Overriding entry 'only_a' for database 'common/traits/' in 'file: common/traits/zz_base_a.txt line: 1'\n",
    )
    .unwrap();
    let game = tmp.path().join("game");
    std::fs::create_dir_all(game.join("common")).unwrap();
    cmd(&ud)
        .arg("calibrate")
        .arg("--game")
        .arg(&game)
        .assert()
        .success()
        .stdout(contains("logged with a single definition here: 1"))
        .stdout(contains("  common/traits: 1"));
}

#[test]
fn unknown_playset_fails() {
    let (_tmp, ud) = fixture();
    cmd(&ud)
        .args(["conflicts", "--playset", "Nope"])
        .assert()
        .code(1)
        .stderr(contains("Nope"));
}

#[test]
fn missing_database_fails_without_panic() {
    let tmp = tempfile::tempdir().unwrap();
    cmd(tmp.path()).arg("playsets").assert().code(1);
}

fn write_log(ud: &Path) {
    std::fs::create_dir_all(ud.join("logs")).unwrap();
    std::fs::write(
        ud.join("logs/database_conflicts.log"),
        "[01:59:17][W][game_database.h:272]: Overriding entry 'shared_trait' for database 'common/traits/' in 'file: common/traits/zz_base_a.txt line: 1'\n",
    )
    .unwrap();
}

#[test]
fn calibrate_reads_log() {
    let (tmp, ud) = fixture();
    write_log(&ud);
    let game = tmp.path().join("game");
    std::fs::create_dir_all(game.join("common")).unwrap();
    cmd(&ud)
        .arg("calibrate")
        .arg("--game")
        .arg(&game)
        .assert()
        .success()
        .stdout(contains("matched: 1"));
}

#[test]
fn calibrate_without_log_fails() {
    let (tmp, ud) = fixture();
    let game = tmp.path().join("game");
    std::fs::create_dir_all(game.join("common")).unwrap();
    cmd(&ud)
        .arg("calibrate")
        .arg("--game")
        .arg(&game)
        .assert()
        .code(1);
}

#[test]
fn closed_stdout_pipe_is_not_a_panic() {
    let (_tmp, ud) = fixture();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_ck3h"))
        .arg("--user-dir")
        .arg(&ud)
        .env("HOME", &ud)
        .env("XDG_CONFIG_HOME", &ud)
        .arg("conflicts")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take()); // the reader goes away before the child writes
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(out.status.code(), Some(0), "{stderr}");
}

#[test]
fn explicit_missing_rules_file_fails() {
    let (tmp, ud) = fixture();
    let missing = tmp.path().join("nope_rules.toml");
    cmd(&ud)
        .arg("sort")
        .arg("--rules")
        .arg(&missing)
        .assert()
        .code(1)
        .stderr(contains("nope_rules.toml"));
}

#[test]
fn misspelled_rules_file_fails_naming_the_file() {
    let (tmp, ud) = fixture();
    let rules = tmp.path().join("typo_rules.toml");
    std::fs::write(&rules, "[[rules]]\nbefore = \"A\"\nafter = \"B\"\n").unwrap();
    cmd(&ud)
        .arg("sort")
        .arg("--rules")
        .arg(&rules)
        .assert()
        .code(1)
        .stderr(contains("typo_rules.toml"));
}

#[test]
fn index_warnings_are_printed() {
    let (_tmp, ud) = fixture();
    for args in [&["conflicts"][..], &["sort"][..]] {
        cmd(&ud)
            .args(args)
            .assert()
            .success()
            .stdout(contains("warning: Broken: "));
    }
}

#[test]
fn usage_errors_do_not_use_exit_code_2() {
    let (_tmp, ud) = fixture();
    cmd(&ud).arg("bogus").assert().code(1);
    cmd(&ud).arg("--help").assert().code(0);
}

#[test]
fn invalid_game_folder_fails_for_conflicts_and_sort() {
    let (tmp, ud) = fixture();
    let bad = tmp.path().join("no_such_game");
    for sub in ["conflicts", "sort"] {
        cmd(&ud)
            .arg(sub)
            .arg("--game")
            .arg(&bad)
            .assert()
            .code(1)
            .stderr(contains("not a CK3 game folder (no common/): "))
            .stderr(contains("no_such_game"));
    }
}

#[test]
fn sort_apply_with_sorted_order_writes_nothing() {
    let (_tmp, ud) = fixture();
    cmd(&ud)
        .args(["sort", "--apply"])
        .assert()
        .success()
        .stdout(contains("nothing to apply: the order is already sorted"));
    assert_eq!(playset_count(&ud.join("launcher-v2.sqlite")), 1);
}

#[test]
fn help_describes_json_and_apply() {
    let (_tmp, ud) = fixture();
    cmd(&ud)
        .args(["sort", "--help"])
        .assert()
        .success()
        .stdout(contains("only flag that writes"))
        .stdout(contains("Print JSON"));
}

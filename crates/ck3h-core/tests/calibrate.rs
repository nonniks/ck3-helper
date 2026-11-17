#![cfg(feature = "testkit")]

use ck3h_core::calibrate::*;
use ck3h_core::launcher::LauncherDb;
use ck3h_core::playset::{ModEntry, load_playset};
use ck3h_core::testkit::*;
use std::path::Path;

fn load(user: &Path) -> Vec<ModEntry> {
    let db = LauncherDb::open_read_only(&user.join("launcher-v2.sqlite")).unwrap();
    let playset = db.active_playset().unwrap();
    load_playset(&db, &playset, "l_english").unwrap()
}

fn log_line(key: &str, db: &str, file: &str) -> String {
    format!(
        "[01:59:17][W][game_database.h:272]: Overriding entry '{key}' for database '{db}' in 'file: {file} line: 1'\n"
    )
}

#[test]
fn calibrate_counts_all_categories() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = load(&standard_fixture(tmp.path()));
    // Base A (zz_base_a.txt) loads after Base B (00_base_b.txt): shared_trait is won by A.
    let text = [
        log_line(
            "shared_trait",
            "common/traits/",
            "common/traits/zz_base_a.txt",
        ),
        log_line("only_a", "common/traits", "common/traits/00_base_b.txt"),
        log_line("ghost_key", "common/traits", "common/traits/x.txt"),
    ]
    .concat();
    let r = calibrate(&parse_conflicts_log(&text), &mods, None, None);
    assert_eq!(
        (
            r.matched,
            r.mismatched.len(),
            r.log_unknown["common/traits"]
        ),
        (1, 1, 1)
    );
    assert_eq!(
        r.mismatched[0].predicted_file,
        "common/traits/zz_base_a.txt"
    );
    assert!(r.predicted_not_logged.is_empty());
    assert!(!r.truncated);

    // Without the shared_trait line, the contested pair is reported as predicted-not-logged.
    let r = calibrate(
        &parse_conflicts_log(&log_line("ghost_key", "common/traits", "x.txt")),
        &mods,
        None,
        None,
    );
    assert_eq!(
        r.predicted_not_logged,
        [("common/traits".to_string(), "shared_trait".to_string())]
    );
}

#[test]
fn last_run_mismatch_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let user = standard_fixture(tmp.path());
    let mods = load(&user);
    let other = tmp.path().join("elsewhere");
    std::fs::create_dir_all(user.join("mod")).unwrap();
    std::fs::write(
        user.join("mod/ugc_1.mod"),
        format!("name=\"X\"\npath=\"{}\"\n", other.display()),
    )
    .unwrap();
    std::fs::write(
        user.join("dlc_load.json"),
        r#"{"enabled_mods":["mod/ugc_1.mod","mod/missing.mod"],"disabled_dlcs":[]}"#,
    )
    .unwrap();
    let rc = check_last_run(&user, &mods).unwrap();
    assert_eq!(rc.only_in_last_run.len(), 2);
    assert!(rc.only_in_last_run.contains(&"mod/missing.mod".to_string()));
    assert_eq!(rc.not_in_last_run.len(), 5);
}

#[test]
fn last_run_matches_enabled_mods_and_tolerates_bad_input() {
    let tmp = tempfile::tempdir().unwrap();
    let user = standard_fixture(tmp.path());
    let mods = load(&user);
    assert_eq!(check_last_run(&user, &mods), None);
    std::fs::write(user.join("dlc_load.json"), "{ not json").unwrap();
    assert_eq!(check_last_run(&user, &mods), None);

    std::fs::create_dir_all(user.join("mod")).unwrap();
    let mut listed = Vec::new();
    for (i, m) in mods
        .iter()
        .filter(|m| m.name == "Base A" || m.name == "Base B")
        .enumerate()
    {
        std::fs::write(
            user.join(format!("mod/m{i}.mod")),
            format!("name=\"{}\"\npath=\"{}\"\n", m.name, m.dir.display()),
        )
        .unwrap();
        listed.push(format!("mod/m{i}.mod"));
    }
    let json = serde_json::json!({ "enabled_mods": listed });
    std::fs::write(user.join("dlc_load.json"), json.to_string()).unwrap();
    let rc = check_last_run(&user, &mods).unwrap();
    assert!(rc.only_in_last_run.is_empty());
    assert_eq!(rc.not_in_last_run.len(), 3);
}

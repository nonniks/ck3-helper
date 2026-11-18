use ck3h_app::dto::{AnalysisView, MarkState, VerdictView};
use ck3h_app::error::ErrorCode;
use ck3h_app::rules_file::{append_rule, remove_rule};
use ck3h_app::settings::{Settings, save_settings};
use ck3h_app::state::AppState;
use ck3h_core::paths::RULES_FILE;
use ck3h_core::sort::{Rule, parse_rules};
use ck3h_core::testkit::{
    add_mod, add_playset, add_playset_mod, create_launcher_db, standard_fixture, write_mod,
};
use std::path::{Path, PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    config: tempfile::TempDir,
    state: AppState,
}

impl Fixture {
    fn rules_path(&self) -> PathBuf {
        self.config.path().join(RULES_FILE)
    }
}

fn setup(user: impl FnOnce(&Path) -> PathBuf) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let user = user(root.path());
    let game = write_mod(root.path(), "game", &[]);
    save_settings(
        config.path(),
        &Settings {
            user_dir: Some(user),
            game_dir: Some(game),
            ..Settings::default()
        },
    )
    .unwrap();
    let state = AppState::new(config.path().to_path_buf());
    Fixture {
        _root: root,
        config,
        state,
    }
}

fn rule(before: &str, after: &str, note: Option<&str>) -> Rule {
    Rule {
        before: before.into(),
        after: after.into(),
        note: note.map(Into::into),
    }
}

fn id(v: &AnalysisView, name: &str) -> String {
    v.mods.iter().find(|m| m.name == name).unwrap().id.clone()
}

fn befores(path: &Path) -> Vec<String> {
    parse_rules(&std::fs::read_to_string(path).unwrap())
        .unwrap()
        .into_iter()
        .map(|r| r.before)
        .collect()
}

#[test]
fn append_keeps_comments_and_escapes_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("rules.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "# my pins\n[[rule]]\nbefore = \"X\"\nafter = \"Y\" # why\n",
    )
    .unwrap();
    append_rule(
        &path,
        &rule("Аутентичные \"шрифты\"", "steam:123", Some("n")),
    )
    .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# my pins") && text.contains("# why"));
    let rules = parse_rules(&text).unwrap();
    assert_eq!(rules[1].before, "Аутентичные \"шрифты\"");
    assert_eq!(rules[1].note.as_deref(), Some("n"));
}

#[test]
fn append_creates_missing_file_and_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new").join("rules.toml");
    append_rule(&path, &rule("A", "B", None)).unwrap();
    assert_eq!(
        parse_rules(&std::fs::read_to_string(&path).unwrap()).unwrap(),
        [rule("A", "B", None)]
    );
}

#[test]
fn remove_by_index_keeps_others() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rules.toml");
    std::fs::write(
        &path,
        "[[rule]]\nbefore = \"A\"\nafter = \"X\"\n\n# keep\n[[rule]]\nbefore = \"B\"\nafter = \"X\"\n\n[[rule]]\nbefore = \"C\"\nafter = \"X\"\n",
    )
    .unwrap();
    remove_rule(&path, 1).unwrap();
    assert_eq!(befores(&path), ["A", "C"]);
    let err = remove_rule(&path, 5).unwrap_err();
    assert_eq!(
        (err.code, err.items),
        (ErrorCode::Rules, vec!["5".to_string()])
    );
}

#[test]
fn broken_rules_file_is_left_untouched() {
    let f = setup(standard_fixture);
    let rules = f.rules_path();
    std::fs::write(&rules, "[[rule]\n").unwrap();
    let r = rule("A", "B", None);
    assert_eq!(append_rule(&rules, &r).unwrap_err().code, ErrorCode::Rules);
    assert_eq!(remove_rule(&rules, 0).unwrap_err().code, ErrorCode::Rules);
    assert_eq!(std::fs::read_to_string(&rules).unwrap(), "[[rule]\n");
    assert!(
        f.state
            .analyze(None, &|_| {})
            .unwrap()
            .rules_error
            .is_some()
    );
    assert!(f.state.rules().error.is_some());
}

#[cfg(not(target_os = "linux"))]
#[test]
fn mark_intended_turns_pair_intended() {
    let f = setup(standard_fixture);
    let v = f.state.analyze(None, &|_| {}).unwrap();
    let (a, b) = (id(&v, "Base A"), id(&v, "Base B"));
    f.state
        .mark_intended(&a, &b, "intended: Base A overrides Base B".into())
        .unwrap();
    let v = f.state.summary().unwrap();
    let pair = |w: &str, l: &str| {
        v.pairs
            .iter()
            .find(|p| p.winner.name == w && p.loser.name == l)
            .unwrap()
    };
    assert_eq!(pair("Base A", "Base B").verdict, VerdictView::Intended);
    assert_eq!(pair("Base B", "Base A").mark, MarkState::Cycle);
    assert_eq!(
        f.state.mark_intended(&b, &a, "x".into()).unwrap_err().code,
        ErrorCode::Cycle
    );
    assert_eq!(
        parse_rules(&std::fs::read_to_string(f.rules_path()).unwrap()).unwrap(),
        [rule(
            "Base B",
            "Base A",
            Some("intended: Base A overrides Base B")
        )]
    );
    assert!(f.state.rules().rules[0].applied);
}

#[test]
fn mark_intended_rejects_unknown_mod() {
    let f = setup(standard_fixture);
    assert_eq!(
        f.state
            .mark_intended("a", "b", "x".into())
            .unwrap_err()
            .code,
        ErrorCode::NotAnalyzed
    );
    let v = f.state.analyze(None, &|_| {}).unwrap();
    let err = f
        .state
        .mark_intended(&id(&v, "Base A"), "nope", "x".into())
        .unwrap_err();
    assert_eq!(
        (err.code, err.items),
        (ErrorCode::Rules, vec!["nope".to_string()])
    );
    assert!(!f.rules_path().exists());
}

#[test]
fn workshop_mods_are_referenced_by_steam_id() {
    let f = setup(|root| {
        let user = root.join("user");
        std::fs::create_dir_all(&user).unwrap();
        let conn = create_launcher_db(&user.join("launcher-v2.sqlite"));
        let playset = add_playset(&conn, "Main", true);
        for (pos, (name, steam)) in [("First", "111"), ("Second", "222")]
            .into_iter()
            .enumerate()
        {
            let dir = write_mod(
                &root.join("mods"),
                steam,
                &[
                    ("descriptor.mod", format!("name=\"{name}\"\n").as_bytes()),
                    ("common/traits/x.txt", b"shared_trait = { }\n"),
                ],
            );
            let m = add_mod(&conn, name, &dir, Some(steam));
            add_playset_mod(&conn, &playset, &m, pos as i64, true);
        }
        user
    });
    let v = f.state.analyze(None, &|_| {}).unwrap();
    f.state
        .mark_intended(&id(&v, "Second"), &id(&v, "First"), "n".into())
        .unwrap();
    let rules = parse_rules(&std::fs::read_to_string(f.rules_path()).unwrap()).unwrap();
    assert_eq!(rules[0].before, "steam:111");
    assert_eq!(rules[0].after, "steam:222");
}

#[test]
fn rules_view_marks_unapplied() {
    let f = setup(standard_fixture);
    assert!(f.state.rules().rules.is_empty());
    f.state.analyze(None, &|_| {}).unwrap();
    let view = f
        .state
        .add_rule("Nope".into(), "Base A".into(), None)
        .unwrap();
    assert_eq!(view.path, f.rules_path().display().to_string());
    assert!(!view.rules[0].applied);
    let view = f
        .state
        .add_rule("Base A".into(), "Base B".into(), None)
        .unwrap();
    assert!(!view.rules[0].applied && view.rules[1].applied);
    let view = f.state.remove_rule(0).unwrap();
    assert_eq!(view.rules.len(), 1);
    assert_eq!((view.rules[0].index, view.rules[0].applied), (0, true));
}

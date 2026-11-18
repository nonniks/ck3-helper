use ck3h_app::dto::{
    AnalysisView, ConflictQuery, DepIssueView, LevelView, MarkState, ModView, Progress, SourceKind,
    VerdictView,
};
use ck3h_app::error::ErrorCode;
use ck3h_app::settings::{Settings, load_settings, save_settings};
use ck3h_app::state::AppState;
use ck3h_core::testkit::{standard_fixture, write_mod};
use std::sync::Mutex;
use tempfile::TempDir;

const GAME: &[(&str, &[u8])] = &[("common/traits/00_traits.txt", b"only_a = {}")];

struct Fixture {
    root: TempDir,
    config: TempDir,
    state: AppState,
}

/// The standard fixture as `userDir`, `root/game` holding `game_files` as `gameDir`.
fn fixture(game_files: &[(&str, &[u8])]) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let user = standard_fixture(root.path());
    let game = write_mod(root.path(), "game", game_files);
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
        root,
        config,
        state,
    }
}

fn m<'a>(v: &'a AnalysisView, name: &str) -> &'a ModView {
    v.mods.iter().find(|m| m.name == name).unwrap()
}

fn id(v: &AnalysisView, name: &str) -> String {
    m(v, name).id.clone()
}

fn all() -> ConflictQuery {
    ConflictQuery {
        text: String::new(),
        levels: vec![],
        verdicts: vec![],
    }
}

#[test]
fn analyze_active_playset_summary() {
    let f = fixture(GAME);
    let v = f.state.analyze(None, &|_| {}).unwrap();
    assert_eq!(v.playset.name, "Main");
    assert_eq!(v.mods.len(), 5);
    assert_eq!(v.lang, "l_english");
    assert!(!v.vanilla);
    assert_eq!(v.rules_error, None);
    assert!(m(&v, "Ghost").missing);
    assert!(m(&v, "A+B Compatch").compatch);
    assert_eq!(
        m(&v, "Broken").dep_issues,
        [DepIssueView {
            dependency: "Missing Mod".into(),
            ambiguous: false
        }]
    );
    assert_eq!(m(&v, "Broken").warnings, 1);
    assert_eq!(
        v.mods.iter().map(|m| m.position).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
}

#[test]
fn analyze_playset_by_id() {
    let f = fixture(GAME);
    let playset = f.state.list_playsets().unwrap().remove(0);
    let v = f.state.analyze(Some(&playset.id), &|_| {}).unwrap();
    assert_eq!(v.playset.id, playset.id);
    assert_eq!(
        f.state
            .analyze(Some("no-such-playset"), &|_| {})
            .unwrap_err()
            .code,
        ErrorCode::PlaysetNotFound
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn pairs_and_counts() {
    let f = fixture(GAME);
    let v = f.state.analyze(None, &|_| {}).unwrap();
    assert_eq!(v.pairs.len(), 2);
    let p = &v.pairs[0];
    assert_eq!(
        (
            p.winner.name.as_str(),
            p.loser.name.as_str(),
            p.objects,
            p.loc,
            p.mark
        ),
        ("Base A", "Base B", 1, 1, MarkState::Available)
    );
    assert_eq!(v.counts.suspicious, 3);
    assert_eq!((v.counts.files, v.counts.objects, v.counts.loc), (1, 1, 1));
}

#[test]
fn progress_reaches_total() {
    let f = fixture(GAME);
    let events = Mutex::new(Vec::new());
    f.state
        .analyze(None, &|p| events.lock().unwrap().push(p))
        .unwrap();
    let events = events.into_inner().unwrap();
    assert!(events.contains(&Progress::Mods { done: 5, total: 5 }));
    assert!(!events.contains(&Progress::Vanilla));
    assert_eq!(events.last(), Some(&Progress::Conflicts));
}

#[test]
fn pair_page_has_loc_texts_and_split_targets() {
    let f = fixture(GAME);
    let v = f.state.analyze(None, &|_| {}).unwrap();
    let (a, b) = (id(&v, "Base A"), id(&v, "Base B"));
    let page = f.state.pair_conflicts(Some(&a), Some(&b), 0, 50).unwrap();
    assert_eq!((page.total, page.offset), (2, 0));
    let loc = page
        .items
        .iter()
        .find(|c| c.level == LevelView::Loc)
        .unwrap();
    assert_eq!(
        (loc.winner.text.as_deref(), loc.losers[0].text.as_deref()),
        (Some("From A"), Some("From B"))
    );
    assert_eq!(
        (loc.database.as_deref(), loc.key.as_str()),
        (None, "shared_key")
    );
    let obj = page
        .items
        .iter()
        .find(|c| c.level == LevelView::Object)
        .unwrap();
    assert_eq!(
        (
            obj.database.as_deref(),
            obj.key.as_str(),
            obj.winner.file.as_str()
        ),
        (
            Some("common/traits"),
            "shared_trait",
            "common/traits/zz_base_a.txt"
        )
    );
    assert_eq!(obj.winner.text, None);
}

#[test]
fn pages_past_end_are_empty_with_total() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    let p = f.state.search_conflicts(&all(), 100, 10).unwrap();
    assert!(p.items.is_empty());
    assert_eq!(p.offset, 100);
    assert_eq!(
        p.total,
        f.state
            .search_conflicts(&all(), 0, 1000)
            .unwrap()
            .items
            .len()
    );
    let first = f.state.search_conflicts(&all(), 0, 1).unwrap();
    assert_eq!((first.items.len(), first.total), (1, p.total));
}

#[test]
fn search_filters_text_level_verdict() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    let p = f
        .state
        .search_conflicts(
            &ConflictQuery {
                text: "SHARED_KEY".into(),
                levels: vec![LevelView::Loc],
                verdicts: vec![],
            },
            0,
            10,
        )
        .unwrap();
    assert_eq!(p.total, 1);
    assert_eq!(p.items[0].target, "l_english::shared_key");
    let intended = ConflictQuery {
        verdicts: vec![VerdictView::Intended],
        ..all()
    };
    assert_eq!(f.state.search_conflicts(&intended, 0, 10).unwrap().total, 0);
}

#[test]
fn vanilla_toggle_needs_game_dir() {
    let f = fixture(&[]);
    f.state.analyze(None, &|_| {}).unwrap();
    assert_eq!(
        f.state.set_vanilla(true, &|_| {}).unwrap_err().code,
        ErrorCode::NotGameDir
    );
    assert!(!load_settings(f.config.path()).settings.show_vanilla);
    assert!(!f.state.summary().unwrap().vanilla);
}

#[test]
fn analyze_without_game_folder_skips_vanilla_and_keeps_setting() {
    let f = fixture(&[]);
    let mut s = f.state.settings();
    s.show_vanilla = true;
    f.state.save_settings(s).unwrap();
    let v = f.state.analyze(None, &|_| {}).unwrap();
    assert!(!v.vanilla);
    assert!(v.pairs.iter().all(|p| p.loser.kind != SourceKind::Vanilla));
    assert!(f.state.settings().show_vanilla);
    assert!(load_settings(f.config.path()).settings.show_vanilla);
}

#[test]
fn vanilla_toggle_adds_vanilla_pair_and_persists() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    let v = f.state.set_vanilla(true, &|_| {}).unwrap();
    assert!(v.vanilla);
    let p = v
        .pairs
        .iter()
        .find(|p| p.loser.kind == SourceKind::Vanilla)
        .unwrap();
    assert_eq!(p.verdict, VerdictView::Vanilla);
    assert_eq!((p.loser.id.as_deref(), p.loser.name.as_str()), (None, ""));
    assert_eq!(p.mark, MarkState::NotApplicable);
    assert!(load_settings(f.config.path()).settings.show_vanilla);
    assert!(f.state.settings().show_vanilla);
}

#[test]
fn vanilla_index_is_reused_and_analysis_follows_setting() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    let events = Mutex::new(Vec::new());
    f.state
        .set_vanilla(true, &|p| events.lock().unwrap().push(p))
        .unwrap();
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [Progress::Vanilla, Progress::Conflicts]
    );
    let off = f.state.set_vanilla(false, &|_| {}).unwrap();
    assert!(!off.vanilla);
    assert!(
        off.pairs
            .iter()
            .all(|p| p.loser.kind != SourceKind::Vanilla)
    );
    assert!(!load_settings(f.config.path()).settings.show_vanilla);

    events.lock().unwrap().clear();
    f.state
        .set_vanilla(true, &|p| events.lock().unwrap().push(p))
        .unwrap();
    assert_eq!(events.lock().unwrap().as_slice(), [Progress::Conflicts]);

    // A new analysis takes vanilla from the setting and the cached index.
    events.lock().unwrap().clear();
    let v = f
        .state
        .analyze(None, &|p| events.lock().unwrap().push(p))
        .unwrap();
    assert!(v.vanilla);
    assert!(!events.lock().unwrap().contains(&Progress::Vanilla));
}

#[test]
fn vanilla_loc_text_comes_from_vanilla_index() {
    let f = fixture(&[
        (
            "localization/english/game_l_english.yml",
            "\u{feff}l_english:\n shared_key:0 \"From game\"\n".as_bytes(),
        ),
        ("common/traits/00_traits.txt", b"only_a = {}"),
    ]);
    f.state.analyze(None, &|_| {}).unwrap();
    f.state.set_vanilla(true, &|_| {}).unwrap();
    let p = f
        .state
        .search_conflicts(
            &ConflictQuery {
                text: "shared_key".into(),
                ..all()
            },
            0,
            10,
        )
        .unwrap();
    let c = &p.items[0];
    let vanilla = c
        .losers
        .iter()
        .chain([&c.winner])
        .find(|x| x.source.kind == SourceKind::Vanilla)
        .unwrap();
    assert_eq!(vanilla.text.as_deref(), Some("From game"));
}

#[test]
fn summary_before_analysis_is_not_analyzed() {
    let f = fixture(GAME);
    assert_eq!(f.state.summary().unwrap_err().code, ErrorCode::NotAnalyzed);
    assert_eq!(
        f.state.search_conflicts(&all(), 0, 10).unwrap_err().code,
        ErrorCode::NotAnalyzed
    );
    assert_eq!(
        f.state.pair_conflicts(None, None, 0, 10).unwrap_err().code,
        ErrorCode::NotAnalyzed
    );
    assert_eq!(
        f.state.mod_warnings("x").unwrap_err().code,
        ErrorCode::NotAnalyzed
    );
    assert_eq!(
        f.state.set_vanilla(true, &|_| {}).unwrap_err().code,
        ErrorCode::NotAnalyzed
    );
}

#[test]
fn changing_analysis_settings_resets_analysis() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    let mut s = f.state.settings();
    s.ui_lang = Some("ru".into());
    f.state.save_settings(s.clone()).unwrap();
    assert!(f.state.summary().is_ok());
    s.analysis_lang = Some("l_russian".into());
    f.state.save_settings(s).unwrap();
    assert_eq!(f.state.summary().unwrap_err().code, ErrorCode::NotAnalyzed);
}

#[test]
fn changing_game_dir_resets_analysis() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    let mut s = f.state.settings();
    s.game_dir = Some(write_mod(f.root.path(), "game2", GAME));
    f.state.save_settings(s).unwrap();
    assert_eq!(f.state.summary().unwrap_err().code, ErrorCode::NotAnalyzed);
}

#[test]
fn changing_analysis_lang_drops_vanilla_index() {
    let f = fixture(GAME);
    f.state.analyze(None, &|_| {}).unwrap();
    f.state.set_vanilla(true, &|_| {}).unwrap();
    let mut s = f.state.settings();
    s.analysis_lang = Some("l_russian".into());
    f.state.save_settings(s.clone()).unwrap();
    // Back to the game's language: the same (game folder, language) as the dropped index.
    s.analysis_lang = None;
    f.state.save_settings(s).unwrap();
    let events = Mutex::new(Vec::new());
    f.state
        .analyze(None, &|p| events.lock().unwrap().push(p))
        .unwrap();
    assert!(events.lock().unwrap().contains(&Progress::Vanilla));
}

#[test]
fn broken_rules_file_runs_without_rules() {
    let f = fixture(GAME);
    std::fs::write(f.config.path().join("rules.toml"), "[[rule]\n").unwrap();
    let v = f.state.analyze(None, &|_| {}).unwrap();
    assert!(v.rules_error.is_some());
    assert!(!v.pairs.is_empty());
}

#[test]
fn mod_warnings_list_index_warnings() {
    let f = fixture(GAME);
    let v = f.state.analyze(None, &|_| {}).unwrap();
    let w = f.state.mod_warnings(&id(&v, "Broken")).unwrap();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].file, "common/traits/bad.txt");
    assert!(f.state.mod_warnings(&id(&v, "Base A")).unwrap().is_empty());
}

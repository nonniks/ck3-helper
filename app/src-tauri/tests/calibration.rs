use ck3h_app::dto::UpdateView;
use ck3h_app::error::ErrorCode;
use ck3h_app::settings::{Settings, save_settings};
use ck3h_app::state::AppState;
use ck3h_core::session::CONFLICTS_LOG;
use ck3h_core::testkit::{standard_fixture, write_mod};
use tempfile::TempDir;

struct Fixture {
    _root: TempDir,
    _config: TempDir,
    user: std::path::PathBuf,
    state: AppState,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let user = standard_fixture(root.path());
    let game = write_mod(
        root.path(),
        "game",
        &[("common/traits/00_traits.txt", b"shared_trait = {}")],
    );
    save_settings(
        config.path(),
        &Settings {
            user_dir: Some(user.clone()),
            game_dir: Some(game),
            ..Settings::default()
        },
    )
    .unwrap();
    let state = AppState::new(config.path().to_path_buf());
    Fixture {
        _root: root,
        _config: config,
        user,
        state,
    }
}

fn write_log(f: &Fixture, text: &str) {
    let path = f.user.join(CONFLICTS_LOG);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[test]
fn calibrate_matches_log_line() {
    let f = fixture();
    write_log(
        &f,
        "[01:59:17][W][game_database.h:272]: Overriding entry 'shared_trait' for database 'common/traits/' in 'file: common/traits/zz_base_a.txt line: 1'\n",
    );
    f.state.analyze(None, &|_| {}).unwrap();
    let v = f.state.calibrate(&|_| {}).unwrap();
    assert_eq!(v.matched, 1);
    assert!(v.mismatched.is_empty());
}

#[test]
fn calibrate_without_log_is_log_not_found() {
    let f = fixture();
    f.state.analyze(None, &|_| {}).unwrap();
    assert_eq!(
        f.state.calibrate(&|_| {}).unwrap_err().code,
        ErrorCode::LogNotFound
    );
}

#[test]
fn calibrate_needs_analysis() {
    let f = fixture();
    assert_eq!(
        f.state.calibrate(&|_| {}).unwrap_err().code,
        ErrorCode::NotAnalyzed
    );
}

const REPO: &str = "https://github.com/o/ck3-helper";

/// A release fetcher that never reaches the network: records the API URLs it was asked for.
struct FakeFetch {
    calls: std::cell::RefCell<Vec<String>>,
    answer: Option<UpdateView>,
}

impl FakeFetch {
    fn new(answer: Option<UpdateView>) -> Self {
        Self {
            calls: Default::default(),
            answer,
        }
    }

    fn fetch(&self, api: &str) -> Result<Option<UpdateView>, String> {
        self.calls.borrow_mut().push(api.to_string());
        Ok(self.answer.clone())
    }
}

#[test]
fn check_update_without_repository_does_nothing() {
    let f = fixture();
    let fake = FakeFetch::new(None);
    assert_eq!(
        f.state.check_update_with("", 1_000_000, |a| fake.fetch(a)),
        None
    );
    assert_eq!(f.state.settings().update_checked_at, None);
    assert!(fake.calls.borrow().is_empty());
}

#[test]
fn check_update_when_due_fetches_once_and_records_the_day() {
    let f = fixture();
    let release = UpdateView {
        version: "9.9.9".into(),
        url: "https://example.test/r".into(),
    };
    let fake = FakeFetch::new(Some(release.clone()));
    assert_eq!(
        f.state
            .check_update_with(REPO, 1_000_000, |a| fake.fetch(a)),
        Some(release)
    );
    assert_eq!(
        *fake.calls.borrow(),
        ["https://api.github.com/repos/o/ck3-helper/releases/latest"]
    );
    assert_eq!(f.state.settings().update_checked_at, Some(1_000_000));
}

#[test]
fn check_update_not_due_does_nothing() {
    let f = fixture();
    let mut s = f.state.settings();
    s.update_checked_at = Some(1_000_000);
    f.state.save_settings(s).unwrap();
    let fake = FakeFetch::new(None);
    assert_eq!(
        f.state
            .check_update_with(REPO, 1_000_100, |a| fake.fetch(a)),
        None
    );
    assert_eq!(f.state.settings().update_checked_at, Some(1_000_000));
    assert!(fake.calls.borrow().is_empty());
}

#[test]
fn check_update_leaves_broken_settings_alone() {
    let config = tempfile::tempdir().unwrap();
    let file = config.path().join("settings.toml");
    std::fs::write(&file, "this is = = not toml").unwrap();
    let state = AppState::new(config.path().to_path_buf());
    // Not `setup()`: with broken settings it falls back to the real default CK3 folders.
    assert!(state.settings_error().is_some());
    let fake = FakeFetch::new(None);
    assert_eq!(
        state.check_update_with(REPO, 1_000_000, |a| fake.fetch(a)),
        None
    );
    assert!(fake.calls.borrow().is_empty());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "this is = = not toml"
    );
    assert!(!config.path().join("settings.toml.bak").exists());
    assert!(state.settings_error().is_some());
}

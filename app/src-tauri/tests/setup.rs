use ck3h_app::dto::PlaysetRow;
use ck3h_app::error::{AppError, ErrorCode};
use ck3h_app::settings::{Settings, SettingsFile, load_settings, save_settings};
use ck3h_app::state::AppState;
use ck3h_core::error::CoreError;
use ck3h_core::testkit::standard_fixture;
use std::path::Path;

fn state_for(config: &Path, user_dir: &Path, game_dir: &Path) -> AppState {
    save_settings(
        config,
        &Settings {
            user_dir: Some(user_dir.to_path_buf()),
            game_dir: Some(game_dir.to_path_buf()),
            ..Settings::default()
        },
    )
    .unwrap();
    AppState::new(config.to_path_buf())
}

fn rows(v: &[PlaysetRow]) -> Vec<(&str, bool, usize, usize)> {
    v.iter()
        .map(|r| (r.name.as_str(), r.active, r.mods, r.enabled))
        .collect()
}

#[test]
fn updates_are_checked_by_default() {
    assert!(Settings::default().check_updates);
}

#[test]
fn settings_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let s = Settings {
        user_dir: Some("/u".into()),
        game_dir: Some("/g".into()),
        analysis_lang: Some("l_russian".into()),
        ui_lang: Some("ru".into()),
        show_vanilla: true,
        check_updates: false,
        launcher_process_names: Some(vec!["Paradox Launcher".into()]),
        update_checked_at: Some(1_700_000_000),
    };
    save_settings(dir.path(), &s).unwrap();
    assert_eq!(
        load_settings(dir.path()),
        SettingsFile {
            settings: s,
            error: None
        }
    );
}

#[test]
fn broken_settings_use_defaults_and_keep_backup() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    std::fs::write(dir.join("settings.toml"), "userDir = [").unwrap();
    let f = load_settings(dir);
    assert!(f.error.is_some());
    assert_eq!(f.settings, Settings::default());
    save_settings(dir, &Settings::default()).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.toml.bak")).unwrap(),
        "userDir = ["
    );
}

#[test]
fn lists_playsets_from_configured_user_dir() {
    let root = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let user = standard_fixture(root.path());
    let state = state_for(config.path(), &user, root.path());
    assert_eq!(
        rows(&state.list_playsets().unwrap()),
        [("Main", true, 5, 5)]
    );
}

#[test]
fn missing_launcher_db_is_error_code() {
    let empty = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let state = state_for(config.path(), empty.path(), empty.path());
    assert_eq!(
        state.list_playsets().unwrap_err().code,
        ErrorCode::LauncherDbNotFound
    );
}

#[test]
fn core_errors_map_to_codes() {
    assert_eq!(
        AppError::from(CoreError::OrderMismatch).code,
        ErrorCode::PlaysetChanged
    );
    let e = AppError::from(CoreError::LauncherRunning(vec!["Paradox Launcher".into()]));
    assert_eq!(
        (e.code, e.items),
        (
            ErrorCode::LauncherRunning,
            vec!["Paradox Launcher".to_string()]
        )
    );
    assert_eq!(
        AppError::from(CoreError::NoLauncherNames).code,
        ErrorCode::Settings
    );
}

#[test]
fn setup_reports_db_and_effective_dirs() {
    let root = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let ud = standard_fixture(root.path());
    let state = state_for(config.path(), &ud, root.path());
    let s = state.setup();
    assert!(s.launcher_db_found);
    assert_eq!(s.user_dir.as_deref(), Some(ud.to_str().unwrap()));
    assert_eq!(s.game_language.as_deref(), Some("l_english"));
}

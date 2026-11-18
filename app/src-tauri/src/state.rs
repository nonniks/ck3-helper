use crate::analysis::{Analysis, VanillaIndex, matches};
use crate::dto::{
    AnalysisView, ApplyPreviewView, ApplyView, CalibrationView, ConflictQuery, ConflictView,
    DatabaseCount, IndexWarningView, LastRunView, LauncherStatus, MismatchView, ObjectRef, Page,
    PlaysetRow, Progress, ProposalView, RuleView, RulesView, SetupView, UpdateView,
};
use crate::error::{AppError, ErrorCode};
use crate::order::proposal_view;
use crate::rules_file::{append_rule, remove_rule};
use crate::settings::{Settings, SettingsFile, load_settings, save_settings};
use crate::update;
use ck3h_core::apply::{DEFAULT_LAUNCHER_PROCESS_NAMES, apply_order, launcher_processes, preview};
use ck3h_core::calibrate::{calibrate, check_last_run};
use ck3h_core::conflicts::{Source, involves};
use ck3h_core::error::CoreError;
use ck3h_core::index::ModIndex;
use ck3h_core::launcher::LAUNCHER_DB_FILE;
use ck3h_core::paths::{RULES_FILE, default_game_dir, default_user_dir, game_language};
use ck3h_core::session::{
    PlaysetSelector, analysis_lang, launcher_db, load, load_vanilla, playset_unchanged,
    read_conflicts_log,
};
use ck3h_core::sort::{Rule, load_rules, propose};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// Lock order: `settings`, `settings_error`, `analysis`, `vanilla`. A thread holding one of them
/// takes only later ones. No lock is held while mods or vanilla are indexed or progress is sent;
/// the report is computed under `vanilla` (and under `analysis` in `set_vanilla`).
pub struct AppState {
    config_dir: PathBuf,
    settings: Mutex<Settings>,
    settings_error: Mutex<Option<String>>,
    analysis: Mutex<Option<Analysis>>,
    vanilla: Mutex<Option<VanillaIndex>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn not_analyzed() -> AppError {
    AppError::new(ErrorCode::NotAnalyzed)
}

impl AppState {
    pub fn new(config_dir: PathBuf) -> Self {
        let SettingsFile { settings, error } = load_settings(&config_dir);
        Self {
            config_dir,
            settings: Mutex::new(settings),
            settings_error: Mutex::new(error),
            analysis: Mutex::new(None),
            vanilla: Mutex::new(None),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Why `settings.toml` could not be read; `None` once it is read or saved.
    pub fn settings_error(&self) -> Option<String> {
        lock(&self.settings_error).clone()
    }

    /// The configured user folder, else the platform default.
    pub fn user_dir(&self) -> Result<PathBuf, AppError> {
        self.settings()
            .user_dir
            .or_else(default_user_dir)
            .ok_or_else(|| AppError::new(ErrorCode::NoUserDir))
    }

    pub fn setup(&self) -> SetupView {
        let settings = self.settings();
        let user_dir = settings.user_dir.clone().or_else(default_user_dir);
        let game_dir = settings.game_dir.clone().or_else(default_game_dir);
        SetupView {
            launcher_db_found: user_dir
                .as_ref()
                .is_some_and(|d| d.join(LAUNCHER_DB_FILE).is_file()),
            game_language: user_dir.as_deref().and_then(game_language),
            user_dir: user_dir.map(|p| p.display().to_string()),
            game_dir: game_dir.map(|p| p.display().to_string()),
            default_user_dir: default_user_dir().map(|p| p.display().to_string()),
            default_game_dir: default_game_dir().map(|p| p.display().to_string()),
            settings_error: self.settings_error(),
            settings,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// A new user folder, game folder or analysis language drops the current analysis; a new
    /// game folder or language also drops the vanilla index.
    pub fn save_settings(&self, s: Settings) -> Result<SetupView, AppError> {
        let (reanalyze, reindex_vanilla) = {
            let mut settings = lock(&self.settings);
            save_settings(&self.config_dir, &s).map_err(|e| AppError {
                code: ErrorCode::Io,
                items: Vec::new(),
                detail: e.to_string(),
            })?;
            let old = std::mem::replace(&mut *settings, s);
            *lock(&self.settings_error) = None;
            let vanilla =
                old.game_dir != settings.game_dir || old.analysis_lang != settings.analysis_lang;
            (vanilla || old.user_dir != settings.user_dir, vanilla)
        };
        if reanalyze {
            *lock(&self.analysis) = None;
        }
        if reindex_vanilla {
            *lock(&self.vanilla) = None;
        }
        Ok(self.setup())
    }

    pub fn list_playsets(&self) -> Result<Vec<PlaysetRow>, AppError> {
        let (_, db) = launcher_db(&self.user_dir()?)?;
        db.playsets()?
            .into_iter()
            .map(|p| {
                let mods = db.playset_mods(&p.id)?;
                Ok(PlaysetRow {
                    enabled: mods.iter().filter(|m| m.enabled).count(),
                    mods: mods.len(),
                    id: p.id,
                    name: p.name,
                    active: p.is_active,
                })
            })
            .collect::<Result<_, ck3h_core::error::CoreError>>()
            .map_err(AppError::from)
    }

    /// Loads and analyses a playset (`None`: the active one) and keeps it as the current
    /// analysis. Rules come from `rules.toml` in the config folder; a file that does not load
    /// gives `rules_error` and no rules. Vanilla is included when the setting is on and a game
    /// folder is usable; without one the analysis runs without vanilla and the setting is kept.
    pub fn analyze(
        &self,
        playset_id: Option<&str>,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<AnalysisView, AppError> {
        let settings = self.settings();
        let user_dir = self.user_dir()?;
        let lang = analysis_lang(&user_dir, settings.analysis_lang.as_deref());
        let selector = match playset_id {
            Some(id) => PlaysetSelector::ById(id.to_string()),
            None => PlaysetSelector::Active,
        };
        let loaded = load(&user_dir, &selector, &lang, &|done, total| {
            progress(Progress::Mods { done, total })
        })?;
        let (rules, rules_error) = match load_rules(&self.config_dir.join(RULES_FILE)) {
            Ok(rules) => (rules, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };
        let vanilla_on = settings.show_vanilla
            && match self.ensure_vanilla(&lang, progress) {
                Ok(()) => true,
                Err(e) if e.code == ErrorCode::NotGameDir => false,
                Err(e) => return Err(e),
            };
        progress(Progress::Conflicts);
        let analysis = {
            let vanilla = lock(&self.vanilla);
            let index = vanilla.as_ref().filter(|_| vanilla_on).map(|v| &v.index);
            Analysis::new(loaded, rules, rules_error, index)
        };
        let view = analysis.view();
        *lock(&self.analysis) = Some(analysis);
        Ok(view)
    }

    /// Turns vanilla on or off for the current analysis and remembers it in the settings.
    /// Mods are not re-indexed; the vanilla index is reused for the same game folder and language.
    pub fn set_vanilla(
        &self,
        on: bool,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<AnalysisView, AppError> {
        let lang = self.with_analysis(|a, _| a.loaded.lang.clone())?;
        if on {
            self.ensure_vanilla(&lang, progress)?;
        }
        let mut settings = self.settings();
        if settings.show_vanilla != on {
            settings.show_vanilla = on;
            self.save_settings(settings)?;
        }
        progress(Progress::Conflicts);
        let mut analysis = lock(&self.analysis);
        let a = analysis.as_mut().ok_or_else(not_analyzed)?;
        let vanilla = lock(&self.vanilla);
        a.set_vanilla(vanilla.as_ref().filter(|_| on).map(|v| &v.index));
        Ok(a.view())
    }

    pub fn summary(&self) -> Result<AnalysisView, AppError> {
        self.with_analysis(|a, _| a.view())
    }

    /// Conflicts of the pair `winner` → `loser` (`None`: vanilla), in report order.
    pub fn pair_conflicts(
        &self,
        winner_id: Option<&str>,
        loser_id: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<Page<ConflictView>, AppError> {
        let source = |id: Option<&str>| id.map_or(Source::Vanilla, |id| Source::Mod(id.into()));
        let (winner, loser) = (source(winner_id), source(loser_id));
        self.with_analysis(|a, vanilla| {
            a.page(|c| involves(c, &winner, &loser), offset, limit, vanilla)
        })
    }

    pub fn search_conflicts(
        &self,
        query: &ConflictQuery,
        offset: usize,
        limit: usize,
    ) -> Result<Page<ConflictView>, AppError> {
        let text = query.text.to_lowercase();
        self.with_analysis(|a, vanilla| {
            a.page(|c| matches(query, &text, c), offset, limit, vanilla)
        })
    }

    /// Index warnings of a mod of the current analysis; none for an unknown id.
    pub fn mod_warnings(&self, mod_id: &str) -> Result<Vec<IndexWarningView>, AppError> {
        self.with_analysis(|a, _| a.mod_warnings(mod_id))
    }

    fn rules_path(&self) -> PathBuf {
        self.config_dir.join(RULES_FILE)
    }

    /// The rules in `rules.toml` as they are now; `applied` is judged on the current analysis.
    pub fn rules(&self) -> RulesView {
        rules_view(&self.rules_path(), lock(&self.analysis).as_ref())
    }

    pub fn add_rule(
        &self,
        before: String,
        after: String,
        note: Option<String>,
    ) -> Result<RulesView, AppError> {
        self.edit_rules(|_, path| {
            append_rule(
                path,
                &Rule {
                    before,
                    after,
                    note,
                },
            )
        })
    }

    /// Adds the rule "loser before winner", which makes the pair intended. Both ids must be mods
    /// of the current analysis, and the loser must not already depend on the winner.
    pub fn mark_intended(
        &self,
        winner_id: &str,
        loser_id: &str,
        note: String,
    ) -> Result<RulesView, AppError> {
        self.edit_rules(|analysis, path| {
            let a = analysis.ok_or_else(not_analyzed)?;
            let find = |id: &str| {
                a.find(id).ok_or_else(|| AppError {
                    code: ErrorCode::Rules,
                    items: vec![id.to_string()],
                    detail: format!("no mod {id} in the analysis"),
                })
            };
            let (winner, loser) = (find(winner_id)?, find(loser_id)?);
            if a.intent.closure().depends_on(&loser.id, &winner.id) {
                return Err(CoreError::Cycle(vec![loser.name.clone(), winner.name.clone()]).into());
            }
            let reference = |m: &ck3h_core::playset::ModEntry| {
                m.steam_id
                    .as_ref()
                    .map_or_else(|| m.name.clone(), |id| format!("steam:{id}"))
            };
            append_rule(
                path,
                &Rule {
                    before: reference(loser),
                    after: reference(winner),
                    note: Some(note),
                },
            )
        })
    }

    pub fn remove_rule(&self, index: usize) -> Result<RulesView, AppError> {
        self.edit_rules(|_, path| remove_rule(path, index))
    }

    /// Runs the file edit `edit` under the analysis lock (edits do not interleave), then gives the
    /// current analysis the new rules without re-indexing mods.
    fn edit_rules(
        &self,
        edit: impl FnOnce(Option<&Analysis>, &Path) -> Result<(), AppError>,
    ) -> Result<RulesView, AppError> {
        let path = self.rules_path();
        let mut analysis = lock(&self.analysis);
        edit(analysis.as_ref(), &path)?;
        if let Some(a) = analysis.as_mut() {
            let (rules, error) = match load_rules(&path) {
                Ok(rules) => (rules, None),
                Err(e) => (Vec::new(), Some(e.to_string())),
            };
            if a.vanilla_on {
                let vanilla = lock(&self.vanilla);
                a.set_rules(rules, error, vanilla.as_ref().map(|v| &v.index));
            } else {
                a.set_rules(rules, error, None);
            }
        }
        Ok(rules_view(&path, analysis.as_ref()))
    }

    /// The load-order proposal for the current analysis (with vanilla when it is on), kept in the
    /// analysis for `apply`.
    pub fn propose(&self) -> Result<ProposalView, AppError> {
        let mut analysis = lock(&self.analysis);
        let a = analysis.as_mut().ok_or_else(not_analyzed)?;
        let vanilla = if a.vanilla_on {
            Some(lock(&self.vanilla))
        } else {
            None
        };
        let index = vanilla.as_ref().and_then(|v| v.as_ref()).map(|v| &v.index);
        let proposal = propose(&a.loaded.mods, &a.deps, &a.rules, index)?;
        let view = proposal_view(a, &proposal);
        a.proposal = Some(proposal);
        Ok(view)
    }

    fn launcher_names(&self) -> Vec<String> {
        self.settings().launcher_process_names.unwrap_or_else(|| {
            DEFAULT_LAUNCHER_PROCESS_NAMES
                .iter()
                .map(|n| (*n).to_string())
                .collect()
        })
    }

    pub fn launcher_status(&self) -> LauncherStatus {
        let names = self.launcher_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        match launcher_processes(&names) {
            Ok(p) if p.is_empty() => LauncherStatus::NotRunning,
            Ok(processes) => LauncherStatus::Running { processes },
            Err(_) => LauncherStatus::Unknown,
        }
    }

    pub fn apply_preview(&self) -> Result<ApplyPreviewView, AppError> {
        self.with_analysis(|a, _| preview(&a.loaded.db_path, &a.loaded.playset.id))?
            .map(|p| ApplyPreviewView {
                new_playset_name: p.new_playset_name,
                backup_dir: p.backup_dir.display().to_string(),
            })
            .map_err(AppError::from)
    }

    /// Saves the cached proposal as a new playset. The analysis lock is held throughout, so rule
    /// edits wait and the proposal cannot change under the write. Refuses without a proposal that
    /// has moves, and when the playset changed in the launcher since the analysis. The cached
    /// proposal is spent afterwards.
    pub fn apply(&self, now: chrono::DateTime<chrono::Local>) -> Result<ApplyView, AppError> {
        let names = self.launcher_names();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut analysis = lock(&self.analysis);
        let a = analysis.as_mut().ok_or_else(not_analyzed)?;
        let order = match &a.proposal {
            Some(p) if !p.moves.is_empty() => p.order.clone(),
            _ => return Err(AppError::new(ErrorCode::NothingToApply)),
        };
        if !playset_unchanged(&a.loaded)? {
            return Err(AppError::new(ErrorCode::PlaysetChanged));
        }
        let result = apply_order(&a.loaded.db_path, &a.loaded.playset.id, &order, now, &names)?;
        a.proposal = None;
        Ok(ApplyView {
            new_playset_name: result.new_playset_name,
            backup_path: result.backup_path.display().to_string(),
        })
    }

    /// Compares the current analysis with the game's `database_conflicts.log`. Vanilla is indexed
    /// as in `set_vanilla` whatever the setting says; the report is computed under both locks.
    pub fn calibrate(
        &self,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<CalibrationView, AppError> {
        let user_dir = self.user_dir()?;
        let lang = self.with_analysis(|a, _| a.loaded.lang.clone())?;
        self.ensure_vanilla(&lang, progress)?;
        let log = read_conflicts_log(&user_dir)?;
        let analysis = lock(&self.analysis);
        let a = analysis.as_ref().ok_or_else(not_analyzed)?;
        let vanilla = lock(&self.vanilla);
        let mods = &a.loaded.mods;
        let run_check = check_last_run(&user_dir, mods);
        let report = calibrate(&log, mods, vanilla.as_ref().map(|v| &v.index), run_check);
        let counts = |m: std::collections::BTreeMap<String, usize>| {
            m.into_iter()
                .map(|(database, count)| DatabaseCount { database, count })
                .collect()
        };
        Ok(CalibrationView {
            matched: report.matched,
            mismatched: report
                .mismatched
                .into_iter()
                .map(|m| MismatchView {
                    database: m.database,
                    key: m.key,
                    log_file: m.log_file,
                    predicted_file: m.predicted_file,
                })
                .collect(),
            log_unknown: counts(report.log_unknown),
            logged_single_definition: counts(report.logged_single_definition),
            predicted_not_logged: report
                .predicted_not_logged
                .into_iter()
                .map(|(database, key)| ObjectRef { database, key })
                .collect(),
            truncated: report.truncated,
            last_run: report.run_check.map(|r| LastRunView {
                not_in_last_run: r
                    .not_in_last_run
                    .iter()
                    .map(|id| {
                        mods.iter()
                            .find(|m| &m.id == id)
                            .map_or_else(|| id.clone(), |m| m.name.clone())
                    })
                    .collect(),
                only_in_last_run: r.only_in_last_run,
            }),
        })
    }

    /// The once-a-day release check. Not due, or no repository URL yet: `None`, no request and
    /// no settings write. Otherwise the day is recorded whatever the outcome; errors are silent.
    /// Recording it does not reset the analysis (only folder or language changes do).
    pub fn check_update(&self, now: i64) -> Option<UpdateView> {
        self.check_update_with(env!("CARGO_PKG_REPOSITORY"), now)
    }

    /// `check_update` for a given repository URL. A broken `settings.toml` is left alone: no
    /// request and no write (saving would replace the file and clear the error banner).
    pub fn check_update_with(&self, repository: &str, now: i64) -> Option<UpdateView> {
        if lock(&self.settings_error).is_some() {
            return None;
        }
        let settings = self.settings();
        if !update::due(&settings, now) {
            return None;
        }
        let api = update::releases_api(repository)?;
        let found = update::fetch_latest(&api, env!("CARGO_PKG_VERSION"))
            .ok()
            .flatten();
        let mut settings = self.settings();
        settings.update_checked_at = Some(now);
        let _ = self.save_settings(settings);
        found
    }

    /// Runs `f` on the current analysis and, when it includes vanilla, the vanilla index.
    fn with_analysis<T>(
        &self,
        f: impl FnOnce(&Analysis, Option<&ModIndex>) -> T,
    ) -> Result<T, AppError> {
        let analysis = lock(&self.analysis);
        let a = analysis.as_ref().ok_or_else(not_analyzed)?;
        if !a.vanilla_on {
            return Ok(f(a, None));
        }
        let vanilla = lock(&self.vanilla);
        Ok(f(a, vanilla.as_ref().map(|v| &v.index)))
    }

    /// Indexes vanilla for `lang` unless the cached index has the same game folder and language.
    /// Game folder: the setting, else the default Steam location, else `NotGameDir`.
    fn ensure_vanilla(
        &self,
        lang: &str,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<(), AppError> {
        let game_dir = self
            .settings()
            .game_dir
            .or_else(default_game_dir)
            .ok_or_else(|| AppError::new(ErrorCode::NotGameDir))?;
        if lock(&self.vanilla)
            .as_ref()
            .is_some_and(|v| v.game_dir == game_dir && v.lang == lang)
        {
            return Ok(());
        }
        progress(Progress::Vanilla);
        let index = load_vanilla(&game_dir, lang)?;
        *lock(&self.vanilla) = Some(VanillaIndex {
            game_dir,
            lang: lang.to_string(),
            index,
        });
        Ok(())
    }
}

fn rules_view(path: &Path, analysis: Option<&Analysis>) -> RulesView {
    let (rules, error) = match load_rules(path) {
        Ok(rules) => (rules, None),
        Err(e) => (Vec::new(), Some(e.to_string())),
    };
    let applied = analysis
        .map(|a| a.applied_rules(&rules))
        .unwrap_or_default();
    RulesView {
        path: path.display().to_string(),
        error,
        rules: rules
            .into_iter()
            .enumerate()
            .map(|(index, r)| RuleView {
                index,
                before: r.before,
                after: r.after,
                note: r.note,
                applied: applied.contains(&index),
            })
            .collect(),
    }
}

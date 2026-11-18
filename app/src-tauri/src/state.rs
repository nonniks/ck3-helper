use crate::analysis::{Analysis, VanillaIndex, matches};
use crate::dto::{
    AnalysisView, ConflictQuery, ConflictView, IndexWarningView, Page, PlaysetRow, Progress,
    SetupView,
};
use crate::error::{AppError, ErrorCode};
use crate::settings::{Settings, SettingsFile, load_settings, save_settings};
use ck3h_core::conflicts::{Source, involves};
use ck3h_core::index::ModIndex;
use ck3h_core::launcher::LAUNCHER_DB_FILE;
use ck3h_core::paths::{RULES_FILE, default_game_dir, default_user_dir, game_language};
use ck3h_core::session::{PlaysetSelector, analysis_lang, launcher_db, load, load_vanilla};
use ck3h_core::sort::load_rules;
use std::path::PathBuf;
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
            settings_error: self
                .settings_error
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
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

//! The analysis kept in the app between commands (spec §4.8.1) and the views built from it.

use crate::dto::{
    AnalysisView, ConflictQuery, ConflictView, ContenderView, Counts, DepIssueView,
    IndexWarningView, LevelView, MarkState, ModView, Page, PairView, PlaysetRow, SourceKind,
    SourceView, VerdictView,
};
use ck3h_core::conflicts::{
    Conflict, ConflictLevel, ConflictReport, Contender, PairSummary, Source, Verdict, analyze,
    pairs,
};
use ck3h_core::deps::{DepClosure, DepGraph, DepIssueKind, build_dep_graph};
use ck3h_core::index::ModIndex;
use ck3h_core::playset::ModEntry;
use ck3h_core::session::Loaded;
use ck3h_core::sort::{Rule, SortProposal, intent_graph, resolve_rules};
use std::path::PathBuf;

/// An analysed playset. Rules and the vanilla toggle change `intent`, `report` and `pairs`
/// without re-indexing mods.
pub struct Analysis {
    pub loaded: Loaded,
    pub deps: DepGraph,
    pub rules: Vec<Rule>,
    /// `rules.toml` did not load; the analysis runs without rules.
    pub rules_error: Option<String>,
    /// `deps` plus the applied rules.
    pub intent: DepGraph,
    pub report: ConflictReport,
    pub pairs: Vec<PairSummary>,
    /// The report includes vanilla, taken from `AppState`'s vanilla index.
    pub vanilla_on: bool,
    /// The last load-order proposal; every `refresh` drops it, so it never outlives its inputs.
    pub proposal: Option<SortProposal>,
}

/// Vanilla's index for one game folder and language; reused while both match.
pub struct VanillaIndex {
    pub game_dir: PathBuf,
    pub lang: String,
    pub index: ModIndex,
}

impl Analysis {
    pub fn new(
        loaded: Loaded,
        rules: Vec<Rule>,
        rules_error: Option<String>,
        vanilla: Option<&ModIndex>,
    ) -> Self {
        let deps = build_dep_graph(&loaded.mods);
        let (intent, report, pairs) = derive(&loaded, &deps, &rules, vanilla);
        Self {
            loaded,
            deps,
            rules,
            rules_error,
            intent,
            report,
            pairs,
            vanilla_on: vanilla.is_some(),
            proposal: None,
        }
    }

    /// Recomputes `intent`, the report and the pairs from the current rules, with `vanilla` or
    /// without it for `None`. Mods are not re-indexed.
    fn refresh(&mut self, vanilla: Option<&ModIndex>) {
        (self.intent, self.report, self.pairs) =
            derive(&self.loaded, &self.deps, &self.rules, vanilla);
        self.vanilla_on = vanilla.is_some();
        self.proposal = None;
    }

    pub fn set_vanilla(&mut self, vanilla: Option<&ModIndex>) {
        self.refresh(vanilla);
    }

    /// New rules (or the error that left none) from `rules.toml`; `vanilla` as in `set_vanilla`.
    pub fn set_rules(
        &mut self,
        rules: Vec<Rule>,
        rules_error: Option<String>,
        vanilla: Option<&ModIndex>,
    ) {
        self.rules = rules;
        self.rules_error = rules_error;
        self.refresh(vanilla);
    }

    /// Indices of `rules` that apply to this playset.
    pub fn applied_rules(&self, rules: &[Rule]) -> Vec<usize> {
        resolve_rules(&self.loaded.mods, rules)
            .0
            .into_iter()
            .map(|(_, _, i)| i)
            .collect()
    }

    pub fn view(&self) -> AnalysisView {
        let mods = &self.loaded.mods;
        let playset = &self.loaded.playset;
        let mut counts = Counts::default();
        for c in &self.report.conflicts {
            match c.verdict {
                Verdict::Suspicious => counts.suspicious += 1,
                Verdict::Intended => counts.intended += 1,
                Verdict::Vanilla => counts.vanilla += 1,
            }
            match c.level {
                ConflictLevel::File => counts.files += 1,
                ConflictLevel::Object => counts.objects += 1,
                ConflictLevel::Loc => counts.loc += 1,
            }
        }
        let closure = self.intent.closure();
        AnalysisView {
            playset: PlaysetRow {
                id: playset.id.clone(),
                name: playset.name.clone(),
                active: playset.is_active,
                mods: mods.len(),
                enabled: mods.iter().filter(|m| m.enabled).count(),
            },
            lang: self.loaded.lang.clone(),
            vanilla: self.vanilla_on,
            mods: mods
                .iter()
                .enumerate()
                .map(|(position, m)| self.mod_view(position, m))
                .collect(),
            counts,
            pairs: self
                .pairs
                .iter()
                .map(|p| PairView {
                    winner: self.source_view(&p.winner),
                    loser: self.source_view(&p.loser),
                    verdict: p.verdict.into(),
                    files: p.files,
                    objects: p.objects,
                    loc: p.loc,
                    mark: mark(p, &closure),
                })
                .collect(),
            rules_error: self.rules_error.clone(),
        }
    }

    /// The conflicts `keep` accepts, in report order: `limit` of them from `offset`.
    pub fn page(
        &self,
        keep: impl Fn(&Conflict) -> bool,
        offset: usize,
        limit: usize,
        vanilla: Option<&ModIndex>,
    ) -> Page<ConflictView> {
        let mut items = Vec::new();
        let mut total = 0;
        for c in self.report.conflicts.iter().filter(|c| keep(c)) {
            if total >= offset && items.len() < limit {
                items.push(self.conflict_view(c, vanilla));
            }
            total += 1;
        }
        Page {
            items,
            total,
            offset,
        }
    }

    pub fn mod_warnings(&self, mod_id: &str) -> Vec<IndexWarningView> {
        self.find(mod_id)
            .and_then(|m| m.index.as_ref())
            .map(|index| {
                index
                    .warnings
                    .iter()
                    .map(|w| IndexWarningView {
                        file: w.file.clone(),
                        message: w.message.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn find(&self, mod_id: &str) -> Option<&ModEntry> {
        self.loaded.mods.iter().find(|m| m.id == mod_id)
    }

    fn mod_view(&self, position: usize, m: &ModEntry) -> ModView {
        let descriptor = m.descriptor.as_ref();
        ModView {
            id: m.id.clone(),
            name: m.name.clone(),
            position,
            enabled: m.enabled,
            missing: m.missing,
            compatch: self.deps.compatches.contains(&m.id),
            steam_id: m.steam_id.clone(),
            dir: m.dir.display().to_string(),
            version: descriptor.and_then(|d| d.version.clone()),
            supported_version: descriptor.and_then(|d| d.supported_version.clone()),
            dep_issues: self
                .deps
                .issues
                .iter()
                .filter(|i| i.mod_id == m.id)
                .map(|i| DepIssueView {
                    dependency: i.dependency.clone(),
                    ambiguous: i.kind == DepIssueKind::Ambiguous,
                })
                .collect(),
            replace_paths: descriptor
                .map(|d| d.replace_path.clone())
                .unwrap_or_default(),
            warnings: m.index.as_ref().map_or(0, |i| i.warnings.len()),
        }
    }

    pub(crate) fn source_view(&self, source: &Source) -> SourceView {
        match source {
            Source::Mod(id) => SourceView {
                kind: SourceKind::Mod,
                id: Some(id.clone()),
                name: self.find(id).map_or_else(|| id.clone(), |m| m.name.clone()),
            },
            Source::Vanilla => SourceView {
                kind: SourceKind::Vanilla,
                id: None,
                name: String::new(),
            },
        }
    }

    fn conflict_view(&self, c: &Conflict, vanilla: Option<&ModIndex>) -> ConflictView {
        let (database, key) = match (c.level, c.target.split_once("::")) {
            (ConflictLevel::Object, Some((database, key))) => {
                (Some(database.to_string()), key.to_string())
            }
            (ConflictLevel::Loc, Some((_, key))) => (None, key.to_string()),
            _ => (None, c.target.clone()),
        };
        let loc_key = (c.level == ConflictLevel::Loc).then_some(key.as_str());
        ConflictView {
            level: c.level.into(),
            target: c.target.clone(),
            winner: self.contender_view(&c.winner, loc_key, vanilla),
            losers: c
                .losers
                .iter()
                .map(|l| self.contender_view(l, loc_key, vanilla))
                .collect(),
            verdict: c.verdict.into(),
            reason: c.reason.into(),
            database,
            key,
        }
    }

    /// `loc_key` is set for localization: the text is read from the contender's own index.
    fn contender_view(
        &self,
        c: &Contender,
        loc_key: Option<&str>,
        vanilla: Option<&ModIndex>,
    ) -> ContenderView {
        let index = match &c.source {
            Source::Mod(id) => self.find(id).and_then(|m| m.index.as_ref()),
            Source::Vanilla => vanilla,
        };
        ContenderView {
            source: self.source_view(&c.source),
            file: c.file.display.clone(),
            text: loc_key
                .and_then(|key| index?.loc_value(&c.file, key))
                .map(str::to_string),
        }
    }
}

fn derive(
    loaded: &Loaded,
    deps: &DepGraph,
    rules: &[Rule],
    vanilla: Option<&ModIndex>,
) -> (DepGraph, ConflictReport, Vec<PairSummary>) {
    let intent = intent_graph(deps, &loaded.mods, rules);
    let report = analyze(&loaded.mods, &intent, vanilla);
    let pairs = pairs(&report, &intent);
    (intent, report, pairs)
}

/// `text_lower` is `query.text` in lowercase.
pub fn matches(query: &ConflictQuery, text_lower: &str, c: &Conflict) -> bool {
    (query.levels.is_empty() || query.levels.contains(&LevelView::from(c.level)))
        && (query.verdicts.is_empty() || query.verdicts.contains(&VerdictView::from(c.verdict)))
        && (text_lower.is_empty() || c.target.to_lowercase().contains(text_lower))
}

/// A suspicious pair of two mods can be marked intended ("loser before winner") unless the loser
/// already depends on the winner, which would close a cycle.
fn mark(p: &PairSummary, closure: &DepClosure) -> MarkState {
    match (&p.winner, &p.loser) {
        (Source::Mod(winner), Source::Mod(loser)) if p.verdict == Verdict::Suspicious => {
            if closure.depends_on(loser, winner) {
                MarkState::Cycle
            } else {
                MarkState::Available
            }
        }
        _ => MarkState::NotApplicable,
    }
}

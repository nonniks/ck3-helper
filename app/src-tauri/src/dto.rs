use crate::settings::Settings;
use ck3h_core::conflicts::{ConflictLevel, Verdict, WinReason};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetupView {
    pub settings: Settings,
    pub settings_error: Option<String>,
    pub user_dir: Option<String>,
    pub game_dir: Option<String>,
    pub game_language: Option<String>,
    /// The detected platform defaults, shown when the setting is cleared.
    pub default_user_dir: Option<String>,
    pub default_game_dir: Option<String>,
    pub launcher_db_found: bool,
    pub app_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaysetRow {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub mods: usize,
    pub enabled: usize,
}

/// Progress of `analyze`, `set_vanilla` and later heavy commands.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "stage", rename_all = "camelCase")]
#[ts(export)]
pub enum Progress {
    Mods { done: usize, total: usize },
    Vanilla,
    Conflicts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LevelView {
    File,
    Object,
    Loc,
}

impl From<ConflictLevel> for LevelView {
    fn from(l: ConflictLevel) -> Self {
        match l {
            ConflictLevel::File => Self::File,
            ConflictLevel::Object => Self::Object,
            ConflictLevel::Loc => Self::Loc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VerdictView {
    Intended,
    Suspicious,
    Vanilla,
}

impl From<Verdict> for VerdictView {
    fn from(v: Verdict) -> Self {
        match v {
            Verdict::Intended => Self::Intended,
            Verdict::Suspicious => Self::Suspicious,
            Verdict::Vanilla => Self::Vanilla,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WinReasonView {
    LowerInPlayset,
    LaterFilePath,
    EarlierFilePath,
    ReplaceFolder,
}

impl From<WinReason> for WinReasonView {
    fn from(r: WinReason) -> Self {
        match r {
            WinReason::LowerInPlayset => Self::LowerInPlayset,
            WinReason::LaterFilePath => Self::LaterFilePath,
            WinReason::EarlierFilePath => Self::EarlierFilePath,
            WinReason::ReplaceFolder => Self::ReplaceFolder,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SourceKind {
    Mod,
    Vanilla,
}

/// Vanilla has no `id` and an empty `name`; the interface names it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourceView {
    pub kind: SourceKind,
    pub id: Option<String>,
    pub name: String,
}

/// `file` as on disk; `text` only for localization: the line the game takes from that file.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ContenderView {
    pub source: SourceView,
    pub file: String,
    pub text: Option<String>,
}

/// `target` split at `::`: object → `database` and `key`; loc → `key` without the language;
/// file → `key` is the normalized path.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConflictView {
    pub level: LevelView,
    pub target: String,
    pub database: Option<String>,
    pub key: String,
    pub winner: ContenderView,
    pub losers: Vec<ContenderView>,
    pub verdict: VerdictView,
    pub reason: WinReasonView,
}

/// Whether a pair can be marked intended: only a suspicious pair of two mods, and not when the
/// loser already depends on the winner (the rule would close a cycle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MarkState {
    Available,
    Cycle,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PairView {
    pub winner: SourceView,
    pub loser: SourceView,
    pub verdict: VerdictView,
    pub files: usize,
    pub objects: usize,
    pub loc: usize,
    pub mark: MarkState,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DepIssueView {
    pub dependency: String,
    pub ambiguous: bool,
}

/// `position` is the 0-based index in the playset, disabled mods included.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModView {
    pub id: String,
    pub name: String,
    pub position: usize,
    pub enabled: bool,
    pub missing: bool,
    pub compatch: bool,
    pub steam_id: Option<String>,
    pub dir: String,
    pub version: Option<String>,
    pub supported_version: Option<String>,
    pub dep_issues: Vec<DepIssueView>,
    pub replace_paths: Vec<String>,
    pub warnings: usize,
}

/// Conflicts by verdict and by level.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Counts {
    pub suspicious: usize,
    pub intended: usize,
    pub vanilla: usize,
    pub files: usize,
    pub objects: usize,
    pub loc: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisView {
    pub playset: PlaysetRow,
    pub lang: String,
    pub vanilla: bool,
    pub mods: Vec<ModView>,
    pub counts: Counts,
    pub pairs: Vec<PairView>,
    pub rules_error: Option<String>,
}

/// `items` start at `offset` of the filtered list; `total` is its length.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
}

/// `text`: case-insensitive substring of `target`; empty `levels` or `verdicts` mean all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConflictQuery {
    pub text: String,
    pub levels: Vec<LevelView>,
    pub verdicts: Vec<VerdictView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexWarningView {
    pub file: String,
    pub message: String,
}

/// A rule of `rules.toml`; `index` is its position in the file. `applied`: it orders two enabled
/// mods of the current analysis.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleView {
    pub index: usize,
    pub before: String,
    pub after: String,
    pub note: Option<String>,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RulesView {
    pub path: String,
    /// `rules.toml` does not parse; `rules` is empty then.
    pub error: Option<String>,
    pub rules: Vec<RuleView>,
}

/// Why a mod moves. Names, not ids. `before`: the mod now loads earlier than `other`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum MoveReasonView {
    Dependency {
        on: String,
    },
    RequiredBy {
        by: String,
    },
    UserRule {
        other: String,
        before: bool,
        note: Option<String>,
    },
}

/// A mod in the proposed order; `from` and `to` are 0-based positions, disabled mods included.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrderRow {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub from: usize,
    pub to: usize,
    pub moved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MoveView {
    pub id: String,
    pub name: String,
    pub from: usize,
    pub to: usize,
    pub reasons: Vec<MoveReasonView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WinnerChangeView {
    pub level: LevelView,
    pub target: String,
    pub before: SourceView,
    pub after: SourceView,
}

/// `unresolved_rules`: rules that order no two enabled mods of the playset.
/// `already_sorted`: the proposal has no moves.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProposalView {
    pub rows: Vec<OrderRow>,
    pub moves: Vec<MoveView>,
    pub winner_changes: Vec<WinnerChangeView>,
    pub unresolved_rules: Vec<RuleView>,
    pub already_sorted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
#[ts(export)]
pub enum LauncherStatus {
    NotRunning,
    Running { processes: Vec<String> },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApplyPreviewView {
    pub new_playset_name: String,
    pub backup_dir: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApplyView {
    pub new_playset_name: String,
    pub backup_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MismatchView {
    pub database: String,
    pub key: String,
    pub log_file: String,
    pub predicted_file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DatabaseCount {
    pub database: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ObjectRef {
    pub database: String,
    pub key: String,
}

/// `notInLastRun` holds mod names; `onlyInLastRun` what the last launch had instead.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LastRunView {
    pub not_in_last_run: Vec<String>,
    pub only_in_last_run: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalibrationView {
    pub matched: usize,
    pub mismatched: Vec<MismatchView>,
    pub log_unknown: Vec<DatabaseCount>,
    pub logged_single_definition: Vec<DatabaseCount>,
    pub predicted_not_logged: Vec<ObjectRef>,
    pub truncated: bool,
    pub last_run: Option<LastRunView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateView {
    pub version: String,
    pub url: String,
}

//! Conflict engine: which file, database object and localization key wins, and why
//! (spec §4.3, §4.4).

use crate::deps::{DepGraph, DepIssue};
use crate::index::{IndexWarning, LocDef, ModIndex, ObjectDef, RelPath, normalize};
use crate::playset::ModEntry;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Source {
    /// Launcher id of the mod.
    Mod(String),
    Vanilla,
}

/// One copy of a file, or one definition of an object or localization key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Contender {
    pub source: Source,
    pub file: RelPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ConflictLevel {
    File,
    Object,
    Loc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Verdict {
    /// The winning mod depends on every losing mod.
    Intended,
    Suspicious,
    /// A single mod against vanilla.
    Vanilla,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WinReason {
    /// Same file path: the mod lower in the playset loads last.
    LowerInPlayset,
    /// Database object: the last definition in path order wins.
    LaterFilePath,
    /// Localization: the first definition in path order wins.
    EarlierFilePath,
    /// Localization from a `replace` folder overrides regular files.
    ReplaceFolder,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Conflict {
    pub level: ConflictLevel,
    /// File: normalized path; Object: `<database>::<key>`; Loc: `<lang>::<key>`.
    pub target: String,
    pub winner: Contender,
    /// In load order for files, in read order for objects and localization.
    pub losers: Vec<Contender>,
    pub verdict: Verdict,
    pub reason: WinReason,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConflictReport {
    /// Sorted by `(level, target)`.
    pub conflicts: Vec<Conflict>,
    /// Enabled mods whose folder is missing, in playset order.
    pub missing_mods: Vec<String>,
    pub dep_issues: Vec<DepIssue>,
    pub compatches: Vec<String>,
    /// `(mod id, replace_path)` for mods that declare any.
    pub replace_paths: Vec<(String, Vec<String>)>,
    /// `(mod id or "vanilla", warning)`.
    pub warnings: Vec<(String, IndexWarning)>,
}

/// Normalized path → its copies in load order (vanilla first); the last one is loaded.
/// A mod's `replace_path` drops copies of vanilla and of mods above it lying exactly in that folder.
pub fn effective_files(
    mods: &[ModEntry],
    vanilla: Option<&ModIndex>,
) -> BTreeMap<String, Vec<Contender>> {
    let layers = layers(&active(mods), vanilla);
    loaded_files(&layers)
        .into_iter()
        .map(|(path, copies)| {
            let copies = copies
                .into_iter()
                .map(|(layer, file)| layers[layer].contender(file))
                .collect();
            (path.to_string(), copies)
        })
        .collect()
}

/// `(database, key)` → definitions in read order; the last one wins.
pub fn resolve_objects(
    mods: &[ModEntry],
    vanilla: Option<&ModIndex>,
) -> BTreeMap<(String, String), Vec<Contender>> {
    let layers = layers(&active(mods), vanilla);
    object_defs(&layers, &loaded_files(&layers))
        .into_iter()
        .map(|((database, key), defs)| {
            let defs = defs
                .into_iter()
                .map(|(layer, def)| layers[layer].contender(&def.file))
                .collect();
            ((database.to_string(), key.to_string()), defs)
        })
        .collect()
}

/// `(lang, key)` → `(winner, losers in read order, reason)` for keys defined more than once.
pub fn resolve_loc(
    mods: &[ModEntry],
    vanilla: Option<&ModIndex>,
) -> BTreeMap<(String, String), (Contender, Vec<Contender>, WinReason)> {
    let layers = layers(&active(mods), vanilla);
    loc_defs(&layers, &loaded_files(&layers))
        .into_iter()
        .filter(|(_, defs)| defs.len() > 1)
        .map(|((lang, key), defs)| {
            let (winner, losers, reason) = loc_outcome(&layers, &defs);
            (
                (lang.to_string(), key.to_string()),
                (winner, losers, reason),
            )
        })
        .collect()
}

pub fn analyze(mods: &[ModEntry], deps: &DepGraph, vanilla: Option<&ModIndex>) -> ConflictReport {
    let active = active(mods);
    let layers = layers(&active, vanilla);
    let files = loaded_files(&layers);
    let mut conflicts = Vec::new();

    for (path, copies) in &files {
        if mixed_sources(&layers, copies) {
            let mut copies: Vec<Contender> = copies
                .iter()
                .map(|&(layer, file)| layers[layer].contender(file))
                .collect();
            let Some(winner) = copies.pop() else {
                continue;
            };
            conflicts.push(conflict(
                ConflictLevel::File,
                path.to_string(),
                winner,
                copies,
                WinReason::LowerInPlayset,
                deps,
            ));
        }
    }
    for ((database, key), defs) in object_defs(&layers, &files) {
        if mixed_sources(&layers, &defs) {
            let mut defs: Vec<Contender> = defs
                .iter()
                .map(|&(layer, def)| layers[layer].contender(&def.file))
                .collect();
            let Some(winner) = defs.pop() else {
                continue;
            };
            // Each loaded file belongs to one source, so every definition of another source
            // lies in an earlier file.
            conflicts.push(conflict(
                ConflictLevel::Object,
                format!("{database}::{key}"),
                winner,
                defs,
                WinReason::LaterFilePath,
                deps,
            ));
        }
    }
    for ((lang, key), defs) in loc_defs(&layers, &files) {
        if mixed_sources(&layers, &defs) {
            let (winner, losers, reason) = loc_outcome(&layers, &defs);
            conflicts.push(conflict(
                ConflictLevel::Loc,
                format!("{lang}::{key}"),
                winner,
                losers,
                reason,
                deps,
            ));
        }
    }
    conflicts.sort_by(|a, b| (a.level, &a.target).cmp(&(b.level, &b.target)));

    let mut by_position: Vec<&ModEntry> = mods.iter().collect();
    by_position.sort_by_key(|m| m.position);
    ConflictReport {
        conflicts,
        missing_mods: by_position
            .iter()
            .filter(|m| m.enabled && m.missing)
            .map(|m| m.id.clone())
            .collect(),
        dep_issues: deps.issues.clone(),
        compatches: active
            .iter()
            .filter(|m| deps.compatches.contains(&m.id))
            .map(|m| m.id.clone())
            .collect(),
        replace_paths: active
            .iter()
            .filter_map(|m| {
                let paths = &m.descriptor.as_ref()?.replace_path;
                (!paths.is_empty()).then(|| (m.id.clone(), paths.clone()))
            })
            .collect(),
        warnings: layers
            .iter()
            .flat_map(|layer| {
                let label = match &layer.source {
                    Source::Mod(id) => id.clone(),
                    Source::Vanilla => "vanilla".to_string(),
                };
                layer
                    .index
                    .warnings
                    .iter()
                    .map(move |w| (label.clone(), w.clone()))
            })
            .collect(),
    }
}

/// Enabled mods found on disk, by `position`.
fn active(mods: &[ModEntry]) -> Vec<&ModEntry> {
    let mut active: Vec<&ModEntry> = mods.iter().filter(|m| m.enabled && !m.missing).collect();
    active.sort_by_key(|m| m.position);
    active
}

/// One step of the load order: vanilla (with its DLC) or a mod.
struct Layer<'a> {
    source: Source,
    index: &'a ModIndex,
    /// Normalized `replace_path` folders.
    replace_paths: Vec<String>,
}

impl Layer<'_> {
    fn contender(&self, file: &RelPath) -> Contender {
        Contender {
            source: self.source.clone(),
            file: file.clone(),
        }
    }
}

fn layers<'a>(active: &[&'a ModEntry], vanilla: Option<&'a ModIndex>) -> Vec<Layer<'a>> {
    let vanilla = vanilla.map(|index| Layer {
        source: Source::Vanilla,
        index,
        replace_paths: Vec::new(),
    });
    let mods = active.iter().filter_map(|&m| {
        Some(Layer {
            source: Source::Mod(m.id.clone()),
            index: m.index.as_ref()?,
            replace_paths: m
                .descriptor
                .iter()
                .flat_map(|d| &d.replace_path)
                .map(|p| normalize(p))
                .collect(),
        })
    });
    vanilla.into_iter().chain(mods).collect()
}

/// `(layer, item)` pairs.
type Copies<'a, T> = Vec<(usize, &'a T)>;

/// Normalized path → `(layer, file)` copies in load order, after `replace_path`.
fn loaded_files<'a>(layers: &[Layer<'a>]) -> BTreeMap<&'a str, Copies<'a, RelPath>> {
    // Folder → the last layer replacing it; it drops copies from every earlier layer.
    let mut replaced: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, layer) in layers.iter().enumerate() {
        for folder in &layer.replace_paths {
            replaced.insert(folder.as_str(), i);
        }
    }
    let mut files: BTreeMap<&'a str, Copies<'a, RelPath>> = BTreeMap::new();
    for (i, layer) in layers.iter().enumerate() {
        let index: &'a ModIndex = layer.index;
        for file in &index.files {
            let dropped = !replaced.is_empty()
                && replaced
                    .get(file.parent_norm().as_str())
                    .is_some_and(|&by| by > i);
            if !dropped {
                files.entry(file.norm.as_str()).or_default().push((i, file));
            }
        }
    }
    files
}

/// Definitions from loaded files only, in read order: by file path, then top to bottom.
fn in_read_order<'a, T>(
    layers: &[Layer<'a>],
    files: &BTreeMap<&'a str, Copies<'a, RelPath>>,
    defs: impl Fn(&'a ModIndex) -> &'a [T],
    position: impl Fn(&T) -> (&RelPath, u32),
) -> Copies<'a, T> {
    let mut out: Copies<'a, T> = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        out.extend(
            defs(layer.index)
                .iter()
                .filter(|def| {
                    let loaded = files
                        .get(position(def).0.norm.as_str())
                        .and_then(|c| c.last());
                    loaded.is_some_and(|&(by, _)| by == i)
                })
                .map(|def| (i, def)),
        );
    }
    out.sort_by(|(_, a), (_, b)| position(a).cmp(&position(b)));
    out
}

fn object_defs<'a>(
    layers: &[Layer<'a>],
    files: &BTreeMap<&'a str, Copies<'a, RelPath>>,
) -> BTreeMap<(&'a str, &'a str), Copies<'a, ObjectDef>> {
    let mut grouped: BTreeMap<(&str, &str), Copies<ObjectDef>> = BTreeMap::new();
    for (i, def) in in_read_order(layers, files, |x| &x.objects, |o| (&o.file, o.ordinal)) {
        grouped
            .entry((def.database.as_str(), def.key.as_str()))
            .or_default()
            .push((i, def));
    }
    grouped
}

fn loc_defs<'a>(
    layers: &[Layer<'a>],
    files: &BTreeMap<&'a str, Copies<'a, RelPath>>,
) -> BTreeMap<(&'a str, &'a str), Copies<'a, LocDef>> {
    let mut grouped: BTreeMap<(&str, &str), Copies<LocDef>> = BTreeMap::new();
    for (i, def) in in_read_order(layers, files, |x| &x.loc, |l| (&l.file, l.ordinal)) {
        grouped
            .entry((def.lang.as_str(), def.key.as_str()))
            .or_default()
            .push((i, def));
    }
    grouped
}

/// The first regular definition wins unless a `replace` file overrides it;
/// among `replace` definitions the last one wins.
fn loc_outcome(
    layers: &[Layer],
    defs: &[(usize, &LocDef)],
) -> (Contender, Vec<Contender>, WinReason) {
    let (winner, reason) = match defs.iter().rposition(|(_, def)| def.is_replace) {
        Some(w) => (w, WinReason::ReplaceFolder),
        None => (0, WinReason::EarlierFilePath),
    };
    let contender = |&(layer, def): &(usize, &LocDef)| layers[layer].contender(&def.file);
    let losers = defs
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != winner)
        .map(|(_, d)| contender(d))
        .collect();
    (contender(&defs[winner]), losers, reason)
}

/// True when the copies come from at least two sources.
fn mixed_sources<T>(layers: &[Layer], copies: &[(usize, &T)]) -> bool {
    copies
        .iter()
        .any(|&(layer, _)| layers[layer].source != layers[copies[0].0].source)
}

fn conflict(
    level: ConflictLevel,
    target: String,
    winner: Contender,
    losers: Vec<Contender>,
    reason: WinReason,
    deps: &DepGraph,
) -> Conflict {
    let mut others: Vec<&Source> = Vec::new();
    for loser in &losers {
        if loser.source != winner.source && !others.contains(&&loser.source) {
            others.push(&loser.source);
        }
    }
    let verdict = if others.len() == 1
        && (winner.source == Source::Vanilla || *others[0] == Source::Vanilla)
    {
        Verdict::Vanilla
    } else if let Source::Mod(winner_id) = &winner.source
        && others.iter().all(|other| match other {
            Source::Mod(loser_id) => deps.depends_on(winner_id, loser_id),
            Source::Vanilla => true,
        })
    {
        Verdict::Intended
    } else {
        Verdict::Suspicious
    };
    Conflict {
        level,
        target,
        winner,
        losers,
        verdict,
        reason,
    }
}

//! Load-order rules (`rules.toml`) and the sorter that proposes a new playset order (spec §4.5).

use crate::conflicts::{ConflictLevel, Contender, analyze_ordered};
use crate::deps::{DepGraph, NameMatch, match_name};
use crate::error::CoreError;
use crate::index::ModIndex;
use crate::playset::ModEntry;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

/// `before` loads earlier than `after`; both are `descriptor.mod` names or `steam:<id>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub before: String,
    pub after: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MoveReason {
    /// The mod depends on `on`, which must load earlier.
    Dependency { on: String },
    /// `by` depends on the mod, so the mod must load earlier.
    RequiredBy { by: String },
    /// A user rule orders the mod against `other`.
    UserRule { other: String, note: Option<String> },
}

/// A mod that changes index and is in a constraint the current order breaks, or is dragged
/// along by such a mod through a chain of constraints pushing it the way it moved (spec §4.5).
/// Indices are 0-based in the full playset list, disabled mods included; ids are launcher mod ids.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Move {
    pub mod_id: String,
    pub from: usize,
    pub to: usize,
    pub reasons: Vec<MoveReason>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WinnerChange {
    pub level: ConflictLevel,
    pub target: String,
    pub before: Contender,
    pub after: Contender,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SortProposal {
    /// Mod ids of the whole playset in the proposed order.
    pub order: Vec<String>,
    /// Sorted by `from`.
    pub moves: Vec<Move>,
    /// Conflicts in both orders whose winner differs, sorted by `(level, target)`.
    pub winner_changes: Vec<WinnerChange>,
    /// Rules not applied: a name matches no mod or several, or a disabled mod.
    pub unresolved_rules: Vec<Rule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesFile {
    #[serde(default)]
    rule: Vec<Rule>,
}

pub fn parse_rules(toml_text: &str) -> Result<Vec<Rule>, CoreError> {
    parse(toml_text).map_err(|e| CoreError::Rules(e.to_string()))
}

/// Rules from `path`; a missing file means no rules.
pub fn load_rules(path: &Path) -> Result<Vec<Rule>, CoreError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CoreError::Rules(format!("{}: {e}", path.display()))),
    };
    parse(&text).map_err(|e| CoreError::Rules(format!("{}: {e}", path.display())))
}

fn parse(text: &str) -> Result<Vec<Rule>, toml::de::Error> {
    toml::from_str::<RulesFile>(text).map(|file| file.rule)
}

/// "Load `earlier` before `later`", as indices into the current playset list.
struct Constraint<'a> {
    earlier: usize,
    later: usize,
    /// `None` for a dependency.
    rule: Option<&'a Rule>,
}

pub fn propose(
    mods: &[ModEntry],
    deps: &DepGraph,
    rules: &[Rule],
    vanilla: Option<&ModIndex>,
) -> Result<SortProposal, CoreError> {
    let mut current: Vec<&ModEntry> = mods.iter().collect();
    current.sort_by_key(|m| m.position);
    let enabled: Vec<usize> = (0..current.len()).filter(|&i| current[i].enabled).collect();
    let index_of: BTreeMap<&str, usize> = enabled
        .iter()
        .map(|&i| (current[i].id.as_str(), i))
        .collect();

    let mut constraints = Vec::new();
    for (dependency, dependent) in &deps.edges {
        if let (Some(&earlier), Some(&later)) = (
            index_of.get(dependency.as_str()),
            index_of.get(dependent.as_str()),
        ) {
            constraints.push(Constraint {
                earlier,
                later,
                rule: None,
            });
        }
    }
    let resolve = |name: &str| match match_name(mods, name) {
        NameMatch::One(id) => index_of.get(id.as_str()).copied(),
        NameMatch::None | NameMatch::Many(_) => None,
    };
    let mut unresolved_rules = Vec::new();
    for rule in rules {
        match (resolve(&rule.before), resolve(&rule.after)) {
            (Some(earlier), Some(later)) if earlier != later => constraints.push(Constraint {
                earlier,
                later,
                rule: Some(rule),
            }),
            _ => unresolved_rules.push(rule.clone()),
        }
    }
    constraints.sort_by_key(|c| (c.earlier, c.later));

    // Kahn's algorithm; among the mods ready to place, the one earliest in the current order goes first.
    let mut successors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); current.len()];
    let mut waiting_for = vec![0usize; current.len()];
    for c in &constraints {
        if successors[c.earlier].insert(c.later) {
            waiting_for[c.later] += 1;
        }
    }
    let mut ready: BTreeSet<usize> = enabled
        .iter()
        .copied()
        .filter(|&i| waiting_for[i] == 0)
        .collect();
    let mut sorted = Vec::with_capacity(enabled.len());
    while let Some(i) = ready.pop_first() {
        sorted.push(i);
        for &next in &successors[i] {
            waiting_for[next] -= 1;
            if waiting_for[next] == 0 {
                ready.insert(next);
            }
        }
    }
    if sorted.len() < enabled.len() {
        let placed: BTreeSet<usize> = sorted.into_iter().collect();
        let remaining: BTreeSet<usize> = enabled
            .into_iter()
            .filter(|i| !placed.contains(i))
            .collect();
        let cycle = find_cycle(&remaining, &constraints);
        return Err(CoreError::Cycle(
            cycle.into_iter().map(|i| current[i].name.clone()).collect(),
        ));
    }

    // Disabled mods keep their index; enabled ones fill the other indices in sorted order.
    let mut to: Vec<usize> = (0..current.len()).collect();
    for (&slot, &i) in enabled.iter().zip(&sorted) {
        to[i] = slot;
    }
    let mut order = current.clone();
    for (i, &m) in current.iter().enumerate() {
        order[to[i]] = m;
    }

    // Mods on a constraint the current order breaks; their reasons are those constraints.
    let mut reasons: BTreeMap<usize, Vec<MoveReason>> = BTreeMap::new();
    for c in constraints.iter().filter(|c| c.earlier > c.later) {
        for i in [c.later, c.earlier] {
            reasons.entry(i).or_default().push(reason(c, i, &current));
        }
    }
    // Mods dragged along: a moved mod pushes the other end of its constraint the same way (an
    // earlier end moving later pushes the later end later, a later end moving earlier pushes
    // the earlier end earlier), if that end moved that way too. Breadth-first by distance;
    // a dragged mod's reasons are the constraints pushing it from one step closer.
    let shift = |i: usize| to[i].cmp(&i);
    let mut distance: Vec<Option<usize>> = vec![None; current.len()];
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    for &i in reasons.keys() {
        distance[i] = Some(0);
        queue.push_back((i, 0));
    }
    while let Some((i, d)) = queue.pop_front() {
        for c in &constraints {
            let other = match shift(i) {
                Ordering::Greater if c.earlier == i => c.later,
                Ordering::Less if c.later == i => c.earlier,
                _ => continue,
            };
            if shift(other) != shift(i) {
                continue;
            }
            if distance[other].is_none() {
                distance[other] = Some(d + 1);
                queue.push_back((other, d + 1));
            }
            if distance[other] == Some(d + 1) {
                reasons
                    .entry(other)
                    .or_default()
                    .push(reason(c, other, &current));
            }
        }
    }
    let moves: Vec<Move> = reasons
        .into_iter()
        .filter(|&(i, _)| to[i] != i)
        .map(|(i, reasons)| Move {
            mod_id: current[i].id.clone(),
            from: i,
            to: to[i],
            reasons,
        })
        .collect();

    let winner_changes = if to.iter().enumerate().all(|(i, &t)| i == t) {
        Vec::new()
    } else {
        winner_changes(&current, &order, deps, vanilla)
    };
    Ok(SortProposal {
        order: order.iter().map(|m| m.id.clone()).collect(),
        moves,
        winner_changes,
        unresolved_rules,
    })
}

/// Constraint `c` seen from mod `i`, one of its two ends.
fn reason(c: &Constraint, i: usize, current: &[&ModEntry]) -> MoveReason {
    let i_is_later = c.later == i;
    let other = current[if i_is_later { c.earlier } else { c.later }]
        .id
        .clone();
    match (c.rule, i_is_later) {
        (None, true) => MoveReason::Dependency { on: other },
        (None, false) => MoveReason::RequiredBy { by: other },
        (Some(rule), _) => MoveReason::UserRule {
            other,
            note: rule.note.clone(),
        },
    }
}

/// Every mod Kahn's algorithm left still waits for another remaining mod, so following
/// "waits for" links from one of them must come back to a mod already visited.
/// Each returned mod must load after the next one, the last after the first.
fn find_cycle(remaining: &BTreeSet<usize>, constraints: &[Constraint]) -> Vec<usize> {
    let mut path: Vec<usize> = Vec::new();
    let mut next = remaining.first().copied();
    while let Some(node) = next {
        if let Some(start) = path.iter().position(|&visited| visited == node) {
            return path.split_off(start);
        }
        path.push(node);
        next = constraints
            .iter()
            .filter(|c| c.later == node && remaining.contains(&c.earlier))
            .map(|c| c.earlier)
            .min();
    }
    path
}

fn winner_changes(
    current: &[&ModEntry],
    proposed: &[&ModEntry],
    deps: &DepGraph,
    vanilla: Option<&ModIndex>,
) -> Vec<WinnerChange> {
    let before = analyze_ordered(current, deps, vanilla);
    let winners: BTreeMap<(ConflictLevel, &str), &Contender> = before
        .conflicts
        .iter()
        .map(|c| ((c.level, c.target.as_str()), &c.winner))
        .collect();
    let mut changes = Vec::new();
    for c in analyze_ordered(proposed, deps, vanilla).conflicts {
        if let Some(&old) = winners.get(&(c.level, c.target.as_str()))
            && *old != c.winner
        {
            changes.push(WinnerChange {
                level: c.level,
                before: old.clone(),
                after: c.winner,
                target: c.target,
            });
        }
    }
    changes
}

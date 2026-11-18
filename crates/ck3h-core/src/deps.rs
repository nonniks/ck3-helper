//! Dependency graph between enabled mods, built from `descriptor.mod` dependency names.

use crate::playset::ModEntry;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum NameMatch {
    None,
    One(String),
    Many(Vec<String>),
}

/// Finds mods (ids, in playset order) by trimmed, case-insensitive name,
/// or by Steam id with a `steam:<id>` prefix.
pub fn match_name(mods: &[ModEntry], name: &str) -> NameMatch {
    match_in(mods.iter(), name)
}

fn match_in<'a>(mods: impl Iterator<Item = &'a ModEntry>, name: &str) -> NameMatch {
    let name = name.trim();
    let ids: Vec<String> = if let Some(steam) = name.strip_prefix("steam:") {
        let steam = steam.trim();
        mods.filter(|m| m.steam_id.as_deref() == Some(steam))
            .map(|m| m.id.clone())
            .collect()
    } else {
        let wanted = name.to_lowercase();
        mods.filter(|m| m.name.trim().to_lowercase() == wanted)
            .map(|m| m.id.clone())
            .collect()
    };
    match ids.len() {
        0 => NameMatch::None,
        1 => NameMatch::One(ids.into_iter().next().unwrap_or_default()),
        _ => NameMatch::Many(ids),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DepIssueKind {
    Missing,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepIssue {
    pub mod_id: String,
    pub dependency: String,
    pub kind: DepIssueKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepGraph {
    /// `(dependency, dependent)` mod ids.
    pub edges: BTreeSet<(String, String)>,
    pub issues: Vec<DepIssue>,
    /// Mods with at least two resolved dependencies.
    pub compatches: BTreeSet<String>,
}

pub fn build_dep_graph(mods: &[ModEntry]) -> DepGraph {
    let enabled: Vec<&ModEntry> = mods.iter().filter(|m| m.enabled).collect();
    let mut graph = DepGraph {
        edges: BTreeSet::new(),
        issues: Vec::new(),
        compatches: BTreeSet::new(),
    };
    for &m in &enabled {
        let Some(descriptor) = &m.descriptor else {
            continue;
        };
        let mut found = BTreeSet::new();
        for dependency in &descriptor.dependencies {
            let kind = match match_in(enabled.iter().copied(), dependency) {
                NameMatch::One(dep_id) => {
                    if dep_id != m.id {
                        found.insert(dep_id);
                    }
                    continue;
                }
                NameMatch::None => DepIssueKind::Missing,
                NameMatch::Many(_) => DepIssueKind::Ambiguous,
            };
            graph.issues.push(DepIssue {
                mod_id: m.id.clone(),
                dependency: dependency.clone(),
                kind,
            });
        }
        if found.len() >= 2 {
            graph.compatches.insert(m.id.clone());
        }
        graph
            .edges
            .extend(found.into_iter().map(|dep| (dep, m.id.clone())));
    }
    graph
}

impl DepGraph {
    /// True when `dependent` reaches `dependency` through the edges (directly or transitively).
    pub fn depends_on(&self, dependent: &str, dependency: &str) -> bool {
        let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (dep, user) in &self.edges {
            dependents.entry(dep).or_default().push(user);
        }
        let mut seen = BTreeSet::new();
        let mut stack = vec![dependency];
        while let Some(current) = stack.pop() {
            for &next in dependents.get(current).into_iter().flatten() {
                if next == dependent {
                    return true;
                }
                if seen.insert(next) {
                    stack.push(next);
                }
            }
        }
        false
    }

    /// Every mod's transitive dependencies, walking the graph once per mod.
    pub fn closure(&self) -> DepClosure {
        let mut direct: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (dep, user) in &self.edges {
            direct.entry(user).or_default().push(dep);
        }
        let mut reached = BTreeMap::new();
        for &start in direct.keys() {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            let mut stack = vec![start];
            while let Some(current) = stack.pop() {
                for &next in direct.get(current).into_iter().flatten() {
                    if seen.insert(next) {
                        stack.push(next);
                    }
                }
            }
            reached.insert(
                start.to_string(),
                seen.into_iter().map(str::to_string).collect(),
            );
        }
        DepClosure { reached }
    }
}

/// Answers `DepGraph::depends_on` without walking the graph again.
#[derive(Debug, Clone, PartialEq)]
pub struct DepClosure {
    /// Dependent → every mod it reaches through the edges (itself only through a cycle).
    reached: BTreeMap<String, BTreeSet<String>>,
}

impl DepClosure {
    pub fn depends_on(&self, dependent: &str, dependency: &str) -> bool {
        self.reached
            .get(dependent)
            .is_some_and(|deps| deps.contains(dependency))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_matches_depends_on() {
        // Edges (dependency, dependent): a chain A → B → C and a cycle X ⇄ Y.
        let g = DepGraph {
            edges: [("A", "B"), ("B", "C"), ("X", "Y"), ("Y", "X")]
                .into_iter()
                .map(|(dep, user)| (dep.to_string(), user.to_string()))
                .collect(),
            issues: Vec::new(),
            compatches: BTreeSet::new(),
        };
        let closure = g.closure();
        for (d, on) in [
            ("C", "A"),
            ("A", "C"),
            ("Y", "X"),
            ("X", "Y"),
            ("C", "X"),
            ("X", "X"),
            ("A", "A"),
            ("nope", "A"),
        ] {
            assert_eq!(
                closure.depends_on(d, on),
                g.depends_on(d, on),
                "{d} on {on}"
            );
        }
    }
}

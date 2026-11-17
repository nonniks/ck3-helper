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
    let name = name.trim();
    let ids: Vec<String> = if let Some(steam) = name.strip_prefix("steam:") {
        let steam = steam.trim();
        mods.iter()
            .filter(|m| m.steam_id.as_deref() == Some(steam))
            .map(|m| m.id.clone())
            .collect()
    } else {
        let wanted = name.to_lowercase();
        mods.iter()
            .filter(|m| m.name.trim().to_lowercase() == wanted)
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
    let enabled: Vec<ModEntry> = mods.iter().filter(|m| m.enabled).cloned().collect();
    let mut graph = DepGraph {
        edges: BTreeSet::new(),
        issues: Vec::new(),
        compatches: BTreeSet::new(),
    };
    for m in &enabled {
        let Some(descriptor) = &m.descriptor else {
            continue;
        };
        let mut found = BTreeSet::new();
        for dependency in &descriptor.dependencies {
            let kind = match match_name(&enabled, dependency) {
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
}

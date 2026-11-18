//! The view of a load-order proposal (spec §4.5).

use crate::analysis::Analysis;
use crate::dto::{
    LevelView, MoveReasonView, MoveView, OrderRow, ProposalView, RuleView, WinnerChangeView,
};
use ck3h_core::sort::{MoveReason, SortProposal};
use std::collections::BTreeMap;

pub fn proposal_view(a: &Analysis, p: &SortProposal) -> ProposalView {
    let mut current: Vec<&_> = a.loaded.mods.iter().collect();
    current.sort_by_key(|m| m.position);
    let from: BTreeMap<&str, usize> = current
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();
    let to: BTreeMap<&str, usize> = p
        .order
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let name = |id: &str| {
        a.find(id)
            .map_or_else(|| id.to_string(), |m| m.name.clone())
    };

    let rows = p
        .order
        .iter()
        .enumerate()
        .map(|(to, id)| {
            let from = from.get(id.as_str()).copied().unwrap_or(to);
            OrderRow {
                id: id.clone(),
                name: name(id),
                enabled: a.find(id).is_some_and(|m| m.enabled),
                from,
                to,
                moved: from != to,
            }
        })
        .collect();
    let moves = p
        .moves
        .iter()
        .map(|m| MoveView {
            id: m.mod_id.clone(),
            name: name(&m.mod_id),
            from: m.from,
            to: m.to,
            reasons: m
                .reasons
                .iter()
                .map(|r| match r {
                    MoveReason::Dependency { on } => MoveReasonView::Dependency { on: name(on) },
                    MoveReason::RequiredBy { by } => MoveReasonView::RequiredBy { by: name(by) },
                    MoveReason::UserRule { other, note } => MoveReasonView::UserRule {
                        other: name(other),
                        before: to.get(m.mod_id.as_str()) < to.get(other.as_str()),
                        note: note.clone(),
                    },
                })
                .collect(),
        })
        .collect();
    let winner_changes = p
        .winner_changes
        .iter()
        .map(|w| WinnerChangeView {
            level: LevelView::from(w.level),
            target: w.target.clone(),
            before: a.source_view(&w.before.source),
            after: a.source_view(&w.after.source),
        })
        .collect();
    let applied = a.applied_rules(&a.rules);
    let unresolved_rules = a
        .rules
        .iter()
        .enumerate()
        .filter(|(i, _)| !applied.contains(i))
        .map(|(index, r)| RuleView {
            index,
            before: r.before.clone(),
            after: r.after.clone(),
            note: r.note.clone(),
            applied: false,
        })
        .collect();
    ProposalView {
        rows,
        moves,
        winner_changes,
        unresolved_rules,
        already_sorted: p.moves.is_empty(),
    }
}

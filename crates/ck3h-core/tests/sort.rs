#![cfg(feature = "testkit")]

use ck3h_core::conflicts::{ConflictLevel, Source};
use ck3h_core::deps::build_dep_graph;
use ck3h_core::error::CoreError;
use ck3h_core::launcher::LauncherDb;
use ck3h_core::playset::{ModEntry, load_playset};
use ck3h_core::sort::*;
use ck3h_core::testkit::*;

type Files<'a> = &'a [(&'a str, &'a [u8])];

/// A playset entry; `files` come on top of the generated `descriptor.mod`.
struct Spec<'a> {
    name: &'a str,
    deps: &'a [&'a str],
    enabled: bool,
    files: Files<'a>,
}

fn on<'a>(name: &'a str, deps: &'a [&'a str]) -> Spec<'a> {
    Spec {
        name,
        deps,
        enabled: true,
        files: &[],
    }
}

fn off(name: &str) -> Spec<'_> {
    Spec {
        name,
        deps: &[],
        enabled: false,
        files: &[],
    }
}

/// An active playset of `specs`; positions follow the order.
fn playset(specs: &[Spec]) -> (tempfile::TempDir, Vec<ModEntry>) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("launcher-v2.sqlite");
    let conn = create_launcher_db(&db_path);
    let ps = add_playset(&conn, "Main", true);
    for (i, spec) in specs.iter().enumerate() {
        let deps: String = spec.deps.iter().map(|d| format!("\t\"{d}\"\n")).collect();
        let descriptor = format!("name=\"{}\"\ndependencies={{\n{deps}}}\n", spec.name);
        let mut files: Vec<(&str, &[u8])> = vec![("descriptor.mod", descriptor.as_bytes())];
        files.extend_from_slice(spec.files);
        let dir = write_mod(tmp.path(), &format!("m{i}"), &files);
        let m = add_mod(&conn, spec.name, &dir, None);
        add_playset_mod(&conn, &ps, &m, i as i64, spec.enabled);
    }
    drop(conn);
    let db = LauncherDb::open_read_only(&db_path).unwrap();
    let mods = load_playset(&db, &db.active_playset().unwrap(), "l_english").unwrap();
    (tmp, mods)
}

fn sort(mods: &[ModEntry], rules: &[Rule]) -> Result<SortProposal, CoreError> {
    propose(mods, &build_dep_graph(mods), rules, None)
}

fn id(mods: &[ModEntry], name: &str) -> String {
    mods.iter().find(|m| m.name == name).unwrap().id.clone()
}

fn name<'a>(mods: &'a [ModEntry], id: &str) -> &'a str {
    &mods.iter().find(|m| m.id == id).unwrap().name
}

fn names<'a>(mods: &'a [ModEntry], ids: &[String]) -> Vec<&'a str> {
    ids.iter().map(|id| name(mods, id)).collect()
}

fn moved<'a>(mods: &'a [ModEntry], p: &SortProposal) -> Vec<&'a str> {
    p.moves.iter().map(|m| name(mods, &m.mod_id)).collect()
}

#[test]
fn dependency_moves_dependent_after_dependency() {
    let (_t, mods) = playset(&[on("Child", &["Parent"]), on("Parent", &[])]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["Parent", "Child"]);
    assert!(matches!(
        p.moves[0].reasons[0],
        MoveReason::Dependency { .. }
    ));
    assert_eq!(
        p.moves,
        [
            Move {
                mod_id: id(&mods, "Child"),
                from: 0,
                to: 1,
                reasons: vec![MoveReason::Dependency {
                    on: id(&mods, "Parent")
                }],
            },
            Move {
                mod_id: id(&mods, "Parent"),
                from: 1,
                to: 0,
                reasons: vec![MoveReason::RequiredBy {
                    by: id(&mods, "Child")
                }],
            },
        ]
    );
}

#[test]
fn already_sorted_with_disabled_in_middle_has_no_moves() {
    let (_t, mods) = playset(&[on("A", &[]), off("Off"), on("B", &["A"])]);
    let p = sort(&mods, &[]).unwrap();
    assert!(p.moves.is_empty());
    assert_eq!(names(&mods, &p.order), ["A", "Off", "B"]);
    assert!(p.winner_changes.is_empty());
}

#[test]
fn disabled_mod_keeps_its_slot() {
    let (_t, mods) = playset(&[on("Child", &["Parent"]), off("Off"), on("Parent", &[])]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["Parent", "Off", "Child"]);
    let spans: Vec<(usize, usize)> = p.moves.iter().map(|m| (m.from, m.to)).collect();
    assert_eq!(spans, [(0, 2), (2, 0)]);
}

#[test]
fn unrelated_shifted_mods_are_not_moves() {
    let (_t, mods) = playset(&[
        on("X", &[]),
        on("Child", &["Parent"]),
        on("Y", &[]),
        on("Parent", &[]),
    ]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["X", "Y", "Parent", "Child"]);
    assert_eq!(moved(&mods, &p), ["Child", "Parent"]);
}

#[test]
fn mod_on_violated_edge_keeping_its_index_is_not_a_move() {
    // A sits on two violated edges (B needs A, A needs Y) yet ends on index 2 again.
    let (_t, mods) = playset(&[on("B", &["A"]), on("X", &[]), on("A", &["Y"]), on("Y", &[])]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["X", "Y", "A", "B"]);
    assert_eq!(moved(&mods, &p), ["B", "Y"]);
    let spans: Vec<(usize, usize)> = p.moves.iter().map(|m| (m.from, m.to)).collect();
    assert_eq!(spans, [(0, 3), (3, 1)]);
}

#[test]
fn mod_dragged_along_a_dependency_is_a_move() {
    // A needs L (broken); C, a compatch, needs A (kept) and travels with A past M.
    let (_t, mods) = playset(&[on("A", &["L"]), on("C", &["A"]), on("M", &[]), on("L", &[])]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["M", "L", "A", "C"]);
    assert_eq!(moved(&mods, &p), ["A", "C", "L"]);
    assert_eq!(
        p.moves[1],
        Move {
            mod_id: id(&mods, "C"),
            from: 1,
            to: 3,
            reasons: vec![MoveReason::Dependency { on: id(&mods, "A") }],
        }
    );
}

#[test]
fn dragged_chain_reasons_point_toward_the_broken_constraint() {
    // D needs C needs A needs L (broken); C also needs Z, which leads away from the chain.
    let (_t, mods) = playset(&[
        on("Z", &[]),
        on("A", &["L"]),
        on("C", &["A", "Z"]),
        on("D", &["C"]),
        on("M", &[]),
        on("L", &[]),
    ]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["Z", "M", "L", "A", "C", "D"]);
    assert_eq!(moved(&mods, &p), ["A", "C", "D", "L"]);
    let reasons = |name: &str| {
        let id = id(&mods, name);
        p.moves
            .iter()
            .find(|m| m.mod_id == id)
            .unwrap()
            .reasons
            .clone()
    };
    assert_eq!(
        reasons("C"),
        [MoveReason::Dependency { on: id(&mods, "A") }]
    );
    assert_eq!(
        reasons("D"),
        [MoveReason::Dependency { on: id(&mods, "C") }]
    );
}

#[test]
fn mod_shifted_up_when_a_dependent_leaves_is_not_dragged() {
    // A needs L (broken) and F; Y needs F. A moving later frees a slot, so Y and M shift up,
    // but "Y needs F" cannot push Y earlier: neither is a move.
    let (_t, mods) = playset(&[
        on("F", &[]),
        on("A", &["L", "F"]),
        on("Y", &["F"]),
        on("M", &[]),
        on("L", &[]),
    ]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["F", "Y", "M", "L", "A"]);
    assert_eq!(moved(&mods, &p), ["A", "L"]);
}

#[test]
fn mod_pulled_by_a_later_moving_dependent_is_not_dragged() {
    // C moves later with A; that cannot pull Z earlier, Z only fills A's old slot.
    let (_t, mods) = playset(&[
        on("A", &["L"]),
        on("Z", &[]),
        on("C", &["A", "Z"]),
        on("D", &["C"]),
        on("M", &[]),
        on("L", &[]),
    ]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(names(&mods, &p.order), ["Z", "M", "L", "A", "C", "D"]);
    assert_eq!(moved(&mods, &p), ["A", "C", "D", "L"]);
    let reasons: Vec<Vec<MoveReason>> = p.moves.iter().map(|m| m.reasons.clone()).collect();
    assert_eq!(
        reasons,
        [
            vec![MoveReason::Dependency { on: id(&mods, "L") }],
            vec![MoveReason::Dependency { on: id(&mods, "A") }],
            vec![MoveReason::Dependency { on: id(&mods, "C") }],
            vec![MoveReason::RequiredBy { by: id(&mods, "A") }],
        ]
    );
}

#[test]
fn user_rule_applies_and_unknown_rule_is_unresolved() {
    let (_t, mods) = playset(&[on("A", &[]), on("B", &[])]);
    let rules = parse_rules(
        "[[rule]]\nbefore = \"B\"\nafter = \"A\"\n[[rule]]\nbefore = \"Nope\"\nafter = \"A\"\n",
    )
    .unwrap();
    let p = sort(&mods, &rules).unwrap();
    assert_eq!(names(&mods, &p.order), ["B", "A"]);
    assert_eq!(p.unresolved_rules.len(), 1);
    assert_eq!(p.unresolved_rules[0].before, "Nope");
    assert_eq!(
        p.moves[0].reasons,
        [MoveReason::UserRule {
            other: id(&mods, "B"),
            note: None
        }]
    );
}

#[test]
fn rule_naming_a_disabled_mod_is_unresolved() {
    let (_t, mods) = playset(&[on("A", &[]), off("Off")]);
    let rules = parse_rules("[[rule]]\nbefore = \"Off\"\nafter = \"A\"\n").unwrap();
    let p = sort(&mods, &rules).unwrap();
    assert_eq!(names(&mods, &p.order), ["A", "Off"]);
    assert_eq!(p.unresolved_rules, rules);
}

#[test]
fn cycle_reports_only_cycle_members() {
    // C comes first, so the search starts from a mod that is only blocked by the cycle.
    let (_t, mods) = playset(&[on("C", &["A"]), on("A", &["B"]), on("B", &["A"])]);
    let sorted = |m: &[String]| {
        let mut m = m.to_vec();
        m.sort();
        m
    };
    assert!(matches!(
        sort(&mods, &[]),
        Err(CoreError::Cycle(m)) if sorted(&m) == ["A", "B"]
    ));
}

#[test]
fn reports_winner_changes() {
    let shared: Files = &[("gfx/shared.dds", b"same path")];
    let (_t, mods) = playset(&[
        Spec {
            files: shared,
            ..on("Child", &["Parent"])
        },
        Spec {
            files: shared,
            ..on("Parent", &[])
        },
    ]);
    let p = sort(&mods, &[]).unwrap();
    assert_eq!(p.winner_changes.len(), 1);
    let change = &p.winner_changes[0];
    assert_eq!(change.level, ConflictLevel::File);
    assert_eq!(change.target, "gfx/shared.dds");
    assert_eq!(change.before.source, Source::Mod(id(&mods, "Parent")));
    assert_eq!(change.after.source, Source::Mod(id(&mods, "Child")));
}

#[test]
fn load_rules_reads_spec_format_and_skips_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("rules.toml");
    assert_eq!(load_rules(&path).unwrap(), []);
    std::fs::write(
        &path,
        "\u{feff}[[rule]]\nbefore = \"Medieval Arts\"          # name or steam:<id>\n\
         after = \"steam:3816257844\"\nnote = \"compatch loads after the base\"\n",
    )
    .unwrap();
    assert_eq!(
        load_rules(&path).unwrap(),
        [Rule {
            before: "Medieval Arts".into(),
            after: "steam:3816257844".into(),
            note: Some("compatch loads after the base".into()),
        }]
    );
}

#[test]
fn malformed_rules_are_an_error() {
    assert!(matches!(
        parse_rules("[[rule]]\nbefore = \"A\"\n"),
        Err(CoreError::Rules(_))
    ));
    assert!(matches!(
        parse_rules("[[rule]\nbefore = \"A\"\nafter = \"B\"\n"),
        Err(CoreError::Rules(_))
    ));
}

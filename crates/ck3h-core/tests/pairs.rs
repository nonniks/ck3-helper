#![cfg(feature = "testkit")]

use ck3h_core::conflicts::*;
use ck3h_core::deps::build_dep_graph;
use ck3h_core::index::{ModIndex, RelPath, index_vanilla};
use ck3h_core::launcher::LauncherDb;
use ck3h_core::playset::{ModEntry, load_playset};
use ck3h_core::sort::{intent_graph, parse_rules, resolve_rules};
use ck3h_core::testkit::*;
use std::path::Path;

type Files<'a> = &'a [(&'a str, &'a [u8])];

fn load(user: &Path) -> Vec<ModEntry> {
    let db = LauncherDb::open_read_only(&user.join("launcher-v2.sqlite")).unwrap();
    let playset = db.active_playset().unwrap();
    load_playset(&db, &playset, "l_english").unwrap()
}

fn standard() -> (tempfile::TempDir, Vec<ModEntry>) {
    let tmp = tempfile::tempdir().unwrap();
    let mods = load(&standard_fixture(tmp.path()));
    (tmp, mods)
}

/// An active playset of mods `(name, enabled, files)`; positions follow the order. Each mod gets
/// a `descriptor.mod` with its name unless `files` brings its own.
fn custom(specs: &[(&str, bool, Files)]) -> (tempfile::TempDir, Vec<ModEntry>) {
    let tmp = tempfile::tempdir().unwrap();
    let conn = create_launcher_db(&tmp.path().join("launcher-v2.sqlite"));
    let ps = add_playset(&conn, "Main", true);
    for (i, &(name, enabled, files)) in specs.iter().enumerate() {
        let descriptor = format!("name=\"{name}\"\n");
        let mut all: Vec<(&str, &[u8])> = vec![("descriptor.mod", descriptor.as_bytes())];
        all.extend_from_slice(files);
        let dir = write_mod(tmp.path(), &format!("m{i}"), &all);
        let m = add_mod(&conn, name, &dir, None);
        add_playset_mod(&conn, &ps, &m, i as i64, enabled);
    }
    drop(conn);
    let mods = load(tmp.path());
    (tmp, mods)
}

fn vanilla(files: Files) -> (tempfile::TempDir, ModIndex) {
    let tmp = tempfile::tempdir().unwrap();
    let game = write_mod(tmp.path(), "game", files);
    let index = index_vanilla(&game, "l_english").unwrap();
    (tmp, index)
}

fn id(mods: &[ModEntry], name: &str) -> String {
    mods.iter().find(|m| m.name == name).unwrap().id.clone()
}

fn find<'a>(r: &'a ConflictReport, level: ConflictLevel, target: &str) -> &'a Conflict {
    r.conflicts
        .iter()
        .find(|c| c.level == level && c.target == target)
        .unwrap_or_else(|| panic!("no {level:?} conflict for {target}"))
}

/// The mod's name, or "vanilla".
fn label<'a>(mods: &'a [ModEntry], source: &Source) -> &'a str {
    match source {
        Source::Mod(id) => &mods.iter().find(|m| m.id == *id).unwrap().name,
        Source::Vanilla => "vanilla",
    }
}

type Row<'a> = (&'a str, &'a str, Verdict, usize, usize, usize);

fn rows<'a>(mods: &'a [ModEntry], pairs: &[PairSummary]) -> Vec<Row<'a>> {
    pairs
        .iter()
        .map(|p| {
            (
                label(mods, &p.winner),
                label(mods, &p.loser),
                p.verdict,
                p.files,
                p.objects,
                p.loc,
            )
        })
        .collect()
}

fn names<'a>(mods: &'a [ModEntry], pairs: &[PairSummary]) -> Vec<(&'a str, &'a str)> {
    pairs
        .iter()
        .map(|p| (label(mods, &p.winner), label(mods, &p.loser)))
        .collect()
}

fn pair<'a>(
    mods: &[ModEntry],
    pairs: &'a [PairSummary],
    winner: &str,
    loser: &str,
) -> &'a PairSummary {
    pairs
        .iter()
        .find(|p| label(mods, &p.winner) == winner && label(mods, &p.loser) == loser)
        .unwrap_or_else(|| panic!("no pair {winner} → {loser}"))
}

#[cfg(not(target_os = "linux"))]
#[test]
fn standard_pairs() {
    // A beats B on shared_trait and shared_key; B beats A on the events file.
    let (_t, mods) = standard();
    let deps = build_dep_graph(&mods);
    let r = analyze(&mods, &deps, None);
    assert_eq!(
        rows(&mods, &pairs(&r, &deps)),
        [
            ("Base A", "Base B", Verdict::Suspicious, 0, 1, 1),
            ("Base B", "Base A", Verdict::Suspicious, 1, 0, 0)
        ]
    );
}

#[test]
fn rule_makes_conflicts_and_pair_intended() {
    let (_t, mods) = standard();
    let deps = build_dep_graph(&mods);
    let rules = parse_rules("[[rule]]\nbefore = \"Base B\"\nafter = \"Base A\"\n").unwrap();
    let intent = intent_graph(&deps, &mods, &rules);
    assert!(
        intent
            .edges
            .contains(&(id(&mods, "Base B"), id(&mods, "Base A")))
    );
    let r = analyze(&mods, &intent, None);
    assert_eq!(
        find(&r, ConflictLevel::Object, "common/traits::shared_trait").verdict,
        Verdict::Intended
    );
    assert_eq!(
        pair(&mods, &pairs(&r, &intent), "Base A", "Base B").verdict,
        Verdict::Intended
    );
    assert!(intent.compatches == deps.compatches && intent.issues == deps.issues);
}

#[test]
fn disabled_or_ambiguous_rule_is_not_intent() {
    let (_t, mods) = custom(&[
        ("Dup", true, &[("common/traits/a.txt", b"k = { }\n")]),
        ("Dup", true, &[("common/traits/a2.txt", b"k = { }\n")]),
        ("A", true, &[("common/traits/z.txt", b"k = { }\n")]),
        ("Off", false, &[("common/traits/z.txt", b"k = { }\n")]),
    ]);
    let deps = build_dep_graph(&mods);
    let rules = parse_rules(
        "[[rule]]\nbefore = \"Dup\"\nafter = \"A\"\n[[rule]]\nbefore = \"Off\"\nafter = \"A\"\n",
    )
    .unwrap();
    let (applied, unresolved) = resolve_rules(&mods, &rules);
    assert!(applied.is_empty());
    assert_eq!(unresolved.len(), 2);
    assert_eq!(unresolved, rules);
    let intent = intent_graph(&deps, &mods, &rules);
    let r = analyze(&mods, &intent, None);
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(r.conflicts[0].winner.source, Source::Mod(id(&mods, "A")));
    assert_eq!(r.conflicts[0].verdict, Verdict::Suspicious);
    assert!(
        pairs(&r, &intent)
            .iter()
            .all(|p| p.verdict == Verdict::Suspicious)
    );
}

fn three_way() -> (tempfile::TempDir, Vec<ModEntry>) {
    custom(&[
        ("X", true, &[("common/traits/a.txt", b"k = { }\n")]),
        ("Y", true, &[("common/traits/b.txt", b"k = { }\n")]),
        ("Z", true, &[("common/traits/c.txt", b"k = { }\n")]),
    ])
}

#[test]
fn multi_loser_conflict_counts_in_each_pair() {
    let (_t, mods) = three_way();
    let deps = build_dep_graph(&mods);
    let p = pairs(&analyze(&mods, &deps, None), &deps);
    assert_eq!(
        rows(&mods, &p),
        [
            ("Z", "X", Verdict::Suspicious, 0, 1, 0),
            ("Z", "Y", Verdict::Suspicious, 0, 1, 0)
        ]
    );
}

#[test]
fn pair_skips_the_winner_and_counts_a_loser_once() {
    // Z also loses k to itself (0_z.txt); X defines k in two files.
    let (_t, mods) = custom(&[
        (
            "X",
            true,
            &[
                ("common/traits/a.txt", b"k = { }\n"),
                ("common/traits/a2.txt", b"k = { }\n"),
            ],
        ),
        (
            "Z",
            true,
            &[
                ("common/traits/0_z.txt", b"k = { }\n"),
                ("common/traits/c.txt", b"k = { }\n"),
            ],
        ),
    ]);
    let deps = build_dep_graph(&mods);
    let r = analyze(&mods, &deps, None);
    assert_eq!(
        rows(&mods, &pairs(&r, &deps)),
        [("Z", "X", Verdict::Suspicious, 0, 1, 0)]
    );
    let (x, z) = (Source::Mod(id(&mods, "X")), Source::Mod(id(&mods, "Z")));
    let c = find(&r, ConflictLevel::Object, "common/traits::k");
    assert!(involves(c, &z, &x));
    assert!(!involves(c, &z, &z));
}

#[test]
fn vanilla_pairs_follow_spec() {
    // a: M's file reads after vanilla's, so M wins; b: vanilla's reads after M's, so vanilla wins.
    let (_t, mods) = custom(&[(
        "M",
        true,
        &[("common/traits/50_m.txt", b"a = {}\nb = {}\n")],
    )]);
    let (_v, vanilla) = vanilla(&[
        ("common/traits/00_v.txt", b"a = {}\n"),
        ("common/traits/zz_v.txt", b"b = {}\n"),
    ]);
    let deps = build_dep_graph(&mods);
    let p = pairs(&analyze(&mods, &deps, Some(&vanilla)), &deps);
    assert_eq!(pair(&mods, &p, "M", "vanilla").verdict, Verdict::Vanilla);
    assert_eq!(pair(&mods, &p, "vanilla", "M").verdict, Verdict::Suspicious);
    assert_eq!(
        rows(&mods, &p),
        [
            ("vanilla", "M", Verdict::Suspicious, 0, 1, 0),
            ("M", "vanilla", Verdict::Vanilla, 0, 1, 0)
        ]
    );
}

#[test]
fn pairs_sorted_by_verdict_then_count() {
    let (_t, mods) = custom(&[
        ("R", true, &[("common/traits/z_r.txt", b"r1 = { }\n")]),
        ("S", true, &[("common/traits/a_s.txt", b"r1 = { }\n")]),
        (
            "I",
            true,
            &[
                (
                    "descriptor.mod",
                    b"name=\"I\"\ndependencies={\n\t\"J\"\n}\n",
                ),
                (
                    "common/traits/z_i.txt",
                    b"i1 = { }\ni2 = { }\ni3 = { }\ni4 = { }\ni5 = { }\n",
                ),
            ],
        ),
        (
            "J",
            true,
            &[(
                "common/traits/a_j.txt",
                b"i1 = { }\ni2 = { }\ni3 = { }\ni4 = { }\ni5 = { }\n",
            )],
        ),
        (
            "P",
            true,
            &[("common/traits/z_p.txt", b"p1 = { }\np2 = { }\np3 = { }\n")],
        ),
        (
            "Q",
            true,
            &[("common/traits/a_q.txt", b"p1 = { }\np2 = { }\np3 = { }\n")],
        ),
    ]);
    let deps = build_dep_graph(&mods);
    let p = pairs(&analyze(&mods, &deps, None), &deps);
    assert_eq!(names(&mods, &p), [("P", "Q"), ("R", "S"), ("I", "J")]);
    assert_eq!(pair(&mods, &p, "I", "J").verdict, Verdict::Intended);
}

#[test]
fn involves_checks_winner_and_any_loser() {
    let (_t, mods) = three_way();
    let r = analyze(&mods, &build_dep_graph(&mods), None);
    let c = find(&r, ConflictLevel::Object, "common/traits::k");
    let [x, y, z] = ["X", "Y", "Z"].map(|name| Source::Mod(id(&mods, name)));
    assert!(involves(c, &z, &y));
    assert!(involves(c, &z, &x));
    assert!(!involves(c, &y, &z));
    assert!(!involves(c, &Source::Vanilla, &x));
}

#[test]
fn loc_value_by_file_and_key() {
    let (_t, mods) = standard();
    let a = mods
        .iter()
        .find(|m| m.name == "Base A")
        .unwrap()
        .index
        .as_ref()
        .unwrap();
    assert_eq!(
        a.loc_value(
            &RelPath::new("localization/english/base_a_l_english.yml"),
            "shared_key"
        ),
        Some("From A")
    );
    assert_eq!(
        a.loc_value(
            &RelPath::new("localization/english/base_a_l_english.yml"),
            "nope"
        ),
        None
    );

    // The same key in two files of one mod: each file gives its own text, the first line wins.
    let (_t2, mods) = custom(&[(
        "Two",
        true,
        &[
            (
                "localization/english/aa_l_english.yml",
                "\u{feff}l_english:\n k:0 \"AA\"\n".as_bytes(),
            ),
            (
                "localization/english/zz_l_english.yml",
                "\u{feff}l_english:\n j:0 \"J\"\n k:0 \"ZZ\"\n k:1 \"ZZ again\"\n".as_bytes(),
            ),
        ],
    )]);
    let two = mods[0].index.as_ref().unwrap();
    let aa = RelPath::new("localization/english/aa_l_english.yml");
    let zz = RelPath::new("localization/english/zz_l_english.yml");
    assert_eq!(two.loc_value(&aa, "k"), Some("AA"));
    assert_eq!(two.loc_value(&zz, "k"), Some("ZZ"));
    assert_eq!(two.loc_value(&aa, "j"), None);
    assert_eq!(
        two.loc_value(
            &RelPath::new("localization/english/missing_l_english.yml"),
            "k"
        ),
        None
    );
}

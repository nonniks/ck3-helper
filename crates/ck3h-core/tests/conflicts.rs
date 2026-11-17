#![cfg(feature = "testkit")]

use ck3h_core::conflicts::*;
use ck3h_core::deps::build_dep_graph;
use ck3h_core::index::{ModIndex, index_vanilla};
use ck3h_core::launcher::LauncherDb;
use ck3h_core::playset::{ModEntry, load_playset};
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

/// An active playset of enabled mods `(name, files)`; positions follow the order.
fn custom(specs: &[(&str, Files)]) -> (tempfile::TempDir, Vec<ModEntry>) {
    let tmp = tempfile::tempdir().unwrap();
    let conn = create_launcher_db(&tmp.path().join("launcher-v2.sqlite"));
    let ps = add_playset(&conn, "Main", true);
    for (i, (name, files)) in specs.iter().enumerate() {
        let dir = write_mod(tmp.path(), &format!("m{i}"), files);
        let m = add_mod(&conn, name, &dir, None);
        add_playset_mod(&conn, &ps, &m, i as i64, true);
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

fn report(mods: &[ModEntry], vanilla: Option<&ModIndex>) -> ConflictReport {
    analyze(mods, &build_dep_graph(mods), vanilla)
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

fn sources(contenders: &[Contender]) -> Vec<Source> {
    contenders.iter().map(|c| c.source.clone()).collect()
}

#[cfg(not(target_os = "linux"))]
#[test]
fn file_conflict_ignores_case_and_lower_mod_wins() {
    let (_t, mods) = standard();
    let r = report(&mods, None);
    let c = find(&r, ConflictLevel::File, "events/shared_events.txt");
    assert_eq!(c.winner.source, Source::Mod(id(&mods, "Base B")));
    assert_eq!(c.winner.file.display, "Events/Shared_Events.txt");
    assert_eq!(sources(&c.losers), [Source::Mod(id(&mods, "Base A"))]);
    assert_eq!(c.reason, WinReason::LowerInPlayset);
    assert_eq!(c.verdict, Verdict::Suspicious);
}

#[cfg(target_os = "linux")]
#[test]
fn case_different_paths_do_not_conflict_on_linux() {
    let (_t, mods) = standard();
    let r = report(&mods, None);
    assert!(r.conflicts.iter().all(|c| c.level != ConflictLevel::File));
}

#[test]
fn object_winner_is_later_file_path_not_lower_mod() {
    let (_t, mods) = standard();
    let r = report(&mods, None);
    let c = find(&r, ConflictLevel::Object, "common/traits::shared_trait");
    assert_eq!(c.winner.source, Source::Mod(id(&mods, "Base A")));
    assert_eq!(c.winner.file.norm, "common/traits/zz_base_a.txt");
    assert_eq!(sources(&c.losers), [Source::Mod(id(&mods, "Base B"))]);
    assert_eq!(c.reason, WinReason::LaterFilePath);
    assert_eq!(c.verdict, Verdict::Suspicious);
}

#[test]
fn loc_first_definition_wins() {
    let (_t, mods) = standard();
    let r = report(&mods, None);
    let c = find(&r, ConflictLevel::Loc, "l_english::shared_key");
    assert_eq!(c.winner.source, Source::Mod(id(&mods, "Base A")));
    assert_eq!(sources(&c.losers), [Source::Mod(id(&mods, "Base B"))]);
    assert_eq!(c.reason, WinReason::EarlierFilePath);
}

#[test]
fn replace_folder_beats_regular_loc() {
    // The regular file sorts before `replace/`, so first-wins alone would pick it.
    let (_t, mods) = custom(&[
        (
            "Early",
            &[
                ("descriptor.mod", b"name=\"Early\"\n"),
                (
                    "localization/english/early_l_english.yml",
                    "\u{feff}l_english:\n replace_key:0 \"Early\"\n".as_bytes(),
                ),
            ],
        ),
        (
            "Compat",
            &[
                ("descriptor.mod", b"name=\"Compat\"\n"),
                (
                    "localization/english/replace/compat_l_english.yml",
                    "\u{feff}l_english:\n replace_key:0 \"Compat\"\n".as_bytes(),
                ),
            ],
        ),
    ]);
    let r = report(&mods, None);
    let c = find(&r, ConflictLevel::Loc, "l_english::replace_key");
    assert_eq!(c.winner.source, Source::Mod(id(&mods, "Compat")));
    assert_eq!(sources(&c.losers), [Source::Mod(id(&mods, "Early"))]);
    assert_eq!(c.reason, WinReason::ReplaceFolder);
}

#[test]
fn last_replace_file_wins() {
    let (_t, mods) = custom(&[
        (
            "Late Path",
            &[
                ("descriptor.mod", b"name=\"Late Path\"\n"),
                (
                    "localization/replace/english/zz_l_english.yml",
                    "\u{feff}l_english:\n k:0 \"zz\"\n".as_bytes(),
                ),
            ],
        ),
        (
            "Early Path",
            &[
                ("descriptor.mod", b"name=\"Early Path\"\n"),
                (
                    "localization/english/replace/aa_l_english.yml",
                    "\u{feff}l_english:\n k:0 \"aa\"\n".as_bytes(),
                ),
            ],
        ),
    ]);
    let (winner, losers, reason) =
        &resolve_loc(&mods, None)[&("l_english".to_string(), "k".to_string())];
    assert_eq!(winner.source, Source::Mod(id(&mods, "Late Path")));
    assert_eq!(sources(losers), [Source::Mod(id(&mods, "Early Path"))]);
    assert_eq!(*reason, WinReason::ReplaceFolder);
}

#[test]
fn dependent_winner_is_intended() {
    let (_t, mods) = custom(&[
        (
            "Base A",
            &[
                ("descriptor.mod", b"name=\"Base A\"\n"),
                ("common/traits/zz_base_a.txt", b"shared_trait = { a = 1 }\n"),
            ],
        ),
        (
            "Base B",
            &[
                ("descriptor.mod", b"name=\"Base B\"\n"),
                ("common/traits/00_base_b.txt", b"shared_trait = { b = 2 }\n"),
            ],
        ),
        (
            "A+B Compatch",
            &[
                (
                    "descriptor.mod",
                    b"name=\"A+B Compatch\"\ndependencies={\n\t\"Base A\"\n\t\"Base B\"\n}\n",
                ),
                (
                    "common/traits/zzz_compat.txt",
                    b"shared_trait = { c = 3 }\n",
                ),
            ],
        ),
    ]);
    let r = report(&mods, None);
    let c = find(&r, ConflictLevel::Object, "common/traits::shared_trait");
    assert_eq!(c.winner.source, Source::Mod(id(&mods, "A+B Compatch")));
    assert_eq!(
        sources(&c.losers),
        [
            Source::Mod(id(&mods, "Base B")),
            Source::Mod(id(&mods, "Base A"))
        ]
    );
    assert_eq!(c.verdict, Verdict::Intended);
    assert_eq!(r.compatches, [id(&mods, "A+B Compatch")]);
}

#[test]
fn replace_path_drops_files_of_earlier_mods() {
    let (_t, mods) = custom(&[
        (
            "Old",
            &[
                ("descriptor.mod", b"name=\"Old\"\n"),
                ("history/characters/x.txt", b"1 = { }\n"),
                ("history/characters/sub/y.txt", b"2 = { }\n"),
            ],
        ),
        (
            "TC",
            &[
                (
                    "descriptor.mod",
                    b"name=\"TC\"\nreplace_path=\"history/characters\"\n",
                ),
                ("history/characters/z.txt", b"3 = { }\n"),
            ],
        ),
    ]);
    let files = effective_files(&mods, None);
    assert!(!files.contains_key("history/characters/x.txt"));
    assert!(files.contains_key("history/characters/sub/y.txt"));
    assert_eq!(
        sources(&files["history/characters/z.txt"]),
        [Source::Mod(id(&mods, "TC"))]
    );
    let (_v, vanilla) = vanilla(&[
        ("history/characters/v.txt", b"4 = { }\n"),
        ("history/titles/t.txt", b"k_x = { }\n"),
    ]);
    let files = effective_files(&mods, Some(&vanilla));
    assert!(!files.contains_key("history/characters/v.txt"));
    assert_eq!(sources(&files["history/titles/t.txt"]), [Source::Vanilla]);
    let r = report(&mods, None);
    assert_eq!(
        r.replace_paths,
        [(id(&mods, "TC"), vec!["history/characters".to_string()])]
    );
}

#[test]
fn overridden_file_contributes_no_objects() {
    let (_t, mods) = custom(&[
        (
            "First",
            &[
                ("descriptor.mod", b"name=\"First\"\n"),
                ("common/traits/same.txt", b"t = { a = 1 }\ngone = { }\n"),
            ],
        ),
        (
            "Second",
            &[
                ("descriptor.mod", b"name=\"Second\"\n"),
                ("common/traits/same.txt", b"t = { b = 2 }\n"),
            ],
        ),
    ]);
    let objects = resolve_objects(&mods, None);
    assert_eq!(
        sources(&objects[&("common/traits".to_string(), "t".to_string())]),
        [Source::Mod(id(&mods, "Second"))]
    );
    assert!(!objects.contains_key(&("common/traits".to_string(), "gone".to_string())));
    let r = report(&mods, None);
    assert_eq!(
        find(&r, ConflictLevel::File, "common/traits/same.txt")
            .winner
            .source,
        Source::Mod(id(&mods, "Second"))
    );
    assert!(r.conflicts.iter().all(|c| c.level != ConflictLevel::Object));
}

#[test]
fn vanilla_only_conflict_marked_vanilla() {
    let (_t, mods) = standard();
    let (_v, vanilla) = vanilla(&[("common/traits/00_traits.txt", b"only_a = {}\n")]);
    let r = report(&mods, Some(&vanilla));
    let c = find(&r, ConflictLevel::Object, "common/traits::only_a");
    assert_eq!(c.winner.source, Source::Mod(id(&mods, "Base A")));
    assert_eq!(sources(&c.losers), [Source::Vanilla]);
    assert_eq!(c.verdict, Verdict::Vanilla);
}

#[test]
fn vanilla_winning_over_a_mod_is_suspicious() {
    let (_t, mods) = custom(&[(
        "M",
        &[
            ("descriptor.mod", b"name=\"M\"\n"),
            ("common/traits/00_a.txt", b"brave = { }\n"),
            ("common/traits/zz_m.txt", b"craven = { }\n"),
            (
                "localization/english/zz_m_l_english.yml",
                "\u{feff}l_english:\n brave:0 \"M\"\n".as_bytes(),
            ),
            (
                "localization/english/aa_m_l_english.yml",
                "\u{feff}l_english:\n craven:0 \"M\"\n".as_bytes(),
            ),
        ],
    )]);
    let (_v, vanilla) = vanilla(&[
        (
            "common/traits/00_traits.txt",
            b"brave = { }\ncraven = { }\n",
        ),
        (
            "localization/english/traits_l_english.yml",
            "\u{feff}l_english:\n brave:0 \"V\"\n craven:0 \"V\"\n".as_bytes(),
        ),
    ]);
    let r = report(&mods, Some(&vanilla));
    let m = Source::Mod(id(&mods, "M"));
    for (level, target, winner, verdict) in [
        (
            ConflictLevel::Object,
            "common/traits::brave",
            Source::Vanilla,
            Verdict::Suspicious,
        ),
        (
            ConflictLevel::Object,
            "common/traits::craven",
            m.clone(),
            Verdict::Vanilla,
        ),
        (
            ConflictLevel::Loc,
            "l_english::brave",
            Source::Vanilla,
            Verdict::Suspicious,
        ),
        (
            ConflictLevel::Loc,
            "l_english::craven",
            m.clone(),
            Verdict::Vanilla,
        ),
    ] {
        let c = find(&r, level, target);
        assert_eq!(
            (&c.winner.source, c.verdict),
            (&winner, verdict),
            "{target}"
        );
    }
}

#[test]
fn replace_path_tolerates_trailing_and_repeated_slashes() {
    let (_t, mods) = custom(&[
        (
            "Old",
            &[
                ("descriptor.mod", b"name=\"Old\"\n"),
                ("history/characters/x.txt", b"1 = { }\n"),
                ("history/titles/t.txt", b"k_x = { }\n"),
            ],
        ),
        (
            "TC",
            &[(
                "descriptor.mod",
                b"name=\"TC\"\nreplace_path=\"history/characters/\"\nreplace_path=\"history//titles\"\n",
            )],
        ),
    ]);
    let files = effective_files(&mods, None);
    assert!(!files.contains_key("history/characters/x.txt"));
    assert!(!files.contains_key("history/titles/t.txt"));
}

#[test]
fn report_lists_missing_mods_and_is_deterministic() {
    let (_t, mods) = standard();
    let r = report(&mods, None);
    assert_eq!(r.missing_mods, [id(&mods, "Ghost")]);
    assert!(
        r.warnings
            .iter()
            .any(|(m, w)| *m == id(&mods, "Broken") && w.file == "common/traits/bad.txt")
    );
    assert!(
        r.conflicts
            .windows(2)
            .all(|w| (w[0].level, &w[0].target) < (w[1].level, &w[1].target))
    );
    assert_eq!(
        serde_json::to_string(&report(&mods, None)).unwrap(),
        serde_json::to_string(&report(&mods, None)).unwrap()
    );
}

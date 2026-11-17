#![cfg(feature = "testkit")]

use ck3h_core::deps::*;
use ck3h_core::launcher::LauncherDb;
use ck3h_core::playset::*;
use ck3h_core::testkit::*;

fn load(user: &std::path::Path) -> Vec<ModEntry> {
    let db = LauncherDb::open_read_only(&user.join("launcher-v2.sqlite")).unwrap();
    let playset = db.active_playset().unwrap();
    load_playset(&db, &playset, "l_english").unwrap()
}

fn by_name<'a>(mods: &'a [ModEntry], name: &str) -> &'a ModEntry {
    mods.iter().find(|m| m.name == name).unwrap()
}

fn id(mods: &[ModEntry], name: &str) -> String {
    by_name(mods, name).id.clone()
}

/// Builds a playset from `(name, dependencies)`; every mod is enabled, positions follow the order.
fn custom(specs: &[(&str, &[&str])]) -> (tempfile::TempDir, Vec<ModEntry>) {
    let tmp = tempfile::tempdir().unwrap();
    let conn = create_launcher_db(&tmp.path().join("launcher-v2.sqlite"));
    let ps = add_playset(&conn, "Main", true);
    for (i, (name, deps)) in specs.iter().enumerate() {
        let deps: String = deps.iter().map(|d| format!("\t\"{d}\"\n")).collect();
        let descriptor = format!("name=\"{name}\"\ndependencies={{\n{deps}}}\n");
        let dir = write_mod(
            tmp.path(),
            &format!("m{i}"),
            &[("descriptor.mod", descriptor.as_bytes())],
        );
        let m = add_mod(&conn, name, &dir, None);
        add_playset_mod(&conn, &ps, &m, i as i64, true);
    }
    drop(conn);
    let mods = load(tmp.path());
    (tmp, mods)
}

#[test]
fn loads_in_position_order_with_indexes() {
    let tmp = tempfile::tempdir().unwrap();
    let user = standard_fixture(tmp.path());
    let mods = load(&user);
    let names: Vec<&str> = mods.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        ["Base A", "Base B", "A+B Compatch", "Ghost", "Broken"]
    );
    assert!(mods.windows(2).all(|w| w[0].position < w[1].position));
    let a = by_name(&mods, "Base A");
    assert!(!a.missing && a.enabled);
    assert!(a.descriptor.is_some());
    assert!(a.index.as_ref().unwrap().objects.len() >= 2);
}

#[test]
fn missing_folder_is_flagged_not_indexed() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = load(&standard_fixture(tmp.path()));
    let ghost = by_name(&mods, "Ghost");
    assert!(ghost.missing && ghost.index.is_none() && ghost.descriptor.is_none());
}

#[test]
fn disabled_mod_is_not_indexed() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = create_launcher_db(&tmp.path().join("launcher-v2.sqlite"));
    let ps = add_playset(&conn, "Main", true);
    let dir = write_mod(tmp.path(), "off", &[("descriptor.mod", b"name=\"Off\"")]);
    let m = add_mod(&conn, "Off", &dir, None);
    add_playset_mod(&conn, &ps, &m, 0, false);
    drop(conn);
    let mods = load(tmp.path());
    assert!(!mods[0].enabled && !mods[0].missing && mods[0].index.is_none());
    assert!(mods[0].descriptor.is_some());
}

#[test]
fn broken_descriptor_becomes_warning_and_display_name() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = create_launcher_db(&tmp.path().join("launcher-v2.sqlite"));
    let ps = add_playset(&conn, "Main", true);
    let dir = write_mod(tmp.path(), "bad", &[("descriptor.mod", b"version=\"1\"")]);
    let m = add_mod(&conn, "Shown Name", &dir, None);
    add_playset_mod(&conn, &ps, &m, 0, true);
    drop(conn);
    let mods = load(tmp.path());
    assert_eq!(mods[0].name, "Shown Name");
    assert!(mods[0].descriptor.is_none());
    let warnings = &mods[0].index.as_ref().unwrap().warnings;
    assert!(warnings.iter().any(|w| w.file == "descriptor.mod"));
}

#[test]
fn compatch_detected_and_edges_built() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = load(&standard_fixture(tmp.path()));
    let g = build_dep_graph(&mods);
    let id = |n: &str| id(&mods, n);
    assert!(g.compatches.contains(&id("A+B Compatch")));
    assert!(g.depends_on(&id("A+B Compatch"), &id("Base A")));
    assert!(g.depends_on(&id("A+B Compatch"), &id("Base B")));
    assert!(!g.depends_on(&id("Base A"), &id("A+B Compatch")));
}

#[test]
fn depends_on_is_transitive() {
    let (_t, mods) = custom(&[("A", &[]), ("B", &["A"]), ("C", &["B"])]);
    let g = build_dep_graph(&mods);
    let id = |n: &str| id(&mods, n);
    assert!(g.depends_on(&id("C"), &id("A")));
    assert!(!g.depends_on(&id("A"), &id("C")));
}

#[test]
fn dependency_cycle_terminates() {
    let (_t, mods) = custom(&[("A", &["B"]), ("B", &["A"])]);
    let g = build_dep_graph(&mods);
    assert!(g.depends_on(&id(&mods, "A"), &id(&mods, "B")));
    assert!(!g.depends_on(&id(&mods, "A"), "nope"));
}

#[test]
fn missing_dependency_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = load(&standard_fixture(tmp.path()));
    let g = build_dep_graph(&mods);
    assert_eq!(
        g.issues,
        [DepIssue {
            mod_id: id(&mods, "Broken"),
            dependency: "Missing Mod".into(),
            kind: DepIssueKind::Missing
        }]
    );
}

#[test]
fn duplicate_names_are_ambiguous() {
    let (_t, mods) = custom(&[
        ("Same Name", &[]),
        ("Same Name", &[]),
        ("User", &["same name "]),
    ]);
    let g = build_dep_graph(&mods);
    assert_eq!(g.issues.len(), 1);
    assert_eq!(g.issues[0].kind, DepIssueKind::Ambiguous);
    assert_eq!(g.issues[0].mod_id, mods[2].id);
    assert!(g.edges.is_empty());
}

#[test]
fn steam_prefix_matches_steam_id() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = create_launcher_db(&tmp.path().join("launcher-v2.sqlite"));
    let ps = add_playset(&conn, "Main", true);
    let dir = write_mod(tmp.path(), "s", &[("descriptor.mod", b"name=\"S\"")]);
    let m = add_mod(&conn, "S", &dir, Some("111"));
    add_playset_mod(&conn, &ps, &m, 0, true);
    drop(conn);
    let mods = load(tmp.path());
    assert_eq!(match_name(&mods, "steam:111"), NameMatch::One(m));
    assert_eq!(match_name(&mods, "steam:222"), NameMatch::None);
    assert_eq!(match_name(&mods, " s "), NameMatch::One(mods[0].id.clone()));
}

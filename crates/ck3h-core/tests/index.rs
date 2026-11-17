#![cfg(feature = "testkit")]

use ck3h_core::databases::database_root;
use ck3h_core::index::*;
use ck3h_core::testkit::write_mod;

fn norms(files: &[RelPath]) -> Vec<&str> {
    files.iter().map(|f| f.norm.as_str()).collect()
}

fn keys(objects: &[ObjectDef]) -> Vec<(&str, &str, u32)> {
    objects
        .iter()
        .map(|o| (o.database.as_str(), o.key.as_str(), o.ordinal))
        .collect()
}

fn loc_keys(loc: &[LocDef]) -> Vec<(&str, &str, bool)> {
    loc.iter()
        .map(|l| (l.key.as_str(), l.value.as_str(), l.is_replace))
        .collect()
}

fn index(files: &[(&str, &[u8])]) -> ModIndex {
    let tmp = tempfile::tempdir().unwrap();
    let dir = write_mod(tmp.path(), "m", files);
    index_mod(&dir, "l_english").unwrap()
}

#[test]
fn indexes_files_except_root_and_hidden() {
    let idx = index(&[
        ("descriptor.mod", b"name=\"M\""),
        ("thumbnail.png", b"png"),
        (".git/x", b"x"),
        ("events/a.txt", b"namespace = a"),
    ]);
    assert_eq!(norms(&idx.files), ["events/a.txt"]);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn paths_fold_case_off_linux() {
    let idx = index(&[("Events/A_Events.txt", b"")]);
    assert_eq!(idx.files[0].norm, "events/a_events.txt");
    assert_eq!(idx.files[0].display, "Events/A_Events.txt");
}

#[cfg(target_os = "linux")]
#[test]
fn paths_keep_case_on_linux() {
    let idx = index(&[("Events/A_Events.txt", b"")]);
    assert_eq!(idx.files[0].norm, "Events/A_Events.txt");
}

#[test]
fn relpath_orders_by_components() {
    assert!(RelPath::new("common/a/z.txt") < RelPath::new("common/a-b/a.txt"));
}

#[test]
fn database_root_longest_prefix() {
    assert_eq!(
        database_root("common/decisions/dlc_decisions/ep_3/x.txt").as_deref(),
        Some("common/decisions")
    );
    assert_eq!(
        database_root("common/culture/cultures/x.txt").as_deref(),
        Some("common/culture/cultures")
    );
    assert_eq!(
        database_root("gfx/portraits/portrait_animations/x.txt").as_deref(),
        Some("gfx/portraits/portrait_animations")
    );
    assert_eq!(
        database_root("common/brand_new_dir/x.txt").as_deref(),
        Some("common/brand_new_dir")
    );
    assert_eq!(
        database_root("common/religion/rite_types/00_rites.txt").as_deref(),
        Some("common/religion/rite_types")
    );
    assert_eq!(
        database_root("common/puppets/actions/x.txt").as_deref(),
        Some("common/puppets/actions")
    );
    assert_eq!(database_root("events/x.txt"), None);
}

#[test]
fn top_level_objects_in_subfolders() {
    let idx = index(&[
        (
            "common/traits/00_t.txt",
            b"brave_plus = { a = 1 }\n@var = 5\nshared = { }\n",
        ),
        ("common/decisions/sub/d.txt", b"my_decision = { }"),
    ]);
    assert_eq!(
        keys(&idx.objects),
        [
            ("common/decisions", "my_decision", 0),
            ("common/traits", "brave_plus", 0),
            ("common/traits", "shared", 1)
        ]
    );
}

#[test]
fn excluded_and_container_databases_have_no_objects() {
    let idx = index(&[
        ("common/on_action/x.txt", b"on_game_start = { }"),
        (
            "common/defines/x.txt",
            b"NGame = { START_DATE = \"867.1.1\" }",
        ),
        ("common/named_colors/c.txt", b"colors = { red = {} }"),
        ("common/landed_titles/t.txt", b"e_x = { k_y = { } }"),
        (
            "common/province_terrain/x.txt",
            b"default_land=plains\n1=mountains\n",
        ),
    ]);
    assert!(idx.objects.is_empty());
    assert_eq!(idx.files.len(), 5);
    assert!(norms(&idx.files).contains(&"common/province_terrain/x.txt"));
}

#[test]
fn localization_for_selected_language_only() {
    let idx_en = index(&[
        (
            "localization/english/a_l_english.yml",
            "\u{FEFF}l_english:\n # c\n k1:0 \"One\"\n k2: \"Two\"\n".as_bytes(),
        ),
        (
            "localization/english/l_english_b.yml",
            b"l_english:\n k3:0 \"Three\"\n",
        ),
        (
            "localization/russian/a_l_russian.yml",
            "\u{FEFF}l_russian:\n k1:0 \"Один\"\n".as_bytes(),
        ),
    ]);
    assert_eq!(
        loc_keys(&idx_en.loc),
        [
            ("k1", "One", false),
            ("k2", "Two", false),
            ("k3", "Three", false)
        ]
    );
}

#[test]
fn replace_folder_marks_loc() {
    let idx = index(&[
        (
            "localization/english/replace/r_l_english.yml",
            b"l_english:\n r1:0 \"R1\"\n",
        ),
        (
            "localization/replace/english/r2_l_english.yml",
            b"l_english:\n r2:0 \"R2\"\n",
        ),
    ]);
    assert_eq!(idx.loc.len(), 2);
    assert!(idx.loc.iter().all(|l| l.is_replace));
}

#[test]
fn broken_inputs_give_warnings_and_continue() {
    let idx = index(&[
        ("common/traits/bad.txt", b"x = { y = 1"),
        ("common/traits/good.txt", b"ok = {}"),
        (
            "localization/english/z_l_english.yml",
            b"l_english:\n z:0 \"a\xFFb\"\n",
        ),
    ]);
    assert_eq!(idx.warnings.len(), 2);
    assert_eq!(keys(&idx.objects), [("common/traits", "ok", 0)]);
    assert_eq!(idx.loc.len(), 1);
}

#[test]
fn stray_top_level_value_breaks_file() {
    let idx = index(&[("common/traits/stray.txt", b"a = { }\nstray\nc = { }")]);
    assert_eq!(
        idx.warnings,
        [IndexWarning {
            file: "common/traits/stray.txt".into(),
            message: "unexpected value at top level".into()
        }]
    );
    assert!(idx.objects.is_empty());
}

#[test]
fn index_types_are_serializable() {
    fn serializable<T: serde::Serialize>() {}
    serializable::<RelPath>();
    serializable::<ObjectDef>();
    serializable::<LocDef>();
    serializable::<IndexWarning>();
    serializable::<ModIndex>();
}

#[test]
fn vanilla_dlc_layer_replaces_base_file() {
    let tmp = tempfile::tempdir().unwrap();
    let game = write_mod(
        tmp.path(),
        "game",
        &[
            ("common/traits/00_traits.txt", b"base_t = {}"),
            ("dlc/dlc001/common/traits/00_traits.txt", b"dlc_t = {}"),
        ],
    );
    let v = index_vanilla(&game, "l_english").unwrap();
    assert_eq!(norms(&v.files), ["common/traits/00_traits.txt"]);
    assert_eq!(keys(&v.objects), [("common/traits", "dlc_t", 0)]);
}

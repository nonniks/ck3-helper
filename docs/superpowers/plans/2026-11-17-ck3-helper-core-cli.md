# CK3 Helper — ядро и CLI: план реализации (план 1 из 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust-библиотека `ck3h-core` и консольная программа `ck3h`, которые читают плейсет из лаунчера CK3, строят карту конфликтов между модами, предлагают порядок загрузки и умеют создать отсортированную копию плейсета.

**Architecture:** Cargo workspace из двух крейтов. `ck3h-core` — чистая логика без UI и сети: чтение базы лаунчера, разбор `descriptor.mod`, индекс файлов/объектов/локализации, движок конфликтов, сортировщик, запись копии плейсета, сверка с `database_conflicts.log`. `ck3h-cli` — тонкая обёртка на `clap`, вывод текстом или JSON. План 2 (Tauri-приложение) использует то же ядро.

**Tech Stack:** Rust 1.95 (edition 2024), rusqlite (bundled SQLite), jomini (парсер формата Paradox), rayon, serde/serde_json, toml, thiserror, sysinfo, chrono, uuid, dirs, clap; тесты — tempfile, assert_cmd, predicates.

**Spec:** `docs/superpowers/specs/2026-11-17-ck3-helper-v1-design.md`

## Global Constraints

- Rust stable 1.95, `edition = "2024"`; workspace в корне `~/Code/ck3-helper`; работа на ветке `feat/core-cli`.
- Крейты: `crates/ck3h-core` (lib), `crates/ck3h-cli` (bin с именем `ck3h`). `license = "GPL-3.0-or-later"` в каждом `Cargo.toml`.
- Зависимости (только эти): `rusqlite = { version = "0.43", features = ["bundled"] }`, `jomini = "0.37"`, `rayon`, `serde` (derive), `serde_json`, `toml`, `thiserror`, `sysinfo = "0.40"`, `chrono`, `uuid` (v4), `dirs`, `clap = { version = "4", features = ["derive"] }`; dev: `tempfile`, `assert_cmd`, `predicates`.
- Ядро не ходит в сеть и пишет на диск только в `apply::apply_order` (база лаунчера и её бэкап). В папки модов не пишет никогда.
- Пути: разделитель `/`. Нормализованная форма — нижний регистр на Windows и macOS, как есть на Linux (`cfg!(target_os = "linux")`). Сравнение и сортировка — по нормализованной форме, покомпонентно.
- Детерминизм: одинаковый вход → одинаковый порядок в отчётах (`BTreeMap`/`BTreeSet`, явная сортировка).
- Вывод CLI — на английском. Все публичные типы отчётов — `#[derive(Debug, Clone, PartialEq, Serialize)]`.
- Коммит после каждой задачи на ветке `feat/core-cli`, локально, без push. После каждой задачи — ревью агентом `rust-reviewer`; после задач 4, 6, 7, 8, 9 — ещё и `ck3-rules-auditor`.

## Review Focus

1. Папка мода из базы лаунчера не существует (подписка не скачана — так сейчас на Mac владельца) → мод в `missing`, исключён из анализа, без паники. Тест — Task 5.
2. BOM, битые байты UTF-8, незакрытая скобка в скрипте → предупреждение на файл, остальное проиндексировано. Тест — Task 4.
3. Пути, отличающиеся только регистром (`Events/A.txt` и `events/a.txt`) → один файл на Windows/macOS, два на Linux. Тест — Task 4 и Task 6.
4. База лаунчера другой версии без нужного столбца → `CoreError::Schema { table, column }`, ничего не записано. Тест — Task 2 и Task 8.
5. Лог `database_conflicts.log` оборван на середине строки или снят с другого набора модов → предупреждение, сверка не выдаётся за достоверную. Тест — Task 9.

---

### Task 1: Каркас workspace, декодирование текста, пути пользователя

Проект уже инициализирован: git, `LICENSE` (GPLv3), `README.md`, `.gitignore`, `AGENTS.md`, `CLAUDE.md`, `.claude/` (скилл `ck3-engine-rules`, агенты `rust-reviewer` и `ck3-rules-auditor`), `docs/`.

**Files:**
- Create: `Cargo.toml` (workspace, `members = ["crates/*"]`, `resolver = "3"`)
- Create: `crates/ck3h-core/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/text.rs`, `src/paths.rs`
- Create: `crates/ck3h-cli/Cargo.toml`, `crates/ck3h-cli/src/main.rs` (пустой `fn main() {}`)

**Interfaces:**
- Produces:
  - `ck3h_core::error::CoreError` (thiserror): `Io(#[from] std::io::Error)`, `Db(#[from] rusqlite::Error)`, `Schema { table: String, column: String }`, `PlaysetNotFound(String)`, `NoActivePlayset`, `Descriptor(String)`, `Rules(String)`, `Cycle(Vec<String>)`, `LauncherRunning(Vec<String>)`.
  - `ck3h_core::text::decode(bytes: &[u8]) -> (String, bool)` — убрать UTF-8 BOM, `String::from_utf8_lossy`; `bool` = были ли битые байты.
  - `ck3h_core::paths::default_user_dir() -> Option<PathBuf>` — Windows/macOS: `dirs::document_dir()/Paradox Interactive/Crusader Kings III`; Linux: `dirs::data_dir()/Paradox Interactive/Crusader Kings III`.
  - `ck3h_core::paths::game_language(user_dir: &Path) -> Option<String>` — из `pdx_settings.txt` значение `value` внутри блока `"language"`, например `"l_russian"`.

- [ ] **Step 1: `git checkout -b feat/core-cli`; workspace и оба крейта**
- [ ] **Step 2: Тесты в `src/text.rs` и `src/paths.rs`**

```rust
#[test] fn decode_strips_bom() { assert_eq!(decode(b"\xEF\xBB\xBFl_english:"), ("l_english:".to_string(), false)); }
#[test] fn decode_flags_invalid_bytes() { assert_eq!(decode(b"a\xFFb"), ("a\u{FFFD}b".to_string(), true)); }
#[test] fn game_language_reads_pdx_settings() {
    // tempdir с pdx_settings.txt: "\"language\"={\n\tversion=0\n\tvalue=\"l_russian\"\n}\n"
    assert_eq!(game_language(dir.path()), Some("l_russian".to_string()));
}
#[test] fn game_language_missing_file_is_none() { assert_eq!(game_language(empty.path()), None); }
```

- [ ] **Step 3: `cargo test -p ck3h-core` → FAIL (функций нет)**
- [ ] **Step 4: Реализовать `decode`, `default_user_dir`, `game_language`** (`game_language` — поиском строк, без парсера)
- [ ] **Step 5: `cargo test -p ck3h-core` → PASS; `cargo build --workspace` → OK**
- [ ] **Step 6: Commit** `feat(core): workspace skeleton, text decoding, user dir paths`

---

### Task 2: Чтение базы лаунчера и testkit

**Files:**
- Create: `crates/ck3h-core/src/launcher.rs`, `crates/ck3h-core/src/testkit.rs` (за фичей `testkit`)
- Create: `crates/ck3h-core/fixtures/launcher_schema.sql`: `sqlite3 -readonly "$HOME/Documents/Paradox Interactive/Crusader Kings III/launcher-v2.sqlite" ".schema --nosys" > crates/ck3h-core/fixtures/launcher_schema.sql` (без `--nosys` в дампе есть `sqlite_sequence`, и SQLite отказывается его выполнять)
- Create: `crates/ck3h-core/tests/launcher.rs`
- Modify: `crates/ck3h-core/Cargo.toml` — `[features] testkit = ["dep:tempfile"]`, `tempfile` как optional dependency

**Interfaces:**
- Consumes: `CoreError` (Task 1).
- Produces:
  - `pub struct Playset { pub id: String, pub name: String, pub is_active: bool }`
  - `pub struct PlaysetMod { pub mod_id: String, pub display_name: String, pub steam_id: Option<String>, pub source: String, pub dir_path: PathBuf, pub position: i64, pub enabled: bool }`
  - `pub struct LauncherDb` с методами: `open_read_only(path: &Path) -> Result<Self, CoreError>`, `playsets(&self) -> Result<Vec<Playset>, CoreError>` (без `isRemoved = 1`, по имени), `active_playset(&self) -> Result<Playset, CoreError>`, `playset_by_name(&self, name: &str) -> Result<Playset, CoreError>`, `playset_mods(&self, playset_id: &str) -> Result<Vec<PlaysetMod>, CoreError>` (по `position` возрастанию).
  - `pub fn check_columns(conn: &rusqlite::Connection, required: &[(&str, &[&str])]) -> Result<(), CoreError>` — через `PRAGMA table_info`; нет столбца → `Schema { table, column }`.
  - `pub const READ_COLUMNS: &[(&str, &[&str])]` = `playsets(id, name, isActive, isRemoved)`, `playsets_mods(playsetId, modId, enabled, position)`, `mods(id, displayName, steamId, source, dirPath)`.
  - `pub const LAUNCHER_DB_FILE: &str = "launcher-v2.sqlite";`
  - testkit: `create_launcher_db(path: &Path) -> rusqlite::Connection` (выполняет `launcher_schema.sql`); `add_playset(conn, name: &str, active: bool) -> String`; `add_mod(conn, display_name: &str, dir: &Path, steam_id: Option<&str>) -> String` (`source` = `"steam"` при `steam_id`, иначе `"local"`; `status = "ready_to_play"`); `add_playset_mod(conn, playset_id: &str, mod_id: &str, position: i64, enabled: bool)`; `add_playset_dlc(conn, playset_id: &str, dlc_id: &str, enabled: bool)`; `write_mod(root: &Path, dir_name: &str, files: &[(&str, &[u8])]) -> PathBuf`; `playset_count(db: &Path) -> usize`; `playset_mod_names(db: &Path, playset_id: &str) -> Vec<String>` (по позиции); `playset_dlcs(db: &Path, playset_id: &str) -> Vec<(String, bool)>`.

- [ ] **Step 1: Тесты в `tests/launcher.rs`**

```rust
#[test] fn lists_playsets_and_active() { /* 2 плейсета, активен "Main" */ assert_eq!(db.active_playset()?.name, "Main"); assert_eq!(db.playsets()?.len(), 2); }
#[test] fn removed_playsets_hidden() { /* UPDATE playsets SET isRemoved = 1 для одного */ assert_eq!(db.playsets()?.len(), 1); }
#[test] fn playset_mods_ordered_by_position() { /* позиции 2,0,1 */ assert_eq!(names, ["A", "B", "C"]); }
#[test] fn playset_by_name_not_found() { assert!(matches!(db.playset_by_name("Nope"), Err(CoreError::PlaysetNotFound(_)))); }
#[test] fn schema_drift_reports_missing_column() {
    // база, где mods создана без столбца dirPath
    assert!(matches!(LauncherDb::open_read_only(&p), Err(CoreError::Schema { table, column }) if table == "mods" && column == "dirPath"));
}
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test launcher` → FAIL**
- [ ] **Step 3: Реализовать `launcher.rs` и `testkit.rs`** (база открывается с `OpenFlags::SQLITE_OPEN_READ_ONLY`)
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): read playsets and mods from launcher db`

---

### Task 3: Разбор `descriptor.mod`

**Files:**
- Create: `crates/ck3h-core/src/descriptor.rs`

**Interfaces:**
- Consumes: `text::decode`, `CoreError::Descriptor`.
- Produces:
  - `pub struct ModDescriptor { pub name: String, pub version: Option<String>, pub supported_version: Option<String>, pub dependencies: Vec<String>, pub replace_path: Vec<String>, pub remote_file_id: Option<String>, pub path: Option<String> }` — `path` есть только в `.mod`-файлах папки `mod/`.
  - `pub fn parse_descriptor(bytes: &[u8]) -> Result<ModDescriptor, CoreError>` — `jomini::TextTape::from_slice`, `tape.utf8_reader().fields()`; `replace_path` может повторяться; `dependencies` — массив строк; прочие ключи пропускаются; нет `name` → `Descriptor("missing name")`.
  - `pub fn read_descriptor(mod_dir: &Path) -> Result<ModDescriptor, CoreError>` — файл `descriptor.mod`.

- [ ] **Step 1: Тесты в модуле**

```rust
#[test] fn parses_name_version_and_dependencies() {
    let d = parse_descriptor(b"version=\"1.2.0\"\ntags={\n\t\"Graphics\"\n}\nname=\"Compat\"\ndependencies={\n\t\"Medieval Arts\"\n\t\"Simple Holding Sprawl\"\n}\nsupported_version=\"1.20.0.4\"\nremote_file_id=\"3816257844\"").unwrap();
    assert_eq!(d.name, "Compat"); assert_eq!(d.dependencies, ["Medieval Arts", "Simple Holding Sprawl"]);
    assert_eq!(d.supported_version.as_deref(), Some("1.20.0.4")); assert_eq!(d.remote_file_id.as_deref(), Some("3816257844"));
}
#[test] fn collects_repeated_replace_path() { assert_eq!(d.replace_path, ["history/characters", "common/landed_titles"]); }
#[test] fn reads_path_from_dot_mod_file() { assert_eq!(parse_descriptor(b"name=\"X\"\npath=\"/a/b\"").unwrap().path.as_deref(), Some("/a/b")); }
#[test] fn bom_and_cyrillic_name() { assert_eq!(parse_descriptor("\u{FEFF}name=\"Аутентичные шрифты\"".as_bytes()).unwrap().name, "Аутентичные шрифты"); }
#[test] fn missing_name_is_error() { assert!(parse_descriptor(b"version=\"1\"").is_err()); }
```

- [ ] **Step 2: `cargo test -p ck3h-core descriptor` → FAIL**
- [ ] **Step 3: Реализовать** (перед разбором — `text::decode`)
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): parse descriptor.mod`

---

### Task 4: Индекс мода и ванили — файлы, базы данных, локализация

**Files:**
- Create: `crates/ck3h-core/src/databases.rs`, `crates/ck3h-core/src/index.rs`
- Create: `crates/ck3h-core/tests/index.rs`

**Interfaces:**
- Consumes: `text::decode`, jomini, `testkit::write_mod`.
- Produces:
  - `databases.rs`: `pub const COMMON_DATABASES: &[&str]` — список CK3 `COMMON_DIRS` из ck3-tiger `src/ck3/tables/misc.rs` (164 строки; над списком комментарий с источником и лицензией GPL-3.0); `pub const GFX_DATABASES: &[&str]` — 10 корней из spec §4.2; `pub const OBJECT_ANALYSIS_EXCLUDED: &[&str] = &["common/on_action", "common/defines", "common/genes", "common/named_colors", "common/coat_of_arms/coat_of_arms", "common/landed_titles"];`; `pub fn database_root(norm_path: &str) -> Option<String>` — самый длинный префикс из обоих списков (граница по `/`), иначе для `common/<x>/…` → `common/<x>`, иначе `None`.
  - `pub fn normalize(display: &str) -> String` — `\` → `/`; нижний регистр, если не Linux.
  - `pub struct RelPath { pub display: String, pub norm: String }` — `RelPath::new(display: &str) -> RelPath`; `impl Ord` по `norm.split('/')` покомпонентно; `RelPath::parent_norm(&self) -> String`.
  - `pub struct ObjectDef { pub database: String, pub key: String, pub file: RelPath, pub ordinal: u32 }`
  - `pub struct LocDef { pub lang: String, pub key: String, pub value: String, pub file: RelPath, pub is_replace: bool, pub ordinal: u32 }`
  - `pub struct IndexWarning { pub file: String, pub message: String }`
  - `pub struct ModIndex { pub files: Vec<RelPath>, pub objects: Vec<ObjectDef>, pub loc: Vec<LocDef>, pub warnings: Vec<IndexWarning> }` (векторы отсортированы по `(file, ordinal)`)
  - `pub fn index_mod(root: &Path, lang: &str) -> Result<ModIndex, CoreError>`
  - `pub fn index_vanilla(game_dir: &Path, lang: &str) -> Result<ModIndex, CoreError>` — `game/` без `dlc/`, затем каждая `game/dlc/<папка>/` по имени папки; файл слоя DLC с тем же `norm` заменяет файл ванили (и его объекты/ключи).

Правила — spec §4.2. Файлы в корне мода и скрытые (сегмент с `.`) не индексируются. Объекты — ключи верхнего уровня в `.txt` под корнем из `database_root`, если корень не в `OBJECT_ANALYSIS_EXCLUDED`; ключи на `@` пропускаются. Файл считается сломанным, если jomini вернул ошибку **или** фигурные скобки не сбалансированы (подсчёт вне `"…"` и после `#` до конца строки) → `IndexWarning`, объекты файла не добавляются. Локализация — `localization/**/*.yml`, имя файла содержит `l_<lang>`; строка ` key:N "value"` или ` key: "value"`; строка `l_<lang>:` и комментарии пропускаются; `is_replace` — в пути есть сегмент `replace`. `decode` вернул `true` → `IndexWarning` («invalid UTF-8»), файл всё равно индексируется.

- [ ] **Step 1: Тесты в `tests/index.rs`**

```rust
#[test] fn indexes_files_except_root_and_hidden() { /* descriptor.mod, thumbnail.png, .git/x, events/a.txt */ assert_eq!(norms(&idx.files), ["events/a.txt"]); }
#[cfg(not(target_os = "linux"))] #[test] fn paths_fold_case_off_linux() { assert_eq!(idx.files[0].norm, "events/a_events.txt"); assert_eq!(idx.files[0].display, "Events/A_Events.txt"); }
#[cfg(target_os = "linux")] #[test] fn paths_keep_case_on_linux() { assert_eq!(idx.files[0].norm, "Events/A_Events.txt"); }
#[test] fn relpath_orders_by_components() { assert!(RelPath::new("common/a/z.txt") < RelPath::new("common/a-b/a.txt")); }
#[test] fn database_root_longest_prefix() {
    assert_eq!(database_root("common/decisions/dlc_decisions/ep_3/x.txt").as_deref(), Some("common/decisions"));
    assert_eq!(database_root("common/culture/cultures/x.txt").as_deref(), Some("common/culture/cultures"));
    assert_eq!(database_root("gfx/portraits/portrait_animations/x.txt").as_deref(), Some("gfx/portraits/portrait_animations"));
    assert_eq!(database_root("common/brand_new_dir/x.txt").as_deref(), Some("common/brand_new_dir"));
    assert_eq!(database_root("events/x.txt"), None);
}
#[test] fn top_level_objects_in_subfolders() {
    // common/traits/00_t.txt: "brave_plus = { a = 1 }\n@var = 5\nshared = { }\n"; common/decisions/sub/d.txt: "my_decision = { }"
    assert_eq!(keys(&idx.objects), [("common/decisions", "my_decision", 0), ("common/traits", "brave_plus", 0), ("common/traits", "shared", 1)]);
}
#[test] fn excluded_and_container_databases_have_no_objects() {
    /* common/on_action/x.txt, common/defines/x.txt, common/named_colors/c.txt: "colors = { red = {} }", common/landed_titles/t.txt */
    assert!(idx.objects.is_empty()); assert_eq!(idx.files.len(), 4);
}
#[test] fn localization_for_selected_language_only() {
    // localization/english/a_l_english.yml: "\u{FEFF}l_english:\n # c\n k1:0 \"One\"\n k2: \"Two\"\n"; localization/english/l_english_b.yml: "l_english:\n k3:0 \"Three\"\n"; localization/russian/a_l_russian.yml
    assert_eq!(loc_keys(&idx_en), [("k1", "One", false), ("k2", "Two", false), ("k3", "Three", false)]);
}
#[test] fn replace_folder_marks_loc() { /* localization/english/replace/r_l_english.yml и localization/replace/english/r2_l_english.yml */ assert!(idx.loc.iter().all(|l| l.is_replace)); }
#[test] fn broken_inputs_give_warnings_and_continue() {
    // common/traits/bad.txt: "x = { y = 1"; common/traits/good.txt: "ok = {}"; localization/english/z_l_english.yml с байтом 0xFF в значении
    assert_eq!(idx.warnings.len(), 2); assert_eq!(keys(&idx.objects), [("common/traits", "ok", 0)]); assert_eq!(idx.loc.len(), 1);
}
#[test] fn vanilla_dlc_layer_replaces_base_file() {
    // game/common/traits/00_traits.txt: "base_t = {}"; game/dlc/dlc001/common/traits/00_traits.txt: "dlc_t = {}"
    assert_eq!(norms(&v.files), ["common/traits/00_traits.txt"]); assert_eq!(keys(&v.objects), [("common/traits", "dlc_t", 0)]);
}
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test index` → FAIL**
- [ ] **Step 3: Реализовать `databases.rs` и `index.rs`** (обход — `std::fs::read_dir` рекурсивно)
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): index mods and vanilla — files, databases, localization`

---

### Task 5: Сборка плейсета и граф зависимостей

**Files:**
- Create: `crates/ck3h-core/src/playset.rs`, `crates/ck3h-core/src/deps.rs`, `crates/ck3h-core/tests/playset.rs`
- Modify: `crates/ck3h-core/src/testkit.rs` — `standard_fixture`

**Interfaces:**
- Consumes: `LauncherDb`, `PlaysetMod`, `read_descriptor`, `index_mod`.
- Produces:
  - `pub struct ModEntry { pub id: String, pub name: String, pub steam_id: Option<String>, pub position: i64, pub enabled: bool, pub dir: PathBuf, pub descriptor: Option<ModDescriptor>, pub index: Option<ModIndex>, pub missing: bool }` — `name` = `descriptor.name`, иначе `display_name`.
  - `pub fn load_playset(db: &LauncherDb, playset: &Playset, lang: &str) -> Result<Vec<ModEntry>, CoreError>` — по позиции; индекс строится только для включённых модов с существующей папкой, параллельно (`rayon`); нет папки → `missing = true`, `index = None`; битый `descriptor.mod` → `descriptor = None` и `IndexWarning` в индексе.
  - `pub enum NameMatch { None, One(String), Many(Vec<String>) }` (id модов); `pub fn match_name(mods: &[ModEntry], name: &str) -> NameMatch` — `trim` + без учёта регистра по `name`; `"steam:<id>"` — по `steam_id`.
  - `pub enum DepIssueKind { Missing, Ambiguous }`; `pub struct DepIssue { pub mod_id: String, pub dependency: String, pub kind: DepIssueKind }`
  - `pub struct DepGraph { pub edges: BTreeSet<(String, String)>, pub issues: Vec<DepIssue>, pub compatches: BTreeSet<String> }` — ребро `(зависимость, зависящий)`; compatch — мод с ≥2 найденными зависимостями.
  - `pub fn build_dep_graph(mods: &[ModEntry]) -> DepGraph` (только включённые моды); `impl DepGraph { pub fn depends_on(&self, dependent: &str, dependency: &str) -> bool }` — **транзитивно** (есть путь по рёбрам).
  - testkit: `pub fn standard_fixture(root: &Path) -> PathBuf` — user dir с `launcher-v2.sqlite`, `pdx_settings.txt` (`l_english`) и активным плейсетом `"Main"`. Все пять модов включены:

| pos | Папка / name | Содержимое |
|---|---|---|
| 0 | `base_a` / "Base A" | `common/traits/zz_base_a.txt`: `shared_trait = { a = 1 }`, `only_a = { }`; `events/shared_events.txt`; `localization/english/base_a_l_english.yml`: `shared_key:0 "From A"` |
| 1 | `base_b` / "Base B" | `common/traits/00_base_b.txt`: `shared_trait = { b = 2 }`; `Events/Shared_Events.txt`; `localization/english/base_b_l_english.yml`: `shared_key:0 "From B"` |
| 2 | `compat_ab` / "A+B Compatch", deps "Base A", "Base B" | `common/traits/zzz_compat.txt`: `compat_trait = { }`; `localization/english/replace/compat_l_english.yml`: `replace_key:0 "Compat"` |
| 3 | `ghost` / "Ghost" | в базе есть, папки на диске нет |
| 4 | `broken` / "Broken", deps "Missing Mod" | `common/traits/bad.txt`: `x = { y = 1` |

- [ ] **Step 1: Тесты в `tests/playset.rs`**

```rust
#[test] fn missing_folder_is_flagged_not_indexed() { let ghost = by_name(&mods, "Ghost"); assert!(ghost.missing && ghost.index.is_none()); }
#[test] fn compatch_detected_and_edges_built() {
    assert!(g.compatches.contains(&id("A+B Compatch")));
    assert!(g.depends_on(&id("A+B Compatch"), &id("Base A")) && g.depends_on(&id("A+B Compatch"), &id("Base B")));
}
#[test] fn depends_on_is_transitive() { /* C deps B, B deps A */ assert!(g.depends_on(&id("C"), &id("A"))); assert!(!g.depends_on(&id("A"), &id("C"))); }
#[test] fn missing_dependency_reported() { assert_eq!(g.issues, [DepIssue { mod_id: id("Broken"), dependency: "Missing Mod".into(), kind: DepIssueKind::Missing }]); }
#[test] fn duplicate_names_are_ambiguous() { /* два мода "Same Name" + мод с dependency "same name " */ assert_eq!(g.issues[0].kind, DepIssueKind::Ambiguous); assert!(g.edges.is_empty()); }
#[test] fn steam_prefix_matches_steam_id() { assert!(matches!(match_name(&mods, "steam:111"), NameMatch::One(_))); }
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test playset` → FAIL**
- [ ] **Step 3: Реализовать `playset.rs`, `deps.rs`, `standard_fixture`**
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): load playset and build dependency graph`

---

### Task 6: Движок конфликтов

**Files:**
- Create: `crates/ck3h-core/src/conflicts.rs`, `crates/ck3h-core/tests/conflicts.rs`

**Interfaces:**
- Consumes: `ModEntry`, `ModIndex`, `RelPath`, `DepGraph`.
- Produces:
  - `pub enum Source { Mod(String), Vanilla }`; `pub struct Contender { pub source: Source, pub file: RelPath }`
  - `pub enum ConflictLevel { File, Object, Loc }`; `pub enum Verdict { Intended, Suspicious, Vanilla }`
  - `pub enum WinReason { LowerInPlayset, LaterFilePath, EarlierFilePath, ReplaceFolder }`
  - `pub struct Conflict { pub level: ConflictLevel, pub target: String, pub winner: Contender, pub losers: Vec<Contender>, pub verdict: Verdict, pub reason: WinReason }` — `target`: File — `norm` пути; Object — `"<database>::<key>"`; Loc — `"<lang>::<key>"`.
  - `pub struct ConflictReport { pub conflicts: Vec<Conflict>, pub missing_mods: Vec<String>, pub dep_issues: Vec<DepIssue>, pub compatches: Vec<String>, pub replace_paths: Vec<(String, Vec<String>)>, pub warnings: Vec<(String, IndexWarning)> }` — конфликты по `(level, target)`; `replace_paths` — (id мода, его `replace_path`) для модов с непустым списком.
  - `pub fn effective_files(mods: &[ModEntry], vanilla: Option<&ModIndex>) -> BTreeMap<String, Vec<Contender>>` — путь → претенденты в порядке загрузки (ваниль первой); последний загружается. `replace_path` мода убирает претендентов-ваниль и претендентов-моды выше него, если `parent_norm()` файла равен нормализованному `replace_path`.
  - `pub fn resolve_objects(mods: &[ModEntry], vanilla: Option<&ModIndex>) -> BTreeMap<(String, String), Vec<Contender>>` — (база, ключ) → определения в порядке чтения; последнее побеждает.
  - `pub fn resolve_loc(mods: &[ModEntry], vanilla: Option<&ModIndex>) -> BTreeMap<(String, String), (Contender, Vec<Contender>, WinReason)>`
  - `pub fn analyze(mods: &[ModEntry], deps: &DepGraph, vanilla: Option<&ModIndex>) -> ConflictReport`

Правила — spec §4.3, §4.4. Учитываются только включённые найденные моды, по `position`. Объекты и локализация считаются только по файлам, оставшимся после `effective_files`. Порядок чтения — `RelPath::cmp`, внутри файла — `ordinal`. Конфликт — ≥2 разных источника среди претендентов. `Vanilla` — источников ровно два и один из них ваниль. `Intended` — `deps.depends_on(победитель, проигравший)` для каждого проигравшего мода; иначе `Suspicious`.

- [ ] **Step 1: Тесты в `tests/conflicts.rs`** (на `standard_fixture`, кроме отмеченных)

```rust
#[cfg(not(target_os = "linux"))] #[test] fn file_conflict_ignores_case_and_lower_mod_wins() {
    let c = find(&r, ConflictLevel::File, "events/shared_events.txt");
    assert_eq!(c.winner.source, Source::Mod(id("Base B"))); assert_eq!(c.reason, WinReason::LowerInPlayset); assert_eq!(c.verdict, Verdict::Suspicious);
}
#[cfg(target_os = "linux")] #[test] fn case_different_paths_do_not_conflict_on_linux() { assert!(r.conflicts.iter().all(|c| c.level != ConflictLevel::File)); }
#[test] fn object_winner_is_later_file_path_not_lower_mod() {
    let c = find(&r, ConflictLevel::Object, "common/traits::shared_trait");
    assert_eq!(c.winner.source, Source::Mod(id("Base A"))); assert_eq!(c.reason, WinReason::LaterFilePath);
}
#[test] fn loc_first_definition_wins() {
    let c = find(&r, ConflictLevel::Loc, "l_english::shared_key");
    assert_eq!(c.winner.source, Source::Mod(id("Base A"))); assert_eq!(c.reason, WinReason::EarlierFilePath);
}
#[test] fn replace_folder_beats_regular_loc() { /* своя сборка: мод выше compat_ab с обычным replace_key */ assert_eq!(c.reason, WinReason::ReplaceFolder); }
#[test] fn dependent_winner_is_intended() { /* своя сборка: compat_ab ещё определяет shared_trait в zzz_compat.txt */ assert_eq!(c.verdict, Verdict::Intended); }
#[test] fn replace_path_drops_files_of_earlier_mods() {
    /* своя сборка: мод "Old" (pos 0) с history/characters/x.txt и history/characters/sub/y.txt; мод "TC" (pos 1) с replace_path="history/characters" и history/characters/z.txt */
    assert!(!effective_files(&mods, None).contains_key("history/characters/x.txt"));
    assert!(effective_files(&mods, None).contains_key("history/characters/sub/y.txt"));
    assert_eq!(r.replace_paths, [(id("TC"), vec!["history/characters".to_string()])]);
}
#[test] fn vanilla_only_conflict_marked_vanilla() { /* vanilla index: common/traits/00_traits.txt: "only_a = {}" */ assert_eq!(c.verdict, Verdict::Vanilla); }
#[test] fn report_lists_missing_mods_and_is_deterministic() { assert_eq!(r.missing_mods, [id("Ghost")]); assert_eq!(serde_json::to_string(&analyze(..))?, serde_json::to_string(&analyze(..))?); }
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test conflicts` → FAIL**
- [ ] **Step 3: Реализовать `conflicts.rs`**
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): conflict engine for files, objects and localization`

---

### Task 7: Правила и сортировщик

**Files:**
- Create: `crates/ck3h-core/src/sort.rs`, `crates/ck3h-core/tests/sort.rs`

**Interfaces:**
- Consumes: `ModEntry`, `DepGraph`, `match_name`, `analyze`.
- Produces:
  - `pub struct Rule { pub before: String, pub after: String, pub note: Option<String> }`
  - `pub fn parse_rules(toml_text: &str) -> Result<Vec<Rule>, CoreError>` — формат spec §4.5 (`[[rule]]`); ошибка → `Rules(msg)`.
  - `pub fn load_rules(path: &Path) -> Result<Vec<Rule>, CoreError>` — файла нет → пустой список.
  - `pub enum MoveReason { Dependency { on: String }, UserRule { other: String, note: Option<String> } }`
  - `pub struct Move { pub mod_id: String, pub from: usize, pub to: usize, pub reasons: Vec<MoveReason> }` — `from`/`to` — 0-based индексы в полном списке плейсета.
  - `pub struct WinnerChange { pub level: ConflictLevel, pub target: String, pub before: Contender, pub after: Contender }`
  - `pub struct SortProposal { pub order: Vec<String>, pub moves: Vec<Move>, pub winner_changes: Vec<WinnerChange>, pub unresolved_rules: Vec<Rule> }`
  - `pub fn propose(mods: &[ModEntry], deps: &DepGraph, rules: &[Rule], vanilla: Option<&ModIndex>) -> Result<SortProposal, CoreError>`

Алгоритм — spec §4.5. Рёбра: `deps.edges` + правила (оба имени через `match_name`; не `One` → `unresolved_rules`). Выключенные моды остаются на своих индексах; включённые сортируются Каном с приоритетом «меньшая текущая позиция» и раскладываются по свободным индексам по порядку. Если Кан не взял все узлы — поиском в глубину среди оставшихся найти цикл и вернуть `Cycle(имена модов цикла)`. `moves` — моды, у которых есть нарушенное в текущем порядке ребро; `reasons` — эти рёбра. `winner_changes` — сравнение победителей `analyze` до и после (позиции подставлены из `order`).

- [ ] **Step 1: Тесты в `tests/sort.rs`**

```rust
#[test] fn dependency_moves_dependent_after_dependency() { /* "Child" (deps "Parent") выше "Parent" */ assert_eq!(names(&p.order), ["Parent", "Child"]); assert!(matches!(p.moves[0].reasons[0], MoveReason::Dependency { .. })); }
#[test] fn already_sorted_with_disabled_in_middle_has_no_moves() { /* A, Off(выключен), B; B deps A */ assert!(p.moves.is_empty()); assert_eq!(names(&p.order), ["A", "Off", "B"]); }
#[test] fn disabled_mod_keeps_its_slot() { /* Child, Off(выключен), Parent */ assert_eq!(names(&p.order), ["Parent", "Off", "Child"]); }
#[test] fn unrelated_shifted_mods_are_not_moves() { /* X, Child, Y, Parent */ assert_eq!(names(&p.order), ["X", "Y", "Parent", "Child"]); assert_eq!(moved(&p), ["Child", "Parent"]); }
#[test] fn user_rule_applies_and_unknown_rule_is_unresolved() {
    let rules = parse_rules("[[rule]]\nbefore = \"B\"\nafter = \"A\"\n[[rule]]\nbefore = \"Nope\"\nafter = \"A\"\n")?;
    assert_eq!(names(&p.order), ["B", "A"]); assert_eq!(p.unresolved_rules.len(), 1);
}
#[test] fn cycle_reports_only_cycle_members() { /* A deps B, B deps A, C deps A */ assert!(matches!(propose(..), Err(CoreError::Cycle(m)) if sorted(m) == ["A", "B"])); }
#[test] fn reports_winner_changes() { /* одинаковый файл у Child и Parent; после перестановки побеждает Child */ assert_eq!(p.winner_changes.len(), 1); }
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test sort` → FAIL**
- [ ] **Step 3: Реализовать `sort.rs`**
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): load-order rules and stable topological sort`

---

### Task 8: Применение порядка — копия плейсета

**Files:**
- Create: `crates/ck3h-core/src/apply.rs`, `crates/ck3h-core/tests/apply.rs`

**Interfaces:**
- Consumes: `CoreError`, `check_columns`, `READ_COLUMNS`.
- Produces:
  - `pub const DEFAULT_LAUNCHER_PROCESS_NAMES: &[&str] = &["paradox launcher", "paradox-launcher"];`
  - `pub fn launcher_processes(names: &[&str]) -> Vec<String>` — имена запущенных процессов (`sysinfo`), содержащие любую строку из `names` без учёта регистра.
  - `pub struct ApplyResult { pub new_playset_id: String, pub new_playset_name: String, pub backup_path: PathBuf }`
  - `pub fn apply_order(db_path: &Path, source_playset_id: &str, order: &[String], now: chrono::DateTime<chrono::Local>, launcher_names: &[&str]) -> Result<ApplyResult, CoreError>`

Поведение — spec §4.6, по шагам: (1) `launcher_processes(launcher_names)` не пуст → `LauncherRunning`; (2) `order` должен содержать ровно моды исходного плейсета, иначе `Rules("order does not match playset")`; (3) `check_columns` для `READ_COLUMNS` плюс `playsets(loadOrder, createdOn, updatedOn, syncState)` и `playsets_dlcs(playsetId, dlcId, enabled)`; (4) бэкап `launcher-v2_ck3helper_%Y-%m-%d_%H-%M-%S.sqlite` — `OpenOptions::new().write(true).create_new(true)`, при `AlreadyExists` — суффикс `_2`, `_3`… перед `.sqlite`, затем копирование содержимого; (5) одна транзакция: `INSERT` в `playsets`, `playsets_mods` (позиции `0..n-1`, `enabled` из исходного), `INSERT INTO playsets_dlcs … SELECT` из исходного. Любая ошибка на шагах 1–3 — до бэкапа и без записи.

- [ ] **Step 1: Тесты в `tests/apply.rs`** (`NONE = &["ck3h-test-no-such-process"]`)

```rust
#[test] fn creates_sorted_copy_and_backup() {
    let r = apply_order(&db, &src, &[id_c, id_a, id_b], now, NONE)?;
    assert_eq!(r.new_playset_name, "Main (CK3 Helper)"); assert!(r.backup_path.exists());
    assert_eq!(playset_mod_names(&db, &r.new_playset_id), ["C", "A", "B"]);
}
#[test] fn source_playset_untouched() { assert_eq!(playset_mod_names(&db, &src), ["A", "B", "C"]); assert!(active_is(&db, "Main")); }
#[test] fn second_apply_gets_counter_and_keeps_first_backup() {
    let r1 = apply_order(.., now, NONE)?; let first = std::fs::read(&r1.backup_path)?;
    let r2 = apply_order(.., now, NONE)?;
    assert_eq!(r2.new_playset_name, "Main (CK3 Helper 2)"); assert_ne!(r1.backup_path, r2.backup_path);
    assert_eq!(std::fs::read(&r1.backup_path)?, first);
}
#[test] fn dlc_rows_copied() { assert_eq!(playset_dlcs(&db, &r.new_playset_id), playset_dlcs(&db, &src)); }
#[test] fn wrong_order_writes_nothing() { assert!(apply_order(&db, &src, &[id_a], now, NONE).is_err()); assert_eq!(playset_count(&db), 1); assert_eq!(backups_in(&dir), 0); }
#[test] fn schema_drift_writes_nothing() { /* без столбца syncState */ assert!(matches!(e, CoreError::Schema { .. })); assert_eq!(playset_count(&db), 1); }
#[test] fn running_launcher_blocks_apply() {
    let me = std::env::current_exe()?.file_name().unwrap().to_string_lossy().to_string();
    assert!(matches!(apply_order(.., now, &[me.as_str()]), Err(CoreError::LauncherRunning(_)))); assert_eq!(playset_count(&db), 1);
}
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test apply` → FAIL**
- [ ] **Step 3: Реализовать `apply.rs`**
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): write sorted playset copy with backup`

---

### Task 9: Сверка с `database_conflicts.log`

**Files:**
- Create: `crates/ck3h-core/src/calibrate.rs`, `crates/ck3h-core/tests/calibrate.rs`
- Create: `crates/ck3h-core/fixtures/database_conflicts_sample.log` — ~20 строк из `~/Documents/Paradox Interactive/Crusader Kings III/logs/database_conflicts.log`: строка с `common/decisions` из подпапки `dlc_decisions/ep_3`, строка `gfx/portraits/portrait_animations`, строка с базой `'common/graphical_unit_types/'`, строка `gene database`, и последняя строка, оборванная на `'file: commo`.

**Interfaces:**
- Consumes: `resolve_objects`, `ModEntry`, `ModIndex`, `parse_descriptor`, `normalize`.
- Produces:
  - `pub struct LogOverride { pub database: String, pub key: String, pub file: String }` — `database` без завершающего `/` и нормализован; `file` нормализован.
  - `pub struct ParsedLog { pub overrides: Vec<LogOverride>, pub truncated: bool }`
  - `pub fn parse_conflicts_log(text: &str) -> ParsedLog` — строки `Overriding entry '<key>' for database '<db>' in 'file: <path> line: <n>'`; прочие строки (в том числе `gene database`) пропускаются; на каждую пару (база, ключ) остаётся последняя строка; `truncated` — последняя непустая строка не заканчивается на `'`.
  - `pub struct RunCheck { pub not_in_last_run: Vec<String>, pub only_in_last_run: Vec<String> }` (id модов / пути `.mod`)
  - `pub fn check_last_run(user_dir: &Path, mods: &[ModEntry]) -> Option<RunCheck>` — читает `dlc_load.json` → `enabled_mods` (`mod/<x>.mod`), из каждого `.mod` берёт `path`, сравнивает с `dir` включённых модов; файла нет → `None`.
  - `pub struct Mismatch { pub database: String, pub key: String, pub log_file: String, pub predicted_file: String }`
  - `pub struct CalibrationReport { pub matched: usize, pub mismatched: Vec<Mismatch>, pub log_unknown: BTreeMap<String, usize>, pub predicted_not_logged: Vec<(String, String)>, pub truncated: bool, pub run_check: Option<RunCheck> }`
  - `pub fn calibrate(log: &ParsedLog, mods: &[ModEntry], vanilla: Option<&ModIndex>, run_check: Option<RunCheck>) -> CalibrationReport` — по каждой строке лога: нашей пары нет → `log_unknown[database] += 1`; файл последнего претендента `resolve_objects` совпал → `matched`; иначе → `mismatched`. Наши пары с ≥2 претендентами, которых нет в логе → `predicted_not_logged`.

- [ ] **Step 1: Тесты** — разбор в модуле `calibrate.rs`, сверка в `tests/calibrate.rs`

```rust
#[test] fn parses_and_keeps_last_line_per_key() {
    let log = "[01:59:17][W][game_database.h:272]: Overriding entry 'difficulty' for database 'common/game_rules' in 'file: common/game_rules/a.txt line: 1'\n[01:59:18][W][game_database.h:272]: Overriding entry 'difficulty' for database 'common/game_rules' in 'file: common/game_rules/b.txt line: 3'\n";
    assert_eq!(parse_conflicts_log(log).overrides, [LogOverride { database: "common/game_rules".into(), key: "difficulty".into(), file: "common/game_rules/b.txt".into() }]);
}
#[test] fn sample_log_handles_real_formats() {
    let p = parse_conflicts_log(include_str!("../fixtures/database_conflicts_sample.log"));
    assert!(p.truncated); assert!(p.overrides.iter().any(|o| o.database == "common/graphical_unit_types"));
    assert!(p.overrides.iter().any(|o| o.database == "common/decisions" && o.file.contains("dlc_decisions/ep_3")));
    assert!(p.overrides.iter().all(|o| !o.database.contains("gene")));
}
// tests/calibrate.rs, на своей сборке из testkit:
#[test] fn calibrate_counts_all_categories() {
    // лог: shared_trait → верный файл; other → чужой файл; ghost_key → неизвестен; в модах есть ещё пара с 2 определениями без строки лога
    assert_eq!((r.matched, r.mismatched.len(), r.log_unknown["common/traits"], r.predicted_not_logged.len()), (1, 1, 1, 1));
}
#[test] fn last_run_mismatch_detected() { /* dlc_load.json с одним .mod, путь которого не совпадает ни с одним модом */ assert_eq!(rc.only_in_last_run.len(), 1); }
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit calibrate` → FAIL**
- [ ] **Step 3: Реализовать `calibrate.rs`**
- [ ] **Step 4: Тот же прогон → PASS**
- [ ] **Step 5: Commit** `feat(core): calibrate predictions against database_conflicts.log`

---

### Task 10: CLI `ck3h` и сквозной тест

**Files:**
- Modify: `crates/ck3h-cli/src/main.rs`, `crates/ck3h-cli/Cargo.toml` (dev-dependency `ck3h-core` с фичей `testkit`, `assert_cmd`, `predicates`, `tempfile`)
- Create: `crates/ck3h-cli/tests/cli.rs`
- Modify: `README.md` — раздел «Usage» с четырьмя командами

**Interfaces:**
- Consumes: всё ядро.
- Produces: команды из spec §4.7. Глобальный `--user-dir` (по умолчанию `default_user_dir()`), `--json` у каждой команды. `--lang` по умолчанию `game_language(user_dir)`, иначе `l_english`. `--game` — путь к папке `game/` → `index_vanilla`. `--rules` по умолчанию `dirs::config_dir()/ck3-helper/rules.toml`. `sort --apply` вызывает `apply_order(…, DEFAULT_LAUNCHER_PROCESS_NAMES)`; `LauncherRunning` → код выхода 2 и `Close the Paradox Launcher first: <names>`. Прочие ошибки ядра → stderr, код 1. `calibrate` читает `<user-dir>/logs/database_conflicts.log` и `check_last_run`.

Текстовый вывод (по строке на элемент; позиции с 1):
- `playsets`: `* Main (5 mods, 5 enabled)` — `*` у активного.
- `conflicts`: `[suspicious] file events/shared_events.txt — winner: Base B; losers: Base A (lower in playset)`; затем `missing on disk: Ghost`, `dependency missing: Broken -> Missing Mod`, `replaces vanilla folder: <mod> -> <path>`, `compatch: A+B Compatch`.
- `sort`: `move Child: #1 -> #2 (depends on Parent)`; затем `winner changes: N`; при `--apply` — `created playset "Main (CK3 Helper)"; backup: <path>`.
- `calibrate`: `matched: N, mismatched: M, unknown in log: K, predicted but not logged: P`; затем `unknown by database:` по строке на базу; при `truncated` — `warning: log is truncated`; при расхождении с последним запуском — `warning: log is from a different set of mods`.

- [ ] **Step 1: Тесты в `tests/cli.rs`** (на `standard_fixture`)

```rust
#[test] fn playsets_marks_active() { cmd(&ud).arg("playsets").assert().success().stdout(contains("* Main (5 mods, 5 enabled)")); }
#[cfg(not(target_os = "linux"))] #[test] fn conflicts_text_output() { cmd(&ud).arg("conflicts").assert().success()
    .stdout(contains("[suspicious] file events/shared_events.txt — winner: Base B"))
    .stdout(contains("missing on disk: Ghost")).stdout(contains("dependency missing: Broken -> Missing Mod")).stdout(contains("compatch: A+B Compatch")); }
#[test] fn conflicts_json_is_valid() { let v: serde_json::Value = serde_json::from_slice(&out)?; assert!(v["conflicts"].as_array().unwrap().len() >= 2); }
#[test] fn sort_dry_run_does_not_write() { cmd(&ud).arg("sort").assert().success(); assert_eq!(playset_count(&ud.join("launcher-v2.sqlite")), 1); }
#[test] fn unknown_playset_fails() { cmd(&ud).args(["conflicts", "--playset", "Nope"]).assert().failure().stderr(contains("Nope")); }
#[test] fn calibrate_reads_log() { /* logs/database_conflicts.log: одна строка про shared_trait из common/traits/zz_base_a.txt */ cmd(&ud).arg("calibrate").assert().success().stdout(contains("matched: 1")); }
```

- [ ] **Step 2: `cargo test -p ck3h-cli` → FAIL**
- [ ] **Step 3: Реализовать `main.rs`** (`clap` derive)
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS; `cargo clippy --workspace --all-features -- -D warnings` → чисто**
- [ ] **Step 5: Ручной прогон на реальной базе (только чтение):** `cargo run -p ck3h-cli -- playsets`, `cargo run -p ck3h-cli -- conflicts`, `cargo run -p ck3h-cli -- calibrate` → видны плейсеты владельца, отсутствующие на диске моды перечислены (Workshop на Mac сейчас не скачан), `calibrate` предупреждает об оборванном логе, паники нет.
- [ ] **Step 6: Commit** `feat(cli): ck3h playsets, conflicts, sort, calibrate`

---

## После плана 1

1. Владелец восстанавливает установку CK3 и подписки, запускает игру с плейсетом → `ck3h calibrate --game <путь>` на реальном плейсете; расхождения разбираются, правила в spec §4.3 и списки в `databases.rs` уточняются.
2. Ручная проверка `sort --apply` на **копии** реальной базы, затем запуск лаунчера (spec §6).
3. План 2 — приложение Tauri.

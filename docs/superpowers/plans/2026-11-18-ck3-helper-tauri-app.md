# CK3 Helper — приложение Tauri: план реализации (план 2 из 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Десктоп-приложение CK3 Helper на Tauri v2 поверх готового ядра: плейсеты, карта конфликтов по парам модов, предложение порядка с сохранением копии, сверка с игрой, настройки; интерфейс на русском и английском; сборки под Windows, macOS и Linux.

**Architecture:** Сначала ядро получает модуль `session` (загрузка плейсета и лога, раньше жившая в CLI), исправления из бэклога, оптимизацию памяти, правила-как-намерение и пары модов; CLI переходит на `session`. Затем крейт `ck3h-app` (`app/src-tauri`) держит анализ в памяти и отдаёт интерфейсу сводки и страницы через команды Tauri; типы TypeScript генерирует `ts-rs`. Интерфейс `app/src` — React 19 + Mantine 9, данные через TanStack Query, тексты через i18next.

**Tech Stack:** Rust 1.95 (edition 2024); Tauri 2 (2.11+), `tauri-plugin-dialog`, `tauri-plugin-opener`, `ts-rs` 12, `toml_edit`, `ureq` 3, `semver`; Node 20.19+ и npm, Vite 8, React 19.2, TypeScript, Mantine 9, `@tanstack/react-query` 5, `@tanstack/react-virtual` 3, `i18next` + `react-i18next`, `@tabler/icons-react`; тесты — Vitest, Testing Library, jsdom; ESLint, Prettier; GitHub Actions + `tauri-apps/tauri-action`.

**Spec:** `docs/superpowers/specs/2026-11-17-ck3-helper-v1-design.md` — §4.4, §4.5, §4.7, §4.8, решения Q18–Q32.

## Global Constraints

- Ветка `feat/tauri-app` в отдельном git worktree от `main`; начинать после того, как в `main` закоммичены правки CLI параллельной сессии (`crates/ck3h-cli/src/main.rs`, `crates/ck3h-cli/tests/cli.rs`, `docs/BACKLOG.md`). Коммит после каждой задачи, локально, без push.
- Инварианты `AGENTS.md` в силе: моды и сейвы только читаются; базу лаунчера пишет только `ck3h_core::apply::apply_order`; тесты не трогают реальную базу; вывод детерминирован; пользовательские данные не вызывают паники.
- В `ck3h-core` нет зависимостей UI и сети. Сеть — только `app/src-tauri/src/update.rs`.
- Приложение пишет только `<config>/ck3-helper/settings.toml`, `settings.toml.bak`, `rules.toml`; база и бэкап — только через `apply_order`.
- Workspace: `members = ["crates/*", "app/src-tauri"]`. Крейт приложения `ck3h-app` (lib `ck3h_app` + bin), `edition = "2024"`, `license = "GPL-3.0-or-later"`. Версия `0.1.0` во всех `Cargo.toml`, `app/package.json`, `app/src-tauri/tauri.conf.json`.
- Tauri: `productName` "CK3 Helper", `identifier` "org.ck3helper.desktop", окно 1280×800, минимум 1024×640, заголовок "CK3 Helper". Плагины — только `dialog` и `opener`. Права (`capabilities/default.json`): `core:default`, `dialog:allow-open`, `opener:allow-reveal-item-in-dir`, `opener:allow-open-url` только для `https://github.com/**`. CSP: `default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:`.
- DTO приложения: `#[derive(Serialize, TS)]` (входные — ещё `Deserialize`), `#[serde(rename_all = "camelCase")]`, `#[ts(export)]`. Вывод — `app/src/api/bindings/` через `.cargo/config.toml`: `[env] TS_RS_EXPORT_DIR = { value = "app/src/api/bindings", relative = true }` и `TS_RS_LARGE_INT = "number"`. Сгенерированные файлы коммитятся. Позиции в DTO — с 0, на экране — с 1.
- Логика приложения — методы `AppState`; команды Tauri только переносят тяжёлую работу в `tauri::async_runtime::spawn_blocking` и пересылают прогресс в `tauri::ipc::Channel`. Rust-тесты вызывают методы `AppState` напрямую, всегда с явными `userDir` и `gameDir` в настройках и `config_dir` во временной папке — реальные папки пользователя и игры тесты не видят.
- Интерфейс: никаких строк для пользователя вне `app/src/i18n/ru.ts` и `en.ts`; ключи одинаковы (тест). Тексты ошибок — по `ErrorCode`.
- Код, идентификаторы, коммиты — английский; тексты интерфейса — RU и EN; документы `docs/` — русский.
- Версии крейтов и npm-пакетов без явного номера — последние на момент установки в пределах названных мажорных версий; `Cargo.lock` и `app/package-lock.json` коммитятся.
- Перед коммитом Rust-задачи: `cargo fmt --all --check`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo test --workspace --all-features`. Интерфейсной: в `app/` — `npm run lint`, `npm run typecheck`, `npm test`.
- Модели (решение владельца): задачи с пометкой **Opus** реализует Opus, остальные — Sonnet; все ревью — Opus. Ворота ревью: после задачи 5 (`rust-reviewer`; задачи 2–4 ещё и `ck3-rules-auditor`), после задачи 11 (`rust-reviewer`), после задачи 16 (ревью интерфейса), финальное ревью ветки. Мелкие задачи отдельно не ревьюятся.

## Review Focus

1. Правило `rules.toml` с выключенным модом или с именем двух модов → не делает пару задуманной и попадает в неприменённые, без ошибки. Тест — Task 4.
2. Плейсет изменили в лаунчере между анализом и применением (порядок или «включён») → `playsetChanged`, база не тронута. Тест — Task 10.
3. `rules.toml`, написанный руками (комментарии, кириллица и кавычки в имени), → приложение дописывает и удаляет правила, не теряя комментариев; битый файл → код `rules`, анализ без правил, файл не тронут. Тест — Task 9.
4. Битый `settings.toml` → значения по умолчанию и баннер; сохранение не теряет старый файл (`settings.toml.bak`). Тест — Task 7.
5. Пара на тысячи конфликтов и плоский список в десятки тысяч → только страницы; запрос за концом — пустая страница с верным `total`; в DOM — только видимые строки. Тесты — Task 8 и Task 14.

---

### Task 1: Модуль `session`; CLI на нём — Sonnet

**Files:**
- Create: `crates/ck3h-core/src/session.rs`, `crates/ck3h-core/tests/session.rs`
- Modify: `crates/ck3h-core/src/{error.rs, launcher.rs, playset.rs, paths.rs, lib.rs}`, `crates/ck3h-cli/src/main.rs`, `docs/GLOSSARY.md` (строка «Сессия | `session` | …»)

**Interfaces:**
- Consumes: `LauncherDb`, `load_playset`, `index_vanilla`, `game_dir_if_valid`, `game_language`, `parse_conflicts_log`, `text::decode`.
- Produces:
  - `CoreError::LauncherDbNotFound(PathBuf)` — `"launcher database not found: {path}"`; `CoreError::NotGameDir(PathBuf)` — `"not a CK3 game folder (no common/): {path}"`; `CoreError::LogNotFound(PathBuf)` — `"log not found: {path}"` (тексты первых двух — как у CLI сейчас).
  - `LauncherDb::playset_by_id(&self, id: &str) -> Result<Playset, CoreError>` — без `isRemoved = 1`; нет → `PlaysetNotFound(id)`.
  - `playset::load_playset_with_progress(db: &LauncherDb, playset: &Playset, lang: &str, progress: &(dyn Fn(usize, usize) + Sync)) -> Result<Vec<ModEntry>, CoreError>` — `progress(done, total)` после каждого мода плейсета (`AtomicUsize`, порядок вызовов любой); `load_playset` вызывает её с пустым замыканием. `load_mod` возвращает `ModEntry` (пункт бэклога).
  - `paths::config_dir() -> Option<PathBuf>` — `dirs::config_dir()/ck3-helper`; `paths::RULES_FILE: &str = "rules.toml"`.
  - `session::PlaysetSelector { Active, ByName(String), ById(String) }`
  - `session::Loaded { pub db_path: PathBuf, pub playset: Playset, pub lang: String, pub mods: Vec<ModEntry> }`
  - `session::launcher_db(user_dir: &Path) -> Result<(PathBuf, LauncherDb), CoreError>` — файла нет → `LauncherDbNotFound`.
  - `session::analysis_lang(user_dir: &Path, explicit: Option<&str>) -> String` — явный, иначе `game_language`, иначе `"l_english"`.
  - `session::load(user_dir: &Path, selector: &PlaysetSelector, lang: &str, progress: &(dyn Fn(usize, usize) + Sync)) -> Result<Loaded, CoreError>`
  - `session::load_vanilla(game_dir: &Path, lang: &str) -> Result<ModIndex, CoreError>` — не папка игры → `NotGameDir`.
  - `session::playset_unchanged(loaded: &Loaded) -> Result<bool, CoreError>` — перечитать `playset_mods` (база только для чтения); сравнить `(mod_id, position, enabled)` с `loaded.mods` по порядку позиций.
  - `session::CONFLICTS_LOG: &str = "logs/database_conflicts.log"`; `session::read_conflicts_log(user_dir: &Path) -> Result<ParsedLog, CoreError>` — нет файла → `LogNotFound`, другая ошибка чтения → `Io`; байты через `text::decode`.

- [ ] **Step 1: Тесты в `tests/session.rs`** (на `standard_fixture`)

```rust
#[test] fn loads_active_by_name_and_by_id() {
    let active = load(&ud, &PlaysetSelector::Active, "l_english", &|_, _| {})?;
    let by_id = load(&ud, &PlaysetSelector::ById(active.playset.id.clone()), "l_english", &|_, _| {})?;
    assert_eq!(by_id.playset.name, "Main"); assert_eq!(active.mods.len(), 5);
    assert_eq!(load(&ud, &PlaysetSelector::ByName("Main".into()), "l_english", &|_, _| {})?.playset, active.playset);
}
#[test] fn missing_db_is_launcher_db_not_found() { assert!(matches!(launcher_db(empty.path()), Err(CoreError::LauncherDbNotFound(p)) if p.ends_with("launcher-v2.sqlite"))); }
#[test] fn unknown_id_is_playset_not_found() { assert!(matches!(load(&ud, &PlaysetSelector::ById("nope".into()), "l_english", &|_, _| {}), Err(CoreError::PlaysetNotFound(_)))); }
#[test] fn progress_reports_every_mod() { /* Mutex<Vec<(usize, usize)>> */ calls.sort(); assert_eq!(calls, [(1, 5), (2, 5), (3, 5), (4, 5), (5, 5)]); }
#[test] fn analysis_lang_prefers_explicit_then_game() {
    // tempdir с pdx_settings.txt, value="l_german"
    assert_eq!(analysis_lang(&de, Some("l_russian")), "l_russian"); assert_eq!(analysis_lang(&de, None), "l_german"); assert_eq!(analysis_lang(empty.path(), None), "l_english");
}
#[test] fn load_vanilla_rejects_non_game_dir() { assert!(matches!(load_vanilla(empty.path(), "l_english"), Err(CoreError::NotGameDir(_)))); }
#[test] fn playset_unchanged_detects_reorder_and_toggle() {
    assert!(playset_unchanged(&loaded)?);
    // UPDATE playsets_mods SET position = 9 WHERE position = 0
    assert!(!playset_unchanged(&loaded)?);
    // вернуть позицию; UPDATE playsets_mods SET enabled = 0 WHERE position = 1
    assert!(!playset_unchanged(&loaded)?);
}
#[test] fn conflicts_log_missing_then_read() {
    assert!(matches!(read_conflicts_log(&ud), Err(CoreError::LogNotFound(_))));
    // logs/database_conflicts.log: одна строка Overriding entry 'shared_trait' … zz_base_a.txt line: 1'
    assert_eq!(read_conflicts_log(&ud)?.overrides.len(), 1);
}
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test session` → FAIL (модуля нет)**
- [ ] **Step 3: Реализовать `session.rs` и правки `error.rs`, `launcher.rs`, `playset.rs`, `paths.rs`**
- [ ] **Step 4: CLI на `session`:** функции `load` и `playsets` в `main.rs` берут базу через `session::launcher_db`, плейсет — через `session::load`, ваниль — через `session::load_vanilla`, лог — через `session::read_conflicts_log`, путь правил по умолчанию — `paths::config_dir()` + `RULES_FILE`. Вывод и коды выхода CLI не меняются; единственное новое — текст ошибки при отсутствии лога: `error: log not found: <путь>`.
- [ ] **Step 5: `cargo test --workspace --all-features` → PASS (тесты CLI без изменений)**
- [ ] **Step 6: Commit** `feat(core): session module for loading playsets and the conflicts log`

---

### Task 2: Применение — проверка лаунчера fail-closed, бэкап через `VACUUM INTO`, имя копии заранее — Sonnet

**Files:**
- Modify: `crates/ck3h-core/src/{apply.rs, error.rs}`, `crates/ck3h-core/tests/apply.rs`, `crates/ck3h-cli/src/main.rs`

**Interfaces:**
- Consumes: `apply_order`, `create_backup` (существующие).
- Produces:
  - `CoreError::ProcessListUnavailable` — `"cannot read the process list to check whether the launcher is running"`. Текст `OrderMismatch` → `"the playset changed since it was analysed; analyse it again"`.
  - `pub struct ProcessInfo { pub pid: u32, pub name: String, pub exe: String }`
  - `pub fn blocking_processes(processes: &[ProcessInfo], own_pid: u32, names: &[&str]) -> Result<Vec<String>, CoreError>` — своего `pid` в списке нет → `ProcessListUnavailable`; иначе совпадения без учёта регистра: по имени → имя, иначе по пути → путь; отсортированы, без повторов.
  - `pub fn launcher_processes(names: &[&str]) -> Result<Vec<String>, CoreError>` — список из `sysinfo`, свой pid — `sysinfo::get_current_pid()`; ошибка `System`/pid → `ProcessListUnavailable`.
  - `pub struct ApplyPreview { pub new_playset_name: String, pub backup_dir: PathBuf }` (`Serialize`)
  - `pub fn preview(db_path: &Path, source_playset_id: &str) -> Result<ApplyPreview, CoreError>` — база только для чтения; имя считает та же приватная функция, что и в `apply_order`; `backup_dir` — папка базы.
  - Бэкап: имя по-прежнему через `create_new` (пустой файл), затем `VACUUM INTO ?1` на открытом соединении до транзакции; ошибка → свой файл удаляется.

- [ ] **Step 1: Тесты** — в модуле `apply.rs` (чистая функция) и в `tests/apply.rs`

```rust
#[test] fn process_list_without_own_pid_is_unavailable() {
    let procs = [ProcessInfo { pid: 1, name: "launchd".into(), exe: "/sbin/launchd".into() }];
    assert!(matches!(blocking_processes(&procs, 42, &["paradox launcher"]), Err(CoreError::ProcessListUnavailable)));
}
#[test] fn blocking_matches_name_or_exe_without_case() {
    let procs = [own(42), p(7, "Paradox Launcher", ""), p(8, "Electron", "/Applications/Paradox Launcher.app/Contents/MacOS/Electron")];
    assert_eq!(blocking_processes(&procs, 42, &["paradox launcher"])?, ["/Applications/Paradox Launcher.app/Contents/MacOS/Electron", "Paradox Launcher"]);
}
// tests/apply.rs
#[test] fn backup_is_a_consistent_sqlite_copy() { let r = apply_order(&db, &src, &order, now, NONE)?; assert_eq!(playset_count(&r.backup_path), 1); assert_eq!(playset_count(&db), 2); }
#[test] fn preview_matches_applied_name() {
    let p = preview(&db, &src)?; assert_eq!(p.backup_dir, db.parent().unwrap());
    assert_eq!(apply_order(&db, &src, &order, now, NONE)?.new_playset_name, p.new_playset_name);
    assert_eq!(preview(&db, &src)?.new_playset_name, "Main (CK3 Helper 2)"); assert_eq!(playset_count(&db), 2);
}
#[test] fn order_mismatch_text_names_no_command() { assert!(!CoreError::OrderMismatch.to_string().contains("ck3h")); }
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit apply` → FAIL**
- [ ] **Step 3: Реализовать.** В CLI: `ProcessListUnavailable` → код выхода 1, `Cannot check whether the Paradox Launcher is running: the process list is unavailable`.
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS (существующие тесты `apply` без изменений)**
- [ ] **Step 5: Commit** `fix(core): fail-closed launcher check, VACUUM INTO backup, copy name preview`

---

### Task 3: Память и скорость ядра — Opus

**Files:**
- Modify: `crates/ck3h-core/src/{index.rs, conflicts.rs, deps.rs, sort.rs, calibrate.rs}`, `crates/ck3h-core/Cargo.toml` (`serde` с фичей `rc`), `docs/BACKLOG.md`

**Interfaces:**
- Consumes: всё ядро.
- Produces (JSON всех отчётов не меняется — `Arc<T>` сериализуется как `T`):
  - `ObjectDef { database: Arc<str>, key: String, file: Arc<RelPath>, ordinal: u32 }`; `LocDef { lang: Arc<str>, key: String, value: String, file: Arc<RelPath>, is_replace: bool, ordinal: u32 }` — один `Arc<RelPath>` на файл, один `Arc<str>` на базу и на язык индекса.
  - `pub struct DepClosure` и `DepGraph::closure(&self) -> DepClosure`; `DepClosure::depends_on(&self, dependent: &str, dependency: &str) -> bool` — те же ответы, что `DepGraph::depends_on`, но обход графа — один раз на узел. `analyze_ordered` строит одно замыкание на вызов.
  - Определения объектов и локализации сначала группируются по `(база, ключ)` / `(язык, ключ)`, затем сортируется каждая группа (по `(file, ordinal)`), а не все определения сразу.
  - `Contender.file` остаётся `RelPath` (клон из `Arc` только для конфликтов).

- [ ] **Step 1: Эталон до изменений.** Собрать `cargo build --release -p ck3h-cli`. Скрипт `measure.sh` — в scratchpad сессии, не в репозитории; запускать `bash <путь>`. Он прогоняет на реальном плейсете владельца (только чтение), каждый раз через `/usr/bin/time -l`, и пишет JSON в папку, переданную аргументом:
  - `ck3h conflicts --json`;
  - `ck3h conflicts --game "<Steam>/steamapps/common/Crusader Kings III/game" --json`;
  - `ck3h sort --game <то же> --json`;
  - `ck3h calibrate --json`.

  Записать `maximum resident set size` и `real` каждого прогона.
- [ ] **Step 2: Тест замыкания в `deps.rs`**

```rust
#[test] fn closure_matches_depends_on() {
    // рёбра: (A, B), (B, C), (X, Y), (Y, X) — цепочка и цикл
    for (d, on) in [("C", "A"), ("A", "C"), ("Y", "X"), ("X", "Y"), ("C", "X")] { assert_eq!(g.closure().depends_on(d, on), g.depends_on(d, on)); }
}
```

- [ ] **Step 3: `cargo test -p ck3h-core closure` → FAIL**
- [ ] **Step 4: Реализовать перечисленные изменения**
- [ ] **Step 5: `cargo test --workspace --all-features` → PASS**
- [ ] **Step 6: Замер после.** Тот же скрипт с другой папкой вывода; `cmp` каждой пары JSON → идентичны. Если хоть одна пара различается, задача не готова.
- [ ] **Step 7: Итог в `docs/BACKLOG.md`.** Пункт «Производительность и память» заменить таблицей «до / после» (память и время, с ванилью и без). Цель — пиковая память `conflicts --game` не больше половины эталона. Не достигнута — числа остаются в бэклоге с пометкой «решает владелец»; сверх перечисленных изменений задача не оптимизирует.
- [ ] **Step 8: Commit** `perf(core): share paths and names across definitions; one dependency closure per analysis`

---

### Task 4: Правила как намерение, пары модов, текст строки локализации — Opus

**Files:**
- Modify: `crates/ck3h-core/src/{sort.rs, conflicts.rs, index.rs}`, `docs/GLOSSARY.md` (строки «Пара модов | `PairSummary`», «Граф намерений | `intent_graph`»)
- Create: `crates/ck3h-core/tests/pairs.rs`

**Interfaces:**
- Consumes: `match_name`, `DepGraph`, `DepClosure` (Task 3), `analyze`, `propose`.
- Produces:
  - `pub fn resolve_rules(mods: &[ModEntry], rules: &[Rule]) -> (Vec<(String, String, usize)>, Vec<Rule>)` — `(id before, id after, индекс правила)` для правил, где оба имени дают `NameMatch::One` по всем модам, оба мода включены и различны; остальные — неприменённые. `propose` переходит на неё; результаты `propose` не меняются.
  - `pub fn intent_graph(deps: &DepGraph, mods: &[ModEntry], rules: &[Rule]) -> DepGraph` — копия `deps` плюс ребро `(before, after)` на каждое применённое правило; `issues` и `compatches` без изменений.
  - `Source` получает `PartialOrd, Ord` (`Mod(_) < Vanilla`).
  - `pub struct PairSummary { pub winner: Source, pub loser: Source, pub verdict: Verdict, pub files: usize, pub objects: usize, pub loc: usize }` (`Debug, Clone, PartialEq, Eq, Serialize`)
  - `pub fn pairs(report: &ConflictReport, intent: &DepGraph) -> Vec<PairSummary>` — правила spec §4.4: проигравшие источники конфликта без победителя и без повторов; пометка пары: проигравший ваниль → `Vanilla`, победитель ваниль → `Suspicious`, `depends_on(winner, loser)` → `Intended`, иначе `Suspicious`; порядок: `Suspicious`, `Intended`, `Vanilla`, затем по убыванию `files + objects + loc`, затем `(winner, loser)`.
  - `pub fn involves(c: &Conflict, winner: &Source, loser: &Source) -> bool`
  - `impl ModIndex { pub fn loc_value(&self, file: &RelPath, key: &str) -> Option<&str> }` — `partition_point` по файлу в отсортированном `loc`, затем первая строка с ключом в этом файле.

- [ ] **Step 1: Тесты в `tests/pairs.rs`** (хелперы как в `tests/conflicts.rs`: `standard`, `custom`, `vanilla`, `id`, `find`)

```rust
#[cfg(not(target_os = "linux"))] #[test] fn standard_pairs() {
    // A бьёт B по shared_trait и shared_key; B бьёт A по файлу событий
    assert_eq!(rows(&pairs(&r, &deps)), [("Base A", "Base B", Verdict::Suspicious, 0, 1, 1), ("Base B", "Base A", Verdict::Suspicious, 1, 0, 0)]);
}
#[test] fn rule_makes_conflicts_and_pair_intended() {
    let rules = parse_rules("[[rule]]\nbefore = \"Base B\"\nafter = \"Base A\"\n")?;
    let intent = intent_graph(&deps, &mods, &rules); let r = analyze(&mods, &intent, None);
    assert_eq!(find(&r, ConflictLevel::Object, "common/traits::shared_trait").verdict, Verdict::Intended);
    assert_eq!(pair(&pairs(&r, &intent), "Base A", "Base B").verdict, Verdict::Intended);
    assert!(intent.compatches == deps.compatches && intent.issues == deps.issues);
}
#[test] fn disabled_or_ambiguous_rule_is_not_intent() {
    // плейсет: два включённых мода "Dup" (common/traits/a.txt и a2.txt: k = {}), "A" (common/traits/z.txt: k = {}) — A побеждает обоих;
    // выключенный "Off" с тем же common/traits/z.txt. Правила: before = "Dup" after = "A"; before = "Off" after = "A"
    let (applied, unresolved) = resolve_rules(&mods, &rules); assert!(applied.is_empty()); assert_eq!(unresolved.len(), 2);
    assert_eq!(analyze(&mods, &intent_graph(&deps, &mods, &rules), None).conflicts[0].verdict, Verdict::Suspicious);
}
#[test] fn multi_loser_conflict_counts_in_each_pair() {
    // X, Y, Z: common/traits/a.txt, b.txt, c.txt — все определяют k; побеждает Z (c.txt)
    assert_eq!(rows(&p), [("Z", "X", Verdict::Suspicious, 0, 1, 0), ("Z", "Y", Verdict::Suspicious, 0, 1, 0)]);
}
#[test] fn vanilla_pairs_follow_spec() {
    // ваниль: common/traits/00_v.txt: "a = {}", common/traits/zz_v.txt: "b = {}"; мод M: common/traits/50_m.txt: "a = {}\nb = {}"
    // a: M позже ванили → M победил; b: ваниль позже M → ваниль победила
    assert_eq!(pair(&p, "M", "vanilla").verdict, Verdict::Vanilla); assert_eq!(pair(&p, "vanilla", "M").verdict, Verdict::Suspicious);
}
#[test] fn pairs_sorted_by_verdict_then_count() {
    // R бьёт S по 1 объекту; P бьёт Q по 3 объектам; I (зависит от J) бьёт J по 5 объектам (имена файлов z… у победителя, a… у проигравшего)
    assert_eq!(names(&p), [("P", "Q"), ("R", "S"), ("I", "J")]);
}
#[test] fn involves_checks_winner_and_any_loser() { assert!(involves(c, &Source::Mod(z), &Source::Mod(y))); assert!(!involves(c, &Source::Mod(y), &Source::Mod(z))); }
#[test] fn loc_value_by_file_and_key() {
    let a = mods.iter().find(|m| m.name == "Base A").unwrap().index.as_ref().unwrap();
    assert_eq!(a.loc_value(&RelPath::new("localization/english/base_a_l_english.yml"), "shared_key"), Some("From A"));
    assert_eq!(a.loc_value(&RelPath::new("localization/english/base_a_l_english.yml"), "nope"), None);
}
```

- [ ] **Step 2: `cargo test -p ck3h-core --features testkit --test pairs` → FAIL**
- [ ] **Step 3: Реализовать** (`pairs` использует `intent.closure()`)
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS (тесты `sort` без изменений)**
- [ ] **Step 5: Commit** `feat(core): user rules count as intent; mod pairs; loc text lookup`

---

### Task 5: CLI — правила в `conflicts`, JSON с именами и предупреждениями — Sonnet

**Files:**
- Modify: `crates/ck3h-cli/src/main.rs`, `crates/ck3h-cli/tests/cli.rs`, `README.md`, `docs/BACKLOG.md`

**Interfaces:**
- Consumes: `intent_graph`, `load_rules`, `session::*`.
- Produces:
  - `ck3h conflicts [--rules <файл>]` — те же правила выбора файла, что у `sort` (явный путь должен существовать; иначе `<config>/ck3-helper/rules.toml`, если есть); отчёт строится на `intent_graph`.
  - JSON `conflicts`, `sort`, `calibrate` получает верхние поля `mods: [{ "id", "name", "position" }]` (все моды плейсета, позиция с 0, как в данных) и `warnings: [{ "mod", "file", "message" }]` (`mod` — имя мода или `"vanilla"`). Остальные поля не меняются.

- [ ] **Step 1: Тесты в `tests/cli.rs`**

```rust
#[test] fn conflicts_respects_rules() {
    // rules.toml: before = "Base B", after = "Base A"
    cmd(&ud).args(["conflicts", "--rules"]).arg(&rules).assert().success().stdout(contains("[intended] object common/traits::shared_trait"));
}
#[test] fn json_outputs_carry_mod_names_and_warnings() {
    // calibrate: --game на мини-папку игры (write_mod "game": common/traits/00.txt) и лог с одной строкой
    for args in [vec!["conflicts", "--json"], vec!["sort", "--json"], calibrate_args] {
        let v = json(&ud, &args);
        assert!(v["mods"].as_array().unwrap().iter().any(|m| m["name"] == "Base A"));
        assert!(v["warnings"].as_array().unwrap().iter().any(|w| w["file"] == "common/traits/bad.txt"));
    }
}
```

- [ ] **Step 2: `cargo test -p ck3h-cli` → FAIL**
- [ ] **Step 3: Реализовать** (обёртка с `#[serde(flatten)]` вокруг отчёта)
- [ ] **Step 4: README:** `--rules` у `conflicts`; правило означает и «B намеренно перекрывает A»; поля `mods` и `warnings` в JSON. **BACKLOG:** удалить закрытые пункты раздела «До плана 2» — все, кроме таблицы замеров из Task 3.
- [ ] **Step 5: `cargo test --workspace --all-features` → PASS**
- [ ] **Step 6: Commit** `feat(cli): conflicts honours rules; JSON carries mod names and warnings`

**Ворота ревью:** задачи 1–5 — `rust-reviewer`; задачи 2–4 — ещё `ck3-rules-auditor`.

---

### Task 6: Каркас приложения — Sonnet

**Files:**
- Modify: `Cargo.toml` (`members`), `.gitignore` (`app/node_modules/`, `app/dist/`), `AGENTS.md` (карта репозитория и команды приложения)
- Create: `.cargo/config.toml` (переменные `ts-rs` из Global Constraints)
- Create: `app/` — `package.json`, `index.html`, `vite.config.ts`, `tsconfig.json`, `eslint.config.js`, `.prettierrc.json`, `src/main.tsx`, `src/App.tsx`, `src/i18n/{index.ts, ru.ts, en.ts}`, `src/test/setup.ts`, `src/App.test.tsx`
- Create: `app/src-tauri/` — `Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `icons/app-icon.svg` (+ сгенерированные иконки), `src/main.rs`, `src/lib.rs`

**Interfaces:**
- Produces:
  - Скрипты `app/package.json`: `dev` (vite), `build` (`tsc -b && vite build`), `typecheck` (`tsc --noEmit`), `lint` (`eslint .`), `format` (`prettier --write .`), `test` (`vitest run`), `tauri` (`tauri`).
  - Зависимости: `react@^19.2`, `react-dom@^19.2`, `@mantine/core@^9`, `@mantine/hooks@^9`, `@mantine/notifications@^9`, `@tanstack/react-query@^5`, `@tanstack/react-virtual@^3`, `i18next`, `react-i18next`, `@tabler/icons-react`, `@tauri-apps/api@^2`, `@tauri-apps/plugin-dialog@^2`, `@tauri-apps/plugin-opener@^2`; dev: `@tauri-apps/cli@^2`, `vite@^8`, `@vitejs/plugin-react`, `typescript`, `@types/react`, `@types/react-dom`, `vitest`, `jsdom`, `@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom`, `eslint`, `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, `prettier`.
  - `ck3h-app`: `tauri = "2"`, `tauri-build = "2"` (build), `tauri-plugin-dialog = "2"`, `tauri-plugin-opener = "2"`, `serde`, `serde_json`, `ts-rs = "12"`, `ck3h-core` (path); dev — `ck3h-core` с `testkit`, `tempfile`.
  - `tauri.conf.json`: значения из Global Constraints; `build.devUrl` `http://localhost:5173`, `frontendDist` `../dist`, `beforeDevCommand` `npm run dev`, `beforeBuildCommand` `npm run build`. `vite.config.ts`: `server: { port: 5173, strictPort: true }`, `clearScreen: false`, `test: { environment: 'jsdom', setupFiles: ['src/test/setup.ts'] }`.
  - `ck3h_app::run()` — строит приложение с двумя плагинами; `main.rs` вызывает его.
  - `src/i18n/index.ts`: `initI18n(lang: Lang)`, `type Lang = 'ru' | 'en'`, `detectLanguage(setting: string | null | undefined, navigatorLanguage: string): Lang` — `ru`/`en` из настройки, иначе `navigatorLanguage` на `ru` → `'ru'`, иначе `'en'`.
  - `App`: `MantineProvider` (`defaultColorScheme="auto"`), `QueryClientProvider`, `AppShell` с боковым меню из пяти пунктов (`nav.playset`, `nav.conflicts`, `nav.order`, `nav.calibration`, `nav.settings`) и пустой областью.
  - `src/test/setup.ts`: `@testing-library/jest-dom`; заглушки `window.matchMedia`, `ResizeObserver`, `Element.prototype.scrollIntoView` (нужны Mantine в jsdom); `afterEach(clearMocks)` из `@tauri-apps/api/mocks`.

- [ ] **Step 1: Каркас.** `npm create vite@latest app -- --template react-ts`, затем `npm install` пакетов выше. Далее `npx tauri init --ci` с флагами имени, заголовка, `--frontend-dist ../dist`, `--dev-url http://localhost:5173`, `--before-dev-command "npm run dev"`, `--before-build-command "npm run build"`. После этого привести файлы к Interfaces (крейт `ck3h-app`, workspace, права, CSP).
- [ ] **Step 2: Иконка.** `app/src-tauri/icons/app-icon.svg` → `npm --prefix app run tauri icon src-tauri/icons/app-icon.svg`

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <rect width="1024" height="1024" rx="224" fill="#1f2430"/>
  <path d="M512 160 L800 256 V520 C800 700 672 820 512 880 C352 820 224 700 224 520 V256 Z" fill="#c9a24a"/>
  <rect x="320" y="360" width="384" height="56" rx="28" fill="#1f2430"/>
  <rect x="320" y="480" width="384" height="56" rx="28" fill="#1f2430"/>
  <rect x="320" y="600" width="384" height="56" rx="28" fill="#1f2430"/>
</svg>
```

- [ ] **Step 3: Тест `src/App.test.tsx`**

```tsx
test('navigation is in Russian for a Russian system', async () => { await initI18n(detectLanguage(null, 'ru-RU')); render(<App />); expect(screen.getByText('Конфликты')).toBeInTheDocument(); });
test('navigation is in English otherwise', async () => { await initI18n(detectLanguage(null, 'de-DE')); render(<App />); expect(screen.getByText('Conflicts')).toBeInTheDocument(); });
test('setting overrides the system language', () => { expect(detectLanguage('en', 'ru-RU')).toBe('en'); });
```

- [ ] **Step 4: `npm test`, `npm run lint`, `npm run typecheck`, `npm run build` в `app/` → PASS**
- [ ] **Step 5: `cargo test --workspace --all-features` → PASS.** Если сборка `ck3h-app` требует `app/dist` (паника `frontendDist … doesn't exist`), в `AGENTS.md` и в CI перед `cargo` добавляется `npm --prefix app run build`. Других обходов не делать.
- [ ] **Step 6: `AGENTS.md`:** строка `app/` в карте; команды `cd app && npm ci && npm run lint && npm run typecheck && npm test` и `npm --prefix app run tauri dev`; инвариант «приложение пишет только `settings.toml`, `settings.toml.bak`, `rules.toml`».
- [ ] **Step 7: Commit** `feat(app): scaffold Tauri v2 + React app`

---

### Task 7: Состояние, настройки, коды ошибок, плейсеты — Sonnet

**Files:**
- Create: `app/src-tauri/src/{state.rs, settings.rs, error.rs, dto.rs, commands.rs}`, `app/src-tauri/tests/setup.rs`
- Modify: `app/src-tauri/src/lib.rs` (`manage(AppState::new(..))`, регистрация команд), `app/src-tauri/Cargo.toml` (`toml`, `dirs`)

**Interfaces:**
- Consumes: `session::launcher_db`, `paths::{config_dir, default_user_dir, default_game_dir, game_language}`, `CoreError`.
- Produces:
  - `settings.rs`: `Settings { user_dir: Option<PathBuf>, game_dir: Option<PathBuf>, analysis_lang: Option<String>, ui_lang: Option<String>, show_vanilla: bool, check_updates: bool, launcher_process_names: Option<Vec<String>>, update_checked_at: Option<i64> }` (`Serialize, Deserialize, TS, Default` вручную: `check_updates = true`, остальное пусто; `#[serde(default, rename_all = "camelCase")]` — те же имена ключей в `settings.toml`). `SettingsFile { settings: Settings, error: Option<String> }`. `load_settings(dir: &Path) -> SettingsFile` — нет файла → по умолчанию без ошибки; не разбирается → по умолчанию + текст ошибки. `save_settings(dir: &Path, s: &Settings) -> std::io::Result<()>` — создать папку; если существующий файл не разбирается, переименовать в `settings.toml.bak` (заменяя старый `.bak`); запись через `settings.toml.tmp` + `rename`.
  - `error.rs`: `enum ErrorCode { NoUserDir, LauncherDbNotFound, Schema, PlaysetNotFound, NoActivePlayset, LauncherRunning, ProcessListUnavailable, PlaysetChanged, NothingToApply, Cycle, NotGameDir, LogNotFound, Rules, Settings, NotAnalyzed, Io, Db }` (`camelCase`); `struct AppError { code: ErrorCode, items: Vec<String>, detail: String }`; `impl From<CoreError> for AppError`: `LauncherRunning(n)`/`Cycle(n)` → `items = n`; `LauncherDbNotFound(p)`/`NotGameDir(p)`/`LogNotFound(p)` → `items = [p]`; `Schema` → `["table.column"]`; `PlaysetNotFound(x)` → `[x]`; `OrderMismatch` → `PlaysetChanged`; `NoLauncherNames` → `Settings`; `Rules` → `Rules`; `Descriptor`/`Io` → `Io`; `Db` → `Db`; `detail = e.to_string()`.
  - `dto.rs`: `SetupView { settings: Settings, settings_error: Option<String>, user_dir: Option<String>, game_dir: Option<String>, game_language: Option<String>, launcher_db_found: bool, app_version: String }` — `user_dir`/`game_dir` действующие (настройка, иначе по умолчанию); `PlaysetRow { id, name, active: bool, mods: usize, enabled: usize }`.
  - `state.rs`: `AppState::new(config_dir: PathBuf) -> AppState` (читает настройки); `setup(&self) -> SetupView`; `save_settings(&self, s: Settings) -> Result<SetupView, AppError>` — смена `user_dir`, `analysis_lang` или `game_dir` сбрасывает текущий анализ (и индекс ванили при смене `game_dir`/языка); `list_playsets(&self) -> Result<Vec<PlaysetRow>, AppError>`; `user_dir(&self) -> Result<PathBuf, AppError>` — настройка, иначе `default_user_dir()`, иначе `NoUserDir`.
  - `lib.rs`: `AppState::new(config_dir().unwrap_or_else(|| PathBuf::from("ck3-helper")))`.
  - Команды: `get_setup`, `save_settings`, `list_playsets`.

- [ ] **Step 1: Тесты в `tests/setup.rs`**

```rust
#[test] fn updates_are_checked_by_default() { assert!(Settings::default().check_updates); }
#[test] fn settings_round_trip() { save_settings(dir, &s)?; assert_eq!(load_settings(dir), SettingsFile { settings: s, error: None }); }
#[test] fn broken_settings_use_defaults_and_keep_backup() {
    std::fs::write(dir.join("settings.toml"), "userDir = [")?;
    let f = load_settings(dir); assert!(f.error.is_some()); assert_eq!(f.settings, Settings::default());
    save_settings(dir, &Settings::default())?; assert_eq!(std::fs::read_to_string(dir.join("settings.toml.bak"))?, "userDir = [");
}
#[test] fn lists_playsets_from_configured_user_dir() {
    // settings.userDir = standard_fixture
    assert_eq!(names(&state.list_playsets()?), [("Main", true, 5, 5)]);
}
#[test] fn missing_launcher_db_is_error_code() { /* userDir = пустая папка */ assert_eq!(state.list_playsets().unwrap_err().code, ErrorCode::LauncherDbNotFound); }
#[test] fn core_errors_map_to_codes() {
    assert_eq!(AppError::from(CoreError::OrderMismatch).code, ErrorCode::PlaysetChanged);
    let e = AppError::from(CoreError::LauncherRunning(vec!["Paradox Launcher".into()])); assert_eq!((e.code, e.items), (ErrorCode::LauncherRunning, vec!["Paradox Launcher".to_string()]));
    assert_eq!(AppError::from(CoreError::NoLauncherNames).code, ErrorCode::Settings);
}
#[test] fn setup_reports_db_and_effective_dirs() { let s = state.setup(); assert!(s.launcher_db_found); assert_eq!(s.user_dir.as_deref(), Some(ud.to_str().unwrap())); assert_eq!(s.game_language.as_deref(), Some("l_english")); }
```

- [ ] **Step 2: `cargo test -p ck3h-app --test setup` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS; в `app/src/api/bindings/` появились `Settings.ts`, `SetupView.ts`, `PlaysetRow.ts`, `AppError.ts`, `ErrorCode.ts`**
- [ ] **Step 5: Commit** `feat(app): settings, error codes, setup and playset commands`

---

### Task 8: Анализ, пары и страницы конфликтов — Opus

**Files:**
- Create: `app/src-tauri/src/analysis.rs`, `app/src-tauri/tests/analysis.rs`
- Modify: `app/src-tauri/src/{state.rs, dto.rs, commands.rs, lib.rs}`

**Interfaces:**
- Consumes: `session::{load, load_vanilla, analysis_lang}`, `build_dep_graph`, `load_rules`, `intent_graph`, `analyze`, `pairs`, `involves`, `ModIndex::loc_value`, `DepGraph::closure`.
- Produces:
  - Состояние: `AppState` хранит `Mutex<Option<Analysis>>` (`loaded`, `deps`, `rules`, `rules_error`, `intent`, `report`, `pairs`, `vanilla_on`) и отдельно `Mutex<Option<VanillaIndex { game_dir, lang, index: ModIndex }>>` — переиспользуется, пока совпадают папка игры и язык.
  - `Progress` — `#[serde(tag = "stage")]`: `Mods { done: usize, total: usize }`, `Vanilla`, `Conflicts`.
  - Зеркала перечислений ядра для TS (с `From`): `LevelView` (`file`/`object`/`loc`), `VerdictView` (`intended`/`suspicious`/`vanilla`), `WinReasonView` (`lowerInPlayset`/`laterFilePath`/`earlierFilePath`/`replaceFolder`).
  - `SourceView { kind: SourceKind (mod|vanilla), id: Option<String>, name: String }` — у ванили `id = None`, `name = ""` (название переводит интерфейс).
  - `ContenderView { source: SourceView, file: String, text: Option<String> }` — `file` = `display`; `text` только у локализации (`loc_value` индекса этого источника).
  - `ConflictView { level: LevelView, target: String, database: Option<String>, key: String, winner: ContenderView, losers: Vec<ContenderView>, verdict: VerdictView, reason: WinReasonView }` — `target` делится по `::`: объект → `database`, `key`; локализация → `key`; файл → `key` = путь.
  - `MarkState` — `available` (два мода, подозрительно, `!closure.depends_on(loser, winner)`), `cycle` (два мода, подозрительно, проигравший уже зависит от победителя), `notApplicable`.
  - `PairView { winner: SourceView, loser: SourceView, verdict: VerdictView, files, objects, loc: usize, mark: MarkState }`
  - `ModView { id, name, position: usize, enabled, missing, compatch: bool, steam_id: Option<String>, dir: String, version: Option<String>, supported_version: Option<String>, dep_issues: Vec<DepIssueView { dependency: String, ambiguous: bool }>, replace_paths: Vec<String>, warnings: usize }`
  - `Counts { suspicious, intended, vanilla, files, objects, loc: usize }` — по конфликтам.
  - `AnalysisView { playset: PlaysetRow, lang: String, vanilla: bool, mods: Vec<ModView>, counts: Counts, pairs: Vec<PairView>, rules_error: Option<String> }`
  - `Page<T> { items: Vec<T>, total: usize, offset: usize }`; `ConflictQuery { text: String, levels: Vec<LevelView>, verdicts: Vec<VerdictView> }` (пустой список — все; `text` — подстрока `target` без учёта регистра); `IndexWarningView { file: String, message: String }`.
  - Методы `AppState`: `analyze(&self, playset_id: Option<&str>, progress: &(dyn Fn(Progress) + Sync)) -> Result<AnalysisView, AppError>` (`None` — активный; правила из `config_dir/rules.toml`; ошибка правил → `rules_error`, анализ без правил; ваниль — если `settings.show_vanilla`); `set_vanilla(&self, on: bool, progress) -> Result<AnalysisView, AppError>` (сохраняет `show_vanilla`; папка игры — настройка, иначе `default_game_dir()`, иначе `NotGameDir` без `items`; моды не переиндексируются); `summary(&self) -> Result<AnalysisView, AppError>`; `pair_conflicts(&self, winner_id: Option<&str>, loser_id: Option<&str>, offset: usize, limit: usize) -> Result<Page<ConflictView>, AppError>` (`None` — ваниль; порядок — как в отчёте); `search_conflicts(&self, query: &ConflictQuery, offset: usize, limit: usize) -> Result<Page<ConflictView>, AppError>`; `mod_warnings(&self, mod_id: &str) -> Result<Vec<IndexWarningView>, AppError>`. Без анализа → `NotAnalyzed`.
  - Команды: `analyze(app, playset_id: Option<String>, on_progress: Channel<Progress>)`, `set_vanilla(app, on: bool, on_progress)`, `summary`, `pair_conflicts`, `search_conflicts`, `mod_warnings`.

- [ ] **Step 1: Тесты в `tests/analysis.rs`** (`standard_fixture`; `gameDir` — мини-папка игры `write_mod(root, "game", &[("common/traits/00_traits.txt", b"only_a = {}")])`)

```rust
#[test] fn analyze_active_playset_summary() {
    let v = state.analyze(None, &|_| {})?;
    assert_eq!(v.playset.name, "Main"); assert_eq!(v.mods.len(), 5);
    assert!(m(&v, "Ghost").missing); assert!(m(&v, "A+B Compatch").compatch);
    assert_eq!(m(&v, "Broken").dep_issues, [DepIssueView { dependency: "Missing Mod".into(), ambiguous: false }]); assert_eq!(m(&v, "Broken").warnings, 1);
}
#[cfg(not(target_os = "linux"))] #[test] fn pairs_and_counts() {
    assert_eq!(v.pairs.len(), 2); let p = &v.pairs[0];
    assert_eq!((p.winner.name.as_str(), p.loser.name.as_str(), p.objects, p.loc, p.mark), ("Base A", "Base B", 1, 1, MarkState::Available));
    assert_eq!(v.counts.suspicious, 3);
}
#[test] fn progress_reaches_total() { assert!(events.contains(&Progress::Mods { done: 5, total: 5 })); }
#[test] fn pair_page_has_loc_texts_and_split_targets() {
    let page = state.pair_conflicts(Some(&a), Some(&b), 0, 50)?;
    let loc = page.items.iter().find(|c| c.level == LevelView::Loc).unwrap();
    assert_eq!((loc.winner.text.as_deref(), loc.losers[0].text.as_deref()), (Some("From A"), Some("From B")));
    let obj = page.items.iter().find(|c| c.level == LevelView::Object).unwrap();
    assert_eq!((obj.database.as_deref(), obj.key.as_str(), obj.winner.file.as_str()), (Some("common/traits"), "shared_trait", "common/traits/zz_base_a.txt"));
}
#[test] fn pages_past_end_are_empty_with_total() { let p = state.search_conflicts(&all(), 100, 10)?; assert!(p.items.is_empty()); assert_eq!(p.total, state.search_conflicts(&all(), 0, 1000)?.items.len()); }
#[test] fn search_filters_text_level_verdict() { let p = state.search_conflicts(&ConflictQuery { text: "SHARED_KEY".into(), levels: vec![LevelView::Loc], verdicts: vec![] }, 0, 10)?; assert_eq!(p.total, 1); }
#[test] fn vanilla_toggle_needs_game_dir() { /* gameDir = пустая папка */ assert_eq!(state.set_vanilla(true, &|_| {}).unwrap_err().code, ErrorCode::NotGameDir); }
#[test] fn vanilla_toggle_adds_vanilla_pair_and_persists() {
    let v = state.set_vanilla(true, &|_| {})?; assert!(v.vanilla);
    assert_eq!(v.pairs.iter().find(|p| p.loser.kind == SourceKind::Vanilla).unwrap().verdict, VerdictView::Vanilla);
    assert!(load_settings(&cfg).settings.show_vanilla);
}
#[test] fn summary_before_analysis_is_not_analyzed() { assert_eq!(state.summary().unwrap_err().code, ErrorCode::NotAnalyzed); }
```

- [ ] **Step 2: `cargo test -p ck3h-app --test analysis` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS**
- [ ] **Step 5: Commit** `feat(app): analysis, mod pairs and paged conflicts`

---

### Task 9: Правила — чтение, «Задумано», удаление — Sonnet

**Files:**
- Create: `app/src-tauri/src/rules_file.rs`, `app/src-tauri/tests/rules.rs`
- Modify: `app/src-tauri/src/{state.rs, dto.rs, commands.rs, lib.rs}`, `app/src-tauri/Cargo.toml` (`toml_edit`)

**Interfaces:**
- Consumes: `parse_rules`, `load_rules`, `resolve_rules`, `intent_graph`, `analyze`, `pairs`, `Analysis` (Task 8).
- Produces:
  - `rules_file::append_rule(path: &Path, rule: &Rule) -> Result<(), AppError>` — текст (нет файла → пусто) сначала проверяется `parse_rules` (ошибка → `Rules`, файл не тронут), затем `toml_edit::DocumentMut`: в массив таблиц `rule` добавляется `{ before, after, note? }`; запись через `.tmp` + `rename`.
  - `rules_file::remove_rule(path: &Path, index: usize) -> Result<(), AppError>` — та же проверка; индекса нет → `Rules` с `items = [index]`.
  - `RuleView { index: usize, before: String, after: String, note: Option<String>, applied: bool }`; `RulesView { path: String, error: Option<String>, rules: Vec<RuleView> }` — `applied` по `resolve_rules` текущего анализа (без анализа — `false`).
  - Методы `AppState`: `rules(&self) -> RulesView`; `add_rule(&self, before: String, after: String, note: Option<String>) -> Result<RulesView, AppError>`; `mark_intended(&self, winner_id: &str, loser_id: &str, note: String) -> Result<RulesView, AppError>` — оба мода из текущего анализа; `closure.depends_on(loser, winner)` → `Cycle` с именами двух модов, файл не тронут; правило `before` = ссылка на проигравшего, `after` = на победителя, ссылка — `steam:<id>`, если у мода есть Steam id, иначе имя; `note` — текст от интерфейса (уже на языке пользователя); `remove_rule(&self, index: usize) -> Result<RulesView, AppError>`. После любой правки текущий анализ получает новые правила, `intent`, отчёт и пары без переиндексации.
  - Команды: `rules`, `add_rule`, `mark_intended`, `remove_rule`.

- [ ] **Step 1: Тесты в `tests/rules.rs`**

```rust
#[test] fn append_keeps_comments_and_escapes_names() {
    std::fs::write(&path, "# my pins\n[[rule]]\nbefore = \"X\"\nafter = \"Y\" # why\n")?;
    append_rule(&path, &Rule { before: "Аутентичные \"шрифты\"".into(), after: "steam:123".into(), note: Some("n".into()) })?;
    let text = std::fs::read_to_string(&path)?; assert!(text.contains("# my pins") && text.contains("# why"));
    assert_eq!(parse_rules(&text)?[1].before, "Аутентичные \"шрифты\"");
}
#[test] fn remove_by_index_keeps_others() { /* три правила, remove_rule(&path, 1) */ assert_eq!(befores(&path), ["A", "C"]); }
#[test] fn broken_rules_file_is_left_untouched() {
    std::fs::write(&rules, "[[rule]\n")?;
    assert_eq!(append_rule(&rules, &r).unwrap_err().code, ErrorCode::Rules); assert_eq!(std::fs::read_to_string(&rules)?, "[[rule]\n");
    assert!(state.analyze(None, &|_| {})?.rules_error.is_some());
}
#[cfg(not(target_os = "linux"))] #[test] fn mark_intended_turns_pair_intended() {
    state.analyze(None, &|_| {})?; state.mark_intended(&a, &b, "intended: Base A overrides Base B".into())?;
    let v = state.summary()?; assert_eq!(pair(&v, "Base A", "Base B").verdict, VerdictView::Intended);
    assert_eq!(pair(&v, "Base B", "Base A").mark, MarkState::Cycle);
    assert_eq!(state.mark_intended(&b, &a, "x".into()).unwrap_err().code, ErrorCode::Cycle);
    assert_eq!(parse_rules(&std::fs::read_to_string(&rules)?)?, [Rule { before: "Base B".into(), after: "Base A".into(), note: Some("intended: Base A overrides Base B".into()) }]);
}
#[test] fn workshop_mods_are_referenced_by_steam_id() { /* custom: моды со steam_id "111" и "222", общий объект */ assert_eq!(parse_rules(&text)?[0].before, "steam:111"); }
#[test] fn rules_view_marks_unapplied() { /* правило before = "Nope" */ assert!(!state.rules().rules[0].applied); }
```

- [ ] **Step 2: `cargo test -p ck3h-app --test rules` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS**
- [ ] **Step 5: Commit** `feat(app): rules file editing and mark-as-intended`

---

### Task 10: Порядок и применение — Sonnet

**Files:**
- Create: `app/src-tauri/src/order.rs`, `app/src-tauri/tests/order.rs`
- Modify: `app/src-tauri/src/{state.rs, dto.rs, commands.rs, lib.rs}`, `app/src-tauri/Cargo.toml` (`chrono`)

**Interfaces:**
- Consumes: `propose`, `apply_order`, `preview`, `launcher_processes`, `DEFAULT_LAUNCHER_PROCESS_NAMES`, `session::playset_unchanged`.
- Produces:
  - `MoveReasonView` — `#[serde(tag = "kind")]`: `Dependency { on: String }`, `RequiredBy { by: String }`, `UserRule { other: String, before: bool, note: Option<String> }` — имена модов; `before` — мод встаёт раньше `other` в предложенном порядке.
  - `OrderRow { id, name, enabled: bool, from: usize, to: usize, moved: bool }` (в предложенном порядке); `MoveView { id, name, from, to, reasons: Vec<MoveReasonView> }`; `WinnerChangeView { level: LevelView, target: String, before: SourceView, after: SourceView }`; `ProposalView { rows, moves, winner_changes, unresolved_rules: Vec<RuleView>, already_sorted: bool }` — `unresolved_rules` — `RuleView` (Task 9) тех правил анализа, чьих индексов нет среди применённых `resolve_rules`.
  - `LauncherStatus` — `#[serde(tag = "state")]`: `NotRunning`, `Running { processes: Vec<String> }`, `Unknown`.
  - `ApplyPreviewView { new_playset_name: String, backup_dir: String }`; `ApplyView { new_playset_name: String, backup_path: String }`.
  - Методы `AppState`: `propose(&self) -> Result<ProposalView, AppError>` (ваниль — если включена; предложение кэшируется в анализе; `Cycle` → `items` = имена); `launcher_status(&self) -> LauncherStatus` (имена процессов — настройка, иначе по умолчанию; не блокирует анализ); `apply_preview(&self) -> Result<ApplyPreviewView, AppError>`; `apply(&self, now: chrono::DateTime<chrono::Local>) -> Result<ApplyView, AppError>` — нет кэшированного предложения или нет перемещений → `NothingToApply`; `playset_unchanged` = false → `PlaysetChanged`; затем `apply_order`.
  - Команды: `propose`, `launcher_status`, `apply_preview`, `apply` (`now = Local::now()`).

- [ ] **Step 1: Тесты в `tests/order.rs`** (`launcherProcessNames = ["ck3h-test-no-such-process"]`, кроме отмеченного)

```rust
#[test] fn proposal_moves_dependent_after_dependency() {
    // custom: "Child" (deps "Parent") на 0, "Parent" на 1
    let p = state.propose()?; assert_eq!(row_names(&p), ["Parent", "Child"]);
    assert_eq!(p.moves[0].reasons, [MoveReasonView::Dependency { on: "Parent".into() }]); assert!(!p.already_sorted);
}
#[test] fn sorted_playset_has_nothing_to_apply() { let p = state.propose()?; assert!(p.already_sorted); assert_eq!(state.apply(now).unwrap_err().code, ErrorCode::NothingToApply); }
#[test] fn cycle_is_error_with_names() { /* A deps B, B deps A */ let e = state.propose().unwrap_err(); assert_eq!(e.code, ErrorCode::Cycle); assert_eq!(sorted(e.items), ["A", "B"]); }
#[test] fn apply_creates_copy_and_backup() {
    state.propose()?; assert_eq!(state.apply_preview()?.new_playset_name, "Main (CK3 Helper)");
    let r = state.apply(now)?; assert_eq!(playset_count(&db), 2); assert!(Path::new(&r.backup_path).exists());
}
#[test] fn apply_refuses_changed_playset() {
    state.propose()?; /* UPDATE playsets_mods SET enabled = 0 WHERE position = 0 */
    assert_eq!(state.apply(now).unwrap_err().code, ErrorCode::PlaysetChanged); assert_eq!(playset_count(&db), 1);
}
#[test] fn apply_blocked_by_running_launcher() {
    // launcherProcessNames = [имя текущего тестового бинарника]
    assert_eq!(state.apply(now).unwrap_err().code, ErrorCode::LauncherRunning); assert_eq!(playset_count(&db), 1);
}
#[test] fn launcher_status_not_running() { assert_eq!(state.launcher_status(), LauncherStatus::NotRunning); }
```

- [ ] **Step 2: `cargo test -p ck3h-app --test order` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS**
- [ ] **Step 5: Commit** `feat(app): load-order proposal and apply as a new playset`

---

### Task 11: Сверка и проверка обновлений — Sonnet

**Files:**
- Create: `app/src-tauri/src/update.rs`, `app/src-tauri/tests/calibration.rs`
- Modify: `app/src-tauri/src/{state.rs, dto.rs, commands.rs, lib.rs}`, `app/src-tauri/Cargo.toml` (`ureq = "3"`, `semver = "1"`)

**Interfaces:**
- Consumes: `calibrate`, `check_last_run`, `session::read_conflicts_log`, `VanillaIndex` (Task 8).
- Produces:
  - `CalibrationView { matched: usize, mismatched: Vec<MismatchView { database, key, log_file, predicted_file }>, log_unknown: Vec<DatabaseCount { database: String, count: usize }>, logged_single_definition: Vec<DatabaseCount>, predicted_not_logged: Vec<ObjectRef { database: String, key: String }>, truncated: bool, last_run: Option<LastRunView { not_in_last_run: Vec<String>, only_in_last_run: Vec<String> }> }` — `not_in_last_run` — имена модов.
  - `AppState::calibrate(&self, progress) -> Result<CalibrationView, AppError>` — нужен анализ; индекс ванили (как в `set_vanilla`) → `NotGameDir`; лог → `LogNotFound`.
  - `update::releases_api(repository: &str) -> Option<String>` — `https://github.com/<owner>/<repo>` (допускаются хвостовые `/` и `.git`) → `https://api.github.com/repos/<owner>/<repo>/releases/latest`; иначе `None`.
  - `update::newer_release(current: &str, tag: &str) -> bool` — `tag` без ведущего `v`, `semver`; ошибка разбора → `false`.
  - `update::due(settings: &Settings, now: i64) -> bool` — `check_updates` и (`update_checked_at` нет или `now - t >= 86_400`).
  - `UpdateView { version: String, url: String }`; `update::fetch_latest(api: &str, current: &str) -> Result<Option<UpdateView>, String>` — `ureq`, таймаут 5 с, `User-Agent: ck3-helper/<current>`, `Accept: application/vnd.github+json`; поля `tag_name`, `html_url`.
  - `AppState::check_update(&self, now: i64) -> Option<UpdateView>` — не `due` или `releases_api(env!("CARGO_PKG_REPOSITORY"))` = `None` → `None`, без сети; иначе запрос, запись `update_checked_at = now` в любом исходе, ошибки молча.
  - Команды: `calibrate(app, on_progress)`, `check_update` (`now` = текущее Unix-время).

- [ ] **Step 1: Тесты** — `update.rs` (модуль) и `tests/calibration.rs`

```rust
#[test] fn releases_api_from_repository_url() {
    assert_eq!(releases_api("https://github.com/o/ck3-helper.git").as_deref(), Some("https://api.github.com/repos/o/ck3-helper/releases/latest"));
    assert_eq!(releases_api("https://github.com/o/ck3-helper/").as_deref(), Some("https://api.github.com/repos/o/ck3-helper/releases/latest"));
    assert_eq!(releases_api(""), None); assert_eq!(releases_api("https://gitlab.com/o/r"), None);
}
#[test] fn newer_release_compares_semver() { assert!(newer_release("0.1.0", "v0.2.0")); assert!(!newer_release("0.2.0", "v0.2.0")); assert!(!newer_release("0.1.0", "garbage")); }
#[test] fn due_once_a_day_when_enabled() {
    let mut s = Settings::default(); assert!(due(&s, 100_000)); s.update_checked_at = Some(100_000);
    assert!(!due(&s, 100_000 + 86_399)); assert!(due(&s, 100_000 + 86_400)); s.check_updates = false; assert!(!due(&s, 999_999));
}
// tests/calibration.rs
#[test] fn calibrate_matches_log_line() {
    // gameDir — мини-игра с common/traits/00_traits.txt: "shared_trait = {}"; лог: shared_trait из common/traits/zz_base_a.txt
    assert_eq!(state.calibrate(&|_| {})?.matched, 1);
}
#[test] fn calibrate_without_log_is_log_not_found() { assert_eq!(state.calibrate(&|_| {}).unwrap_err().code, ErrorCode::LogNotFound); }
```

- [ ] **Step 2: `cargo test -p ck3h-app` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `cargo test --workspace --all-features` → PASS**
- [ ] **Step 5: Commit** `feat(app): calibration and update check commands`

**Ворота ревью:** задачи 6–11 — `rust-reviewer`.

---

### Task 12: Основа интерфейса — API, языки, ошибки, оболочка — Sonnet

**Files:**
- Create: `app/src/api/{client.ts, queries.ts}`, `app/src/errors.ts`, `app/src/layout/{Shell.tsx, Header.tsx}`, `app/src/screens/{PlaysetScreen,ConflictsScreen,OrderScreen,CalibrationScreen,SettingsScreen}.tsx` (заглушки с заголовком), тесты `app/src/{i18n/i18n.test.ts, errors.test.ts, layout/Shell.test.tsx}`
- Modify: `app/src/App.tsx`, `app/src/i18n/{ru.ts, en.ts, index.ts}`

**Interfaces:**
- Consumes: типы из `app/src/api/bindings/`; команды задач 7–11.
- Produces:
  - `api/client.ts` — по функции на команду, через `invoke` из `@tauri-apps/api/core`, имена команд snake_case, аргументы camelCase: `getSetup()`, `saveSettings(settings)`, `listPlaysets()`, `analyze(playsetId: string | null, onProgress: (p: Progress) => void)`, `setVanilla(on, onProgress)`, `summary()`, `pairConflicts(winnerId: string | null, loserId: string | null, offset, limit)`, `searchConflicts(query, offset, limit)`, `modWarnings(modId)`, `rules()`, `addRule(before, after, note: string | null)`, `markIntended(winnerId, loserId, note)`, `removeRule(index)`, `propose()`, `launcherStatus()`, `applyPreview()`, `apply()`, `calibrate(onProgress)`, `checkUpdate()`. Прогресс — `new Channel<Progress>()` с `onmessage`. `isAppError(e: unknown): e is AppError`.
  - `api/queries.ts` — ключи TanStack Query: `['setup']`, `['playsets']`, `['summary']`, `['pair', winnerId, loserId]`, `['search', query]`, `['rules']`, `['proposal']`, `['launcher']`, `['calibration']`; хук `useAnalyze()` — мутация, кладёт результат в `['summary']` и сбрасывает `pair`, `search`, `rules`, `proposal`, `calibration`.
  - `errors.ts` — `errorMessage(t: TFunction, e: unknown): string`: `AppError` → `t('errors.' + code, { items: items.join(', ') })`; прочее → `t('errors.unknown')`.
  - Оболочка: при старте `getSetup`; `!launcherDbFound` → экран настроек с текстом `setup.noLauncherDb`; иначе `analyze(null)` с полосой прогресса (`progress.mods` «Мод {{done}} из {{total}}», `progress.vanilla`, `progress.conflicts`). Пункты «Конфликты», «Порядок», «Сверка» недоступны, пока нет `['summary']`. В шапке — имя плейсета, кнопка «Обновить» (`analyze` текущего id), баннер `settingsError`, баннер новой версии (`checkUpdate` один раз при старте; ссылка — `openUrl` плагина `opener`). Язык — `detectLanguage(settings.uiLang, navigator.language)`.
  - Ресурсы: множественное число — суффиксы i18next (`_one`, `_few`, `_many`, `_other` в ru; `_one`, `_other` в en).

- [ ] **Step 1: Тесты**

```ts
test('ru and en have the same keys, plural suffixes folded', () => { expect(keys(ru)).toEqual(keys(en)); }); // keys() убирает _(zero|one|two|few|many|other)$
test('Russian plurals', async () => { await initI18n('ru'); expect(i18n.t('counts.objects', { count: 3 })).toBe('3 объекта'); expect(i18n.t('counts.objects', { count: 5 })).toBe('5 объектов'); });
test('launcherRunning message names the processes', async () => { await initI18n('ru'); expect(errorMessage(i18n.t, { code: 'launcherRunning', items: ['Paradox Launcher'], detail: '' })).toContain('Paradox Launcher'); });
test('first run without launcher db opens settings', async () => { mockIPC(cmd => cmd === 'get_setup' ? { ...setup, launcherDbFound: false } : null); render(<App />); expect(await screen.findByText(ru.setup.noLauncherDb)).toBeInTheDocument(); });
test('analysis screens are disabled until analysis finishes', async () => { /* mockIPC: analyze — вечный Promise */ expect(screen.getByRole('link', { name: 'Конфликты' })).toHaveAttribute('data-disabled', 'true'); });
test('update banner links to the release', async () => { /* check_update → { version: '0.2.0', url: 'https://github.com/o/r/releases/tag/v0.2.0' } */ expect(await screen.findByText(/0\.2\.0/)).toBeInTheDocument(); });
```

- [ ] **Step 2: `npm test` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `npm run lint && npm run typecheck && npm test` → PASS**
- [ ] **Step 5: Commit** `feat(app): API client, i18n, error texts and app shell`

---

### Task 13: Экран «Плейсет» — Sonnet

**Files:**
- Modify: `app/src/screens/PlaysetScreen.tsx`
- Create: `app/src/components/{ModTable.tsx, WarningsDrawer.tsx}`, `app/src/screens/PlaysetScreen.test.tsx`

**Interfaces:**
- Consumes: `listPlaysets`, `useAnalyze`, `summary`, `modWarnings`, `revealItemInDir` (`@tauri-apps/plugin-opener`).
- Produces: экран spec §4.8.2 п. 1. Список плейсетов: активный — значок, анализируемый — выделен, клик по другому — `analyze(id)`. Сводка: модов, включено, не найдено на диске, проблемы зависимостей, compatch. Таблица модов: `#` (позиция + 1), имя и версия, значки `missing`, `depIssues` («нет зависимости X» / «неоднозначная зависимость X»), `compatch`, `replacePaths` («заменяет папку X»), кнопка с числом предупреждений → `WarningsDrawer` (`modWarnings`), «Открыть папку» (`revealItemInDir(dir)`, недоступна у `missing`).

- [ ] **Step 1: Тесты** (данные — `AnalysisView` в духе `standard_fixture`)

```tsx
test('shows mod statuses', () => { expect(screen.getByText('не найден на диске')).toBeInTheDocument(); expect(screen.getByText('нет зависимости Missing Mod')).toBeInTheDocument(); expect(screen.getByText('compatch')).toBeInTheDocument(); });
test('warnings drawer lists files', async () => { /* mod_warnings → [{ file: 'common/traits/bad.txt', message: 'unbalanced braces' }] */ await user.click(warningsButtonFor('Broken')); expect(await screen.findByText('common/traits/bad.txt')).toBeInTheDocument(); expect(invoked('mod_warnings')).toEqual({ modId: 'broken-id' }); });
test('choosing another playset analyzes it', async () => { /* list_playsets → Main и Other */ await user.click(screen.getByText('Other')); expect(invoked('analyze')).toMatchObject({ playsetId: 'other-id' }); });
test('open folder is disabled for a missing mod', () => { expect(openButtonFor('Ghost')).toBeDisabled(); });
```

- [ ] **Step 2: `npm test` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `npm run lint && npm run typecheck && npm test` → PASS**
- [ ] **Step 5: Commit** `feat(app): playset screen`

---

### Task 14: Экран «Конфликты» — Opus

**Files:**
- Modify: `app/src/screens/ConflictsScreen.tsx`
- Create: `app/src/components/{PairList.tsx, ConflictList.tsx, ConflictItem.tsx, MarkIntendedButton.tsx}`, `app/src/explain.ts`, тесты `app/src/{explain.test.ts, screens/ConflictsScreen.test.tsx}`

**Interfaces:**
- Consumes: `summary`, `setVanilla`, `pairConflicts`, `searchConflicts`, `markIntended`, `getSetup`.
- Produces:
  - `explain(t: TFunction, c: ConflictView): string[]` — строки spec §4.8.2 «Объяснения» (тексты ru — оттуда дословно, en — их перевод): по `(level, reason)` — `explain.fileLower`, `explain.objectLater` (с файлами победителя и первого проигравшего и подсказкой про compatch), `explain.locEarlier`, `explain.locReplace`; если победила ваниль — ещё `explain.vanillaWins`.
  - `confirm.markIntended` — текст подтверждения из spec §4.8.2 п. 2 («Y будет грузиться раньше X; если сейчас X выше Y, на экране «Порядок» появится перестановка»); `rules.intendedNote` — «задумано: {{winner}} перекрывает {{loser}}» / «intended: {{winner}} overrides {{loser}}».
  - Экран spec §4.8.2 п. 2:
    - переключатель ванили (`setVanilla` с прогрессом; недоступен с подсказкой, если в `setup` нет `gameDir`);
    - вкладки «Пары модов» (по умолчанию) и «Все конфликты»;
    - фильтр пометок по умолчанию — `suspicious` и `intended`;
    - списки виртуализированы `useVirtualizer`, конфликты грузятся страницами по 100 через `useInfiniteQuery`;
    - поиск — с задержкой 300 мс.
  - «Задумано»:
    - при `mark === 'available'`: подтверждение `confirm.markIntended` → `markIntended(winner.id, loser.id, t('rules.intendedNote', { winner, loser }))` → сброс `summary`, `pair`, `search`, `rules`, `proposal`;
    - при `mark === 'cycle'`: кнопка недоступна, подсказка `markIntended.cycle`;
    - при `notApplicable`: кнопки нет.
  - Локализация: под каждой стороной — `text`.

- [ ] **Step 1: Тесты**

```ts
test('object conflict explains file order and suggests a compatch', () => { const lines = explain(t, objectConflict); expect(lines.join(' ')).toContain('zz_base_a.txt'); expect(lines.join(' ')).toContain('compatch'); });
test('vanilla winner adds the not-in-effect line', () => { expect(explain(t, vanillaWins)).toContain(t('explain.vanillaWins')); });
test('each win reason has its own text', () => { expect(new Set(reasons.map(r => explain(t, withReason(r))[0])).size).toBe(4); });
```
```tsx
test('vanilla pairs are hidden by default', () => { /* summary с парой Base A → ваниль */ expect(screen.queryByText(ru.source.vanilla)).toBeNull(); });
test('pair row shows counts with plurals', () => { expect(screen.getByText(/1 объект/)).toBeInTheDocument(); expect(screen.getByText(/1098 строк/)).toBeInTheDocument(); });
test('mark intended sends a localized note', async () => {
  await user.click(screen.getByRole('button', { name: 'Задумано' })); await user.click(screen.getByRole('button', { name: 'Подтвердить' }));
  expect(invoked('mark_intended')).toEqual({ winnerId: 'a', loserId: 'b', note: 'задумано: Base A перекрывает Base B' });
});
test('cycle pair has a disabled button', () => { expect(markButtonFor('Base B', 'Base A')).toBeDisabled(); });
test('loc conflict shows both texts', async () => { await user.click(pairRow('Base A', 'Base B')); expect(await screen.findByText('From A')).toBeInTheDocument(); expect(screen.getByText('From B')).toBeInTheDocument(); });
test('search is debounced and sends the text', async () => { vi.useFakeTimers({ shouldAdvanceTime: true }); await user.type(searchBox(), 'shared'); vi.advanceTimersByTime(300); expect(lastInvoked('search_conflicts').query.text).toBe('shared'); });
test('long lists render only visible rows', async () => { /* search_conflicts → total 5000, страницы по 100 */ expect(screen.getAllByTestId('conflict-row').length).toBeLessThan(100); });
```

- [ ] **Step 2: `npm test` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `npm run lint && npm run typecheck && npm test` → PASS**
- [ ] **Step 5: Commit** `feat(app): conflicts screen with mod pairs and search`

---

### Task 15: Экран «Порядок» и применение — Sonnet

**Files:**
- Modify: `app/src/screens/OrderScreen.tsx`
- Create: `app/src/components/{OrderTable.tsx, MoveReasons.tsx, ApplyDialog.tsx}`, `app/src/screens/OrderScreen.test.tsx`

**Interfaces:**
- Consumes: `propose`, `launcherStatus`, `applyPreview`, `apply`, `useAnalyze`, `revealItemInDir`.
- Produces: экран spec §4.8.2 п. 3 и §4.8.3.
  - Предложение — `['proposal']`.
  - Таблица в предложенном порядке: новая позиция, имя, «было #N» у перемещённых.
  - Перемещения с причинами — `order.reason.dependency`, `order.reason.requiredBy`, `order.reason.ruleBefore` / `ruleAfter` с заметкой.
  - Смена победителей, неприменённые правила.
  - `alreadySorted` — текст `order.alreadySorted`, кнопки нет. `Cycle` — предупреждение с именами и `order.cycleHint`.
  - Статус лаунчера — `['launcher']` с `refetchInterval: 2000`, пока экран открыт: `running` — кнопка недоступна, «Закройте Paradox Launcher: имена»; `unknown` — недоступна, `apply.processListUnavailable`.
  - Применение: «Сохранить как новый плейсет» → `applyPreview` → диалог (имя, папка бэкапа, `apply.sourceUntouched`) → `apply` → итог (имя, «Показать бэкап в папке» → `revealItemInDir(backupPath)`, `apply.pickInLauncher`).
  - `playsetChanged` — сообщение и кнопка «Пересчитать» (`analyze`).

- [ ] **Step 1: Тесты**

```tsx
test('running launcher disables saving and names the process', async () => { /* launcher_status → { state: 'running', processes: ['Paradox Launcher'] } */ expect(await saveButton()).toBeDisabled(); expect(screen.getByText(/Paradox Launcher/)).toBeInTheDocument(); });
test('unknown process list disables saving with its own text', async () => { expect(await screen.findByText(ru.apply.processListUnavailable)).toBeInTheDocument(); });
test('already sorted shows no save button', async () => { expect(await screen.findByText(ru.order.alreadySorted)).toBeInTheDocument(); expect(screen.queryByRole('button', { name: ru.apply.save })).toBeNull(); });
test('apply flow shows the copy name and reveals the backup', async () => {
  await user.click(await saveButton()); expect(await screen.findByText('Main (CK3 Helper)')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: ru.apply.confirm })); await user.click(await screen.findByRole('button', { name: ru.apply.revealBackup }));
  expect(invoked('plugin:opener|reveal_item_in_dir')).toBeTruthy();
});
test('changed playset offers to recalculate', async () => { /* apply → { code: 'playsetChanged' } */ await user.click(await screen.findByRole('button', { name: ru.apply.recalculate })); expect(invoked('analyze')).toBeTruthy(); });
test('user rule reason shows side and note', () => { expect(screen.getByText(/раньше Base A — B first/)).toBeInTheDocument(); });
```

- [ ] **Step 2: `npm test` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `npm run lint && npm run typecheck && npm test` → PASS**
- [ ] **Step 5: Commit** `feat(app): order screen and apply flow`

---

### Task 16: Экраны «Сверка» и «Настройки» — Sonnet

**Files:**
- Modify: `app/src/screens/{CalibrationScreen.tsx, SettingsScreen.tsx}`
- Create: `app/src/components/RulesEditor.tsx`, тесты `app/src/screens/{CalibrationScreen,SettingsScreen}.test.tsx`

**Interfaces:**
- Consumes: `calibrate`, `getSetup`, `saveSettings`, `rules`, `addRule`, `removeRule`, `summary`, `useAnalyze`, `open` (`@tauri-apps/plugin-dialog`).
- Produces:
  - **«Сверка»** (spec §4.8.2 п. 4):
    - без `gameDir` — текст и ссылка на настройки; иначе кнопка «Запустить сверку» → `calibrate` с прогрессом;
    - итог `calibration.summary` («Совпало N, разошлось M»), предупреждения `truncated` и `lastRun` с именами;
    - раскрываемые списки;
    - `logNotFound` → `calibration.noLog` (сначала запустить игру с этим плейсетом).
  - **«Настройки»** (spec §4.8.2 п. 5):
    - папки пользователя и игры: поле, «Выбрать…» (`open({ directory: true })`), «По умолчанию» (сбрасывает настройку, показывает найденную папку);
    - язык анализа — `Autocomplete` с вариантами `l_english`, `l_french`, `l_german`, `l_spanish`, `l_russian`, `l_korean`, `l_simp_chinese` и свободным вводом; пусто — язык игры;
    - язык интерфейса — «Как в системе / Русский / English»;
    - переключатель проверки обновлений;
    - в разделе «Дополнительно» — имена процесса лаунчера через запятую, «По умолчанию» — пусто;
    - `RulesEditor`: список правил (`before → after`, заметка, значок «не применено»), добавление (два `Select` из модов анализа и заметка → `addRule` с именами модов), удаление с подтверждением; ошибка `rules.toml` — баннер с путём, правка недоступна.
    - Сохранение — `saveSettings`; при смене папки пользователя или языка анализа — `analyze` заново.

- [ ] **Step 1: Тесты**

```tsx
test('no log explains how to get one', async () => { /* calibrate → { code: 'logNotFound' } */ expect(await screen.findByText(ru.calibration.noLog)).toBeInTheDocument(); });
test('calibration summary', async () => { /* matched 1370, mismatched [] */ expect(await screen.findByText(/1370/)).toBeInTheDocument(); });
test('folder picker fills user dir and saves', async () => { /* plugin:dialog|open → '/tmp/ck3' */ await user.click(pickUserDir()); await user.click(saveButton()); expect(invoked('save_settings').settings.userDir).toBe('/tmp/ck3'); });
test('changing analysis language re-analyzes', async () => { /* ввести l_russian, сохранить */ expect(invoked('analyze')).toBeTruthy(); });
test('unapplied rule is marked', async () => { /* rules → [{ before: 'Nope', after: 'Base A', applied: false }] */ expect(await screen.findByText(ru.rules.unapplied)).toBeInTheDocument(); });
test('broken rules file disables editing', async () => { /* rules → { error: 'expected ]' } */ expect(await addRuleButton()).toBeDisabled(); });
test('add rule sends mod names', async () => { expect(invoked('add_rule')).toEqual({ before: 'Base B', after: 'Base A', note: 'B first' }); });
```

- [ ] **Step 2: `npm test` → FAIL**
- [ ] **Step 3: Реализовать**
- [ ] **Step 4: `npm run lint && npm run typecheck && npm test` → PASS**
- [ ] **Step 5: Commit** `feat(app): calibration and settings screens`

**Ворота ревью:** задачи 12–16 — ревью интерфейса (React, доступность, тексты только из ресурсов, обработка ошибок).

---

### Task 17: Сборка на macOS и документация — Sonnet

**Files:**
- Modify: `app/src-tauri/tauri.conf.json`, `README.md`, `AGENTS.md`, `docs/GLOSSARY.md`, `docs/BACKLOG.md`

**Interfaces:**
- Produces:
  - `bundle`: `active: true`, `targets: ["nsis", "dmg", "app", "appimage", "deb"]`, `category: "Utility"`, `license: "GPL-3.0-or-later"`, `shortDescription`, `macOS.signingIdentity: "-"`, `windows.nsis.installMode: "currentUser"`.
  - README:
    - раздел «Приложение / App» на русском и английском: скачать из Releases;
    - первый запуск неподписанной программы: Windows — «Подробнее → Выполнить в любом случае»; macOS — «Системные настройки → Конфиденциальность и безопасность → Всё равно открыть» или `xattr -dr com.apple.quarantine "/Applications/CK3 Helper.app"`; Linux — `chmod +x` для AppImage;
    - сеть — только проверка обновлений, выключается в настройках;
    - приложение пишет только `settings.toml`, `rules.toml` и новую копию плейсета.
  - `GLOSSARY`: «Пара модов», «Граф намерений», «Сессия», если ещё нет.
  - `BACKLOG`: раздел «После плана 2» — universal-сборка проверяется только в CI; проверка обновлений — после первого релиза.

- [ ] **Step 1: Конфиг бандла по Interfaces**
- [ ] **Step 2: `npm --prefix app run tauri build -- --bundles app,dmg` → в `target/release/bundle/` есть `CK3 Helper.app` и `.dmg`; `.app` запускается и анализирует активный плейсет владельца**
- [ ] **Step 3: README, AGENTS.md, GLOSSARY, BACKLOG**
- [ ] **Step 4: Commit** `chore(app): bundle config and docs`

---

### Task 18: CI и релизы на GitHub — Sonnet; выполняется, когда есть репозиторий

**Предусловия (решает владелец, план их не выполняет):** репозиторий на GitHub создан и его адрес известен; автор коммитов исправлен до первого push.

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`
- Modify: `repository = "<адрес репозитория>"` в `[package]` всех трёх `Cargo.toml` (включает проверку обновлений, Task 11)

**Interfaces:**
- Produces:
  - `ci.yml` — на `push` в `main` и на `pull_request`. Матрица `ubuntu-22.04`, `macos-latest`, `windows-latest`. Шаги:
    - `actions/checkout`, `dtolnay/rust-toolchain@stable` (`clippy`, `rustfmt`), `Swatinem/rust-cache`;
    - на Linux — `apt-get install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`;
    - `actions/setup-node` (Node 22, кэш npm по `app/package-lock.json`), `npm ci --prefix app`;
    - `npm --prefix app run lint`, `typecheck`, `test`;
    - `cargo fmt --all --check`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo test --workspace --all-features`;
    - `git diff --exit-code app/src/api/bindings`.
  - `release.yml` — на тег `v*`, `permissions: contents: write`. Матрица:
    - `macos-latest` с `args: --target universal-apple-darwin` и Rust-целями `aarch64-apple-darwin,x86_64-apple-darwin`;
    - `ubuntu-22.04` (те же apt-пакеты);
    - `windows-latest`.

    Шаг `tauri-apps/tauri-action@v0`: `projectPath: app`, `tagName: ${{ github.ref_name }}`, `releaseName: CK3 Helper ${{ github.ref_name }}`, `releaseBody` — многострочный текст: инструкции первого запуска неподписанной программы для трёх ОС на русском и английском (те же, что в README, Task 17) и ссылка на README через `${{ github.server_url }}/${{ github.repository }}`; `releaseDraft: true`, `prerelease: false`.

- [ ] **Step 1: Оба workflow по Interfaces; `repository` в `Cargo.toml`**
- [ ] **Step 2: Push ветки (с согласия владельца) → CI зелёный на трёх ОС**
- [ ] **Step 3: Тег `v0.1.0` (с согласия владельца) → черновик релиза с `.dmg`, установщиком NSIS `.exe`, `.AppImage`, `.deb`; `.dmg` скачан на Mac владельца и открыт по инструкции README**
- [ ] **Step 4: Commit** `ci: tests on three platforms and tagged releases`

---

### Task 19: Ручная проверка на реальном плейсете — владелец и агент

Сборка из Task 17 или `npm --prefix app run tauri dev`; плейсет владельца. Применение — только на копии папки пользователя (`cp -R "…/Crusader Kings III" <копия>`, в настройках — папка копии).

- [ ] **Step 1: Запуск.** Активный плейсет анализируется с прогрессом; записать время анализа без ванили и с ванилью.
- [ ] **Step 2: «Плейсет».** Моды без «не найден на диске»; предупреждения открываются; «Открыть папку» показывает папку мода.
- [ ] **Step 3: «Конфликты».** Сверху пара `Divine Intervention CM [RUS] → Divine Intervention`, около 1098 строк локализации, «подозрительно»; тексты обеих сторон видны.
- [ ] **Step 4: «Задумано»** на этой паре:
  - пара стала задуманной, подозрительных стало на столько же меньше;
  - в `rules.toml` правило с `steam:<id>` и русской заметкой;
  - прежние комментарии файла на месте.
- [ ] **Step 5: Ваниль.** Включение показывает прогресс «индексирую игру»; пары с ванилью скрыты фильтром по умолчанию и видны при включении фильтра.
- [ ] **Step 6: «Порядок».** Предложение с причинами или «порядок уже правильный».
- [ ] **Step 7: Применение на копии.**
  - Лаунчер открыт → кнопка недоступна, имя процесса показано.
  - Лаунчер закрыт → диалог с «… (CK3 Helper)» → сохранено.
  - Бэкап лежит в папке копии; `launcher-v2.sqlite` в настоящей папке не изменился (время изменения то же).
- [ ] **Step 8: «Сверка».** Числа совпадают с `ck3h calibrate --game <папка игры>` на том же логе.
- [ ] **Step 9: English.** После переключения все тексты интерфейса английские; затем вернуть.
- [ ] **Step 10: Память.** Пиковая память приложения после анализа с ванилью (Activity Monitor) — записать.
- [ ] **Step 11: Windows.** Если у владельца есть машина — шаги 1–9 на ней. Если нет — в черновике релиза пометка «Windows проверена только автотестами; сообщите о проблемах в Issues» (Q32).
- [ ] **Step 12: Итоги шагов 1, 10 и найденные проблемы — в `docs/BACKLOG.md`; commit** `docs: manual check of the app on a real playset`

---

## После плана 2

1. Пункты BACKLOG «До публичного релиза» (лаунчер принимает копию, свежая калибровка, проверка на Windows/Linux) — до анонса.
2. v2 по spec §3: разбор `error.log`, ck3-tiger, проверка после патча.

# CK3 Helper

**English** — A tool for Crusader Kings III players with large playsets: see which mods override
which (files, game objects, localization), why each one wins, and get a suggested load order.
Status: in development, nothing released yet.

**Русский** — Программа для игроков Crusader Kings III с большими плейсетами: показывает, какие
моды перезаписывают друг друга (файлы, объекты игры, локализация), почему победил именно этот
вариант, и предлагает порядок загрузки. Статус: в разработке, релизов пока нет.

Mods and saves are never modified. Applying a load order creates a new playset copy, after a
backup of the launcher database.

License: [GPL-3.0-or-later](LICENSE).

## Приложение / App

### Русский

Настольное приложение (Tauri) на том же ядре, что и `ck3h`: конфликты, объяснения «почему победил
этот мод», правила и применение порядка загрузки.

- **Скачать:** со страницы Releases репозитория на GitHub. Релизы появятся, когда будет создан
  репозиторий; до этого приложение собирается локально.
- **Первый запуск неподписанной программы.**
  - Windows: в окне SmartScreen нажмите «Подробнее → Выполнить в любом случае».
  - macOS: «Системные настройки → Конфиденциальность и безопасность → Всё равно открыть» или
    `xattr -dr com.apple.quarantine "/Applications/CK3 Helper.app"`.
  - Linux: для AppImage — `chmod +x CK3.Helper*.AppImage`, затем запуск.
- **Сеть:** только проверка обновлений (запрос последнего релиза на GitHub, не чаще раза в сутки).
  Выключается в настройках.
- **Что пишет приложение:** только `settings.toml`, `settings.toml.bak` и `rules.toml` в
  `<config>/ck3-helper/` и новую копию плейсета (после резервной копии базы лаунчера). Моды и
  сейвы не изменяются.
- **Запуск из исходников** (нужен Node 22.22+ и Rust):

```bash
npm --prefix app ci
npm --prefix app run tauri dev
```

### English

A desktop app (Tauri) on the same core as `ck3h`: conflicts, "why did this mod win" explanations,
rules, and applying a load order.

- **Download:** from the repository's GitHub Releases page. Releases appear once the GitHub
  repository exists; until then the app is built locally.
- **First launch of the unsigned app.**
  - Windows: in the SmartScreen dialog choose "More info → Run anyway".
  - macOS: "System Settings → Privacy & Security → Open Anyway", or
    `xattr -dr com.apple.quarantine "/Applications/CK3 Helper.app"`.
  - Linux: `chmod +x CK3.Helper*.AppImage`, then run it.
- **Network:** only the update check (a request for the latest GitHub release, at most once a
  day). It can be turned off in Settings.
- **What the app writes:** only `settings.toml`, `settings.toml.bak` and `rules.toml` in
  `<config>/ck3-helper/`, and a new playset copy (after a backup of the launcher database). Mods
  and saves are never modified.
- **Run from source** (Node 22.22+ and Rust required):

```bash
npm --prefix app ci
npm --prefix app run tauri dev
```

## Install (CLI)

```bash
cargo install --path crates/ck3h-cli      # installs the `ck3h` binary
# or: cargo build --release               # binary at target/release/ck3h
```

## Usage

`ck3h` reads the Paradox Launcher database (`launcher-v2.sqlite`) in the CK3 user directory.
Every command takes `--user-dir <path>` (default: `Documents/Paradox Interactive/Crusader Kings III`
on Windows and macOS, `~/.local/share/Paradox Interactive/Crusader Kings III` on Linux) and
`--json`; all of them analyse the active playset unless `--playset <name>` is given.

```bash
ck3h playsets                              # list playsets, * marks the active one
ck3h conflicts [--lang l_russian] [--game <path to game/>] [--rules rules.toml]
ck3h sort [--rules rules.toml]             # propose a load order, writes nothing
ck3h sort --apply                          # save it as a new playset "<name> (CK3 Helper)"
ck3h calibrate [--game <path to game/>]    # compare with logs/database_conflicts.log
```

- `--game` points at the install's `game/` folder so vanilla files count as contenders.
  `conflicts` and `sort` use vanilla only when it is given. `calibrate` needs it: without
  `--game` it looks in the default Steam library (`C:\Program Files (x86)\Steam` on Windows,
  `~/Library/Application Support/Steam` on macOS, `~/.local/share/Steam` on Linux, then
  `steamapps/common/Crusader Kings III/game`) and fails if it is not there.
- `sort --apply` backs the database up first, writes a new playset only, and refuses while the
  Paradox Launcher is running (exit code 2). If the order is already sorted it writes nothing.
  Exit code 1 is any other error, message on stderr.
- Files that cannot be read or parsed are reported as `warning: <mod>: <file>: <message>`;
  the analysis continues without them.
- With `--json`, `conflicts`, `sort` and `calibrate` also print `mods` (every mod of the playset:
  `id`, `name`, `position` counted from 0) and `warnings` (`mod`, `file`, `message`; `mod` is the
  mod name or `"vanilla"`), the same warnings the text output prints.

### rules.toml

Pin mods against each other for `sort`. A rule "A before B" also means "B intentionally overrides A": `conflicts` marks the conflicts where B beats A as intended. `before` and `after` are a mod name from its
`descriptor.mod` or `steam:<workshop id>`; `note` is optional. Unknown keys are an error.

```toml
[[rule]]
before = "Medieval Arts"
after = "steam:3816257844"
note = "compatch loads after the base"
```

`--rules <file>` (`sort` and `conflicts`) must exist. Without it, `<config dir>/ck3-helper/rules.toml` is used if present
(`~/Library/Application Support` on macOS, `~/.config` on Linux, `%APPDATA%` on Windows).

## Credits

The list of database root folders (`common/...`) comes from
[ck3-tiger](https://github.com/amtep/tiger) (GPL-3.0-or-later).

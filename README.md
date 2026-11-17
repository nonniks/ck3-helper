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

## Install

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
ck3h conflicts [--lang l_russian] [--game <path to game/>]
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

### rules.toml

Pin mods against each other for `sort`. `before` and `after` are a mod name from its
`descriptor.mod` or `steam:<workshop id>`; `note` is optional. Unknown keys are an error.

```toml
[[rule]]
before = "Medieval Arts"
after = "steam:3816257844"
note = "compatch loads after the base"
```

`--rules <file>` must exist. Without it, `<config dir>/ck3-helper/rules.toml` is used if present
(`~/Library/Application Support` on macOS, `~/.config` on Linux, `%APPDATA%` on Windows).

## Credits

The list of database root folders (`common/...`) comes from
[ck3-tiger](https://github.com/amtep/tiger) (GPL-3.0-or-later).

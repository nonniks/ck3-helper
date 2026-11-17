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

## Usage

`ck3h` reads the Paradox Launcher database (`launcher-v2.sqlite`) in the CK3 user directory.
Every command takes `--user-dir <path>` (default: the platform's CK3 user directory) and
`--json`; all of them analyse the active playset unless `--playset <name>` is given.

```bash
ck3h playsets                              # list playsets, * marks the active one
ck3h conflicts [--lang l_russian] [--game <path to game/>]
ck3h sort [--rules rules.toml]             # propose a load order, writes nothing
ck3h sort --apply                          # save it as a new playset "<name> (CK3 Helper)"
ck3h calibrate [--game <path to game/>]    # compare with logs/database_conflicts.log
```

- `--game` points at the install's `game/` folder so vanilla files count as contenders.
- `--rules` defaults to `<config dir>/ck3-helper/rules.toml`; a missing file means no rules.
- `sort --apply` backs the database up first and refuses while the Paradox Launcher is running
  (exit code 2). Exit code 1 is any other error, message on stderr.

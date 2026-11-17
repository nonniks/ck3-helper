---
name: ck3-engine-rules
description: CK3 engine facts behind ck3-helper's analysis — evidence, external file formats and the calibration loop. Use when changing or reviewing how a conflict winner is decided, how load order is sorted, how localization overrides resolve, how the launcher DB is read or written, or how database_conflicts.log is parsed.
---

# CK3 engine rules: evidence and verification

The rules themselves (file override, LIOS objects, first-wins localization, `replace`) live in
**spec §4.3** — `docs/superpowers/specs/2026-11-17-ck3-helper-v1-design.md`. This skill holds what
backs them and how to check them. Read §4.3 first; come here for the evidence.

## Evidence for each rule

| Rule (spec §4.3) | Evidence | Confidence |
|---|---|---|
| Same relative path → mod lower in playset wins the whole file | ck3-tiger `src/fileset.rs`: entries ordered by path, then `FileKind`; later kind wins | High |
| `common/` objects: files read in path order, last definition wins | tiger `src/db.rs` `add_inner` replaces the earlier entry; game writes `Overriding entry` lines for each override | High, calibrate per patch |
| Localization: first definition wins; `replace` paths override | tiger `src/data/localization.rs`: "in loca the older definition overrides the later one"; `is_replace_path` inserts over it | Medium — contradicts the community modding reference ("last loaded wins"); verify in game |
| `replace_path = "X"` drops files lying exactly in folder `X` from vanilla, DLC and every earlier mod | tiger `src/fileset.rs` `should_replace` (exact `==` on the parent dir) | High for tiger; verify in game |
| A database is a known root, not the file's parent folder; files may sit in subfolders | real log: `'common/decisions'` for `common/decisions/dlc_decisions/ep_3/…`; tiger `src/ck3/tables/misc.rs` `COMMON_DIRS` ("all common dirs can have subdirectories") | High |
| `gfx/` has keyed databases too | real log: `gfx/portraits/portrait_animations`, `gfx/portraits/accessories`, `gfx/models/units/entity_links` … (~30% of override lines) | High for the roots seen in the log |
| Path EQUALITY (same file replaced) follows the platform's filesystem: case-insensitive on Windows/macOS, case-sensitive on Linux | filesystem semantics | Medium |
| File READ ORDER is case-sensitive, byte-wise per path component, on every OS | real log: `gfx/portraits/portrait_modifiers` `developer_characters_accessories` — vanilla `02_all_developer_characters.txt` beat EPE `02_EPE_Easter_Eggs_characters.txt` because 'E' < 'a'; tiger orders paths case-sensitively; after the change calibrate went to 1370 matched / 0 mismatched | High on macOS; Windows unverified |
| `common/province_terrain` merges: vanilla repeats province ids across files and the game logs none of them as overrides | calibrate on the real playset: 11,481 "predicted but not logged" pairs, all in this folder | High — file-level only |
| `on_action`, `defines`, `genes` and container databases (`named_colors`, `coat_of_arms/coat_of_arms`, `landed_titles`) are not plain keyed databases | on_action lists merge while `trigger`/`effect` overwrite; defines merge per key; gene categories merge; tiger `data/colors.rs` (`colors = {}`), `coa.rs` (`template = {}`); titles nest | Medium — file-level only in v1 |

tiger source: <https://github.com/amtep/tiger> (the owner keeps a clone at `~/Code/tiger`).

## External formats

- **Launcher DB** `launcher-v2.sqlite` in the Paradox user dir. Load order = `playsets_mods.position`
  (0-based, per playset). `mods.dirPath` is an absolute mod folder path. `mods_dependencies` is
  empty in practice — dependencies come only from `descriptor.mod`, **by mod name**, not by id.
  The launcher keeps the DB open: writes happen with the launcher closed, after a file backup.
- **`descriptor.mod`**: Paradox script; `name`, `version`, `supported_version`, `dependencies = { "Name" … }`,
  repeatable `replace_path`, `remote_file_id`.
- **Localization line**: ` key:0 "value"` (the number is optional); first line `l_<lang>:`; files are
  UTF-8 with BOM; `#` starts a comment.
- **`logs/database_conflicts.log`**, rewritten at each game launch:
  `[hh:mm:ss][W][game_database.h:272]: Overriding entry '<key>' for database '<db>' in 'file: <path> line: <n>'`.
  The file named is the overriding (newer) definition; the last line per `(db, key)` is the final winner.
  The log names neither the mod nor the loser. Quirks seen in a real log: a database name with a
  trailing slash (`'common/graphical_unit_types/'`), gene overrides in another format
  (`genedatabase.cpp … for database gene database`), and a last line cut off mid-write.
- **`dlc_load.json`** in the user dir: `enabled_mods` lists `mod/<name>.mod` files of the last launch;
  each `.mod` has `path="…"` pointing at the mod folder. Use it to check that a log belongs to the
  playset being analysed.
- **`pdx_settings.txt`**: game language in the `"language"` block, `value="l_russian"`.
- **`logs/error.log`** prints `Duplicate localization key … defined in both 'A' and 'B'` without saying
  which one is used.

## Calibration loop

A rule change is done when the real playset agrees with the game:

1. The owner launches CK3 with the playset (this rewrites `database_conflicts.log`).
2. `cargo run -p ck3h-cli -- calibrate` → `matched / mismatched / unknown`.
3. Every mismatch is explained: a rule fix (spec §4.3 + tests) or a documented exception.
4. Localization has no log of winners — check one contested key on screen in game.

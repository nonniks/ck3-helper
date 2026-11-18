# AGENTS.md

CK3 Helper is a desktop tool for Crusader Kings III players with large playsets: it maps which
mods override which (files, `common/` objects, localization keys), explains why each winner
won, and proposes a load order. Rust core, CLI, and a Tauri app on the same core.
License: GPL-3.0-or-later.

## Repo map

| Path | What |
|---|---|
| `crates/ck3h-core` | All CK3 knowledge: launcher DB, `descriptor.mod`, mod index, conflict engine, sorter, playset writer, calibration. No UI, no network. |
| `crates/ck3h-cli` | `ck3h` binary, a thin `clap` layer over the core. |
| `app/` | Desktop app: Tauri v2 shell (`app/src-tauri`, crate `ck3h-app`) + React 19, TypeScript, Mantine 9 UI. Needs Node 22.22+ (Node 24 recommended). |
| `docs/` | Design specs, implementation plans, glossary — start at `docs/README.md`. |

## Commands

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all
cargo run -p ck3h-cli -- conflicts        # reads the real launcher DB, read-only
cd app && npm ci && npm run lint && npm run typecheck && npm test
npm --prefix app run tauri dev            # run the desktop app (Node 22.22+)
npm --prefix app run tauri build -- --bundles app,dmg   # local macOS bundle in target/release/bundle/
```

## Invariants

These hold in every change. A change that needs to break one goes to the owner first.

- **Mods and saves are read-only.** Nothing under a mod folder, the workshop folder or
  `save games/` is ever written.
- **One writer.** `ck3h_core::apply::apply_order` is the only code that writes the launcher DB:
  backup first, one transaction, a new playset only. Existing playsets stay untouched.
- **Tests never touch the real launcher DB** — they build one from
  `crates/ck3h-core/fixtures/launcher_schema.sql` through `testkit`.
- **Deterministic output**: identical input gives byte-identical reports (`BTreeMap`, explicit sorts).
- **User data never panics the program**: bad bytes, broken braces, missing folders and odd
  schemas become warnings or `CoreError`, never `unwrap` failures.
- **Engine rules live in one place**: spec §4.3. Changing how a winner is decided means updating
  that section, its tests, and re-running `ck3h calibrate` on a real playset.
- **The app writes only** `<config>/ck3-helper/settings.toml`, `settings.toml.bak` and `rules.toml`;
  the launcher DB goes through `apply_order` alone. Its only network call is the update check
  (`app/src-tauri/src/update.rs`).
- No telemetry. The core makes no network calls.

## Conventions

- Code, identifiers, comments, commit messages and CLI output: English.
- Design docs in `docs/` are written in Russian for the owner; keep their structure when editing.
- TDD: a failing test before the code that makes it pass. Plans in `docs/superpowers/plans/` carry
  the exact test names and assertions.
- Commits: conventional prefixes (`feat(core):`, `fix(cli):`, `docs:`, `chore:`), one task per commit.

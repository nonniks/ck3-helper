# CLAUDE.md

@AGENTS.md

## Claude Code specifics

- **CK3 engine questions** (who wins a conflict, load order, localization override, launcher DB,
  `database_conflicts.log`): load the `ck3-engine-rules` skill before answering or editing.
- **Review lanes**: Rust changes go to the `rust-reviewer` agent; changes to `index`, `conflicts`,
  `sort`, `apply` or `calibrate` also go to `ck3-rules-auditor`.
- **Plans** are executed with `superpowers:subagent-driven-development` or
  `superpowers:executing-plans`; the plan file names the spec it implements.
- **Branches**: `main` is the public branch. Work on `feat/…`, `fix/…`, `docs/…` branches and land
  through a pull request; commit and push only when the owner asks.
- **The owner writes in Russian** — answer in Russian; keep code and commits in English.

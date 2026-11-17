//! Calibration: our predicted object winners against the game's `database_conflicts.log`
//! (spec §6).

use crate::conflicts::resolve_objects;
use crate::descriptor::parse_descriptor;
use crate::index::{ModIndex, normalize};
use crate::playset::ModEntry;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LogOverride {
    pub database: String,
    pub key: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParsedLog {
    pub overrides: Vec<LogOverride>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunCheck {
    pub not_in_last_run: Vec<String>,
    pub only_in_last_run: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mismatch {
    pub database: String,
    pub key: String,
    pub log_file: String,
    pub predicted_file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalibrationReport {
    pub matched: usize,
    pub mismatched: Vec<Mismatch>,
    pub log_unknown: BTreeMap<String, usize>,
    pub predicted_not_logged: Vec<(String, String)>,
    pub truncated: bool,
    pub run_check: Option<RunCheck>,
}

/// Reads the game's override lines. Anything else (gene overrides, a line cut off mid-write) is
/// skipped; the last line per `(database, key)` is the final winner. `truncated` is set when the
/// last non-empty line does not end with a closing quote.
pub fn parse_conflicts_log(text: &str) -> ParsedLog {
    let mut last: BTreeMap<(String, String), String> = BTreeMap::new();
    for line in text.lines() {
        if let Some((database, key, file)) = parse_line(line) {
            last.insert((database, key), file);
        }
    }
    let truncated = text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| !l.trim_end().ends_with('\''));
    ParsedLog {
        overrides: last
            .into_iter()
            .map(|((database, key), file)| LogOverride {
                database,
                key,
                file,
            })
            .collect(),
        truncated,
    }
}

fn parse_line(line: &str) -> Option<(String, String, String)> {
    let rest = &line[line.find("Overriding entry '")? + "Overriding entry '".len()..];
    let (key, rest) = rest.split_once("' for database '")?;
    let (database, rest) = rest.split_once("' in 'file: ")?;
    let file = &rest[..rest.rfind(" line: ")?];
    let database = normalize(database.strip_suffix('/').unwrap_or(database));
    Some((database, key.to_string(), normalize(file)))
}

/// Canonical (when the path exists), normalized, without a trailing slash.
fn path_key(path: &Path) -> String {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    normalize(path.to_string_lossy().trim_end_matches(['/', '\\']))
}

/// Compares the mod folders of `dlc_load.json` (the last launch) with the enabled mods.
/// `None` when the file is missing or unreadable.
pub fn check_last_run(user_dir: &Path, mods: &[ModEntry]) -> Option<RunCheck> {
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(user_dir.join("dlc_load.json")).ok()?).ok()?;
    let mut last_run: Vec<(String, String)> = Vec::new(); // (key, shown)
    for entry in json.get("enabled_mods")?.as_array()? {
        let Some(name) = entry.as_str() else { continue };
        let dir = std::fs::read(user_dir.join(name))
            .ok()
            .and_then(|bytes| parse_descriptor(&bytes).ok())
            .and_then(|d| d.path);
        match dir {
            Some(dir) => last_run.push((path_key(Path::new(&dir)), dir)),
            None => last_run.push((String::new(), name.to_string())),
        }
    }
    let enabled: Vec<(String, &ModEntry)> = mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| (path_key(&m.dir), m))
        .collect();
    let mut not_in_last_run: Vec<String> = enabled
        .iter()
        .filter(|(key, _)| !last_run.iter().any(|(k, _)| k == key))
        .map(|(_, m)| m.id.clone())
        .collect();
    let mut only_in_last_run: Vec<String> = last_run
        .iter()
        .filter(|(key, _)| key.is_empty() || !enabled.iter().any(|(k, _)| k == key))
        .map(|(_, shown)| shown.clone())
        .collect();
    not_in_last_run.sort();
    only_in_last_run.sort();
    Some(RunCheck {
        not_in_last_run,
        only_in_last_run,
    })
}

/// Compares every log line with the last contender of our `resolve_objects`, and lists the
/// contested pairs of ours that the log never mentions.
pub fn calibrate(
    log: &ParsedLog,
    mods: &[ModEntry],
    vanilla: Option<&ModIndex>,
    run_check: Option<RunCheck>,
) -> CalibrationReport {
    let predicted = resolve_objects(mods, vanilla);
    let mut matched = 0;
    let mut mismatched = Vec::new();
    let mut log_unknown: BTreeMap<String, usize> = BTreeMap::new();
    for o in &log.overrides {
        let winner = predicted
            .get(&(o.database.clone(), o.key.clone()))
            .and_then(|defs| defs.last());
        match winner {
            None => *log_unknown.entry(o.database.clone()).or_default() += 1,
            Some(w) if w.file.norm == o.file => matched += 1,
            Some(w) => mismatched.push(Mismatch {
                database: o.database.clone(),
                key: o.key.clone(),
                log_file: o.file.clone(),
                predicted_file: w.file.display.clone(),
            }),
        }
    }
    let logged: BTreeSet<(&str, &str)> = log
        .overrides
        .iter()
        .map(|o| (o.database.as_str(), o.key.as_str()))
        .collect();
    let predicted_not_logged = predicted
        .iter()
        .filter(|((db, key), defs)| {
            defs.len() > 1 && !logged.contains(&(db.as_str(), key.as_str()))
        })
        .map(|(pair, _)| pair.clone())
        .collect();
    CalibrationReport {
        matched,
        mismatched,
        log_unknown,
        predicted_not_logged,
        truncated: log.truncated,
        run_check,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_keeps_last_line_per_key() {
        let log = "[01:59:17][W][game_database.h:272]: Overriding entry 'difficulty' for database 'common/game_rules' in 'file: common/game_rules/a.txt line: 1'\n[01:59:18][W][game_database.h:272]: Overriding entry 'difficulty' for database 'common/game_rules' in 'file: common/game_rules/b.txt line: 3'\n";
        assert_eq!(
            parse_conflicts_log(log).overrides,
            [LogOverride {
                database: "common/game_rules".into(),
                key: "difficulty".into(),
                file: "common/game_rules/b.txt".into()
            }]
        );
    }

    #[test]
    fn sample_log_handles_real_formats() {
        let p = parse_conflicts_log(include_str!("../fixtures/database_conflicts_sample.log"));
        assert!(p.truncated);
        assert!(
            p.overrides
                .iter()
                .any(|o| o.database == "common/graphical_unit_types")
        );
        assert!(
            p.overrides
                .iter()
                .any(|o| o.database == "common/decisions" && o.file.contains("dlc_decisions/ep_3"))
        );
        assert!(
            p.overrides
                .iter()
                .any(|o| o.database == "gfx/portraits/portrait_animations")
        );
        assert!(p.overrides.iter().all(|o| !o.database.contains("gene")));
    }

    #[test]
    fn complete_log_is_not_truncated_and_junk_is_skipped() {
        let log =
            "garbage\n\n[x]: Overriding entry 'k' for database 'd' in 'file: d/a.txt line: 1'\n\n";
        let p = parse_conflicts_log(log);
        assert!(!p.truncated);
        assert_eq!(p.overrides.len(), 1);
        assert!(!parse_conflicts_log("").truncated);
    }
}

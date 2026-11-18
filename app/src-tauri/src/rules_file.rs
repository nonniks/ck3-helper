//! Edits of the user's `rules.toml` that keep its comments and layout (spec §4.8.4).

use crate::error::{AppError, ErrorCode};
use ck3h_core::error::CoreError;
use ck3h_core::sort::{Rule, parse_rules};
use std::path::Path;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

pub fn append_rule(path: &Path, rule: &Rule) -> Result<(), AppError> {
    edit(path, |tables| {
        let mut table = Table::new();
        table["before"] = value(rule.before.as_str());
        table["after"] = value(rule.after.as_str());
        if let Some(note) = &rule.note {
            table["note"] = value(note.as_str());
        }
        tables.push(table);
        Ok(())
    })
}

pub fn remove_rule(path: &Path, index: usize) -> Result<(), AppError> {
    edit(path, |tables| {
        if index >= tables.len() {
            return Err(AppError {
                code: ErrorCode::Rules,
                items: vec![index.to_string()],
                detail: format!("no rule with index {index}"),
            });
        }
        tables.remove(index);
        Ok(())
    })
}

/// Validates the file (a missing one is empty), applies `f` to its `[[rule]]` tables and
/// writes the result through a temporary file. A file that does not parse stays untouched.
fn edit(
    path: &Path,
    f: impl FnOnce(&mut ArrayOfTables) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let rules_error = |detail: String| AppError::from(CoreError::Rules(detail));
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(rules_error(format!("{}: {e}", path.display()))),
    };
    parse_rules(&text)?;
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| rules_error(format!("{}: {e}", path.display())))?;
    let tables = doc
        .entry("rule")
        .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or_else(|| {
            rules_error(format!(
                "{}: `rule` is not an array of tables",
                path.display()
            ))
        })?;
    f(tables)?;
    write_atomic(path, &doc.to_string())
}

fn write_atomic(path: &Path, text: &str) -> Result<(), AppError> {
    let io = |e: std::io::Error| AppError {
        code: ErrorCode::Io,
        items: vec![path.display().to_string()],
        detail: e.to_string(),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, text).map_err(io)?;
    std::fs::rename(&tmp, path).map_err(io)
}

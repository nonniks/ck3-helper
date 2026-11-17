//! Index of one mod (or of vanilla): files, database objects, localization entries (spec §4.2).

use crate::databases::{OBJECT_ANALYSIS_EXCLUDED, database_root};
use crate::error::CoreError;
use crate::text::decode;
use jomini::TextTape;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs::{DirEntry, ReadDir};
use std::path::{Path, PathBuf};

/// `\` → `/`; lowercase except on Linux, whose filesystem is case-sensitive.
pub fn normalize(display: &str) -> String {
    let path = display.replace('\\', "/");
    if cfg!(target_os = "linux") {
        path
    } else {
        path.to_lowercase()
    }
}

/// A path relative to a mod folder: `display` as on disk, `norm` for comparing and sorting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelPath {
    pub display: String,
    pub norm: String,
}

impl RelPath {
    pub fn new(display: &str) -> RelPath {
        RelPath {
            display: display.replace('\\', "/"),
            norm: normalize(display),
        }
    }

    /// Normalized folder of the file; empty for a file at the root.
    pub fn parent_norm(&self) -> String {
        self.norm
            .rsplit_once('/')
            .map(|(dir, _)| dir.to_string())
            .unwrap_or_default()
    }
}

impl Ord for RelPath {
    /// Component by component on `norm`; `display` breaks ties so that `Ord` agrees with `Eq`.
    fn cmp(&self, other: &Self) -> Ordering {
        self.norm
            .split('/')
            .cmp(other.norm.split('/'))
            .then_with(|| self.display.cmp(&other.display))
    }
}

impl PartialOrd for RelPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A top-level key of a database file; `ordinal` counts objects within the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectDef {
    pub database: String,
    pub key: String,
    pub file: RelPath,
    pub ordinal: u32,
}

/// A localization entry; `ordinal` counts entries within the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocDef {
    pub lang: String,
    pub key: String,
    pub value: String,
    pub file: RelPath,
    pub is_replace: bool,
    pub ordinal: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct IndexWarning {
    pub file: String,
    pub message: String,
}

/// Every vector is sorted: files by path, objects and loc by `(file, ordinal)`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ModIndex {
    pub files: Vec<RelPath>,
    pub objects: Vec<ObjectDef>,
    pub loc: Vec<LocDef>,
    pub warnings: Vec<IndexWarning>,
}

impl ModIndex {
    fn sort(&mut self) {
        self.files.sort();
        self.objects
            .sort_by(|a, b| a.file.cmp(&b.file).then(a.ordinal.cmp(&b.ordinal)));
        self.loc
            .sort_by(|a, b| a.file.cmp(&b.file).then(a.ordinal.cmp(&b.ordinal)));
        self.warnings.sort();
    }

    /// Lays `layer` over `self`: its files replace same-`norm` files with their objects and loc.
    /// Leaves `self` unsorted.
    fn overlay(&mut self, layer: ModIndex) {
        let replaced: BTreeSet<String> = layer.files.iter().map(|f| f.norm.clone()).collect();
        self.files.retain(|f| !replaced.contains(&f.norm));
        self.objects.retain(|o| !replaced.contains(&o.file.norm));
        self.loc.retain(|l| !replaced.contains(&l.file.norm));
        self.files.extend(layer.files);
        self.objects.extend(layer.objects);
        self.loc.extend(layer.loc);
        self.warnings.extend(layer.warnings);
    }
}

/// Indexes a mod folder. `lang` is a game language tag such as `l_english` (`english` works too).
pub fn index_mod(root: &Path, lang: &str) -> Result<ModIndex, CoreError> {
    index_layer(root, lang, None)
}

/// Indexes the install's `game/` folder without `dlc/`, then lays each `game/dlc/<folder>/`
/// over it in folder-name order.
pub fn index_vanilla(game_dir: &Path, lang: &str) -> Result<ModIndex, CoreError> {
    let mut index = index_layer(game_dir, lang, Some("dlc"))?;
    let dlc_dir = game_dir.join("dlc");
    if !dlc_dir.is_dir() {
        return Ok(index);
    }
    let listing = std::fs::read_dir(&dlc_dir).and_then(|entries| {
        entries
            .map(|entry| entry.map(|e| (e.file_name().to_string_lossy().into_owned(), e.path())))
            .collect::<std::io::Result<Vec<_>>>()
    });
    let mut layers = listing.unwrap_or_else(|e| {
        index.warnings.push(IndexWarning {
            file: "dlc".into(),
            message: format!("cannot list folder: {e}"),
        });
        Vec::new()
    });
    layers.retain(|(name, dir)| !name.starts_with('.') && dir.is_dir());
    layers.sort();
    for (name, dir) in layers {
        match index_layer(&dir, lang, None) {
            Ok(mut layer) => {
                for w in &mut layer.warnings {
                    w.file = format!("dlc/{name}/{}", w.file);
                }
                index.overlay(layer);
            }
            Err(e) => index.warnings.push(IndexWarning {
                file: format!("dlc/{name}"),
                message: format!("cannot list folder: {e}"),
            }),
        }
    }
    index.sort();
    Ok(index)
}

/// `skip_dir` is a top-level folder left out of the walk (vanilla's `dlc`).
fn index_layer(root: &Path, lang: &str, skip_dir: Option<&str>) -> Result<ModIndex, CoreError> {
    let mut index = ModIndex::default();
    let mut found = Vec::new();
    walk(
        std::fs::read_dir(root)?,
        "",
        skip_dir,
        &mut found,
        &mut index.warnings,
    );
    let lang = format!("l_{}", lang.strip_prefix("l_").unwrap_or(lang));
    for (file, path) in found {
        index_file(&file, &path, &lang, &mut index);
        index.files.push(file);
    }
    index.sort();
    Ok(index)
}

enum EntryKind {
    File,
    Dir,
    LinkedDir,
    Other,
}

fn entry_kind(entry: &DirEntry) -> std::io::Result<EntryKind> {
    let file_type = entry.file_type()?;
    let (is_dir, is_file, linked) = if file_type.is_symlink() {
        let meta = std::fs::metadata(entry.path())?;
        (meta.is_dir(), meta.is_file(), true)
    } else {
        (file_type.is_dir(), file_type.is_file(), false)
    };
    Ok(match (is_dir, is_file, linked) {
        (true, _, false) => EntryKind::Dir,
        (true, _, true) => EntryKind::LinkedDir,
        (_, true, _) => EntryKind::File,
        _ => EntryKind::Other,
    })
}

/// Collects files under `rel`, skipping hidden entries and files lying in the root itself.
/// Symlinked folders are not followed, so a link cycle cannot recurse forever.
fn walk(
    entries: ReadDir,
    rel: &str,
    skip_dir: Option<&str>,
    files: &mut Vec<(RelPath, PathBuf)>,
    warnings: &mut Vec<IndexWarning>,
) {
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warnings.push(IndexWarning {
                    file: rel.to_string(),
                    message: format!("cannot list folder: {e}"),
                });
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let child = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        let mut warn = |message: String| {
            warnings.push(IndexWarning {
                file: child.clone(),
                message,
            })
        };
        match entry_kind(&entry) {
            Ok(EntryKind::Dir) if skip_dir.is_some_and(|dir| normalize(&child) == dir) => {}
            Ok(EntryKind::Dir) => match std::fs::read_dir(entry.path()) {
                Ok(sub) => walk(sub, &child, None, files, warnings),
                Err(e) => warn(format!("cannot list folder: {e}")),
            },
            Ok(EntryKind::LinkedDir) => warn("symlinked folder not followed".into()),
            Ok(EntryKind::File) if !rel.is_empty() => {
                files.push((RelPath::new(&child), entry.path()))
            }
            Ok(EntryKind::File | EntryKind::Other) => {}
            Err(e) => warn(format!("cannot read entry: {e}")),
        }
    }
}

fn index_file(file: &RelPath, path: &Path, lang: &str, index: &mut ModIndex) {
    let database = database_root(&file.norm).filter(|db| {
        file.norm.ends_with(".txt") && !OBJECT_ANALYSIS_EXCLUDED.contains(&db.as_str())
    });
    let file_name = file.norm.rsplit('/').next().unwrap_or_default();
    let is_loc = file.norm.starts_with("localization/")
        && file_name.ends_with(".yml")
        && file_name.contains(lang);
    if database.is_none() && !is_loc {
        return;
    }
    let mut warn = |message: String| {
        index.warnings.push(IndexWarning {
            file: file.display.clone(),
            message,
        })
    };
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return warn(format!("cannot read file: {e}")),
    };
    let (text, invalid_utf8) = decode(&bytes);
    if invalid_utf8 {
        warn("invalid UTF-8".into());
    }
    if let Some(database) = database {
        match top_level_keys(&text) {
            Ok(keys) => index
                .objects
                .extend((0u32..).zip(keys).map(|(ordinal, key)| ObjectDef {
                    database: database.clone(),
                    key,
                    file: file.clone(),
                    ordinal,
                })),
            Err(message) => warn(message),
        }
    } else {
        let is_replace = file.norm.split('/').any(|segment| segment == "replace");
        index.loc.extend(
            (0u32..)
                .zip(loc_entries(&text))
                .map(|(ordinal, (key, value))| LocDef {
                    lang: lang.to_string(),
                    key: key.to_string(),
                    value: value.to_string(),
                    file: file.clone(),
                    is_replace,
                    ordinal,
                }),
        );
    }
}

/// Top-level keys except `@` variables; an error message if the file is broken.
fn top_level_keys(text: &str) -> Result<Vec<String>, String> {
    let tape = TextTape::from_slice(text.as_bytes()).map_err(|e| format!("parse error: {e}"))?;
    if !braces_balanced(text) {
        return Err("unbalanced braces".into());
    }
    Ok(tape
        .utf8_reader()
        .fields()
        .map(|(key, _, _)| key.read_string())
        .filter(|key| !key.starts_with('@'))
        .collect())
}

/// Counts braces outside quoted strings and `#` comments: jomini accepts some unbalanced files.
fn braces_balanced(text: &str) -> bool {
    let mut depth = 0i64;
    let (mut in_string, mut escaped, mut in_comment) = (false, false, false);
    for b in text.bytes() {
        if in_comment {
            in_comment = b != b'\n';
        } else if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'#' => in_comment = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }
    depth == 0
}

/// `(key, value)` of lines ` key:0 "value"` (the number is optional). The value runs to the
/// last quote on the line, as unescaped quotes inside values are common. Other lines, including
/// the `l_<lang>:` header and comments, yield nothing.
fn loc_entries(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.lines().filter_map(|line| {
        let line = line.trim_start();
        if line.starts_with('#') {
            return None;
        }
        let (key, rest) = line.split_once(':')?;
        if key.is_empty() || key.contains(char::is_whitespace) {
            return None;
        }
        let value = rest
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .trim_start()
            .strip_prefix('"')?;
        let end = value.rfind('"')?;
        Some((key, &value[..end]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braces_in_strings_and_comments_do_not_count() {
        assert!(braces_balanced("a = \"{\" # }\"\nb = { c = \"x\\\"{\" }"));
        assert!(!braces_balanced("a = { } }"));
    }

    #[test]
    fn loc_value_runs_to_last_quote() {
        let entries: Vec<_> =
            loc_entries("l_english:\n k:0 \"say \"hi\"\" # note\n not an entry\n").collect();
        assert_eq!(entries, [("k", "say \"hi\"")]);
    }
}

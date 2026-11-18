//! Loads a launcher playset into `ModEntry`s: descriptor plus index per enabled mod.

use crate::descriptor::{ModDescriptor, read_descriptor};
use crate::error::CoreError;
use crate::index::{IndexWarning, ModIndex, index_mod};
use crate::launcher::{LauncherDb, Playset, PlaysetMod};
use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModEntry {
    pub id: String,
    /// `descriptor.name`, else the launcher's display name.
    pub name: String,
    pub steam_id: Option<String>,
    pub position: i64,
    pub enabled: bool,
    pub dir: PathBuf,
    pub descriptor: Option<ModDescriptor>,
    pub index: Option<ModIndex>,
    pub missing: bool,
}

/// Mods of `playset` in `position` order. Only enabled mods with an existing folder are indexed.
pub fn load_playset(
    db: &LauncherDb,
    playset: &Playset,
    lang: &str,
) -> Result<Vec<ModEntry>, CoreError> {
    load_playset_with_progress(db, playset, lang, &|_, _| {})
}

/// Like `load_playset`; `progress(done, total)` is called after each mod, in any order
/// (mods load in parallel).
pub fn load_playset_with_progress(
    db: &LauncherDb,
    playset: &Playset,
    lang: &str,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<Vec<ModEntry>, CoreError> {
    let mods = db.playset_mods(&playset.id)?;
    let total = mods.len();
    let done = AtomicUsize::new(0);
    Ok(mods
        .par_iter()
        .map(|m| {
            let entry = load_mod(m, lang);
            progress(done.fetch_add(1, Ordering::Relaxed) + 1, total);
            entry
        })
        .collect())
}

fn load_mod(m: &PlaysetMod, lang: &str) -> ModEntry {
    let missing = m.dir_path.as_os_str().is_empty() || !m.dir_path.is_dir();
    let mut entry = ModEntry {
        id: m.mod_id.clone(),
        name: m.display_name.clone(),
        steam_id: m.steam_id.clone(),
        position: m.position,
        enabled: m.enabled,
        dir: m.dir_path.clone(),
        descriptor: None,
        index: None,
        missing,
    };
    if missing {
        return entry;
    }
    let descriptor_error = match read_descriptor(&m.dir_path) {
        Ok(d) => {
            entry.name = d.name.clone();
            entry.descriptor = Some(d);
            None
        }
        Err(e) => Some(e),
    };
    if m.enabled {
        let mut index = match index_mod(&m.dir_path, lang) {
            Ok(index) => index,
            Err(e) => {
                entry.index = Some(ModIndex {
                    warnings: vec![IndexWarning {
                        file: String::new(),
                        message: e.to_string(),
                    }],
                    ..ModIndex::default()
                });
                return entry;
            }
        };
        if let Some(e) = descriptor_error {
            index.warnings.push(IndexWarning {
                file: "descriptor.mod".into(),
                message: e.to_string(),
            });
            index.warnings.sort();
        }
        entry.index = Some(index);
    }
    entry
}

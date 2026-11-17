//! Loads a launcher playset into `ModEntry`s: descriptor plus index per enabled mod.

use crate::descriptor::{ModDescriptor, read_descriptor};
use crate::error::CoreError;
use crate::index::{IndexWarning, ModIndex, index_mod};
use crate::launcher::{LauncherDb, Playset, PlaysetMod};
use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;

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
    let mods = db.playset_mods(&playset.id)?;
    mods.par_iter().map(|m| load_mod(m, lang)).collect()
}

fn load_mod(m: &PlaysetMod, lang: &str) -> Result<ModEntry, CoreError> {
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
        return Ok(entry);
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
        let mut index = index_mod(&m.dir_path, lang)?;
        if let Some(e) = descriptor_error {
            index.warnings.push(IndexWarning {
                file: "descriptor.mod".into(),
                message: e.to_string(),
            });
            index.warnings.sort();
        }
        entry.index = Some(index);
    }
    Ok(entry)
}

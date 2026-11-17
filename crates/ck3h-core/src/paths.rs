use std::path::{Path, PathBuf};

const USER_DIR_SUFFIX: &str = "Paradox Interactive/Crusader Kings III";

/// Default CK3 user directory for this platform, if the base directory is known.
pub fn default_user_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "linux") {
        dirs::data_dir()
    } else {
        dirs::document_dir()
    };
    base.map(|b| b.join(USER_DIR_SUFFIX))
}

/// `candidate` if it looks like a CK3 `game` folder (has `common/`).
pub fn game_dir_if_valid(candidate: &Path) -> Option<PathBuf> {
    candidate
        .join("common")
        .is_dir()
        .then(|| candidate.to_path_buf())
}

/// The game folder in the default Steam library of this platform, if it exists.
pub fn default_game_dir() -> Option<PathBuf> {
    let steam_game = "steamapps/common/Crusader Kings III/game";
    let candidate = if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\Program Files (x86)\Steam").join(steam_game)
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()?
            .join("Library/Application Support/Steam")
            .join(steam_game)
    } else {
        dirs::home_dir()?
            .join(".local/share/Steam")
            .join(steam_game)
    };
    game_dir_if_valid(&candidate)
}

/// Game language (e.g. `l_russian`) from the `"language"` block of `pdx_settings.txt`.
pub fn game_language(user_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(user_dir.join("pdx_settings.txt")).ok()?;
    let (text, _) = crate::text::decode(&bytes);
    let mut lines = text
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("\"language\""));
    lines.next()?;
    for line in lines {
        let line = line.trim();
        if line.starts_with('}') {
            return None;
        }
        if let Some(rest) = line.strip_prefix("value") {
            let value = rest.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_language_reads_pdx_settings() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pdx_settings.txt"),
            "\"language\"={\n\tversion=0\n\tvalue=\"l_russian\"\n}\n",
        )
        .unwrap();
        assert_eq!(game_language(dir.path()), Some("l_russian".to_string()));
    }

    #[test]
    fn game_dir_requires_common_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(game_dir_if_valid(dir.path()), None);
        std::fs::create_dir(dir.path().join("common")).unwrap();
        assert_eq!(
            game_dir_if_valid(dir.path()),
            Some(dir.path().to_path_buf())
        );
    }

    #[test]
    fn game_language_missing_file_is_none() {
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(game_language(empty.path()), None);
    }
}

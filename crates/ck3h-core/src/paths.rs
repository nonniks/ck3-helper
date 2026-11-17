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
    fn game_language_missing_file_is_none() {
        let empty = tempfile::tempdir().unwrap();
        assert_eq!(game_language(empty.path()), None);
    }
}

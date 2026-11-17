use crate::error::CoreError;
use crate::text::decode;
use jomini::TextTape;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModDescriptor {
    pub name: String,
    pub version: Option<String>,
    pub supported_version: Option<String>,
    pub dependencies: Vec<String>,
    pub replace_path: Vec<String>,
    pub remote_file_id: Option<String>,
    /// Present only in `.mod` files of the `mod/` folder.
    pub path: Option<String>,
}

pub fn parse_descriptor(bytes: &[u8]) -> Result<ModDescriptor, CoreError> {
    let (text, _) = decode(bytes);
    let tape =
        TextTape::from_slice(text.as_bytes()).map_err(|e| CoreError::Descriptor(e.to_string()))?;
    let mut d = ModDescriptor::default();
    let mut name = None;
    for (key, _op, value) in tape.utf8_reader().fields() {
        match key.read_str().as_ref() {
            "name" => name = value.read_string().ok(),
            "version" => d.version = value.read_string().ok(),
            "supported_version" => d.supported_version = value.read_string().ok(),
            "remote_file_id" => d.remote_file_id = value.read_string().ok(),
            "path" => d.path = value.read_string().ok(),
            "replace_path" => d.replace_path.extend(value.read_string().ok()),
            "dependencies" => {
                if let Ok(items) = value.read_array() {
                    d.dependencies
                        .extend(items.values().filter_map(|v| v.read_string().ok()));
                }
            }
            _ => {}
        }
    }
    d.name = name.ok_or_else(|| CoreError::Descriptor("missing name".into()))?;
    Ok(d)
}

pub fn read_descriptor(mod_dir: &Path) -> Result<ModDescriptor, CoreError> {
    parse_descriptor(&std::fs::read(mod_dir.join("descriptor.mod"))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_version_and_dependencies() {
        let d = parse_descriptor(b"version=\"1.2.0\"\ntags={\n\t\"Graphics\"\n}\nname=\"Compat\"\ndependencies={\n\t\"Medieval Arts\"\n\t\"Simple Holding Sprawl\"\n}\nsupported_version=\"1.20.0.4\"\nremote_file_id=\"3816257844\"").unwrap();
        assert_eq!(d.name, "Compat");
        assert_eq!(d.version.as_deref(), Some("1.2.0"));
        assert_eq!(d.dependencies, ["Medieval Arts", "Simple Holding Sprawl"]);
        assert_eq!(d.supported_version.as_deref(), Some("1.20.0.4"));
        assert_eq!(d.remote_file_id.as_deref(), Some("3816257844"));
        assert_eq!(d.path, None);
    }

    #[test]
    fn collects_repeated_replace_path() {
        let d = parse_descriptor(
            b"name=\"X\"\nreplace_path=\"history/characters\"\nreplace_path=\"common/landed_titles\"",
        )
        .unwrap();
        assert_eq!(
            d.replace_path,
            ["history/characters", "common/landed_titles"]
        );
    }

    #[test]
    fn reads_path_from_dot_mod_file() {
        assert_eq!(
            parse_descriptor(b"name=\"X\"\npath=\"/a/b\"")
                .unwrap()
                .path
                .as_deref(),
            Some("/a/b")
        );
    }

    #[test]
    fn bom_and_cyrillic_name() {
        assert_eq!(
            parse_descriptor("\u{FEFF}name=\"Аутентичные шрифты\"".as_bytes())
                .unwrap()
                .name,
            "Аутентичные шрифты"
        );
    }

    #[test]
    fn missing_name_is_error() {
        assert!(matches!(
            parse_descriptor(b"version=\"1\""),
            Err(CoreError::Descriptor(m)) if m == "missing name"
        ));
    }

    #[test]
    fn empty_dependencies_and_unknown_keys() {
        let d = parse_descriptor(b"name=\"X\"\ndependencies={}\nfoo={ a=1 }").unwrap();
        assert!(d.dependencies.is_empty());
    }

    #[test]
    fn broken_braces_do_not_panic() {
        assert!(parse_descriptor(b"name=\"X\"\ntags={").is_err());
    }

    #[test]
    fn read_descriptor_reads_file_and_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("descriptor.mod"), b"name=\"Y\"").unwrap();
        assert_eq!(read_descriptor(dir.path()).unwrap().name, "Y");
        let empty = tempfile::tempdir().unwrap();
        assert!(matches!(
            read_descriptor(empty.path()),
            Err(CoreError::Io(_))
        ));
    }
}

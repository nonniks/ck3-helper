use std::borrow::Cow;

/// Decodes file bytes as UTF-8, dropping a leading BOM.
/// The flag is true when invalid bytes were replaced with U+FFFD.
pub fn decode(bytes: &[u8]) -> (String, bool) {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    let had_invalid = matches!(text, Cow::Owned(_));
    (text.into_owned(), had_invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_strips_bom() {
        assert_eq!(
            decode(b"\xEF\xBB\xBFl_english:"),
            ("l_english:".to_string(), false)
        );
    }

    #[test]
    fn decode_flags_invalid_bytes() {
        assert_eq!(decode(b"a\xFFb"), ("a\u{FFFD}b".to_string(), true));
    }
}

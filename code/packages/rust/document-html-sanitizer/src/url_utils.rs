//! URL scheme checking for HTML attribute values.
//!
//! This is an intentionally standalone copy of URL scheme extraction logic —
//! it has **no dependency on document-ast**. The HTML sanitizer is designed
//! to work on any HTML string without knowing anything about the document
//! model it came from.
//!
//! The algorithm mirrors the one in the AST sanitizer:
//!
//! 1. Strip C0 control characters and zero-width characters (browser bypass tricks)
//! 2. Extract the scheme (everything before the first `:`)
//! 3. Check the lowercased scheme against the allowlist
//! 4. Relative URLs (no scheme) always pass through
//!
//! See `coding_adventures_document_ast_sanitizer::url_utils` for the detailed
//! rationale and references.

/// Strip C0 control characters (U+0000–U+001F) and zero-width invisible
/// characters from a URL string before scheme extraction.
///
/// ```rust
/// use coding_adventures_document_html_sanitizer::url_utils::strip_control_chars;
///
/// assert_eq!(strip_control_chars("java\x00script:alert(1)"), "javascript:alert(1)");
/// assert_eq!(strip_control_chars("\u{200B}javascript:x"),    "javascript:x");
/// assert_eq!(strip_control_chars("https://ok.example.com"),  "https://ok.example.com");
/// ```
pub fn strip_control_chars(url: &str) -> String {
    url.chars()
        .filter(|&c| {
            let code = c as u32;
            !(code <= 0x1F
                || c == '\u{200B}'
                || c == '\u{200C}'
                || c == '\u{200D}'
                || c == '\u{2060}'
                || c == '\u{FEFF}')
        })
        .collect()
}

/// Check if the URL is permitted by the given scheme allowlist.
///
/// Returns `true` (safe) when:
/// - `allowed_schemes` is `None` (no restriction)
/// - The URL has no scheme (relative URL)
/// - The URL's scheme (case-insensitive) is in the allowlist
///
/// Returns `false` (blocked) when:
/// - The URL has an explicit scheme not in the allowlist
///
/// The function normalizes the URL first by stripping control characters and
/// zero-width characters to prevent bypass attacks.
///
/// ```rust
/// use coding_adventures_document_html_sanitizer::url_utils::is_scheme_allowed;
///
/// let schemes = Some(vec!["http".to_string(), "https".to_string()]);
///
/// assert!(is_scheme_allowed("https://example.com", &schemes));
/// assert!(is_scheme_allowed("../relative.png", &schemes));
/// assert!(!is_scheme_allowed("javascript:alert(1)", &schemes));
/// assert!(!is_scheme_allowed("java\x00script:alert(1)", &schemes)); // bypass blocked
/// assert!(is_scheme_allowed("javascript:alert(1)", &None));         // no restriction
/// ```
pub fn is_scheme_allowed(url: &str, allowed_schemes: &Option<Vec<String>>) -> bool {
    if allowed_schemes.is_none() {
        return true;
    }
    let schemes = allowed_schemes.as_ref().unwrap();

    let normalized = strip_control_chars(url);

    // Determine if the URL is relative
    let colon_pos = normalized.find(':');
    let slash_pos = normalized.find('/');
    let query_pos = normalized.find('?');

    let scheme = match colon_pos {
        None => return true, // no ':' → relative
        Some(c) => {
            let path_sep = match (slash_pos, query_pos) {
                (Some(s), Some(q)) => Some(s.min(q)),
                (Some(s), None) => Some(s),
                (None, Some(q)) => Some(q),
                (None, None) => None,
            };
            if let Some(sep) = path_sep {
                if sep < c {
                    return true; // '/' or '?' before ':' → relative
                }
            }
            &normalized[..c]
        }
    };

    let scheme_lower = scheme.to_lowercase();
    schemes.iter().any(|s| s.to_lowercase() == scheme_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_allowed() {
        let s = Some(vec!["https".to_string()]);
        assert!(is_scheme_allowed("https://example.com", &s));
    }

    #[test]
    fn javascript_blocked() {
        let s = Some(vec!["https".to_string()]);
        assert!(!is_scheme_allowed("javascript:alert(1)", &s));
    }

    #[test]
    fn javascript_uppercase_blocked() {
        let s = Some(vec!["https".to_string()]);
        assert!(!is_scheme_allowed("JAVASCRIPT:alert(1)", &s));
    }

    #[test]
    fn nul_bypass_blocked() {
        let s = Some(vec!["https".to_string()]);
        assert!(!is_scheme_allowed("java\x00script:alert(1)", &s));
    }

    #[test]
    fn relative_url_passes() {
        let s = Some(vec!["https".to_string()]);
        assert!(is_scheme_allowed("../image.png", &s));
        assert!(is_scheme_allowed("/about", &s));
    }

    #[test]
    fn none_allowlist_passes_all() {
        assert!(is_scheme_allowed("javascript:alert(1)", &None));
    }
}

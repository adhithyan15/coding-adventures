//! URL scheme extraction and security normalization.
//!
//! Before checking a URL's scheme we must strip characters that browsers
//! silently ignore but that would trick a naive scheme extractor. For example,
//! the string `java\x00script:alert(1)` contains a NUL byte (U+0000) between
//! "java" and "script". Most browsers strip control characters before parsing
//! the URL, so they see `javascript:alert(1)` and execute it. A sanitizer
//! that only looks for the literal string `"javascript:"` would miss it.
//!
//! # What we strip
//!
//! | Category            | Code points                                    |
//! |---------------------|------------------------------------------------|
//! | C0 control chars    | U+0000 – U+001F                                |
//! | Zero-width chars    | U+200B, U+200C, U+200D, U+2060, U+FEFF        |
//!
//! These are exactly the characters that browsers normalize away before
//! processing URL schemes (per WHATWG URL standard, section 4.1).
//!
//! # Relative URL pass-through
//!
//! Relative URLs (no `:` found, or `:` appears after `/` or `?`) always pass
//! through regardless of the scheme allowlist. A URL like `../images/cat.png`
//! or `/about` has no scheme and is safe by definition — it resolves against
//! the host document's origin.
//!
//! # Algorithm summary
//!
//! ```text
//! 1. strip_control_chars(url)    → normalized
//! 2. extract_scheme(normalized)  → Option<scheme>
//! 3. If None → relative URL → pass
//! 4. If Some(scheme) → check scheme.to_lowercase() against allowlist
//! ```

/// Strip C0 control characters (U+0000–U+001F) and zero-width invisible
/// characters from a URL string.
///
/// Browsers silently ignore these characters during URL parsing, which enables
/// bypass attacks like `java\x00script:alert(1)`. We normalize them away first
/// so the scheme extractor sees the same string the browser would.
///
/// # Examples
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::url_utils::strip_control_chars;
///
/// assert_eq!(strip_control_chars("java\x00script:alert(1)"), "javascript:alert(1)");
/// assert_eq!(strip_control_chars("java\rscript:x"),          "javascript:x");
/// assert_eq!(strip_control_chars("\u{200B}javascript:x"),    "javascript:x");
/// assert_eq!(strip_control_chars("https://example.com"),     "https://example.com");
/// ```
pub fn strip_control_chars(url: &str) -> String {
    // U+200B ZERO WIDTH SPACE
    // U+200C ZERO WIDTH NON-JOINER
    // U+200D ZERO WIDTH JOINER
    // U+2060 WORD JOINER
    // U+FEFF ZERO WIDTH NO-BREAK SPACE (BOM)
    url.chars()
        .filter(|&c| {
            let code = c as u32;
            // Keep character unless it is a C0 control or a known zero-width char
            !(code <= 0x1F
                || c == '\u{200B}'
                || c == '\u{200C}'
                || c == '\u{200D}'
                || c == '\u{2060}'
                || c == '\u{FEFF}')
        })
        .collect()
}

/// Extract the URL scheme from a normalized (already stripped) URL string.
///
/// Returns `Some(scheme)` if a scheme is present, or `None` if the URL is
/// relative. The returned scheme is the raw string slice — the caller is
/// responsible for case-folding before comparison.
///
/// # What makes a URL relative?
///
/// A URL is considered relative (no scheme) when:
/// - No `:` is found at all
/// - The first `:` appears after a `/` or `?` (path separator comes first)
///
/// The second condition handles URLs like `/path?key=value:thing` where the
/// `:` is part of a query string value, not a scheme separator.
///
/// # Examples
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::url_utils::extract_scheme;
///
/// assert_eq!(extract_scheme("https://example.com"), Some("https"));
/// assert_eq!(extract_scheme("javascript:alert(1)"), Some("javascript"));
/// assert_eq!(extract_scheme("mailto:user@example.com"), Some("mailto"));
/// assert_eq!(extract_scheme("../images/cat.png"), None);  // relative
/// assert_eq!(extract_scheme("/about"),            None);  // relative
/// assert_eq!(extract_scheme(""),                  None);  // empty → relative
/// assert_eq!(extract_scheme("/path?a=b:c"),       None);  // ':' after '/'
/// ```
pub fn extract_scheme(url: &str) -> Option<&str> {
    // Find the positions of the first ':', '/', and '?'
    let colon_pos = url.find(':');
    let slash_pos = url.find('/');
    let query_pos = url.find('?');

    match colon_pos {
        None => None, // No ':' at all — definitely relative
        Some(c) => {
            // If a '/' or '?' comes before the ':', the ':' is not a scheme
            // separator — it is part of the path or query string.
            let path_sep = match (slash_pos, query_pos) {
                (Some(s), Some(q)) => Some(s.min(q)),
                (Some(s), None) => Some(s),
                (None, Some(q)) => Some(q),
                (None, None) => None,
            };
            if let Some(sep) = path_sep {
                if sep < c {
                    return None; // path separator comes before ':', so relative URL
                }
            }
            Some(&url[..c])
        }
    }
}

/// Returns `true` if the URL is safe given the provided scheme allowlist.
///
/// Decision logic:
///
/// ```text
/// allowed_schemes = None    → pass (all schemes permitted)
/// URL is relative           → pass (no scheme, always safe)
/// scheme (lowercased) in allowlist → pass
/// otherwise                 → block (return false)
/// ```
///
/// This function expects a **raw** URL; it internally calls
/// `strip_control_chars` before extracting the scheme. This means callers
/// do not need to pre-normalize.
///
/// # Examples
///
/// ```rust
/// use coding_adventures_document_ast_sanitizer::url_utils::is_scheme_allowed;
///
/// let schemes = Some(vec!["http".to_string(), "https".to_string(), "mailto".to_string()]);
///
/// assert!(is_scheme_allowed("https://example.com", &schemes));
/// assert!(is_scheme_allowed("mailto:user@example.com", &schemes));
/// assert!(is_scheme_allowed("../relative/path", &schemes));         // relative → pass
/// assert!(!is_scheme_allowed("javascript:alert(1)", &schemes));
/// assert!(!is_scheme_allowed("java\x00script:alert(1)", &schemes)); // bypass attempt → blocked
/// assert!(!is_scheme_allowed("data:text/html,<h1>x</h1>", &schemes));
///
/// // None allowlist — all schemes pass
/// assert!(is_scheme_allowed("javascript:alert(1)", &None));
/// ```
pub fn is_scheme_allowed(url: &str, allowed_schemes: &Option<Vec<String>>) -> bool {
    // If no restrictions, everything passes
    if allowed_schemes.is_none() {
        return true;
    }
    let schemes = allowed_schemes.as_ref().unwrap();

    // Normalize away control characters and zero-width chars
    let normalized = strip_control_chars(url);

    // Relative URLs always pass — they can't carry a dangerous scheme
    let scheme = match extract_scheme(&normalized) {
        None => return true,
        Some(s) => s,
    };

    // Case-insensitive comparison: `JAVASCRIPT:` must be blocked same as `javascript:`
    let scheme_lower = scheme.to_lowercase();
    schemes.iter().any(|s| s.to_lowercase() == scheme_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── strip_control_chars ─────────────────────────────────────────────────

    #[test]
    fn strip_nul_byte() {
        assert_eq!(strip_control_chars("java\x00script:x"), "javascript:x");
    }

    #[test]
    fn strip_cr_in_scheme() {
        assert_eq!(strip_control_chars("java\rscript:x"), "javascript:x");
    }

    #[test]
    fn strip_lf_in_scheme() {
        assert_eq!(strip_control_chars("java\nscript:x"), "javascript:x");
    }

    #[test]
    fn strip_zero_width_space() {
        // U+200B ZERO WIDTH SPACE
        assert_eq!(strip_control_chars("\u{200B}javascript:x"), "javascript:x");
    }

    #[test]
    fn strip_zero_width_joiner() {
        // U+200D
        assert_eq!(strip_control_chars("java\u{200D}script:x"), "javascript:x");
    }

    #[test]
    fn strip_bom() {
        // U+FEFF
        assert_eq!(strip_control_chars("\u{FEFF}https://example.com"), "https://example.com");
    }

    #[test]
    fn strip_does_not_alter_normal_url() {
        assert_eq!(
            strip_control_chars("https://example.com/path?q=1"),
            "https://example.com/path?q=1"
        );
    }

    // ─── extract_scheme ──────────────────────────────────────────────────────

    #[test]
    fn extract_https_scheme() {
        assert_eq!(extract_scheme("https://example.com"), Some("https"));
    }

    #[test]
    fn extract_javascript_scheme() {
        assert_eq!(extract_scheme("javascript:alert(1)"), Some("javascript"));
    }

    #[test]
    fn extract_mailto_scheme() {
        assert_eq!(extract_scheme("mailto:user@example.com"), Some("mailto"));
    }

    #[test]
    fn extract_data_scheme() {
        assert_eq!(extract_scheme("data:text/html,<h1>x</h1>"), Some("data"));
    }

    #[test]
    fn extract_relative_path_no_scheme() {
        assert_eq!(extract_scheme("../images/cat.png"), None);
    }

    #[test]
    fn extract_absolute_path_no_scheme() {
        assert_eq!(extract_scheme("/about"), None);
    }

    #[test]
    fn extract_empty_string_no_scheme() {
        assert_eq!(extract_scheme(""), None);
    }

    #[test]
    fn extract_colon_after_slash_is_relative() {
        // The ':' is in the query string value, not a scheme separator
        assert_eq!(extract_scheme("/path?a=b:c"), None);
    }

    #[test]
    fn extract_colon_after_query_is_relative() {
        assert_eq!(extract_scheme("?key=val:thing"), None);
    }

    // ─── is_scheme_allowed ───────────────────────────────────────────────────

    #[test]
    fn allowed_https_passes() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(is_scheme_allowed("https://example.com", &schemes));
    }

    #[test]
    fn allowed_mailto_passes() {
        let schemes = Some(vec!["mailto".to_string()]);
        assert!(is_scheme_allowed("mailto:user@example.com", &schemes));
    }

    #[test]
    fn javascript_blocked() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(!is_scheme_allowed("javascript:alert(1)", &schemes));
    }

    #[test]
    fn javascript_uppercase_blocked() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(!is_scheme_allowed("JAVASCRIPT:alert(1)", &schemes));
    }

    #[test]
    fn javascript_nul_bypass_blocked() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        // The NUL byte would be stripped by strip_control_chars → "javascript:..."
        assert!(!is_scheme_allowed("java\x00script:alert(1)", &schemes));
    }

    #[test]
    fn zero_width_bypass_blocked() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(!is_scheme_allowed("\u{200B}javascript:alert(1)", &schemes));
    }

    #[test]
    fn data_url_blocked_by_default() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(!is_scheme_allowed("data:text/html,<script>alert(1)</script>", &schemes));
    }

    #[test]
    fn blob_url_blocked() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(!is_scheme_allowed("blob:https://origin/uuid", &schemes));
    }

    #[test]
    fn vbscript_blocked() {
        let schemes = Some(vec!["http".to_string(), "https".to_string()]);
        assert!(!is_scheme_allowed("vbscript:MsgBox(1)", &schemes));
    }

    #[test]
    fn relative_url_always_passes() {
        let schemes = Some(vec!["https".to_string()]);
        assert!(is_scheme_allowed("../images/cat.png", &schemes));
        assert!(is_scheme_allowed("/about", &schemes));
        assert!(is_scheme_allowed("?q=1", &schemes));
    }

    #[test]
    fn none_allowlist_permits_everything() {
        assert!(is_scheme_allowed("javascript:alert(1)", &None));
        assert!(is_scheme_allowed("data:text/html,x", &None));
        assert!(is_scheme_allowed("vbscript:MsgBox(1)", &None));
    }
}

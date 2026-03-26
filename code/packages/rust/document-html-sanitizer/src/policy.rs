//! HTML sanitization policy types and named presets.
//!
//! An `HtmlSanitizationPolicy` controls every decision the HTML string
//! sanitizer makes when processing an opaque HTML string. Unlike the AST
//! sanitizer, which understands document semantics, this sanitizer operates
//! on raw HTML text using regex-based pattern matching.
//!
//! # Design choice: why a separate policy type?
//!
//! The `HtmlSanitizationPolicy` is intentionally distinct from the AST
//! sanitizer's `SanitizationPolicy` — the two sanitizers address different
//! concerns:
//!
//! | Feature         | AST Sanitizer         | HTML Sanitizer            |
//! |-----------------|-----------------------|---------------------------|
//! | Input           | Typed `DocumentNode`  | Opaque `&str` HTML string |
//! | Mechanism       | Tree walk             | Regex pattern matching    |
//! | Node types      | Exact match           | Tag name match            |
//! | Link targets    | Structured field      | Attribute value parsing   |
//!
//! # Named presets
//!
//! | Preset            | Use case                                    |
//! |-------------------|---------------------------------------------|
//! | `html_strict()`   | Untrusted HTML from external sources        |
//! | `html_relaxed()`  | Authenticated users / internal tools        |
//! | `html_passthrough()` | No sanitization (trusted HTML only)      |

/// Policy that controls what the HTML string sanitizer keeps or strips.
///
/// All fields have defaults matching `HTML_PASSTHROUGH` (no sanitization).
/// Use the named preset functions `html_strict()`, `html_relaxed()`, or
/// `html_passthrough()` as starting points.
///
/// # Design: `Vec<String>` fields
///
/// The `drop_elements` and `drop_attributes` fields use `Vec<String>` rather
/// than `&[&str]` because:
/// 1. Policies are meant to be owned and passed around independently
/// 2. Struct update syntax (`..html_strict()`) works cleanly with owned values
/// 3. The sanitizer is called infrequently — heap allocation is not a concern
#[derive(Debug, Clone, PartialEq)]
pub struct HtmlSanitizationPolicy {
    /// HTML element names (lowercase) to remove entirely, including all
    /// content between the opening and closing tags.
    ///
    /// Default (`html_strict()`): `["script","style","iframe","object","embed",
    /// "applet","form","input","button","select","textarea","noscript","meta",
    /// "link","base"]`
    ///
    /// # Why drop-with-content?
    ///
    /// For dangerous elements like `<script>` and `<style>`, simply stripping
    /// the tags while keeping the content would leave JavaScript or CSS text
    /// in the output — worse than the original! The entire element including
    /// its body is removed.
    pub drop_elements: Vec<String>,

    /// Attribute names (lowercase) stripped from every element.
    ///
    /// Additionally, all `on*` event handler attributes are **always** stripped
    /// regardless of this list — they are stripped by the core logic, not this
    /// field. Use this field for additional non-standard attributes you want
    /// removed.
    ///
    /// Default: `["srcdoc", "formaction"]` in `html_strict()`.
    pub drop_attributes: Vec<String>,

    /// Allowlist of URL schemes for `href` and `src` attributes.
    ///
    /// Attribute values whose scheme is not in this list are replaced with `""`.
    /// Relative URLs always pass. `None` means all schemes are permitted.
    ///
    /// Default (`html_strict()`): `["http", "https", "mailto"]`
    pub allowed_url_schemes: Option<Vec<String>>,

    /// If `true`, HTML comments (`<!-- … -->`) are stripped entirely.
    ///
    /// Comment stripping is important because some browsers can execute code
    /// inside IE conditional comments (`<!--[if IE]><script>…</script><![endif]-->`).
    ///
    /// Default: `true` in `html_strict()`.
    pub drop_comments: bool,

    /// If `true`, `style` attributes containing `expression(` (CSS expression
    /// injection) or `url(` with a non-http/https argument are stripped.
    ///
    /// The entire `style` attribute is removed rather than attempting to parse
    /// and modify CSS — partial CSS sanitization is error-prone.
    ///
    /// Default: `true` in `html_strict()`.
    pub sanitize_style_attributes: bool,
}

// ─── Named preset constructors ────────────────────────────────────────────────
//
// Functions rather than constants because `Vec` is not `const`.

/// Returns the HTML_STRICT policy — for untrusted HTML from external sources.
///
/// Drops the most dangerous HTML elements (script, style, iframe, form, etc.).
/// Strips all on* event handlers, srcdoc, and formaction attributes.
/// Only http, https, and mailto URLs are permitted in href/src.
/// HTML comments are stripped. CSS expression() injection is prevented.
///
/// ```rust
/// use coding_adventures_document_html_sanitizer::policy::html_strict;
///
/// let p = html_strict();
/// assert!(p.drop_elements.contains(&"script".to_string()));
/// assert!(p.drop_comments);
/// assert!(p.sanitize_style_attributes);
/// ```
pub fn html_strict() -> HtmlSanitizationPolicy {
    HtmlSanitizationPolicy {
        drop_elements: vec![
            "script".to_string(),
            "style".to_string(),
            "iframe".to_string(),
            "object".to_string(),
            "embed".to_string(),
            "applet".to_string(),
            "form".to_string(),
            "input".to_string(),
            "button".to_string(),
            "select".to_string(),
            "textarea".to_string(),
            "noscript".to_string(),
            "meta".to_string(),
            "link".to_string(),
            "base".to_string(),
        ],
        drop_attributes: vec!["srcdoc".to_string(), "formaction".to_string()],
        allowed_url_schemes: Some(vec![
            "http".to_string(),
            "https".to_string(),
            "mailto".to_string(),
        ]),
        drop_comments: true,
        sanitize_style_attributes: true,
    }
}

/// Returns the HTML_RELAXED policy — for authenticated users / internal tools.
///
/// Drops only the most dangerous elements (script, iframe, object, embed, applet).
/// Allows style, form, input, etc. Permits http, https, mailto, ftp URLs.
/// Preserves HTML comments. Sanitizes CSS expression() injection.
///
/// ```rust
/// use coding_adventures_document_html_sanitizer::policy::html_relaxed;
///
/// let p = html_relaxed();
/// assert!(!p.drop_elements.contains(&"style".to_string()));
/// assert!(!p.drop_comments);
/// assert!(p.sanitize_style_attributes);
/// ```
pub fn html_relaxed() -> HtmlSanitizationPolicy {
    HtmlSanitizationPolicy {
        drop_elements: vec![
            "script".to_string(),
            "iframe".to_string(),
            "object".to_string(),
            "embed".to_string(),
            "applet".to_string(),
        ],
        drop_attributes: vec![],
        allowed_url_schemes: Some(vec![
            "http".to_string(),
            "https".to_string(),
            "mailto".to_string(),
            "ftp".to_string(),
        ]),
        drop_comments: false,
        sanitize_style_attributes: true,
    }
}

/// Returns the HTML_PASSTHROUGH policy — no sanitization.
///
/// Everything passes through unchanged. Use only for fully trusted HTML
/// (internal documentation, static site generation, etc.).
///
/// ```rust
/// use coding_adventures_document_html_sanitizer::policy::html_passthrough;
///
/// let p = html_passthrough();
/// assert!(p.drop_elements.is_empty());
/// assert!(p.allowed_url_schemes.is_none());
/// assert!(!p.drop_comments);
/// assert!(!p.sanitize_style_attributes);
/// ```
pub fn html_passthrough() -> HtmlSanitizationPolicy {
    HtmlSanitizationPolicy {
        drop_elements: vec![],
        drop_attributes: vec![],
        allowed_url_schemes: None,
        drop_comments: false,
        sanitize_style_attributes: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_has_script_in_drop_elements() {
        let p = html_strict();
        assert!(p.drop_elements.contains(&"script".to_string()));
        assert!(p.drop_elements.contains(&"iframe".to_string()));
        assert!(p.drop_elements.contains(&"style".to_string()));
    }

    #[test]
    fn strict_drops_comments() {
        assert!(html_strict().drop_comments);
    }

    #[test]
    fn strict_sanitizes_style_attrs() {
        assert!(html_strict().sanitize_style_attributes);
    }

    #[test]
    fn relaxed_does_not_drop_style() {
        let p = html_relaxed();
        assert!(!p.drop_elements.contains(&"style".to_string()));
    }

    #[test]
    fn relaxed_preserves_comments() {
        assert!(!html_relaxed().drop_comments);
    }

    #[test]
    fn passthrough_empty_drop_list() {
        assert!(html_passthrough().drop_elements.is_empty());
    }

    #[test]
    fn passthrough_no_url_restriction() {
        assert!(html_passthrough().allowed_url_schemes.is_none());
    }
}

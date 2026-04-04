//! AsciiDoc convenience crate — parse AsciiDoc and render HTML in one call.
//!
//! This is the public-facing convenience crate that combines two constituent
//! crates into a simple one-function API:
//!
//!   - `parse(text)` — from `asciidoc-parser`
//!   - `to_html(doc)` — from `document-ast-to-html`
//!
//! ```text
//!   document-ast               ← format-agnostic types
//!       ↓ types                        ↓ types
//!   asciidoc-parser            document-ast-to-html
//!   parse(text) → Doc          to_html(doc) → String
//!       ↓ depends on both
//!   asciidoc                   ← you are here
//!   let html = to_html(&parse(text), &Default::default());
//! ```
//!
//! # Quick Start
//!
//! ```rust
//! use asciidoc::asciidoc_to_html;
//!
//! let html = asciidoc_to_html("= Hello\n\nWorld *with* bold.\n");
//! assert!(html.contains("<h1>Hello</h1>"));
//! assert!(html.contains("<strong>with</strong>"));
//! ```

// ─── Re-exports ───────────────────────────────────────────────────────────────

pub use asciidoc_parser::parse;
pub use document_ast_to_html::{to_html, RenderOptions};
pub use document_ast;

// ─── Pipeline convenience ─────────────────────────────────────────────────────

/// Parse an AsciiDoc string and render it to HTML in one call.
///
/// This is the most convenient entry point for AsciiDoc → HTML conversion.
/// It is equivalent to:
///
/// ```rust
/// use asciidoc_parser::parse;
/// use document_ast_to_html::{to_html, RenderOptions};
///
/// fn asciidoc_to_html(text: &str) -> String {
///     to_html(&parse(text), &Default::default())
/// }
/// ```
///
/// **Security notice**: Raw HTML passthrough blocks (`++++...++++`) are
/// rendered verbatim by default. If rendering **untrusted** AsciiDoc, use
/// [`asciidoc_to_html_safe`] or call `to_html` with
/// `RenderOptions { sanitize: true }`.
///
/// # Examples
///
/// ```rust
/// use asciidoc::asciidoc_to_html;
///
/// let html = asciidoc_to_html("= Hello\n\nWorld\n");
/// assert!(html.contains("<h1>Hello</h1>"));
/// assert!(html.contains("<p>World</p>"));
/// ```
pub fn asciidoc_to_html(text: &str) -> String {
    to_html(&parse(text), &Default::default())
}

/// Parse an AsciiDoc string and render HTML with raw HTML passthrough stripped.
///
/// This is the safe variant of [`asciidoc_to_html`] for **untrusted input**
/// (e.g. user-supplied content in a web application). All `RawBlockNode` and
/// `RawInlineNode` content is dropped — attackers cannot inject `<script>`
/// tags or other raw HTML through AsciiDoc passthrough blocks.
///
/// # Examples
///
/// ```rust
/// use asciidoc::asciidoc_to_html_safe;
///
/// let html = asciidoc_to_html_safe("++++\n<script>evil</script>\n++++\n\nSafe\n");
/// assert!(!html.contains("<script>"));
/// assert!(html.contains("Safe"));
/// ```
pub fn asciidoc_to_html_safe(text: &str) -> String {
    to_html(&parse(text), &RenderOptions { sanitize: true })
}

pub const VERSION: &str = "0.1.0";

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asciidoc_to_html_heading() {
        let html = asciidoc_to_html("= Hello\n");
        assert!(html.contains("<h1>"), "expected h1 in: {html}");
        assert!(html.contains("Hello"), "expected Hello in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_paragraph() {
        let html = asciidoc_to_html("Hello world\n");
        assert!(html.contains("<p>"), "expected p in: {html}");
        assert!(html.contains("Hello world"), "expected Hello world in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_strong() {
        // AsciiDoc *text* = <strong> (not <em>!)
        let html = asciidoc_to_html("Hello *world*\n");
        assert!(html.contains("<strong>world</strong>"), "expected strong in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_emphasis() {
        let html = asciidoc_to_html("Hello _world_\n");
        assert!(html.contains("<em>world</em>"), "expected em in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_code_block() {
        let html = asciidoc_to_html("[source,go]\n----\nfmt.Println()\n----\n");
        assert!(html.contains("<pre>"), "expected pre in: {html}");
        assert!(html.contains("fmt.Println"), "expected code in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_unordered_list() {
        let html = asciidoc_to_html("* foo\n* bar\n");
        assert!(html.contains("<ul>"), "expected ul in: {html}");
        assert!(html.contains("<li>"), "expected li in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_ordered_list() {
        let html = asciidoc_to_html(". alpha\n. beta\n");
        assert!(html.contains("<ol"), "expected ol in: {html}");
        assert!(html.contains("<li>"), "expected li in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_thematic_break() {
        let html = asciidoc_to_html("'''\n");
        assert!(html.contains("<hr"), "expected hr in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_passthrough() {
        let html = asciidoc_to_html("++++\n<div>raw</div>\n++++\n");
        assert!(html.contains("<div>raw</div>"), "expected raw div in: {html}");
    }

    #[test]
    fn test_asciidoc_to_html_safe_strips_passthrough() {
        let html = asciidoc_to_html_safe("++++\n<script>evil</script>\n++++\n\nSafe text\n");
        assert!(!html.contains("<script>"), "script should be stripped: {html}");
        assert!(html.contains("Safe text"), "safe text should be present: {html}");
    }

    #[test]
    fn test_parse_returns_document_node() {
        let doc = parse("= Title\n");
        assert!(!doc.children.is_empty());
    }

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}

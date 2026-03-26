//! GFM pipeline convenience crate.
//!
//! This is the public-facing convenience crate that combines two constituent
//! crates into a simple two-function API:
//!
//!   - `parse(markdown)` — from `gfm-parser`
//!   - `to_html(doc)`    — from `document-ast-to-html`
//!
//! ```text
//!   document-ast               ← format-agnostic types
//!       ↓ types                        ↓ types
//!   gfm-parser                 document-ast-to-html
//!   parse(markdown) → Doc      to_html(doc) → String
//!       ↓ depends on both
//!   gfm                        ← you are here
//!   let html = to_html(&parse(markdown), &Default::default());
//! ```
//!
//! Users who just want to convert Markdown to HTML:
//!
//! ```rust
//! use gfm::markdown_to_html;
//!
//! let html = markdown_to_html("# Hello\n\nWorld *with* emphasis.\n");
//! // → "<h1>Hello</h1>\n<p>World <em>with</em> emphasis.</p>\n"
//! ```
//!
//! Users who want to work with the AST directly, plug in a different renderer,
//! or build a Markdown → PDF pipeline should use the constituent crates:
//!
//! ```rust
//! use gfm_parser::parse;
//! use document_ast_to_html::{to_html, RenderOptions};
//!
//! let doc = parse("# Hello\n\nWorld\n");
//! let html = to_html(&doc, &Default::default());
//! ```

// ─── Re-exports from constituent crates ──────────────────────────────────────

pub use gfm_parser::parse;
pub use document_ast_to_html::{to_html, escape_html, sanitize_url, normalize_url_for_attr, RenderOptions};
pub use document_ast;

// ─── Pipeline convenience ─────────────────────────────────────────────────────

/// Parse a GitHub Flavored Markdown string and render it to HTML in one call.
///
/// This is the most convenient entry point for the common case of converting
/// Markdown to HTML. It is equivalent to:
///
/// ```rust
/// use gfm_parser::parse;
/// use document_ast_to_html::{to_html, RenderOptions};
///
/// fn markdown_to_html(markdown: &str) -> String {
///     to_html(&parse(markdown), &Default::default())
/// }
/// ```
///
/// **Security notice**: Raw HTML passthrough is enabled by default (required
/// for GFM spec compliance). If rendering **untrusted** Markdown, use
/// [`markdown_to_html_safe`] or call `to_html` directly with
/// `RenderOptions { sanitize: true }`.
///
/// # Examples
///
/// ```rust
/// use gfm::markdown_to_html;
///
/// let html = markdown_to_html("# Hello\n\nWorld\n");
/// assert_eq!(html, "<h1>Hello</h1>\n<p>World</p>\n");
/// ```
pub fn markdown_to_html(markdown: &str) -> String {
    to_html(&parse(markdown), &Default::default())
}

/// Parse a GitHub Flavored Markdown string and render it to HTML with raw HTML
/// stripped from the output.
///
/// This is the safe variant of [`markdown_to_html`] for **untrusted input**
/// (user-supplied content in web applications). All `RawBlockNode` and
/// `RawInlineNode` content is dropped — attackers cannot inject `<script>`
/// tags or other raw HTML through Markdown.
///
/// Equivalent to `to_html(&parse(markdown), &RenderOptions { sanitize: true })`.
///
/// # Examples
///
/// ```rust
/// use gfm::markdown_to_html_safe;
///
/// // Attacker tries to inject a script tag via raw HTML in Markdown:
/// let attacker_md = "<script>alert(1)</script>\n\n**bold**\n";
/// let html = markdown_to_html_safe(attacker_md);
/// // The script tag is stripped — only the bold text remains:
/// assert_eq!(html, "<p><strong>bold</strong></p>\n");
/// ```
pub fn markdown_to_html_safe(markdown: &str) -> String {
    to_html(&parse(markdown), &RenderOptions { sanitize: true })
}

// ─── Version ──────────────────────────────────────────────────────────────────

pub const VERSION: &str = "0.1.0";

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html_basic() {
        let html = markdown_to_html("# Hello\n\nWorld\n");
        assert_eq!(html, "<h1>Hello</h1>\n<p>World</p>\n");
    }

    #[test]
    fn test_markdown_to_html_emphasis() {
        let html = markdown_to_html("Hello *world*\n");
        assert_eq!(html, "<p>Hello <em>world</em></p>\n");
    }

    #[test]
    fn test_markdown_to_html_safe_strips_raw_html() {
        let html = markdown_to_html_safe("<script>alert(1)</script>\n\n**bold**\n");
        assert_eq!(html, "<p><strong>bold</strong></p>\n");
    }

    #[test]
    fn test_markdown_to_html_passthrough_raw_html() {
        let html = markdown_to_html("<div>raw</div>\n\npara\n");
        assert!(html.contains("<div>raw</div>"));
        assert!(html.contains("<p>para</p>"));
    }

    #[test]
    fn test_markdown_to_html_strikethrough() {
        let html = markdown_to_html("~~gone~~\n");
        assert_eq!(html, "<p><del>gone</del></p>\n");
    }

    #[test]
    fn test_markdown_to_html_task_list() {
        let html = markdown_to_html("- [x] done\n");
        assert_eq!(html, "<ul>\n<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> done</li>\n</ul>\n");
    }

    #[test]
    fn test_markdown_to_html_table() {
        let html = markdown_to_html("| A |\n| --- |\n| B |\n");
        assert_eq!(html, "<table>\n<thead>\n<tr>\n<th>A</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>B</td>\n</tr>\n</tbody>\n</table>\n");
    }

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}

//! Document AST → HTML Renderer
//!
//! Converts a Document AST (produced by any front-end parser) into an HTML
//! string. The renderer is a simple recursive tree walk — each node type maps
//! to HTML elements following the CommonMark spec HTML rendering rules.
//!
//! # Node mapping
//!
//! ```text
//! DocumentNode      → rendered children
//! HeadingNode       → <h1>…</h1> through <h6>…</h6>
//! ParagraphNode     → <p>…</p>  (omitted in tight list context)
//! CodeBlockNode     → <pre><code [class="language-X"]>…</code></pre>
//! BlockquoteNode    → <blockquote>\n…</blockquote>
//! ListNode          → <ul> or <ol [start="N"]>
//! ListItemNode      → <li>…</li>
//! ThematicBreakNode → <hr />
//! RawBlockNode      → verbatim if format="html", skipped otherwise
//!
//! TextNode          → HTML-escaped text
//! EmphasisNode      → <em>…</em>
//! StrongNode        → <strong>…</strong>
//! CodeSpanNode      → <code>…</code>
//! LinkNode          → <a href="…" [title="…"]>…</a>
//! ImageNode         → <img src="…" alt="…" [title="…"] />
//! AutolinkNode      → <a href="[mailto:]…">…</a>
//! RawInlineNode     → verbatim if format="html", skipped otherwise
//! HardBreakNode     → <br />\n
//! SoftBreakNode     → \n
//! ```
//!
//! # Tight vs Loose Lists
//!
//! A tight list suppresses `<p>` tags around paragraph content in list items:
//!
//! ```text
//! Tight:   <li>item text</li>
//! Loose:   <li><p>item text</p></li>
//! ```
//!
//! The `tight` flag on `ListNode` controls this.
//!
//! # Security
//!
//! - Text content and attribute values are HTML-escaped via [`escape_html`].
//! - `RawBlockNode` and `RawInlineNode` content is passed through verbatim when
//!   `format == "html"` — this is intentional and spec-required.
//! - Link and image URLs are sanitized to block dangerous schemes:
//!   `javascript:`, `vbscript:`, `data:`, `blob:`.
//! - Pass `sanitize: true` in [`RenderOptions`] when rendering untrusted Markdown.
//!
//! # Quick Start
//!
//! ```rust
//! use document_ast_to_html::to_html;
//! use document_ast::{DocumentNode, BlockNode, ParagraphNode, InlineNode, TextNode};
//!
//! let doc = DocumentNode {
//!     children: vec![
//!         BlockNode::Paragraph(ParagraphNode {
//!             children: vec![InlineNode::Text(TextNode { value: "Hello".into() })],
//!         }),
//!     ],
//! };
//! assert_eq!(to_html(&doc, &Default::default()), "<p>Hello</p>\n");
//! ```

use document_ast::{
    AutolinkNode, BlockNode, BlockquoteNode, CodeBlockNode, CodeSpanNode, DocumentNode,
    EmphasisNode, HeadingNode, ImageNode, InlineNode, LinkNode, ListChildNode, ListItemNode,
    ListNode, ParagraphNode, RawBlockNode, RawInlineNode, StrikethroughNode, StrongNode,
    TableAlignment, TableCellNode, TableNode, TableRowNode, TaskItemNode, TextNode,
};

// ─── Render Options ───────────────────────────────────────────────────────────

/// Options for [`to_html`].
///
/// # Security
///
/// When rendering **untrusted Markdown** (user-supplied content), set
/// `sanitize: true` to strip all raw HTML (`RawBlockNode`, `RawInlineNode`)
/// from the output. Without this, an attacker who controls the Markdown source
/// can inject arbitrary HTML — including `<script>` tags — into the rendered
/// page.
///
/// Raw HTML passthrough is enabled by default for CommonMark spec compliance.
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// When `true`, all `RawBlockNode` and `RawInlineNode` nodes are dropped
    /// from the output (their `value` is not emitted, regardless of `format`).
    ///
    /// Set to `true` for untrusted Markdown (e.g. user-supplied content in a
    /// web application).
    ///
    /// Default: `false` (raw HTML passes through verbatim — spec-compliant).
    pub sanitize: bool,
}

// ─── Public Entry Point ────────────────────────────────────────────────────────

/// Render a Document AST to an HTML string.
///
/// The input is a `DocumentNode` as produced by any front-end parser that
/// implements the Document AST spec (TE00). The output is a valid HTML fragment.
///
/// **Security notice**: Raw HTML passthrough is enabled by default (required for
/// CommonMark spec compliance). If you render **untrusted** Markdown, pass
/// `RenderOptions { sanitize: true }` to strip all raw HTML from the output.
///
/// # Examples
///
/// ```rust
/// use document_ast_to_html::{to_html, RenderOptions};
/// use document_ast::{DocumentNode, BlockNode, ParagraphNode, InlineNode, TextNode};
///
/// let doc = DocumentNode {
///     children: vec![
///         BlockNode::Paragraph(ParagraphNode {
///             children: vec![InlineNode::Text(TextNode { value: "Hello world".into() })],
///         }),
///     ],
/// };
/// let html = to_html(&doc, &Default::default());
/// assert_eq!(html, "<p>Hello world</p>\n");
/// ```
pub fn to_html(document: &DocumentNode, options: &RenderOptions) -> String {
    render_blocks(&document.children, false, options)
}

// ─── Block Rendering ──────────────────────────────────────────────────────────

fn render_blocks(blocks: &[BlockNode], tight: bool, options: &RenderOptions) -> String {
    blocks
        .iter()
        .map(|b| render_block(b, tight, options))
        .collect()
}

fn render_block(block: &BlockNode, tight: bool, options: &RenderOptions) -> String {
    match block {
        BlockNode::Document(d) => render_blocks(&d.children, false, options),
        BlockNode::Heading(h) => render_heading(h, options),
        BlockNode::Paragraph(p) => render_paragraph(p, tight, options),
        BlockNode::CodeBlock(c) => render_code_block(c),
        BlockNode::Blockquote(b) => render_blockquote(b, options),
        BlockNode::List(l) => render_list(l, options),
        BlockNode::ListItem(item) => render_list_item(item, false, options),
        BlockNode::TaskItem(item) => render_task_item(item, false, options),
        BlockNode::ThematicBreak(_) => "<hr />\n".to_string(),
        BlockNode::RawBlock(r) => render_raw_block(r, options),
        BlockNode::Table(t) => render_table(t, options),
        BlockNode::TableRow(r) => render_table_row(r, &[], options),
        BlockNode::TableCell(c) => render_table_cell(c, false, &TableAlignment::None, options),
    }
}

// ─── Block Node Renderers ─────────────────────────────────────────────────────

/// Render an ATX or setext heading.
///
/// ```text
/// HeadingNode { level: 1, children: [TextNode { value: "Hello" }] }
/// → <h1>Hello</h1>\n
/// ```
fn render_heading(node: &HeadingNode, options: &RenderOptions) -> String {
    let inner = render_inlines(&node.children, options);
    format!("<h{0}>{1}</h{0}>\n", node.level, inner)
}

/// Render a paragraph.
///
/// In tight list context, the `<p>` wrapper is omitted and only the inner
/// content is emitted (followed by a newline).
///
/// ```text
/// ParagraphNode → <p>Hello <em>world</em></p>\n
/// ParagraphNode (tight) → Hello <em>world</em>\n
/// ```
fn render_paragraph(node: &ParagraphNode, tight: bool, options: &RenderOptions) -> String {
    let inner = render_inlines(&node.children, options);
    if tight {
        format!("{}\n", inner)
    } else {
        format!("<p>{}</p>\n", inner)
    }
}

/// Render a fenced or indented code block.
///
/// The content is HTML-escaped but not Markdown-processed.
/// If the block has a language (info string), the `<code>` tag gets a
/// `class="language-<lang>"` attribute per CommonMark convention.
///
/// ```text
/// CodeBlockNode { language: Some("ts"), value: "const x = 1;\n" }
/// → <pre><code class="language-ts">const x = 1;
/// </code></pre>\n
/// ```
fn render_code_block(node: &CodeBlockNode) -> String {
    let escaped = escape_html(&node.value);
    match &node.language {
        Some(lang) => format!(
            "<pre><code class=\"language-{}\">{}</code></pre>\n",
            escape_html(lang),
            escaped
        ),
        None => format!("<pre><code>{}</code></pre>\n", escaped),
    }
}

/// Render a blockquote.
///
/// ```text
/// BlockquoteNode → <blockquote>\n<p>…</p>\n</blockquote>\n
/// ```
fn render_blockquote(node: &BlockquoteNode, options: &RenderOptions) -> String {
    let inner = render_blocks(&node.children, false, options);
    format!("<blockquote>\n{}</blockquote>\n", inner)
}

/// Render an ordered or unordered list.
///
/// Ordered lists with a start number other than 1 get a `start` attribute.
/// The `tight` flag is passed to each list item so `<p>` tags are omitted
/// in tight mode.
///
/// ```text
/// ListNode { ordered: false, tight: true }
/// → <ul>\n<li>item1</li>\n</ul>\n
///
/// ListNode { ordered: true, start: Some(3), tight: false }
/// → <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>\n
/// ```
fn render_list(node: &ListNode, options: &RenderOptions) -> String {
    let tag = if node.ordered { "ol" } else { "ul" };

    // Only emit `start` when the list is ordered, the start value is present,
    // and it differs from 1 (default). i64 is always a valid integer.
    let start_attr = if node.ordered {
        match node.start {
            Some(s) if s != 1 => format!(" start=\"{}\"", s),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let items: String = node
        .children
        .iter()
        .map(|item| render_list_child(item, node.tight, options))
        .collect();

    format!("<{0}{1}>\n{2}</{0}>\n", tag, start_attr, items)
}

fn render_list_child(node: &ListChildNode, tight: bool, options: &RenderOptions) -> String {
    match node {
        ListChildNode::ListItem(item) => render_list_item(item, tight, options),
        ListChildNode::TaskItem(item) => render_task_item(item, tight, options),
    }
}

/// Render a single list item.
///
/// Tight single-paragraph items: `<li>text</li>` (no `<p>` wrapper).
/// All other items (multiple blocks, non-paragraph first child):
///   `<li>\ncontent\n</li>`.
///
/// An empty item renders as `<li></li>`.
fn render_list_item(node: &ListItemNode, tight: bool, options: &RenderOptions) -> String {
    if node.children.is_empty() {
        return "<li></li>\n".to_string();
    }

    if tight {
        if let Some(BlockNode::Paragraph(first_para)) = node.children.first() {
            let first_content = render_inlines(&first_para.children, options);
            if node.children.len() == 1 {
                // Only one child — simple tight item
                return format!("<li>{}</li>\n", first_content);
            }
            // Multiple children: inline the first paragraph, then block-render the rest
            let rest = render_blocks(&node.children[1..], tight, options);
            return format!("<li>{}\n{}</li>\n", first_content, rest);
        }
    }

    // Loose or non-paragraph first child: block-level format with newlines.
    let inner = render_blocks(&node.children, tight, options);
    let last_child = node.children.last();
    if tight && matches!(last_child, Some(BlockNode::Paragraph(_))) && inner.ends_with('\n') {
        // Strip trailing \n so it is flush with </li>
        return format!("<li>\n{}</li>\n", &inner[..inner.len() - 1]);
    }
    format!("<li>\n{}</li>\n", inner)
}

fn render_task_item(node: &TaskItemNode, tight: bool, options: &RenderOptions) -> String {
    let checkbox = if node.checked {
        "<input type=\"checkbox\" disabled=\"\" checked=\"\" />"
    } else {
        "<input type=\"checkbox\" disabled=\"\" />"
    };

    if node.children.is_empty() {
        return format!("<li>{}</li>\n", checkbox);
    }

    if tight {
        if let Some(BlockNode::Paragraph(first_para)) = node.children.first() {
            let first_content = render_inlines(&first_para.children, options);
            let content = if first_content.is_empty() {
                checkbox.to_string()
            } else {
                format!("{} {}", checkbox, first_content)
            };
            if node.children.len() == 1 {
                return format!("<li>{}</li>\n", content);
            }
            let rest = render_blocks(&node.children[1..], tight, options);
            return format!("<li>{}\n{}</li>\n", content, rest);
        }
    }

    let inner = render_blocks(&node.children, tight, options);
    format!("<li>{}\n{}</li>\n", checkbox, inner)
}

/// Render a raw block node.
///
/// If `options.sanitize` is `true`, this node is **always skipped**.
/// Otherwise, if `format == "html"`, emit the raw value verbatim.
fn render_raw_block(node: &RawBlockNode, options: &RenderOptions) -> String {
    if options.sanitize {
        return String::new();
    }
    if node.format == "html" {
        return node.value.clone();
    }
    String::new()
}

fn render_table(node: &TableNode, options: &RenderOptions) -> String {
    let mut out = String::from("<table>\n");
    let header = node.children.iter().find(|row| row.is_header);
    let body_rows: Vec<&TableRowNode> = node.children.iter().filter(|row| !row.is_header).collect();

    if let Some(row) = header {
        out.push_str("<thead>\n");
        out.push_str(&render_table_row(row, &node.align, options));
        out.push_str("</thead>\n");
    }

    if !body_rows.is_empty() {
        out.push_str("<tbody>\n");
        for row in body_rows {
            out.push_str(&render_table_row(row, &node.align, options));
        }
        out.push_str("</tbody>\n");
    }

    out.push_str("</table>\n");
    out
}

fn render_table_row(
    node: &TableRowNode,
    align: &[TableAlignment],
    options: &RenderOptions,
) -> String {
    let mut out = String::from("<tr>\n");
    for (index, cell) in node.children.iter().enumerate() {
        let alignment = align.get(index).unwrap_or(&TableAlignment::None);
        out.push_str(&render_table_cell(cell, node.is_header, alignment, options));
    }
    out.push_str("</tr>\n");
    out
}

fn render_table_cell(
    node: &TableCellNode,
    header: bool,
    align: &TableAlignment,
    options: &RenderOptions,
) -> String {
    let tag = if header { "th" } else { "td" };
    let align_attr = match align {
        TableAlignment::Left => " align=\"left\"",
        TableAlignment::Right => " align=\"right\"",
        TableAlignment::Center => " align=\"center\"",
        TableAlignment::None => "",
    };
    format!(
        "<{tag}{align_attr}>{}</{tag}>\n",
        render_inlines(&node.children, options)
    )
}

// ─── Inline Rendering ─────────────────────────────────────────────────────────

fn render_inlines(nodes: &[InlineNode], options: &RenderOptions) -> String {
    nodes.iter().map(|n| render_inline(n, options)).collect()
}

fn render_inline(node: &InlineNode, options: &RenderOptions) -> String {
    match node {
        InlineNode::Text(t) => render_text(t),
        InlineNode::Emphasis(e) => render_emphasis(e, options),
        InlineNode::Strong(s) => render_strong(s, options),
        InlineNode::Strikethrough(s) => render_strikethrough(s, options),
        InlineNode::CodeSpan(c) => render_code_span(c),
        InlineNode::Link(l) => render_link(l, options),
        InlineNode::Image(img) => render_image(img),
        InlineNode::Autolink(a) => render_autolink(a),
        InlineNode::RawInline(r) => render_raw_inline(r, options),
        InlineNode::HardBreak(_) => "<br />\n".to_string(),
        // CommonMark spec §6.12: a soft line break renders as a newline,
        // which browsers collapse to a space. We emit "\n" per the spec.
        InlineNode::SoftBreak(_) => "\n".to_string(),
    }
}

// ─── Inline Node Renderers ────────────────────────────────────────────────────

/// Render plain text — HTML-escape special characters to prevent XSS.
///
/// The four characters with HTML significance in text content are encoded:
///   `&` → `&amp;`   `<` → `&lt;`   `>` → `&gt;`   `"` → `&quot;`
fn render_text(node: &TextNode) -> String {
    escape_html(&node.value)
}

fn render_emphasis(node: &EmphasisNode, options: &RenderOptions) -> String {
    format!("<em>{}</em>", render_inlines(&node.children, options))
}

fn render_strong(node: &StrongNode, options: &RenderOptions) -> String {
    format!(
        "<strong>{}</strong>",
        render_inlines(&node.children, options)
    )
}

fn render_strikethrough(node: &StrikethroughNode, options: &RenderOptions) -> String {
    format!("<del>{}</del>", render_inlines(&node.children, options))
}

/// Render a code span — content is HTML-escaped but not Markdown-processed.
fn render_code_span(node: &CodeSpanNode) -> String {
    format!("<code>{}</code>", escape_html(&node.value))
}

/// Render a raw inline node.
///
/// If `options.sanitize` is `true`, this node is always skipped.
/// Otherwise, if `format == "html"`, emit the raw value verbatim.
fn render_raw_inline(node: &RawInlineNode, options: &RenderOptions) -> String {
    if options.sanitize {
        return String::new();
    }
    if node.format == "html" {
        return node.value.clone();
    }
    String::new()
}

// ─── URL Sanitization ─────────────────────────────────────────────────────────
//
// CommonMark spec §C.3 intentionally leaves URL sanitization to the implementor.
// Without scheme filtering, user-controlled Markdown is vulnerable to XSS via
// `javascript:` and `data:` URIs — both are valid URL characters that
// HTML-escaping does not neutralize.
//
// We use a targeted blocklist of schemes that are execution-capable in browsers:
//
//   javascript:  — executes JS in the browser's origin
//   vbscript:    — executes VBScript (IE legacy, still blocked by practice)
//   data:        — can embed scripts as data:text/html or data:text/javascript
//   blob:        — same-origin blob URLs can execute scripts
//
// All other schemes (irc:, ftp:, mailto:, etc.) pass through unchanged.
// Relative URLs (no scheme) always pass through unchanged.
//
// Before scheme detection, control characters and invisible Unicode characters
// that some URL parsers silently strip are removed — this prevents bypass
// attempts like "java\rscript:".

/// Control characters that some URL parsers silently ignore.
///
/// Stripped:
///   U+0000–U+001F   C0 controls (includes TAB, LF, CR, etc.)
///   U+007F–U+009F   DEL + C1 controls
///   U+200B          ZERO WIDTH SPACE
///   U+200C          ZERO WIDTH NON-JOINER
///   U+200D          ZERO WIDTH JOINER
///   U+2060          WORD JOINER
///   U+FEFF          BOM / ZERO WIDTH NO-BREAK SPACE
fn strip_url_control_chars(url: &str) -> String {
    url.chars()
        .filter(|&c| {
            !('\u{0000}'..='\u{001F}').contains(&c)
                && !('\u{007F}'..='\u{009F}').contains(&c)
                && c != '\u{200B}'
                && c != '\u{200C}'
                && c != '\u{200D}'
                && c != '\u{2060}'
                && c != '\u{FEFF}'
        })
        .collect()
}

/// The set of dangerous URL schemes (case-insensitive prefix check).
const DANGEROUS_SCHEMES: &[&str] = &["javascript:", "vbscript:", "data:", "blob:"];

/// Sanitize a URL by stripping control characters and blocking dangerous schemes.
///
/// Returns the sanitized URL, or `""` if the URL uses an execution-capable
/// scheme (`javascript:`, `vbscript:`, `data:`, `blob:`).
///
/// # Examples
///
/// ```rust
/// use document_ast_to_html::sanitize_url;
/// assert_eq!(sanitize_url("https://example.com"), "https://example.com");
/// assert_eq!(sanitize_url("javascript:alert(1)"), "");
/// assert_eq!(sanitize_url("JAVASCRIPT:alert(1)"), "");
/// assert_eq!(sanitize_url("data:text/html,<h1>x</h1>"), "");
/// assert_eq!(sanitize_url("/relative/path"), "/relative/path");
/// ```
pub fn sanitize_url(url: &str) -> String {
    let stripped = strip_url_control_chars(url);
    let lower = stripped.to_lowercase();
    for scheme in DANGEROUS_SCHEMES {
        if lower.starts_with(scheme) {
            return String::new();
        }
    }
    stripped
}

/// Render an inline link.
///
/// ```text
/// LinkNode { destination: "https://x.com", title: Some("X"), children: […] }
/// → <a href="https://x.com" title="X">…</a>
/// ```
fn render_link(node: &LinkNode, options: &RenderOptions) -> String {
    let href = escape_html(&sanitize_url(&node.destination));
    let title_attr = match &node.title {
        Some(t) => format!(" title=\"{}\"", escape_html(t)),
        None => String::new(),
    };
    let inner = render_inlines(&node.children, options);
    format!("<a href=\"{}\"{}>{}</a>", href, title_attr, inner)
}

/// Render an inline image.
///
/// ```text
/// ImageNode { destination: "cat.png", alt: "a cat", title: None }
/// → <img src="cat.png" alt="a cat" />
/// ```
fn render_image(node: &ImageNode) -> String {
    let src = escape_html(&sanitize_url(&node.destination));
    let alt = escape_html(&node.alt);
    // title_attr is either empty or ` title="..."` (with a leading space).
    // The format string appends " />" (one space then />).
    // With title:    <img src="..." alt="..." title="..." />
    // Without title: <img src="..." alt="..." />
    let title_attr = match &node.title {
        Some(t) => format!(" title=\"{}\"", escape_html(t)),
        None => String::new(),
    };
    format!("<img src=\"{}\" alt=\"{}\"{} />", src, alt, title_attr)
}

/// Render an autolink.
///
/// For email autolinks, the `href` gets a `mailto:` prefix.
/// The link text is the raw address (HTML-escaped).
///
/// ```text
/// AutolinkNode { destination: "user@example.com", is_email: true }
/// → <a href="mailto:user@example.com">user@example.com</a>
///
/// AutolinkNode { destination: "https://example.com", is_email: false }
/// → <a href="https://example.com">https://example.com</a>
/// ```
fn render_autolink(node: &AutolinkNode) -> String {
    let dest = sanitize_url(&node.destination);
    let href = if node.is_email {
        format!("mailto:{}", escape_html(&dest))
    } else {
        // For URL autolinks, percent-encode characters that should not appear
        // unencoded in HTML href attributes (CommonMark spec §6.9).
        escape_html(&normalize_url_for_attr(&dest))
    };
    let text = escape_html(&node.destination);
    format!("<a href=\"{}\">{}</a>", href, text)
}

// ─── URL Normalization ─────────────────────────────────────────────────────────

/// Percent-encode URL characters that must be encoded in HTML href/src attributes.
///
/// This matches the TypeScript `normalizeUrl` function from the CommonMark
/// reference implementation. Characters NOT in the safe set are percent-encoded
/// as their UTF-8 byte sequences.
///
/// The safe set is: `[\w\-._~:/?#@!$&'()*+,;=%]`
/// (word chars, URL-safe punctuation, already-percent-encoded sequences)
///
/// This function is used for autolink destinations (where the parser stores the
/// raw URL text from `<url>` syntax) and any destination that may contain
/// non-URL-safe characters.
///
/// # Examples
///
/// ```rust
/// use document_ast_to_html::normalize_url_for_attr;
/// assert_eq!(normalize_url_for_attr("https://example.com?find=\\*"), "https://example.com?find=%5C*");
/// assert_eq!(normalize_url_for_attr("https://foo.bar.`baz"), "https://foo.bar.%60baz");
/// assert_eq!(normalize_url_for_attr("https://example.com"), "https://example.com");
/// ```
pub fn normalize_url_for_attr(url: &str) -> String {
    let mut result = String::with_capacity(url.len());
    for ch in url.chars() {
        if matches!(ch,
            'a'..='z' | 'A'..='Z' | '0'..='9' |
            '-' | '_' | '.' | '~' | ':' | '/' | '?' | '#' | '@' |
            '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' |
            ';' | '=' | '%'
        ) {
            result.push(ch);
        } else {
            // Percent-encode each byte of the UTF-8 encoding
            for byte in ch.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

// ─── HTML Escaping ─────────────────────────────────────────────────────────────

/// Escape HTML special characters in text content and attribute values.
///
/// The four characters with HTML significance are encoded:
///   `&` → `&amp;`   `<` → `&lt;`   `>` → `&gt;`   `"` → `&quot;`
///
/// Note: apostrophes (`'`) are **not** escaped because CommonMark's reference
/// implementation uses double-quoted attributes.
///
/// # Examples
///
/// ```rust
/// use document_ast_to_html::escape_html;
/// assert_eq!(escape_html("<script>alert('xss')</script>"), "&lt;script&gt;alert('xss')&lt;/script&gt;");
/// assert_eq!(escape_html("Tom & Jerry"), "Tom &amp; Jerry");
/// assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
/// ```
pub fn escape_html(text: &str) -> String {
    // Fast path: check if any escaping is needed
    if !text.contains(['&', '<', '>', '"']) {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(ch),
        }
    }
    result
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use document_ast::*;

    fn opts() -> RenderOptions {
        RenderOptions::default()
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("&"), "&amp;");
        assert_eq!(escape_html("<"), "&lt;");
        assert_eq!(escape_html(">"), "&gt;");
        assert_eq!(escape_html("\""), "&quot;");
        assert_eq!(escape_html("'"), "'"); // apostrophe not escaped
        assert_eq!(escape_html("plain text"), "plain text");
    }

    #[test]
    fn test_sanitize_url_safe() {
        assert_eq!(sanitize_url("https://example.com"), "https://example.com");
        assert_eq!(sanitize_url("/relative/path"), "/relative/path");
        assert_eq!(sanitize_url(""), "");
    }

    #[test]
    fn test_sanitize_url_dangerous() {
        assert_eq!(sanitize_url("javascript:alert(1)"), "");
        assert_eq!(sanitize_url("JAVASCRIPT:alert(1)"), "");
        assert_eq!(sanitize_url("Javascript:alert(1)"), "");
        assert_eq!(sanitize_url("vbscript:msgbox(1)"), "");
        assert_eq!(sanitize_url("data:text/html,<h1>x</h1>"), "");
        assert_eq!(sanitize_url("blob:https://example.com/uuid"), "");
    }

    #[test]
    fn test_paragraph() {
        let doc = DocumentNode {
            children: vec![BlockNode::Paragraph(ParagraphNode {
                children: vec![InlineNode::Text(TextNode {
                    value: "Hello".into(),
                })],
            })],
        };
        assert_eq!(to_html(&doc, &opts()), "<p>Hello</p>\n");
    }

    #[test]
    fn test_heading() {
        let doc = DocumentNode {
            children: vec![BlockNode::Heading(HeadingNode {
                level: 2,
                children: vec![InlineNode::Text(TextNode {
                    value: "Title".into(),
                })],
            })],
        };
        assert_eq!(to_html(&doc, &opts()), "<h2>Title</h2>\n");
    }

    #[test]
    fn test_code_block_with_language() {
        let doc = DocumentNode {
            children: vec![BlockNode::CodeBlock(CodeBlockNode {
                language: Some("rust".into()),
                value: "fn main() {}".into(),
            })],
        };
        assert_eq!(
            to_html(&doc, &opts()),
            "<pre><code class=\"language-rust\">fn main() {}</code></pre>\n"
        );
    }

    #[test]
    fn test_thematic_break() {
        let doc = DocumentNode {
            children: vec![BlockNode::ThematicBreak(ThematicBreakNode)],
        };
        assert_eq!(to_html(&doc, &opts()), "<hr />\n");
    }

    #[test]
    fn test_tight_list() {
        let doc = DocumentNode {
            children: vec![BlockNode::List(ListNode {
                ordered: false,
                start: None,
                tight: true,
                children: vec![
                    ListChildNode::ListItem(ListItemNode {
                        children: vec![BlockNode::Paragraph(ParagraphNode {
                            children: vec![InlineNode::Text(TextNode {
                                value: "item 1".into(),
                            })],
                        })],
                    }),
                    ListChildNode::ListItem(ListItemNode {
                        children: vec![BlockNode::Paragraph(ParagraphNode {
                            children: vec![InlineNode::Text(TextNode {
                                value: "item 2".into(),
                            })],
                        })],
                    }),
                ],
            })],
        };
        let html = to_html(&doc, &opts());
        assert_eq!(html, "<ul>\n<li>item 1</li>\n<li>item 2</li>\n</ul>\n");
    }

    #[test]
    fn test_loose_list() {
        let doc = DocumentNode {
            children: vec![BlockNode::List(ListNode {
                ordered: true,
                start: Some(1),
                tight: false,
                children: vec![ListChildNode::ListItem(ListItemNode {
                    children: vec![BlockNode::Paragraph(ParagraphNode {
                        children: vec![InlineNode::Text(TextNode {
                            value: "item".into(),
                        })],
                    })],
                })],
            })],
        };
        let html = to_html(&doc, &opts());
        assert_eq!(html, "<ol>\n<li>\n<p>item</p>\n</li>\n</ol>\n");
    }

    #[test]
    fn test_raw_block_passthrough() {
        let doc = DocumentNode {
            children: vec![BlockNode::RawBlock(RawBlockNode {
                format: "html".into(),
                value: "<div>raw</div>\n".into(),
            })],
        };
        assert_eq!(to_html(&doc, &opts()), "<div>raw</div>\n");
    }

    #[test]
    fn test_raw_block_sanitized() {
        let doc = DocumentNode {
            children: vec![BlockNode::RawBlock(RawBlockNode {
                format: "html".into(),
                value: "<script>alert(1)</script>\n".into(),
            })],
        };
        let opts = RenderOptions { sanitize: true };
        assert_eq!(to_html(&doc, &opts), "");
    }

    #[test]
    fn test_emphasis_strong() {
        let doc = DocumentNode {
            children: vec![BlockNode::Paragraph(ParagraphNode {
                children: vec![
                    InlineNode::Emphasis(EmphasisNode {
                        children: vec![InlineNode::Text(TextNode { value: "em".into() })],
                    }),
                    InlineNode::Text(TextNode {
                        value: " and ".into(),
                    }),
                    InlineNode::Strong(StrongNode {
                        children: vec![InlineNode::Text(TextNode {
                            value: "strong".into(),
                        })],
                    }),
                ],
            })],
        };
        assert_eq!(
            to_html(&doc, &opts()),
            "<p><em>em</em> and <strong>strong</strong></p>\n"
        );
    }

    #[test]
    fn test_strikethrough() {
        let doc = DocumentNode {
            children: vec![BlockNode::Paragraph(ParagraphNode {
                children: vec![InlineNode::Strikethrough(StrikethroughNode {
                    children: vec![InlineNode::Text(TextNode {
                        value: "gone".into(),
                    })],
                })],
            })],
        };
        assert_eq!(to_html(&doc, &opts()), "<p><del>gone</del></p>\n");
    }

    #[test]
    fn test_task_list() {
        let doc = DocumentNode {
            children: vec![BlockNode::List(ListNode {
                ordered: false,
                start: None,
                tight: true,
                children: vec![ListChildNode::TaskItem(TaskItemNode {
                    checked: true,
                    children: vec![BlockNode::Paragraph(ParagraphNode {
                        children: vec![InlineNode::Text(TextNode {
                            value: "done".into(),
                        })],
                    })],
                })],
            })],
        };
        assert_eq!(
            to_html(&doc, &opts()),
            "<ul>\n<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> done</li>\n</ul>\n"
        );
    }

    #[test]
    fn test_table() {
        let doc = DocumentNode {
            children: vec![BlockNode::Table(TableNode {
                align: vec![TableAlignment::Left],
                children: vec![
                    TableRowNode {
                        is_header: true,
                        children: vec![TableCellNode {
                            children: vec![InlineNode::Text(TextNode { value: "A".into() })],
                        }],
                    },
                    TableRowNode {
                        is_header: false,
                        children: vec![TableCellNode {
                            children: vec![InlineNode::Text(TextNode { value: "B".into() })],
                        }],
                    },
                ],
            })],
        };
        assert_eq!(
            to_html(&doc, &opts()),
            "<table>\n<thead>\n<tr>\n<th align=\"left\">A</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td align=\"left\">B</td>\n</tr>\n</tbody>\n</table>\n"
        );
    }

    #[test]
    fn test_link() {
        let doc = DocumentNode {
            children: vec![BlockNode::Paragraph(ParagraphNode {
                children: vec![InlineNode::Link(LinkNode {
                    destination: "https://example.com".into(),
                    title: Some("Example".into()),
                    children: vec![InlineNode::Text(TextNode {
                        value: "click".into(),
                    })],
                })],
            })],
        };
        assert_eq!(
            to_html(&doc, &opts()),
            "<p><a href=\"https://example.com\" title=\"Example\">click</a></p>\n"
        );
    }

    #[test]
    fn test_hard_break_soft_break() {
        let doc = DocumentNode {
            children: vec![BlockNode::Paragraph(ParagraphNode {
                children: vec![
                    InlineNode::Text(TextNode { value: "a".into() }),
                    InlineNode::HardBreak(HardBreakNode),
                    InlineNode::Text(TextNode { value: "b".into() }),
                    InlineNode::SoftBreak(SoftBreakNode),
                    InlineNode::Text(TextNode { value: "c".into() }),
                ],
            })],
        };
        assert_eq!(to_html(&doc, &opts()), "<p>a<br />\nb\nc</p>\n");
    }
}

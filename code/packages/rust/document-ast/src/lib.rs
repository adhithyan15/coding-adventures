//! Document AST — Format-Agnostic Intermediate Representation
//!
//! The Document AST is the "LLVM IR of documents". It sits between front-end
//! parsers (Markdown, RST, HTML, DOCX) and back-end renderers (HTML, PDF,
//! plain text, LaTeX). Every front-end produces this IR; every back-end
//! consumes it. With N front-ends and M back-ends, you only need N + M
//! implementations instead of N × M.
//!
//! ```text
//!     Markdown ────────────────────────────────► HTML
//!     reStructuredText ────► Document AST ────► PDF
//!     HTML ────────────────────────────────────► Plain text
//!     DOCX ────────────────────────────────────► DOCX
//! ```
//!
//! Spec: TE00 — Document AST
//!
//! # Design Principles
//!
//! 1. **Semantic, not notational** — nodes carry meaning, not syntax
//! 2. **Resolved, not deferred**   — all link references resolved before IR
//! 3. **Format-agnostic**          — `RawBlock`/`RawInline` carry a `format` tag
//! 4. **Cloneable and typed**      — all types implement `Clone`, `Debug`, `PartialEq`
//! 5. **Minimal and stable**       — only universal document concepts
//!
//! # Quick Start
//!
//! ```rust
//! use document_ast::{DocumentNode, BlockNode, InlineNode};
//!
//! // A simple heading + paragraph document
//! let doc = DocumentNode {
//!     children: vec![
//!         BlockNode::Heading(document_ast::HeadingNode {
//!             level: 1,
//!             children: vec![InlineNode::Text(document_ast::TextNode { value: "Hello".to_string() })],
//!         }),
//!     ],
//! };
//! assert_eq!(doc.children.len(), 1);
//! ```

// ─── Block Nodes ──────────────────────────────────────────────────────────────
//
// Block nodes form the structural skeleton of a document. They live at the
// top level of the document and can be nested (e.g. blockquotes, list items).

/// The root of every document produced by a front-end parser.
///
/// Every IR value is exactly one `DocumentNode`. An empty document has an
/// empty `children` vec. `DocumentNode` is the only node type that cannot
/// appear as a child of another node — it is always the root.
///
/// ```text
/// DocumentNode
///   ├── HeadingNode (level 1)
///   ├── ParagraphNode
///   └── ListNode (ordered, tight)
///         ├── ListItemNode
///         └── ListItemNode
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentNode {
    pub children: Vec<BlockNode>,
}

/// A section heading with a nesting depth. Semantically corresponds to
/// `<h1>`–`<h6>` in HTML, `=====` / `-----` underlines in RST,
/// `\section{}` / `\subsection{}` in LaTeX, and Heading 1–6 styles in DOCX.
///
/// Levels are in the range `1..=6`. Levels beyond 6 (if a source format
/// supports them) are clamped to 6.
///
/// ```text
/// HeadingNode { level: 2, children: [TextNode { value: "Hello" }] }
/// → <h2>Hello</h2>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingNode {
    /// Level 1 (most important) to 6 (least important).
    pub level: u8,
    pub children: Vec<InlineNode>,
}

/// A block of prose containing one or more inline nodes.
///
/// Paragraphs are the most common block type. Any content that is not more
/// specifically typed (heading, list, code block, etc.) becomes a paragraph.
///
/// ```text
/// ParagraphNode {
///   children: [
///     TextNode { value: "Hello " },
///     EmphasisNode { children: [TextNode { value: "world" }] },
///   ]
/// }
/// → <p>Hello <em>world</em></p>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphNode {
    pub children: Vec<InlineNode>,
}

/// A block of literal code or pre-formatted text.
///
/// The `value` is raw — it is NOT decoded for HTML entities and NOT processed
/// for inline markup. The `value` field always ends with `\n`. Back-ends
/// should not add extra newlines when rendering.
///
/// ```text
/// // Fenced code block:
/// // ```typescript
/// // const x = 1;
/// // ```
/// CodeBlockNode { language: Some("typescript"), value: "const x = 1;\n" }
/// → <pre><code class="language-typescript">const x = 1;
/// </code></pre>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlockNode {
    /// Syntax language hint, e.g. `"typescript"`, `"python"`. `None` when unknown.
    pub language: Option<String>,
    /// Raw source code, including the trailing newline. Never HTML-encoded.
    pub value: String,
}

/// A block of content set apart as a quotation or aside.
///
/// Can contain any block nodes, including nested blockquotes.
///
/// ```text
/// BlockquoteNode {
///   children: [ParagraphNode { children: [TextNode { value: "quote" }] }]
/// }
/// → <blockquote>\n<p>quote</p>\n</blockquote>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BlockquoteNode {
    pub children: Vec<BlockNode>,
}

/// An ordered (numbered) or unordered (bulleted) list.
///
/// A `ListNode` contains one or more `ListItemNode` children. Each list item
/// contains block-level content (paragraphs, nested lists, code blocks, etc.).
///
/// **Tight vs loose.** The `tight` flag is a rendering hint from the source.
/// A tight list is written without blank lines between items; a loose list has
/// blank lines. In HTML, tight lists suppress `<p>` wrappers around paragraph
/// content. Other back-ends may use this flag differently or ignore it.
///
/// **Ordered list start.** `start` records the opening item number. `Some(1)`
/// is the default; `Some(42)` means the list begins at forty-two. `None` for
/// unordered lists.
///
/// ```text
/// ListNode { ordered: false, start: None, tight: true, children: [...] }
/// → <ul>\n<li>item1</li>\n<li>item2</li>\n</ul>
///
/// ListNode { ordered: true, start: Some(3), tight: false, children: [...] }
/// → <ol start="3">\n<li><p>item1</p>\n</li>\n</ol>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ListNode {
    pub ordered: bool,
    /// Opening number for ordered lists. `None` for unordered lists.
    pub start: Option<i64>,
    /// Tight = no blank lines between items and no blank line inside any item.
    /// In HTML tight mode, paragraph content is rendered without `<p>` tags.
    pub tight: bool,
    pub children: Vec<ListItemNode>,
}

/// One item in a `ListNode`. Contains block-level content.
///
/// For tight lists the children are typically `ParagraphNode`s whose content
/// is rendered without wrapping `<p>` tags (the `tight` flag on the parent
/// `ListNode` controls this).
#[derive(Debug, Clone, PartialEq)]
pub struct ListItemNode {
    pub children: Vec<BlockNode>,
}

/// A visual separator between sections. No children.
///
/// In HTML renders as `<hr />`. In RST `----`. In plain text `---`.
#[derive(Debug, Clone, PartialEq)]
pub struct ThematicBreakNode;

/// A block of raw content to be passed through verbatim to a specific back-end.
///
/// The `format` field identifies the target renderer (e.g. `"html"`,
/// `"latex"`, `"rtf"`). Back-ends that do not recognise `format` **must**
/// skip this node silently.
///
/// **Generalisation of `HtmlBlockNode`.** The CommonMark AST has
/// `html_block`. The Document AST replaces it with
/// `RawBlockNode { format: "html" }`. The semantics are identical for HTML
/// output; the `format` tag extends the concept to any target format.
///
/// ```text
/// Back-end contract:
///   format matches output → emit value verbatim (no escaping)
///   format does not match → skip silently
///
/// format     HTML back-end    LaTeX back-end    plain-text
/// ─────────  ─────────────    ──────────────    ──────────
/// "html"     emit             skip              skip
/// "latex"    skip             emit              skip
/// "rtf"      skip             skip              skip
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RawBlockNode {
    /// Target back-end format tag, e.g. `"html"`, `"latex"`, `"rtf"`.
    pub format: String,
    /// Raw content — never HTML-encoded or otherwise processed.
    pub value: String,
}

/// Union of all block node types.
///
/// Use in `match node { BlockNode::Heading(h) => ..., ... }` for exhaustive
/// handling. `Document` is included so that the full tree can be recursively
/// traversed without special-casing the root type.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockNode {
    Document(DocumentNode),
    Heading(HeadingNode),
    Paragraph(ParagraphNode),
    CodeBlock(CodeBlockNode),
    Blockquote(BlockquoteNode),
    List(ListNode),
    ListItem(ListItemNode),
    ThematicBreak(ThematicBreakNode),
    RawBlock(RawBlockNode),
}

// ─── Inline Nodes ─────────────────────────────────────────────────────────────
//
// Inline nodes live inside block nodes that contain prose content: headings,
// paragraphs, and list items. They represent formatted text spans, links,
// images, and structural characters within a paragraph.

/// Plain text with no markup.
///
/// All HTML character references (`&amp;`, `&#65;`, `&#x41;`) are decoded into
/// their Unicode equivalents before being stored. The `value` field contains
/// the final, display-ready Unicode string.
///
/// Adjacent text nodes are automatically merged during inline parsing — a
/// well-formed IR never has two consecutive `TextNode` siblings.
///
/// ```text
/// "Hello &amp; world" → TextNode { value: "Hello & world" }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    /// Decoded Unicode string, ready for display. Never contains raw HTML entities.
    pub value: String,
}

/// Stressed emphasis. In HTML renders as `<em>`. In Markdown, `*text*` or
/// `_text_`. In RST, `:emphasis:`. In DOCX, italic text.
///
/// ```text
/// EmphasisNode { children: [TextNode { value: "hello" }] }
/// → <em>hello</em>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EmphasisNode {
    pub children: Vec<InlineNode>,
}

/// Strong importance. In HTML renders as `<strong>`. In Markdown, `**text**`
/// or `__text__`. In RST, `**bold**`. In DOCX, bold text.
///
/// ```text
/// StrongNode { children: [TextNode { value: "bold" }] }
/// → <strong>bold</strong>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StrongNode {
    pub children: Vec<InlineNode>,
}

/// Inline code. The value is raw — not decoded for HTML entities and not
/// processed for Markdown. Leading and trailing spaces are stripped when
/// the content is surrounded by spaces on both sides.
///
/// ```text
/// `const x = 1` → CodeSpanNode { value: "const x = 1" }
/// → <code>const x = 1</code>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CodeSpanNode {
    /// Raw code content, not decoded.
    pub value: String,
}

/// A hyperlink with resolved destination.
///
/// The `destination` is always a fully resolved URL — all reference
/// indirections have been resolved by the front-end. The IR never contains
/// unresolved reference links.
///
/// Links cannot be nested — a `LinkNode` cannot contain another `LinkNode`.
///
/// ```text
/// LinkNode {
///   destination: "https://example.com",
///   title: Some("Example"),
///   children: [TextNode { value: "click here" }]
/// }
/// → <a href="https://example.com" title="Example">click here</a>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LinkNode {
    /// Fully resolved URL. Never a `[label]` reference — always an explicit destination.
    pub destination: String,
    /// Optional tooltip / hover text. `None` if absent.
    pub title: Option<String>,
    pub children: Vec<InlineNode>,
}

/// An embedded image.
///
/// Like `LinkNode`, `destination` is always the fully resolved URL. The `alt`
/// field is the plain-text fallback description (all inline markup stripped).
///
/// **Alt text.** The `alt` field is a plain string (not inline nodes) because
/// alt text is by definition a plain-text description for screen readers and
/// fallback contexts. For example, `![**hello**](img.png)` produces
/// `ImageNode { alt: "hello", … }` — markup is stripped before storing.
///
/// ```text
/// ImageNode { destination: "cat.png", alt: "a cat", title: None }
/// → <img src="cat.png" alt="a cat" />
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImageNode {
    /// Fully resolved image URL.
    pub destination: String,
    /// Optional tooltip / hover text. `None` if absent.
    pub title: Option<String>,
    /// Plain-text alt description, markup stripped.
    pub alt: String,
}

/// A URL or email address presented as a direct link, without custom link text.
/// The link text in all back-ends is the raw address itself.
///
/// **Why preserve `is_email`?** Two reasons:
///
///   1. HTML back-ends need to prepend `mailto:` for email autolinks:
///      `<https://example.com>` → `<a href="https://example.com">…</a>` but
///      `<user@example.com>` → `<a href="mailto:user@example.com">…</a>`.
///
///   2. Other back-ends (PDF, DOCX) may format email addresses differently from
///      URLs — e.g. not underlining email addresses in print output.
///
/// ```text
/// AutolinkNode { destination: "user@example.com", is_email: true }
/// → <a href="mailto:user@example.com">user@example.com</a>
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AutolinkNode {
    /// The URL or email address, without the surrounding `< >`.
    pub destination: String,
    /// `true` for email autolinks; `false` for URL autolinks.
    pub is_email: bool,
}

/// An inline span of raw content to be passed through verbatim to a specific
/// back-end. The `format` field names the target renderer.
///
/// **Generalisation of `HtmlInlineNode`.** The CommonMark AST has
/// `html_inline`. The Document AST replaces it with
/// `RawInlineNode { format: "html" }`. The semantics are identical for HTML
/// output; the `format` tag extends the concept to any target.
///
/// ```text
/// RawInlineNode { format: "html", value: "<em>raw</em>" }
/// → (HTML back-end) <em>raw</em>
/// → (LaTeX back-end) (nothing)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RawInlineNode {
    /// Target back-end format tag, e.g. `"html"`, `"latex"`.
    pub format: String,
    /// Raw content — never escaped or processed.
    pub value: String,
}

/// A forced line break within a paragraph.
///
/// Forces `<br />` in HTML, `\newline` in LaTeX, a literal `\n` in plain-text
/// renderers. In Markdown, produced by two or more trailing spaces before a
/// newline, or a backslash `\` immediately before a newline.
#[derive(Debug, Clone, PartialEq)]
pub struct HardBreakNode;

/// A soft line break — a newline within a paragraph that is not a hard break.
///
/// In HTML, soft breaks render as `\n` (browsers collapse to a single space).
/// In plain text, they render as a literal newline. The back-end controls
/// the exact rendering.
///
/// The IR preserves soft breaks so that back-ends controlling line-wrapping
/// behaviour can make the right choice.
#[derive(Debug, Clone, PartialEq)]
pub struct SoftBreakNode;

/// Union of all inline node types.
///
/// Use in `match node { InlineNode::Text(t) => ..., ... }` for exhaustive
/// handling of all inline constructs.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    Text(TextNode),
    Emphasis(EmphasisNode),
    Strong(StrongNode),
    CodeSpan(CodeSpanNode),
    Link(LinkNode),
    Image(ImageNode),
    Autolink(AutolinkNode),
    RawInline(RawInlineNode),
    HardBreak(HardBreakNode),
    SoftBreak(SoftBreakNode),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_node_construction() {
        let doc = DocumentNode { children: vec![] };
        assert_eq!(doc.children.len(), 0);
    }

    #[test]
    fn test_heading_node() {
        let h = HeadingNode {
            level: 1,
            children: vec![InlineNode::Text(TextNode { value: "Hello".to_string() })],
        };
        assert_eq!(h.level, 1);
        assert_eq!(h.children.len(), 1);
    }

    #[test]
    fn test_paragraph_node() {
        let p = ParagraphNode {
            children: vec![InlineNode::Text(TextNode { value: "world".to_string() })],
        };
        if let InlineNode::Text(t) = &p.children[0] {
            assert_eq!(t.value, "world");
        }
    }

    #[test]
    fn test_code_block_no_language() {
        let cb = CodeBlockNode {
            language: None,
            value: "let x = 1;\n".to_string(),
        };
        assert!(cb.language.is_none());
        assert!(cb.value.ends_with('\n'));
    }

    #[test]
    fn test_raw_block_node() {
        let rb = RawBlockNode {
            format: "html".to_string(),
            value: "<div>test</div>\n".to_string(),
        };
        assert_eq!(rb.format, "html");
    }

    #[test]
    fn test_block_node_enum_variants() {
        let doc = BlockNode::Document(DocumentNode { children: vec![] });
        let heading = BlockNode::Heading(HeadingNode { level: 2, children: vec![] });
        let thematic = BlockNode::ThematicBreak(ThematicBreakNode);
        // Pattern match to verify exhaustive enum
        match doc { BlockNode::Document(_) => {}, _ => panic!() }
        match heading { BlockNode::Heading(_) => {}, _ => panic!() }
        match thematic { BlockNode::ThematicBreak(_) => {}, _ => panic!() }
    }

    #[test]
    fn test_inline_node_variants() {
        let text = InlineNode::Text(TextNode { value: "hi".to_string() });
        let hard = InlineNode::HardBreak(HardBreakNode);
        let soft = InlineNode::SoftBreak(SoftBreakNode);
        match text  { InlineNode::Text(_) => {}, _ => panic!() }
        match hard  { InlineNode::HardBreak(_) => {}, _ => panic!() }
        match soft  { InlineNode::SoftBreak(_) => {}, _ => panic!() }
    }

    #[test]
    fn test_link_node_with_title() {
        let link = LinkNode {
            destination: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            children: vec![],
        };
        assert_eq!(link.title, Some("Example".to_string()));
    }

    #[test]
    fn test_image_node() {
        let img = ImageNode {
            destination: "cat.png".to_string(),
            title: None,
            alt: "a cat".to_string(),
        };
        assert_eq!(img.alt, "a cat");
        assert!(img.title.is_none());
    }
}

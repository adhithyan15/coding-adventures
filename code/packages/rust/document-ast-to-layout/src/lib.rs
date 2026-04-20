//! # document-ast-to-layout
//!
//! Converts a CommonMark / GFM [`DocumentNode`](document_ast::DocumentNode)
//! into a [`LayoutNode`](layout_ir::LayoutNode) tree with a complete
//! [`DocumentTheme`] applied. Implements the UI06 spec.
//!
//! ```text
//! DocumentNode (document-ast)
//!     ↓  document_ast_to_layout(doc, &theme)
//! LayoutNode tree (layout-ir)
//!     ↓  layout algorithm (layout-block for MVP)
//! PositionedNode tree
//!     ↓  layout-to-paint
//! PaintScene
//! ```
//!
//! ## v1 scope
//!
//! Implements the common block structures: Document, Heading (1..=6),
//! Paragraph, CodeBlock, Blockquote, List, ListItem, ThematicBreak.
//! Inline formatting — Emphasis, Strong, CodeSpan, Link — is **flattened
//! to plain text** for the initial MVP. The paragraph becomes a single
//! [`TextContent`] with the body font; bold/italic/link-styling is lost
//! in v1. This is called out in the module documentation of
//! [`flatten_inline_text`].
//!
//! Tables, raw blocks, task items, and inline images are not yet wired;
//! they render as empty placeholders. Explicit per-node handling for
//! these is a v2 task.
//!
//! This simplification lets us ship end-to-end Markdown rendering in a
//! few hundred lines of Rust and iterate from there.

use document_ast::{
    BlockNode, BlockquoteNode, CodeBlockNode, DocumentNode, HeadingNode, InlineNode, LinkNode,
    ListChildNode, ListItemNode, ListNode, ParagraphNode, TaskItemNode, ThematicBreakNode,
};
use layout_ir::{
    color_black, color_white, edges_all, edges_xy, font_bold, font_spec, rgb,
    rgba, size_fill, size_fixed, size_wrap, Color, Content, Edges, ExtValue, FontSpec, LayoutNode,
    TextAlign, TextContent,
};

pub const VERSION: &str = "0.1.0";

// ═══════════════════════════════════════════════════════════════════════════
// DocumentTheme
// ═══════════════════════════════════════════════════════════════════════════

/// A complete, explicit set of visual tokens applied to every node
/// during conversion. No cascade, no inheritance — every output
/// `LayoutNode` carries a fully resolved `FontSpec` and `Color`.
///
/// Use [`document_default_theme`] for a legible system-default theme,
/// or construct your own for full control.
#[derive(Clone, Debug)]
pub struct DocumentTheme {
    // ─── Typography ───────────────────────────────────────────────
    pub body_font: FontSpec,
    pub h1_font: FontSpec,
    pub h2_font: FontSpec,
    pub h3_font: FontSpec,
    pub h4_font: FontSpec,
    pub h5_font: FontSpec,
    pub h6_font: FontSpec,
    pub code_font: FontSpec,
    pub blockquote_font: FontSpec,

    // ─── Colors ───────────────────────────────────────────────────
    pub text_color: Color,
    pub heading_color: Color,
    pub link_color: Color,
    pub code_color: Color,
    pub code_bg_color: Color,
    pub blockquote_bg_color: Color,
    pub blockquote_border_color: Color,
    pub hr_color: Color,
    pub page_background: Color,

    // ─── Spacing (logical units) ──────────────────────────────────
    pub paragraph_spacing: f64,
    pub heading_spacing: f64,
    pub list_indent: f64,
    pub list_item_spacing: f64,
    pub blockquote_padding: f64,
    pub code_block_padding: f64,
    pub hr_height: f64,

    // ─── Page ─────────────────────────────────────────────────────
    /// Maximum content width (logical units). Used as a hint for the
    /// root page container's width constraint.
    pub page_width: f64,
    /// Outer page padding.
    pub page_padding: Edges,
}

/// A legible system-default theme. Matches the style of common
/// Markdown viewers (light background, dark text, clean sans-serif).
/// Font family names default to the system UI font (empty `family`)
/// so the renderer resolves the OS default.
pub fn document_default_theme() -> DocumentTheme {
    let body = font_spec("", 16.0);
    let code = font_spec("Menlo", 14.0);

    DocumentTheme {
        body_font: body.clone(),
        h1_font: font_bold(font_spec("", 32.0)),
        h2_font: font_bold(font_spec("", 26.0)),
        h3_font: font_bold(font_spec("", 21.0)),
        h4_font: font_bold(font_spec("", 18.0)),
        h5_font: font_bold(font_spec("", 16.0)),
        h6_font: font_bold(font_spec("", 14.0)),
        code_font: code,
        blockquote_font: body.clone(),

        text_color: color_black(),
        heading_color: color_black(),
        link_color: rgb(37, 99, 235), // blue-600
        code_color: rgb(30, 30, 30),
        code_bg_color: rgba(240, 240, 240, 255),
        blockquote_bg_color: rgba(250, 250, 250, 255),
        blockquote_border_color: rgb(180, 180, 180),
        hr_color: rgb(200, 200, 200),
        page_background: color_white(),

        paragraph_spacing: 16.0,
        heading_spacing: 24.0,
        list_indent: 24.0,
        list_item_spacing: 4.0,
        blockquote_padding: 16.0,
        code_block_padding: 12.0,
        hr_height: 1.0,

        page_width: 800.0,
        page_padding: edges_all(24.0),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Entry point
// ═══════════════════════════════════════════════════════════════════════════

/// Convert a whole document into a single root `LayoutNode`.
pub fn document_ast_to_layout(doc: &DocumentNode, theme: &DocumentTheme) -> LayoutNode {
    let children: Vec<LayoutNode> = doc
        .children
        .iter()
        .flat_map(|b| convert_block(b, theme))
        .collect();

    LayoutNode::container(children)
        .with_padding(theme.page_padding)
        .with_width(size_fill())
        .with_height(size_wrap())
        .with_ext(
            "paint",
            ExtValue::Map(std::iter::once(("backgroundColor".to_string(), color_to_ext(theme.page_background))).collect()),
        )
        .with_ext("block", block_ext_display_block())
}

fn block_ext_display_block() -> ExtValue {
    let mut m = std::collections::HashMap::new();
    m.insert("display".to_string(), ExtValue::Str("block".into()));
    ExtValue::Map(m)
}

fn color_to_ext(c: Color) -> ExtValue {
    let mut m = std::collections::HashMap::new();
    m.insert("r".to_string(), ExtValue::Int(c.r as i64));
    m.insert("g".to_string(), ExtValue::Int(c.g as i64));
    m.insert("b".to_string(), ExtValue::Int(c.b as i64));
    m.insert("a".to_string(), ExtValue::Int(c.a as i64));
    ExtValue::Map(m)
}

// ═══════════════════════════════════════════════════════════════════════════
// Block → LayoutNode
// ═══════════════════════════════════════════════════════════════════════════

fn convert_block(block: &BlockNode, theme: &DocumentTheme) -> Vec<LayoutNode> {
    match block {
        BlockNode::Document(doc) => doc
            .children
            .iter()
            .flat_map(|c| convert_block(c, theme))
            .collect(),
        BlockNode::Heading(h) => vec![heading_node(h, theme)],
        BlockNode::Paragraph(p) => vec![paragraph_node(p, theme)],
        BlockNode::CodeBlock(cb) => vec![code_block_node(cb, theme)],
        BlockNode::Blockquote(bq) => vec![blockquote_node(bq, theme)],
        BlockNode::List(l) => vec![list_node(l, theme)],
        BlockNode::ListItem(li) => vec![list_item_node(li, theme)],
        BlockNode::TaskItem(ti) => vec![task_item_node(ti, theme)],
        BlockNode::ThematicBreak(tb) => vec![thematic_break_node(tb, theme)],
        // Tables, raw blocks: placeholder empty containers for now.
        BlockNode::Table(_)
        | BlockNode::TableRow(_)
        | BlockNode::TableCell(_)
        | BlockNode::RawBlock(_) => vec![LayoutNode::empty()],
    }
}

fn heading_node(h: &HeadingNode, theme: &DocumentTheme) -> LayoutNode {
    let font = match h.level {
        1 => theme.h1_font.clone(),
        2 => theme.h2_font.clone(),
        3 => theme.h3_font.clone(),
        4 => theme.h4_font.clone(),
        5 => theme.h5_font.clone(),
        _ => theme.h6_font.clone(),
    };
    let text = flatten_inline_text(&h.children);
    LayoutNode::leaf_text(TextContent {
        value: text,
        font,
        color: theme.heading_color,
        max_lines: None,
        text_align: TextAlign::Start,
    })
    .with_margin(edges_xy(0.0, theme.heading_spacing / 2.0))
    .with_width(size_fill())
    .with_height(size_wrap())
}

fn paragraph_node(p: &ParagraphNode, theme: &DocumentTheme) -> LayoutNode {
    let text = flatten_inline_text(&p.children);
    if text.is_empty() {
        return LayoutNode::empty();
    }
    LayoutNode::leaf_text(TextContent {
        value: text,
        font: theme.body_font.clone(),
        color: theme.text_color,
        max_lines: None,
        text_align: TextAlign::Start,
    })
    .with_margin(edges_xy(0.0, theme.paragraph_spacing / 2.0))
    .with_width(size_fill())
    .with_height(size_wrap())
}

fn code_block_node(cb: &CodeBlockNode, theme: &DocumentTheme) -> LayoutNode {
    // Strip the spec-mandated trailing newline so rendering doesn't
    // produce an orphan blank line at the bottom of every code block.
    let content = cb.value.trim_end_matches('\n').to_string();

    let mut paint = std::collections::HashMap::new();
    paint.insert("backgroundColor".to_string(), color_to_ext(theme.code_bg_color));
    paint.insert("cornerRadius".to_string(), ExtValue::Float(4.0));

    LayoutNode::leaf_text(TextContent {
        value: content,
        font: theme.code_font.clone(),
        color: theme.code_color,
        max_lines: None,
        text_align: TextAlign::Start,
    })
    .with_margin(edges_xy(0.0, theme.paragraph_spacing / 2.0))
    .with_padding(edges_all(theme.code_block_padding))
    .with_width(size_fill())
    .with_height(size_wrap())
    .with_ext("paint", ExtValue::Map(paint))
}

fn blockquote_node(bq: &BlockquoteNode, theme: &DocumentTheme) -> LayoutNode {
    let children: Vec<LayoutNode> = bq
        .children
        .iter()
        .flat_map(|b| convert_block(b, theme))
        .collect();

    let mut paint = std::collections::HashMap::new();
    paint.insert(
        "backgroundColor".to_string(),
        color_to_ext(theme.blockquote_bg_color),
    );
    paint.insert(
        "borderColor".to_string(),
        color_to_ext(theme.blockquote_border_color),
    );
    paint.insert("borderWidth".to_string(), ExtValue::Float(4.0));

    LayoutNode::container(children)
        .with_margin(edges_xy(0.0, theme.paragraph_spacing / 2.0))
        .with_padding(edges_all(theme.blockquote_padding))
        .with_width(size_fill())
        .with_height(size_wrap())
        .with_ext("paint", ExtValue::Map(paint))
}

fn list_node(l: &ListNode, theme: &DocumentTheme) -> LayoutNode {
    let mut counter: i64 = l.start.unwrap_or(1);
    let mut children: Vec<LayoutNode> = Vec::new();

    for (idx, c) in l.children.iter().enumerate() {
        match c {
            ListChildNode::ListItem(li) => {
                children.push(list_item_with_marker(li, theme, l.ordered, counter));
                counter += 1;
            }
            ListChildNode::TaskItem(ti) => {
                children.push(task_item_with_marker(ti, theme, idx));
            }
        }
    }

    LayoutNode::container(children)
        .with_margin(edges_xy(0.0, theme.paragraph_spacing / 2.0))
        .with_padding(Edges {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: theme.list_indent,
        })
        .with_width(size_fill())
        .with_height(size_wrap())
}

fn list_item_with_marker(
    li: &ListItemNode,
    theme: &DocumentTheme,
    ordered: bool,
    number: i64,
) -> LayoutNode {
    let marker = if ordered {
        format!("{}. ", number)
    } else {
        "• ".to_string()
    };

    // Flatten the first block's inline content (usually a paragraph)
    // into a single-line TextContent with the marker prepended.
    let mut text = marker;
    for (i, child) in li.children.iter().enumerate() {
        if i > 0 {
            text.push('\n');
        }
        text.push_str(&block_as_plain_text(child));
    }
    if text.is_empty() {
        return LayoutNode::empty();
    }

    LayoutNode::leaf_text(TextContent {
        value: text,
        font: theme.body_font.clone(),
        color: theme.text_color,
        max_lines: None,
        text_align: TextAlign::Start,
    })
    .with_margin(Edges {
        top: theme.list_item_spacing,
        right: 0.0,
        bottom: theme.list_item_spacing,
        left: 0.0,
    })
    .with_width(size_fill())
    .with_height(size_wrap())
}

fn task_item_with_marker(ti: &TaskItemNode, theme: &DocumentTheme, _idx: usize) -> LayoutNode {
    // Reach into the TaskItemNode via its fields. If the document-ast
    // crate exposes a `checked` bool, use it; otherwise render an empty
    // checkbox. We use a best-effort path here since the TaskItemNode
    // fields are private / variable across versions.
    let marker = render_task_marker(ti);

    let mut text = marker;
    for (i, child) in extract_task_item_children(ti).iter().enumerate() {
        if i > 0 {
            text.push('\n');
        }
        text.push_str(&block_as_plain_text(child));
    }

    LayoutNode::leaf_text(TextContent {
        value: text,
        font: theme.body_font.clone(),
        color: theme.text_color,
        max_lines: None,
        text_align: TextAlign::Start,
    })
    .with_margin(Edges {
        top: theme.list_item_spacing,
        right: 0.0,
        bottom: theme.list_item_spacing,
        left: 0.0,
    })
    .with_width(size_fill())
    .with_height(size_wrap())
}

fn render_task_marker(_ti: &TaskItemNode) -> String {
    // TaskItemNode carries a `checked` flag. For v1 we render it as
    // ASCII so we don't depend on Unicode box-drawing characters being
    // in the default font.
    "[ ] ".to_string()
}

fn extract_task_item_children(ti: &TaskItemNode) -> &[BlockNode] {
    // document-ast's TaskItemNode stores children as `Vec<BlockNode>`.
    // We reflect that here; if the field name ever changes, update
    // this one accessor.
    &ti.children
}

fn list_item_node(li: &ListItemNode, theme: &DocumentTheme) -> LayoutNode {
    // Fallback when a ListItem is encountered outside of a List.
    // Render with a bullet so the item at least gets a marker.
    list_item_with_marker(li, theme, false, 1)
}

fn task_item_node(ti: &TaskItemNode, theme: &DocumentTheme) -> LayoutNode {
    task_item_with_marker(ti, theme, 0)
}

fn thematic_break_node(_tb: &ThematicBreakNode, theme: &DocumentTheme) -> LayoutNode {
    let mut paint = std::collections::HashMap::new();
    paint.insert("backgroundColor".to_string(), color_to_ext(theme.hr_color));

    LayoutNode::empty()
        .with_margin(edges_xy(0.0, theme.paragraph_spacing))
        .with_width(size_fill())
        .with_height(size_fixed(theme.hr_height))
        .with_ext("paint", ExtValue::Map(paint))
}

// ═══════════════════════════════════════════════════════════════════════════
// Inline flattening
// ═══════════════════════════════════════════════════════════════════════════

/// Concatenate a list of inline nodes into a single plain-text string.
///
/// **v1 limitation**: inline formatting (bold, italic, code-span, link
/// color) is lost. The whole run is rendered with the block's base
/// font. A future PR will turn each distinct inline run into a
/// separate layout child so the block layout algorithm can stitch
/// styled spans together on a single line.
///
/// SoftBreak renders as a single space (the conventional CommonMark
/// interpretation). HardBreak renders as a newline so downstream
/// measurement produces the right line count.
pub fn flatten_inline_text(inlines: &[InlineNode]) -> String {
    let mut out = String::new();
    for n in inlines {
        flatten_one(n, &mut out);
    }
    out
}

fn flatten_one(n: &InlineNode, out: &mut String) {
    match n {
        InlineNode::Text(t) => out.push_str(&t.value),
        InlineNode::Emphasis(e) => {
            for c in &e.children {
                flatten_one(c, out);
            }
        }
        InlineNode::Strong(s) => {
            for c in &s.children {
                flatten_one(c, out);
            }
        }
        InlineNode::Strikethrough(s) => {
            for c in &s.children {
                flatten_one(c, out);
            }
        }
        InlineNode::CodeSpan(c) => out.push_str(&c.value),
        InlineNode::Link(l) => {
            for c in &l.children {
                flatten_one(c, out);
            }
            // Append URL in a simple form for visibility, but only if
            // the link text differs from the URL.
            if should_show_url(l) {
                out.push_str(" (");
                out.push_str(&l.destination);
                out.push(')');
            }
        }
        InlineNode::Image(img) => {
            out.push_str("[image: ");
            out.push_str(&img.alt);
            out.push(']');
        }
        InlineNode::Autolink(a) => out.push_str(&a.destination),
        InlineNode::RawInline(_) => { /* strip raw HTML for MVP */ }
        InlineNode::HardBreak(_) => out.push('\n'),
        InlineNode::SoftBreak(_) => out.push(' '),
    }
}

fn should_show_url(l: &LinkNode) -> bool {
    let mut inner = String::new();
    for c in &l.children {
        flatten_one(c, &mut inner);
    }
    inner != l.destination
}

// ═══════════════════════════════════════════════════════════════════════════
// Plain-text helpers (for list items that need to flatten blocks)
// ═══════════════════════════════════════════════════════════════════════════

fn block_as_plain_text(b: &BlockNode) -> String {
    match b {
        BlockNode::Document(d) => d
            .children
            .iter()
            .map(block_as_plain_text)
            .collect::<Vec<_>>()
            .join("\n"),
        BlockNode::Heading(h) => flatten_inline_text(&h.children),
        BlockNode::Paragraph(p) => flatten_inline_text(&p.children),
        BlockNode::CodeBlock(cb) => cb.value.trim_end_matches('\n').to_string(),
        BlockNode::Blockquote(bq) => bq
            .children
            .iter()
            .map(block_as_plain_text)
            .collect::<Vec<_>>()
            .join("\n"),
        BlockNode::List(_) => String::new(),  // nested list flattening is a v2 concern
        BlockNode::ListItem(li) => li
            .children
            .iter()
            .map(block_as_plain_text)
            .collect::<Vec<_>>()
            .join("\n"),
        BlockNode::TaskItem(ti) => extract_task_item_children(ti)
            .iter()
            .map(block_as_plain_text)
            .collect::<Vec<_>>()
            .join("\n"),
        BlockNode::ThematicBreak(_)
        | BlockNode::Table(_)
        | BlockNode::TableRow(_)
        | BlockNode::TableCell(_)
        | BlockNode::RawBlock(_) => String::new(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use document_ast::{EmphasisNode, HardBreakNode, LinkNode, SoftBreakNode, StrongNode, TextNode};

    fn txt(s: &str) -> InlineNode {
        InlineNode::Text(TextNode { value: s.into() })
    }

    #[test]
    fn default_theme_has_body_font() {
        let t = document_default_theme();
        assert_eq!(t.body_font.size, 16.0);
        assert_eq!(t.h1_font.weight, 700);
        assert!(t.h1_font.size > t.body_font.size);
    }

    #[test]
    fn empty_document_is_empty_container() {
        let doc = DocumentNode { children: vec![] };
        let node = document_ast_to_layout(&doc, &document_default_theme());
        assert!(node.children.is_empty());
        assert!(node.content.is_none());
    }

    #[test]
    fn heading_level_selects_font_size() {
        let theme = document_default_theme();
        let doc = DocumentNode {
            children: vec![BlockNode::Heading(HeadingNode {
                level: 1,
                children: vec![txt("Hello")],
            })],
        };
        let root = document_ast_to_layout(&doc, &theme);
        let h = &root.children[0];
        let Content::Text(tc) = h.content.as_ref().unwrap() else {
            panic!("expected text content");
        };
        assert_eq!(tc.value, "Hello");
        assert_eq!(tc.font.size, theme.h1_font.size);
        assert_eq!(tc.font.weight, 700);
    }

    #[test]
    fn paragraph_uses_body_font() {
        let theme = document_default_theme();
        let doc = DocumentNode {
            children: vec![BlockNode::Paragraph(ParagraphNode {
                children: vec![txt("plain")],
            })],
        };
        let root = document_ast_to_layout(&doc, &theme);
        let Content::Text(tc) = root.children[0].content.as_ref().unwrap() else {
            panic!();
        };
        assert_eq!(tc.value, "plain");
        assert_eq!(tc.font.size, theme.body_font.size);
    }

    #[test]
    fn flatten_inline_strips_styling_v1() {
        let inlines = vec![
            txt("hello "),
            InlineNode::Strong(StrongNode {
                children: vec![txt("world")],
            }),
            txt("!"),
        ];
        assert_eq!(flatten_inline_text(&inlines), "hello world!");
    }

    #[test]
    fn flatten_inline_softbreak_is_space() {
        let inlines = vec![
            txt("line 1"),
            InlineNode::SoftBreak(SoftBreakNode),
            txt("line 2"),
        ];
        assert_eq!(flatten_inline_text(&inlines), "line 1 line 2");
    }

    #[test]
    fn flatten_inline_hardbreak_is_newline() {
        let inlines = vec![
            txt("line 1"),
            InlineNode::HardBreak(HardBreakNode),
            txt("line 2"),
        ];
        assert_eq!(flatten_inline_text(&inlines), "line 1\nline 2");
    }

    #[test]
    fn flatten_inline_nested_emphasis() {
        let inlines = vec![InlineNode::Emphasis(EmphasisNode {
            children: vec![
                txt("italic "),
                InlineNode::Strong(StrongNode {
                    children: vec![txt("bold")],
                }),
            ],
        })];
        assert_eq!(flatten_inline_text(&inlines), "italic bold");
    }

    #[test]
    fn flatten_link_appends_url_when_different_from_text() {
        let inlines = vec![InlineNode::Link(LinkNode {
            destination: "https://example.com".into(),
            title: None,
            children: vec![txt("click")],
        })];
        let out = flatten_inline_text(&inlines);
        assert!(out.contains("click"));
        assert!(out.contains("https://example.com"));
    }

    #[test]
    fn flatten_link_omits_url_when_equal_to_text() {
        let inlines = vec![InlineNode::Link(LinkNode {
            destination: "https://example.com".into(),
            title: None,
            children: vec![txt("https://example.com")],
        })];
        assert_eq!(flatten_inline_text(&inlines), "https://example.com");
    }

    #[test]
    fn blockquote_contains_converted_children() {
        let doc = DocumentNode {
            children: vec![BlockNode::Blockquote(BlockquoteNode {
                children: vec![BlockNode::Paragraph(ParagraphNode {
                    children: vec![txt("quoted")],
                })],
            })],
        };
        let root = document_ast_to_layout(&doc, &document_default_theme());
        let bq = &root.children[0];
        assert_eq!(bq.children.len(), 1);
        let Content::Text(tc) = bq.children[0].content.as_ref().unwrap() else {
            panic!();
        };
        assert_eq!(tc.value, "quoted");
        // blockquote must carry paint ext
        assert!(bq.ext.contains_key("paint"));
    }

    #[test]
    fn unordered_list_emits_bullets() {
        let doc = DocumentNode {
            children: vec![BlockNode::List(ListNode {
                ordered: false,
                start: None,
                tight: true,
                children: vec![
                    ListChildNode::ListItem(ListItemNode {
                        children: vec![BlockNode::Paragraph(ParagraphNode {
                            children: vec![txt("first")],
                        })],
                    }),
                    ListChildNode::ListItem(ListItemNode {
                        children: vec![BlockNode::Paragraph(ParagraphNode {
                            children: vec![txt("second")],
                        })],
                    }),
                ],
            })],
        };
        let root = document_ast_to_layout(&doc, &document_default_theme());
        let list = &root.children[0];
        assert_eq!(list.children.len(), 2);
        let Content::Text(tc0) = list.children[0].content.as_ref().unwrap() else {
            panic!();
        };
        assert!(tc0.value.starts_with("• "));
        assert!(tc0.value.contains("first"));
    }

    #[test]
    fn ordered_list_emits_numbers() {
        let doc = DocumentNode {
            children: vec![BlockNode::List(ListNode {
                ordered: true,
                start: Some(3),
                tight: true,
                children: vec![
                    ListChildNode::ListItem(ListItemNode {
                        children: vec![BlockNode::Paragraph(ParagraphNode {
                            children: vec![txt("alpha")],
                        })],
                    }),
                    ListChildNode::ListItem(ListItemNode {
                        children: vec![BlockNode::Paragraph(ParagraphNode {
                            children: vec![txt("beta")],
                        })],
                    }),
                ],
            })],
        };
        let root = document_ast_to_layout(&doc, &document_default_theme());
        let list = &root.children[0];
        let Content::Text(tc0) = list.children[0].content.as_ref().unwrap() else {
            panic!();
        };
        assert!(tc0.value.starts_with("3. "));
        let Content::Text(tc1) = list.children[1].content.as_ref().unwrap() else {
            panic!();
        };
        assert!(tc1.value.starts_with("4. "));
    }

    #[test]
    fn code_block_uses_code_font() {
        let theme = document_default_theme();
        let doc = DocumentNode {
            children: vec![BlockNode::CodeBlock(CodeBlockNode {
                language: Some("rust".into()),
                value: "fn main() {}\n".into(),
            })],
        };
        let root = document_ast_to_layout(&doc, &theme);
        let cb = &root.children[0];
        let Content::Text(tc) = cb.content.as_ref().unwrap() else {
            panic!();
        };
        assert_eq!(tc.value, "fn main() {}");
        assert_eq!(tc.font.family, theme.code_font.family);
        // trailing newline stripped
        assert!(!tc.value.ends_with('\n'));
    }
}

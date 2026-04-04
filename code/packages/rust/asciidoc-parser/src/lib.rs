//! AsciiDoc parser — converts AsciiDoc text to a Document AST.
//!
//! This is a two-phase parser:
//!
//!   Phase 1 (`block_parser.rs`): Block structure
//!     Input text → lines → block tree with raw inline content strings.
//!     Headings, paragraphs, lists, code blocks, blockquotes, and thematic
//!     breaks are identified and structured into a tree.
//!
//!   Phase 2 (`inline_parser.rs`): Inline content
//!     Each block's raw content string → inline nodes.
//!     Bold, italic, links, images, code spans, etc. are parsed.
//!
//! # AsciiDoc vs Markdown
//!
//! Key differences:
//!   - Headings: `= Title` (level 1), `== Section` (level 2), etc.
//!   - `*bold*` → **strong** (NOT emphasis! opposite of Markdown)
//!   - `_italic_` → emphasis
//!   - Code blocks delimited by `----`, preceded by `[source,lang]`
//!   - Thematic break: `'''`
//!   - Links: `link:url[text]` or `<<anchor,text>>`
//!   - Images: `image:url[alt]`
//!
//! # Quick Start
//!
//! ```rust
//! use asciidoc_parser::parse;
//!
//! let doc = parse("= Hello\n\nWorld *with* bold.\n");
//! assert_eq!(doc.children.len(), 2);
//! ```
//!
//! Spec: TE03 — AsciiDoc Parser

mod block_parser;
mod inline_parser;

pub use document_ast::*;

/// Parse AsciiDoc text and return a Document AST `DocumentNode`.
///
/// The result conforms to the Document AST spec (TE00) — a format-agnostic IR
/// with all inline markup parsed and all block structure resolved.
///
/// # Arguments
///
/// * `text` — The AsciiDoc source string.
///
/// # Returns
///
/// The root `DocumentNode`.
///
/// # Examples
///
/// ```rust
/// use asciidoc_parser::parse;
///
/// let doc = parse("== Section\n\n- item 1\n");
/// assert!(!doc.children.is_empty());
/// ```
pub fn parse(text: &str) -> DocumentNode {
    let blocks = block_parser::parse_blocks(text);
    DocumentNode { children: blocks }
}

pub const VERSION: &str = "0.1.0";

#[cfg(test)]
mod tests {
    use super::*;

    fn text_node(s: &str) -> InlineNode {
        InlineNode::Text(TextNode { value: s.to_string() })
    }

    // ── Block tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_parse_empty() {
        let doc = parse("");
        assert_eq!(doc.children.len(), 0);
    }

    #[test]
    fn test_parse_blank_lines() {
        let doc = parse("\n\n\n");
        assert_eq!(doc.children.len(), 0);
    }

    #[test]
    fn test_parse_heading_1() {
        let doc = parse("= Hello World\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::Heading(h) => {
                assert_eq!(h.level, 1);
                assert_eq!(h.children.len(), 1);
                assert_eq!(h.children[0], text_node("Hello World"));
            }
            other => panic!("expected Heading, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_heading_2() {
        let doc = parse("== Section\n");
        match &doc.children[0] {
            BlockNode::Heading(h) => assert_eq!(h.level, 2),
            other => panic!("expected Heading, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_heading_6() {
        let doc = parse("====== Deep\n");
        match &doc.children[0] {
            BlockNode::Heading(h) => assert_eq!(h.level, 6),
            other => panic!("expected Heading, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse("Hello world\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::Paragraph(p) => {
                assert_eq!(p.children[0], text_node("Hello world"));
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_multi_line_paragraph() {
        let doc = parse("Line one\nLine two\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::Paragraph(_) => {}
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_two_paragraphs() {
        let doc = parse("First\n\nSecond\n");
        assert_eq!(doc.children.len(), 2);
    }

    #[test]
    fn test_parse_thematic_break() {
        let doc = parse("'''\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::ThematicBreak(_) => {}
            other => panic!("expected ThematicBreak, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse("----\nfoo := bar\n----\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::CodeBlock(cb) => {
                assert_eq!(cb.language, None);
                assert_eq!(cb.value, "foo := bar\n");
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_code_block_with_language() {
        let doc = parse("[source,go]\n----\nfmt.Println()\n----\n");
        match &doc.children[0] {
            BlockNode::CodeBlock(cb) => {
                assert_eq!(cb.language, Some("go".to_string()));
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_literal_block() {
        let doc = parse("....\nsome literal\n....\n");
        match &doc.children[0] {
            BlockNode::CodeBlock(cb) => {
                assert_eq!(cb.language, None);
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_passthrough_block() {
        let doc = parse("++++\n<div>raw</div>\n++++\n");
        match &doc.children[0] {
            BlockNode::RawBlock(rb) => {
                assert_eq!(rb.format, "html");
                assert_eq!(rb.value, "<div>raw</div>");
            }
            other => panic!("expected RawBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_quote_block() {
        let doc = parse("____\nSome quote\n____\n");
        match &doc.children[0] {
            BlockNode::Blockquote(bq) => {
                assert!(!bq.children.is_empty());
            }
            other => panic!("expected Blockquote, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = parse("* item one\n* item two\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::List(list) => {
                assert!(!list.ordered);
                assert_eq!(list.children.len(), 2);
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse(". first\n. second\n. third\n");
        match &doc.children[0] {
            BlockNode::List(list) => {
                assert!(list.ordered);
                assert_eq!(list.children.len(), 3);
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_comment_skipped() {
        let doc = parse("// this is a comment\nHello\n");
        assert_eq!(doc.children.len(), 1);
        match &doc.children[0] {
            BlockNode::Paragraph(_) => {}
            other => panic!("expected Paragraph after comment, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_heading_then_paragraph() {
        let doc = parse("= Title\n\nSome text.\n");
        assert_eq!(doc.children.len(), 2);
        assert!(matches!(&doc.children[0], BlockNode::Heading(_)));
        assert!(matches!(&doc.children[1], BlockNode::Paragraph(_)));
    }

    #[test]
    fn test_parse_nested_quote_block() {
        let doc = parse("____\n== Inner Heading\n____\n");
        match &doc.children[0] {
            BlockNode::Blockquote(bq) => {
                assert!(!bq.children.is_empty());
                assert!(matches!(&bq.children[0], BlockNode::Heading(_)));
            }
            other => panic!("expected Blockquote, got {:?}", other),
        }
    }

    // ── Inline tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_inline_strong() {
        // AsciiDoc *text* = strong (not emphasis!)
        let doc = parse("Hello *world*!\n");
        match &doc.children[0] {
            BlockNode::Paragraph(p) => {
                let has_strong = p.children.iter().any(|n| matches!(n, InlineNode::Strong(_)));
                assert!(has_strong, "expected StrongNode in {:?}", p.children);
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn test_inline_emphasis() {
        let doc = parse("Hello _world_!\n");
        match &doc.children[0] {
            BlockNode::Paragraph(p) => {
                let has_em = p.children.iter().any(|n| matches!(n, InlineNode::Emphasis(_)));
                assert!(has_em, "expected EmphasisNode in {:?}", p.children);
            }
            other => panic!("expected Paragraph, got {:?}", other),
        }
    }

    #[test]
    fn test_inline_strong_unconstrained() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("**bold**");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], InlineNode::Strong(_)));
    }

    #[test]
    fn test_inline_emphasis_unconstrained() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("__em__");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(&nodes[0], InlineNode::Emphasis(_)));
    }

    #[test]
    fn test_inline_code_span() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("Use `foo()` now");
        let has_code = nodes.iter().any(|n| {
            if let InlineNode::CodeSpan(cs) = n { cs.value == "foo()" } else { false }
        });
        assert!(has_code, "expected CodeSpan with 'foo()'");
    }

    #[test]
    fn test_inline_link_macro() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("See link:https://example.com[Example].");
        let has_link = nodes.iter().any(|n| {
            if let InlineNode::Link(ln) = n { ln.destination == "https://example.com" } else { false }
        });
        assert!(has_link, "expected link to https://example.com");
    }

    #[test]
    fn test_inline_image_macro() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("image:cat.png[A cat]");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::Image(img) => {
                assert_eq!(img.destination, "cat.png");
                assert_eq!(img.alt, "A cat");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    #[test]
    fn test_inline_cross_ref() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("See <<section-id,Section Title>>.");
        let has_link = nodes.iter().any(|n| {
            if let InlineNode::Link(ln) = n { ln.destination == "#section-id" } else { false }
        });
        assert!(has_link, "expected cross-ref link to #section-id");
    }

    #[test]
    fn test_inline_cross_ref_no_text() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("<<my-anchor>>");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::Link(ln) => assert_eq!(ln.destination, "#my-anchor"),
            other => panic!("expected Link, got {:?}", other),
        }
    }

    #[test]
    fn test_inline_autolink() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("Visit https://example.com for details.");
        let has_autolink = nodes.iter().any(|n| {
            if let InlineNode::Autolink(al) = n { al.destination == "https://example.com" } else { false }
        });
        assert!(has_autolink, "expected autolink to https://example.com");
    }

    #[test]
    fn test_inline_url_with_brackets() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("https://example.com[Click here]");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::Link(ln) => assert_eq!(ln.destination, "https://example.com"),
            other => panic!("expected Link, got {:?}", other),
        }
    }

    #[test]
    fn test_inline_soft_break() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("line one\nline two");
        let has_soft = nodes.iter().any(|n| matches!(n, InlineNode::SoftBreak(_)));
        assert!(has_soft, "expected SoftBreak in {:?}", nodes);
    }

    #[test]
    fn test_inline_hard_break_two_spaces() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("line one  \nline two");
        let has_hard = nodes.iter().any(|n| matches!(n, InlineNode::HardBreak(_)));
        assert!(has_hard, "expected HardBreak in {:?}", nodes);
    }

    #[test]
    fn test_inline_hard_break_backslash() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("line one\\\nline two");
        let has_hard = nodes.iter().any(|n| matches!(n, InlineNode::HardBreak(_)));
        assert!(has_hard, "expected HardBreak in {:?}", nodes);
    }

    #[test]
    fn test_inline_plain_text() {
        use document_ast::InlineNode;
        let nodes = inline_parser::parse_inlines("just plain text");
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            InlineNode::Text(t) => assert_eq!(t.value, "just plain text"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}

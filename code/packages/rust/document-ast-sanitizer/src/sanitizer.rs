//! Core sanitization logic — policy-driven AST tree transformation.
//!
//! The `sanitize` function performs a single recursive descent of the
//! `DocumentNode` tree, producing a **new** tree with all policy violations
//! removed or neutralised. The input is never mutated.
//!
//! # Transformation model
//!
//! For each node the sanitizer applies a three-question rule:
//!
//! ```text
//! 1. Should this node exist at all?  → keep / drop / promote
//! 2. Should its value be changed?    → URL sanitization, level clamping
//! 3. Should its children be walked?  → recurse / leaf
//! ```
//!
//! After recursion, container nodes whose children list is now empty are
//! themselves dropped — preventing empty `<p></p>` tags in rendered output.
//! The only exception is `DocumentNode`, which is always kept even when empty.
//!
//! # Complete truth table
//!
//! ```text
//! Node type          Condition                              Action
//! ──────────────────────────────────────────────────────────────────────────
//! DocumentNode       always                                 recurse into children
//! HeadingNode        max_heading_level = Drop               drop node
//! HeadingNode        level < min_heading_level              clamp level up
//! HeadingNode        level > max_heading_level.Level        clamp level down
//! HeadingNode        otherwise                              recurse into children
//! ParagraphNode      always                                 recurse; drop if empty
//! CodeBlockNode      drop_code_blocks = true                drop node
//! CodeBlockNode      otherwise                              keep as-is (leaf)
//! BlockquoteNode     drop_blockquotes = true                drop node
//! BlockquoteNode     otherwise                              recurse; drop if empty
//! ListNode           always                                 recurse into children
//! ListItemNode       always                                 recurse; drop if empty
//! ThematicBreakNode  always                                 keep as-is (leaf)
//! RawBlockNode       allow_raw_block_formats = DropAll      drop node
//! RawBlockNode       allow_raw_block_formats = Passthrough  keep as-is
//! RawBlockNode       allow_raw_block_formats = Allowlist    keep if format in list
//!
//! TextNode           always                                 keep as-is
//! EmphasisNode       always                                 recurse; drop if empty
//! StrongNode         always                                 recurse; drop if empty
//! CodeSpanNode       transform_code_span_to_text = true     → TextNode { value }
//! CodeSpanNode       otherwise                              keep as-is
//! LinkNode           drop_links = true                      promote children to parent
//! LinkNode           URL scheme not allowed                 keep, set destination=""
//! LinkNode           otherwise                              sanitize URL, recurse
//! ImageNode          drop_images = true                     drop node
//! ImageNode          transform_image_to_text = true         → TextNode { value: alt }
//! ImageNode          URL scheme not allowed                 keep, set destination=""
//! ImageNode          otherwise                              sanitize URL, keep
//! AutolinkNode       URL scheme not allowed                 drop node
//! AutolinkNode       otherwise                              keep as-is
//! RawInlineNode      allow_raw_inline_formats = DropAll     drop node
//! RawInlineNode      allow_raw_inline_formats = Passthrough keep as-is
//! RawInlineNode      allow_raw_inline_formats = Allowlist   keep if format in list
//! HardBreakNode      always                                 keep as-is
//! SoftBreakNode      always                                 keep as-is
//! ```

use document_ast::{
    AutolinkNode, BlockNode, BlockquoteNode, CodeBlockNode, CodeSpanNode, DocumentNode,
    EmphasisNode, HardBreakNode, HeadingNode, ImageNode, InlineNode, LinkNode, ListItemNode,
    ListNode, ParagraphNode, RawBlockNode, RawInlineNode, SoftBreakNode, StrongNode, TextNode,
    ThematicBreakNode,
};

use crate::policy::{MaxHeadingLevel, SanitizationPolicy};
use crate::url_utils::is_scheme_allowed;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Sanitize a `DocumentNode` by applying a `SanitizationPolicy`.
///
/// Returns a new `DocumentNode` with all policy violations removed or
/// neutralised. The input document is never mutated — callers can safely
/// pass the same document through multiple sanitizers with different policies.
///
/// # Examples
///
/// ```rust
/// use document_ast::{DocumentNode, BlockNode, HeadingNode, InlineNode, TextNode};
/// use coding_adventures_document_ast_sanitizer::sanitizer::sanitize;
/// use coding_adventures_document_ast_sanitizer::policy::strict;
///
/// let doc = DocumentNode {
///     children: vec![
///         BlockNode::Heading(HeadingNode {
///             level: 1,
///             children: vec![InlineNode::Text(TextNode { value: "Title".to_string() })],
///         }),
///     ],
/// };
///
/// // STRICT clamps h1 → h2
/// let safe = sanitize(&doc, &strict());
/// if let BlockNode::Heading(h) = &safe.children[0] {
///     assert_eq!(h.level, 2);
/// }
/// ```
pub fn sanitize(doc: &DocumentNode, policy: &SanitizationPolicy) -> DocumentNode {
    let children = sanitize_block_children(&doc.children, policy);
    DocumentNode { children }
}

// ─── Block node processing ────────────────────────────────────────────────────

/// Process a slice of `BlockNode`s, returning only those that survive the
/// policy. Container nodes whose children become empty are dropped (empty
/// `<p></p>` prevention). `DocumentNode` children are recursed but never
/// appear as block children in a well-formed tree — they are kept for
/// completeness.
fn sanitize_block_children(nodes: &[BlockNode], policy: &SanitizationPolicy) -> Vec<BlockNode> {
    let mut out = Vec::new();
    for node in nodes {
        let results = sanitize_block_node(node, policy);
        out.extend(results);
    }
    out
}

/// Apply policy to a single `BlockNode`. Returns 0 or 1 `BlockNode` values
/// (returned as a `Vec` for uniformity with inline promotion).
fn sanitize_block_node(node: &BlockNode, policy: &SanitizationPolicy) -> Vec<BlockNode> {
    match node {
        // ── DocumentNode ──────────────────────────────────────────────────
        // DocumentNode is the root; it can appear in BlockNode::Document
        // during recursive list-item processing. Always recurse.
        BlockNode::Document(d) => {
            vec![BlockNode::Document(sanitize(d, policy))]
        }

        // ── HeadingNode ───────────────────────────────────────────────────
        BlockNode::Heading(h) => {
            // Drop all headings?
            if policy.max_heading_level == MaxHeadingLevel::Drop {
                return vec![];
            }

            // Clamp level to [min_heading_level, max_heading_level]
            let min = policy.min_heading_level.clamp(1, 6);
            let max = match &policy.max_heading_level {
                MaxHeadingLevel::Drop => return vec![], // unreachable due to check above
                MaxHeadingLevel::Level(l) => (*l).clamp(1, 6),
            };

            let clamped_level = h.level.clamp(min, max);

            // Recurse into children
            let children = sanitize_inline_children(&h.children, policy);

            // Even an empty-child heading is kept (it has structural meaning)
            vec![BlockNode::Heading(HeadingNode {
                level: clamped_level,
                children,
            })]
        }

        // ── ParagraphNode ─────────────────────────────────────────────────
        BlockNode::Paragraph(p) => {
            let children = sanitize_inline_children(&p.children, policy);
            if children.is_empty() {
                vec![] // drop empty paragraphs
            } else {
                vec![BlockNode::Paragraph(ParagraphNode { children })]
            }
        }

        // ── CodeBlockNode ─────────────────────────────────────────────────
        BlockNode::CodeBlock(cb) => {
            if policy.drop_code_blocks {
                vec![]
            } else {
                vec![BlockNode::CodeBlock(CodeBlockNode {
                    language: cb.language.clone(),
                    value: cb.value.clone(),
                })]
            }
        }

        // ── BlockquoteNode ────────────────────────────────────────────────
        BlockNode::Blockquote(bq) => {
            if policy.drop_blockquotes {
                return vec![];
            }
            let children = sanitize_block_children(&bq.children, policy);
            if children.is_empty() {
                vec![] // drop empty blockquotes
            } else {
                vec![BlockNode::Blockquote(BlockquoteNode { children })]
            }
        }

        // ── ListNode ──────────────────────────────────────────────────────
        BlockNode::List(list) => {
            // Each list item is processed; items that become empty are dropped.
            let items: Vec<ListItemNode> = list
                .children
                .iter()
                .filter_map(|item| {
                    let children = sanitize_block_children(&item.children, policy);
                    if children.is_empty() {
                        None // drop empty list items
                    } else {
                        Some(ListItemNode { children })
                    }
                })
                .collect();

            if items.is_empty() {
                vec![] // drop a list with no surviving items
            } else {
                vec![BlockNode::List(ListNode {
                    ordered: list.ordered,
                    start: list.start,
                    tight: list.tight,
                    children: items,
                })]
            }
        }

        // ── ListItemNode ──────────────────────────────────────────────────
        // List items normally live inside ListNode and are handled there.
        // This arm handles a bare ListItemNode if it appears at block level
        // (malformed tree — just recurse).
        BlockNode::ListItem(item) => {
            let children = sanitize_block_children(&item.children, policy);
            if children.is_empty() {
                vec![]
            } else {
                vec![BlockNode::ListItem(ListItemNode { children })]
            }
        }

        // ── ThematicBreakNode ─────────────────────────────────────────────
        // A leaf node — no children, no content to check. Always keep.
        BlockNode::ThematicBreak(_) => {
            vec![BlockNode::ThematicBreak(ThematicBreakNode)]
        }

        // ── RawBlockNode ──────────────────────────────────────────────────
        BlockNode::RawBlock(rb) => {
            if policy.allow_raw_block_formats.allows(&rb.format) {
                vec![BlockNode::RawBlock(RawBlockNode {
                    format: rb.format.clone(),
                    value: rb.value.clone(),
                })]
            } else {
                vec![]
            }
        }
    }
}

// ─── Inline node processing ───────────────────────────────────────────────────

/// Process a slice of `InlineNode`s. Returns the surviving inline nodes.
///
/// Note: `LinkNode` with `drop_links: true` returns **multiple** inline nodes
/// (the promoted children). All other cases return 0 or 1 node. We flatten
/// everything into a single `Vec`.
fn sanitize_inline_children(nodes: &[InlineNode], policy: &SanitizationPolicy) -> Vec<InlineNode> {
    let mut out = Vec::new();
    for node in nodes {
        out.extend(sanitize_inline_node(node, policy));
    }
    out
}

/// Apply policy to a single `InlineNode`. Returns 0, 1, or many inline nodes
/// (promotion case). The multi-value case occurs only when a `LinkNode` is
/// dropped with `drop_links: true` — its text children are promoted.
fn sanitize_inline_node(node: &InlineNode, policy: &SanitizationPolicy) -> Vec<InlineNode> {
    match node {
        // ── TextNode ──────────────────────────────────────────────────────
        // Plain text — always keep as-is.
        InlineNode::Text(t) => {
            vec![InlineNode::Text(TextNode {
                value: t.value.clone(),
            })]
        }

        // ── EmphasisNode ──────────────────────────────────────────────────
        InlineNode::Emphasis(e) => {
            let children = sanitize_inline_children(&e.children, policy);
            if children.is_empty() {
                vec![] // drop empty emphasis
            } else {
                vec![InlineNode::Emphasis(EmphasisNode { children })]
            }
        }

        // ── StrongNode ────────────────────────────────────────────────────
        InlineNode::Strong(s) => {
            let children = sanitize_inline_children(&s.children, policy);
            if children.is_empty() {
                vec![]
            } else {
                vec![InlineNode::Strong(StrongNode { children })]
            }
        }

        // ── CodeSpanNode ──────────────────────────────────────────────────
        InlineNode::CodeSpan(cs) => {
            if policy.transform_code_span_to_text {
                // Replace `code` with plain text — removes the monospace wrapper
                vec![InlineNode::Text(TextNode {
                    value: cs.value.clone(),
                })]
            } else {
                vec![InlineNode::CodeSpan(CodeSpanNode {
                    value: cs.value.clone(),
                })]
            }
        }

        // ── LinkNode ──────────────────────────────────────────────────────
        InlineNode::Link(link) => {
            if policy.drop_links {
                // Promote children: the link wrapper disappears but its text
                // content is preserved. "[click here](url)" → "click here".
                return sanitize_inline_children(&link.children, policy);
            }

            let destination = if is_scheme_allowed(&link.destination, &policy.allowed_url_schemes)
            {
                link.destination.clone()
            } else {
                String::new() // inert placeholder — renders as <a href="">
            };

            let children = sanitize_inline_children(&link.children, policy);
            vec![InlineNode::Link(LinkNode {
                destination,
                title: link.title.clone(),
                children,
            })]
        }

        // ── ImageNode ─────────────────────────────────────────────────────
        InlineNode::Image(img) => {
            // drop_images takes precedence over transform_image_to_text
            if policy.drop_images {
                return vec![];
            }

            if policy.transform_image_to_text {
                // Replace image with its alt text — no external resource load,
                // but the alt description is preserved for context.
                return vec![InlineNode::Text(TextNode {
                    value: img.alt.clone(),
                })];
            }

            let destination = if is_scheme_allowed(&img.destination, &policy.allowed_url_schemes) {
                img.destination.clone()
            } else {
                String::new() // inert — browsers won't fetch an empty src
            };

            vec![InlineNode::Image(ImageNode {
                destination,
                title: img.title.clone(),
                alt: img.alt.clone(),
            })]
        }

        // ── AutolinkNode ──────────────────────────────────────────────────
        // Autolinks are dropped entirely if the scheme is not allowed.
        // Unlike LinkNode, there is no link text to promote — the URL IS the
        // displayed text, and displaying "javascript:alert(1)" as plain text
        // is itself potentially confusing. Drop entirely.
        InlineNode::Autolink(al) => {
            if is_scheme_allowed(&al.destination, &policy.allowed_url_schemes) {
                vec![InlineNode::Autolink(AutolinkNode {
                    destination: al.destination.clone(),
                    is_email: al.is_email,
                })]
            } else {
                vec![]
            }
        }

        // ── RawInlineNode ─────────────────────────────────────────────────
        InlineNode::RawInline(ri) => {
            if policy.allow_raw_inline_formats.allows(&ri.format) {
                vec![InlineNode::RawInline(RawInlineNode {
                    format: ri.format.clone(),
                    value: ri.value.clone(),
                })]
            } else {
                vec![]
            }
        }

        // ── HardBreakNode ─────────────────────────────────────────────────
        InlineNode::HardBreak(_) => vec![InlineNode::HardBreak(HardBreakNode)],

        // ── SoftBreakNode ─────────────────────────────────────────────────
        InlineNode::SoftBreak(_) => vec![InlineNode::SoftBreak(SoftBreakNode)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{passthrough, relaxed, strict, MaxHeadingLevel, SanitizationPolicy};

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn text_node(s: &str) -> InlineNode {
        InlineNode::Text(TextNode { value: s.to_string() })
    }

    fn para(inlines: Vec<InlineNode>) -> BlockNode {
        BlockNode::Paragraph(ParagraphNode { children: inlines })
    }

    fn heading(level: u8, inlines: Vec<InlineNode>) -> BlockNode {
        BlockNode::Heading(HeadingNode { level, children: inlines })
    }

    fn raw_block(format: &str, value: &str) -> BlockNode {
        BlockNode::RawBlock(RawBlockNode {
            format: format.to_string(),
            value: value.to_string(),
        })
    }

    fn raw_inline(format: &str, value: &str) -> InlineNode {
        InlineNode::RawInline(RawInlineNode {
            format: format.to_string(),
            value: value.to_string(),
        })
    }

    fn link(dest: &str, children: Vec<InlineNode>) -> InlineNode {
        InlineNode::Link(LinkNode {
            destination: dest.to_string(),
            title: None,
            children,
        })
    }

    fn image(dest: &str, alt: &str) -> InlineNode {
        InlineNode::Image(ImageNode {
            destination: dest.to_string(),
            title: None,
            alt: alt.to_string(),
        })
    }

    fn autolink(dest: &str) -> InlineNode {
        InlineNode::Autolink(AutolinkNode {
            destination: dest.to_string(),
            is_email: false,
        })
    }

    fn code_span(value: &str) -> InlineNode {
        InlineNode::CodeSpan(CodeSpanNode { value: value.to_string() })
    }

    fn doc(children: Vec<BlockNode>) -> DocumentNode {
        DocumentNode { children }
    }

    // ─── Passthrough is identity ──────────────────────────────────────────────

    #[test]
    fn passthrough_preserves_all_nodes() {
        let d = doc(vec![
            heading(1, vec![text_node("Title")]),
            para(vec![text_node("Hello")]),
            raw_block("html", "<div>x</div>"),
        ]);
        let result = sanitize(&d, &passthrough());
        assert_eq!(result, d);
    }

    // ─── Immutability ─────────────────────────────────────────────────────────

    #[test]
    fn sanitize_does_not_mutate_input() {
        let original = doc(vec![para(vec![
            link("javascript:alert(1)", vec![text_node("click")]),
        ])]);
        let _ = sanitize(&original, &strict());
        // The original must still contain the original link
        if let BlockNode::Paragraph(p) = &original.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "javascript:alert(1)");
            }
        }
    }

    // ─── RawBlockNode handling ────────────────────────────────────────────────

    #[test]
    fn raw_block_drop_all() {
        let d = doc(vec![raw_block("html", "<b>hi</b>")]);
        let result = sanitize(&d, &strict());
        assert!(result.children.is_empty());
    }

    #[test]
    fn raw_block_passthrough() {
        let d = doc(vec![raw_block("html", "<b>hi</b>")]);
        let result = sanitize(&d, &passthrough());
        assert_eq!(result.children.len(), 1);
    }

    #[test]
    fn raw_block_allowlist_html_kept() {
        let d = doc(vec![raw_block("html", "<b>hi</b>")]);
        let result = sanitize(&d, &relaxed());
        assert_eq!(result.children.len(), 1);
    }

    #[test]
    fn raw_block_allowlist_latex_dropped() {
        let d = doc(vec![raw_block("latex", r"\textbf{hi}")]);
        let result = sanitize(&d, &relaxed());
        assert!(result.children.is_empty());
    }

    // ─── RawInlineNode handling ───────────────────────────────────────────────

    #[test]
    fn raw_inline_drop_all() {
        let d = doc(vec![para(vec![raw_inline("html", "<em>hi</em>")])]);
        let result = sanitize(&d, &strict());
        // Paragraph becomes empty → paragraph itself dropped
        assert!(result.children.is_empty());
    }

    #[test]
    fn raw_inline_passthrough() {
        let d = doc(vec![para(vec![raw_inline("html", "<em>hi</em>")])]);
        let result = sanitize(&d, &passthrough());
        assert_eq!(result.children.len(), 1);
    }

    // ─── Heading level clamping ───────────────────────────────────────────────

    #[test]
    fn heading_drop_all_headings() {
        let d = doc(vec![heading(1, vec![text_node("Title")])]);
        let policy = SanitizationPolicy {
            max_heading_level: MaxHeadingLevel::Drop,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        assert!(result.children.is_empty());
    }

    #[test]
    fn heading_clamp_min_level() {
        // h1 should be promoted to h2 when min_heading_level = 2
        let d = doc(vec![heading(1, vec![text_node("Title")])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Heading(h) = &result.children[0] {
            assert_eq!(h.level, 2);
        } else {
            panic!("Expected HeadingNode");
        }
    }

    #[test]
    fn heading_clamp_max_level() {
        // h5 should be clamped to h3 when max_heading_level = 3
        let d = doc(vec![heading(5, vec![text_node("Sub")])]);
        let policy = SanitizationPolicy {
            max_heading_level: MaxHeadingLevel::Level(3),
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        if let BlockNode::Heading(h) = &result.children[0] {
            assert_eq!(h.level, 3);
        } else {
            panic!("Expected HeadingNode");
        }
    }

    #[test]
    fn heading_within_range_unchanged() {
        let d = doc(vec![heading(3, vec![text_node("Mid")])]);
        let result = sanitize(&d, &strict()); // min=2, max=6
        if let BlockNode::Heading(h) = &result.children[0] {
            assert_eq!(h.level, 3);
        }
    }

    // ─── Image handling ───────────────────────────────────────────────────────

    #[test]
    fn drop_images_removes_image_node() {
        let d = doc(vec![para(vec![image("https://example.com/cat.png", "a cat")])]);
        let policy = SanitizationPolicy {
            drop_images: true,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        assert!(result.children.is_empty()); // para is empty → dropped
    }

    #[test]
    fn transform_image_to_text() {
        let d = doc(vec![para(vec![image("https://example.com/cat.png", "a cat")])]);
        let result = sanitize(&d, &strict()); // transform_image_to_text=true
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Text(t) = &p.children[0] {
                assert_eq!(t.value, "a cat");
            } else {
                panic!("Expected TextNode");
            }
        }
    }

    #[test]
    fn drop_images_takes_precedence_over_transform() {
        let d = doc(vec![para(vec![image("https://example.com/cat.png", "a cat")])]);
        let policy = SanitizationPolicy {
            drop_images: true,
            transform_image_to_text: true,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        // Image should be gone entirely, not converted to text
        assert!(result.children.is_empty());
    }

    #[test]
    fn image_unsafe_scheme_becomes_empty_destination() {
        let d = doc(vec![para(vec![image("javascript:alert(1)", "xss")])]);
        let policy = SanitizationPolicy {
            transform_image_to_text: false,
            ..strict()
        };
        let result = sanitize(&d, &policy);
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Image(img) = &p.children[0] {
                assert_eq!(img.destination, "");
            }
        }
    }

    // ─── Link handling ────────────────────────────────────────────────────────

    #[test]
    fn drop_links_promotes_children() {
        // [click here](https://example.com) → "click here" (plain text)
        let d = doc(vec![para(vec![
            link("https://example.com", vec![text_node("click here")]),
        ])]);
        let policy = SanitizationPolicy {
            drop_links: true,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        if let BlockNode::Paragraph(p) = &result.children[0] {
            assert_eq!(p.children.len(), 1);
            if let InlineNode::Text(t) = &p.children[0] {
                assert_eq!(t.value, "click here");
            }
        }
    }

    #[test]
    fn javascript_link_gets_empty_destination() {
        let d = doc(vec![para(vec![
            link("javascript:alert(1)", vec![text_node("click me")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "");
            }
        }
    }

    #[test]
    fn https_link_kept_unchanged() {
        let d = doc(vec![para(vec![
            link("https://example.com", vec![text_node("link")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "https://example.com");
            }
        }
    }

    #[test]
    fn relative_link_kept_unchanged() {
        let d = doc(vec![para(vec![
            link("../docs/index.html", vec![text_node("docs")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "../docs/index.html");
            }
        }
    }

    #[test]
    fn vbscript_link_blocked() {
        let d = doc(vec![para(vec![
            link("vbscript:MsgBox(1)", vec![text_node("click")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "");
            }
        }
    }

    #[test]
    fn data_url_link_blocked() {
        let d = doc(vec![para(vec![
            link("data:text/html,<script>alert(1)</script>", vec![text_node("x")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "");
            }
        }
    }

    // ─── AutolinkNode handling ────────────────────────────────────────────────

    #[test]
    fn autolink_javascript_dropped() {
        let d = doc(vec![para(vec![autolink("javascript:alert(1)")])]);
        let result = sanitize(&d, &strict());
        assert!(result.children.is_empty()); // para is empty → dropped
    }

    #[test]
    fn autolink_https_kept() {
        let d = doc(vec![para(vec![autolink("https://example.com")])]);
        let result = sanitize(&d, &strict());
        assert_eq!(result.children.len(), 1);
    }

    // ─── CodeSpanNode handling ────────────────────────────────────────────────

    #[test]
    fn code_span_to_text_when_policy_set() {
        let d = doc(vec![para(vec![code_span("const x = 1")])]);
        let policy = SanitizationPolicy {
            transform_code_span_to_text: true,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Text(t) = &p.children[0] {
                assert_eq!(t.value, "const x = 1");
            } else {
                panic!("Expected TextNode");
            }
        }
    }

    #[test]
    fn code_span_kept_when_policy_not_set() {
        let d = doc(vec![para(vec![code_span("const x = 1")])]);
        let result = sanitize(&d, &passthrough());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            assert!(matches!(&p.children[0], InlineNode::CodeSpan(_)));
        }
    }

    // ─── CodeBlockNode handling ───────────────────────────────────────────────

    #[test]
    fn code_block_dropped_when_policy_set() {
        let d = doc(vec![BlockNode::CodeBlock(CodeBlockNode {
            language: Some("rust".to_string()),
            value: "fn main() {}\n".to_string(),
        })]);
        let policy = SanitizationPolicy {
            drop_code_blocks: true,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        assert!(result.children.is_empty());
    }

    // ─── BlockquoteNode handling ──────────────────────────────────────────────

    #[test]
    fn blockquote_dropped_when_policy_set() {
        let d = doc(vec![BlockNode::Blockquote(BlockquoteNode {
            children: vec![para(vec![text_node("quoted")])],
        })]);
        let policy = SanitizationPolicy {
            drop_blockquotes: true,
            ..passthrough()
        };
        let result = sanitize(&d, &policy);
        assert!(result.children.is_empty());
    }

    // ─── Empty children cleanup ───────────────────────────────────────────────

    #[test]
    fn paragraph_with_only_raw_inline_dropped_when_raw_denied() {
        // When the only child of a paragraph is dropped, the paragraph itself
        // should be dropped — no empty <p></p> in output
        let d = doc(vec![para(vec![raw_inline("html", "<script>alert(1)</script>")])]);
        let result = sanitize(&d, &strict());
        assert!(result.children.is_empty());
    }

    #[test]
    fn empty_document_is_valid() {
        let d = doc(vec![]);
        let result = sanitize(&d, &strict());
        assert!(result.children.is_empty()); // DocumentNode kept even when empty
    }

    // ─── XSS vectors ─────────────────────────────────────────────────────────

    #[test]
    fn xss_nul_byte_javascript_link_blocked() {
        let d = doc(vec![para(vec![
            link("java\x00script:alert(1)", vec![text_node("click")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "");
            }
        }
    }

    #[test]
    fn xss_zero_width_space_bypass_blocked() {
        let d = doc(vec![para(vec![
            link("\u{200B}javascript:alert(1)", vec![text_node("click")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "");
            }
        }
    }

    #[test]
    fn xss_blob_url_blocked() {
        let d = doc(vec![para(vec![
            link("blob:https://origin/some-uuid", vec![text_node("x")]),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            if let InlineNode::Link(l) = &p.children[0] {
                assert_eq!(l.destination, "");
            }
        }
    }

    // ─── ThematicBreak, HardBreak, SoftBreak ─────────────────────────────────

    #[test]
    fn thematic_break_always_kept() {
        let d = doc(vec![BlockNode::ThematicBreak(ThematicBreakNode)]);
        let result = sanitize(&d, &strict());
        assert_eq!(result.children.len(), 1);
    }

    #[test]
    fn hard_break_always_kept() {
        let d = doc(vec![para(vec![
            text_node("line1"),
            InlineNode::HardBreak(HardBreakNode),
            text_node("line2"),
        ])]);
        let result = sanitize(&d, &strict());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            assert_eq!(p.children.len(), 3);
            assert!(matches!(&p.children[1], InlineNode::HardBreak(_)));
        }
    }

    #[test]
    fn soft_break_always_kept() {
        let d = doc(vec![para(vec![
            text_node("line1"),
            InlineNode::SoftBreak(SoftBreakNode),
            text_node("line2"),
        ])]);
        let result = sanitize(&d, &passthrough());
        if let BlockNode::Paragraph(p) = &result.children[0] {
            assert!(matches!(&p.children[1], InlineNode::SoftBreak(_)));
        }
    }
}

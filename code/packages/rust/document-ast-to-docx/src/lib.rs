//! # `document-ast-to-docx` — render a Document AST to a `.docx`
//!
//! The bridge from the shared, format-agnostic [`document_ast`] IR to a real Word
//! `.docx`, over the [`coding_adventures_docx_writer`] writer. It's the OOXML
//! sibling of [`document-ast-to-html`](https://docs.rs/) — where that renders a
//! [`DocumentNode`] to HTML, this renders it to WordprocessingML bytes.
//!
//! ```text
//!   document_ast::DocumentNode
//!        │  to_docx_document      (this crate — map blocks & inlines)
//!        ▼
//!   docx_writer::Document
//!        │  docx_writer::write_docx
//!        ▼
//!   .docx bytes
//! ```
//!
//! Because Markdown already parses to a [`DocumentNode`]
//! (`commonmark-parser::parse`), composing that with [`to_docx_bytes`] gives a
//! native **Markdown → `.docx`** converter — the point of spec
//! `code/specs/MD02-markdown-to-docx.md`. Any other frontend that targets the
//! Document AST (GFM, ASCIIDoc, HTML, …) gets `.docx` output for free.
//!
//! ## What maps to what
//!
//! Blocks become paragraphs (headings → `Heading N` style, code → `Code` style,
//! quotes → `Quote` style, list items → indented `ListParagraph`s with a bullet
//! or number prefix) and tables. Inlines flatten to formatted runs: `Strong` →
//! bold, `Emphasis` → italic, `CodeSpan` → monospace, combining down the tree so
//! `**_x_**` is a single bold+italic run.
//!
//! ## Fidelity (documented, per MD02 §6)
//!
//! - **Raw HTML** (`RawBlock`/`RawInline`) is **dropped**, never injected.
//! - **Links** render as `text (url)`; **images** as `[alt] (url)` — no clickable
//!   hyperlink or embedded media in v1.
//! - **Strikethrough** renders as plain text (the writer has no strike run flag).
//! - **Hard breaks** render as a space (the writer has no `<w:br/>`), a documented
//!   divergence from MD02 §4.2; a follow-up can add `<w:br/>` to `docx-writer`.
//! - **Lists** are prefixed paragraphs, not native Word numbering.
//! - **Table cells** carry flattened inline text (no per-cell bold header yet).
//!
//! Every limit is lossless for the visible *text*; a future enrichment is a
//! deliberate change, pinned by the tests.

#![forbid(unsafe_code)]

use coding_adventures_docx_writer::{write_docx, Document, ParagraphStyle, Run};
use document_ast::{
    BlockNode, DocumentNode, InlineNode, ListChildNode, ListNode, TableNode, TableRowNode,
};

/// Render a [`DocumentNode`] into a [`docx_writer::Document`](Document) ready for
/// [`write_docx`]. Pure and total: any tree yields a valid document (an empty
/// tree yields an empty document), never a panic.
pub fn to_docx_document(doc: &DocumentNode) -> Document {
    let mut out = Document::new();
    render_blocks(&doc.children, &mut out, Ctx::default());
    out
}

/// Render a [`DocumentNode`] straight to `.docx` bytes — `write_docx(&to_docx_document(doc))`.
///
/// The one-call entry a frontend uses: `to_docx_bytes(&commonmark_parser::parse(md))`
/// is a native Markdown → `.docx` conversion.
pub fn to_docx_bytes(doc: &DocumentNode) -> Vec<u8> {
    write_docx(&to_docx_document(doc))
}

// ===========================================================================
// Block rendering
// ===========================================================================

/// Maximum recursion depth for both the block and inline tree walkers. The AST
/// comes from UNTRUSTED Markdown, and the upstream `commonmark-parser` bounds
/// nesting NOWHERE (its container stack is iterative, so a one-line input of
/// `>>>>…> x` yields an arbitrarily deep `Blockquote` chain, and `***…*` an
/// arbitrarily deep emphasis chain). A recursive walker over that would overflow
/// the native stack — an uncatchable SIGSEGV, defeating this crate's "total,
/// panic-free" contract. So every recursive descent is bounded: past this depth
/// we stop descending (drop the over-deep subtree) rather than recurse. The limit
/// sits far above any legitimate document's nesting (256 levels).
const MAX_DEPTH: usize = 256;

/// The ambient state threaded through block rendering: how deep we are inside
/// lists (for indentation), whether we're inside a blockquote (paragraphs get the
/// `Quote` style), and the total recursion depth (bounded by [`MAX_DEPTH`]).
/// `Copy` so a child call gets its own adjusted copy.
#[derive(Clone, Copy, Default)]
struct Ctx {
    /// Nesting depth of the enclosing lists (0 = not in a list). Drives the list
    /// item's leading indentation.
    list_depth: usize,
    /// Inside a blockquote — paragraphs render with `ParagraphStyle::Quote`.
    in_quote: bool,
    /// Total block-recursion depth, checked against [`MAX_DEPTH`] to bound
    /// stack use on adversarially nested input.
    depth: usize,
}

/// Render a sequence of block nodes into `out`.
fn render_blocks(blocks: &[BlockNode], out: &mut Document, ctx: Ctx) {
    for block in blocks {
        render_block(block, out, ctx);
    }
}

/// Render one block node into `out`.
fn render_block(block: &BlockNode, out: &mut Document, ctx: Ctx) {
    // Bound stack use on adversarially nested input: past MAX_DEPTH, stop
    // descending. Leaf content already rendered above this depth is kept; the
    // over-deep subtree is dropped rather than overflowing the stack.
    if ctx.depth > MAX_DEPTH {
        return;
    }
    // Every recursive descent below happens one level deeper.
    let deeper = Ctx {
        depth: ctx.depth + 1,
        ..ctx
    };
    match block {
        // A nested document root — flatten its children (rare; keeps traversal total).
        BlockNode::Document(d) => render_blocks(&d.children, out, deeper),

        BlockNode::Heading(h) => {
            out.add_styled_paragraph(
                ParagraphStyle::Heading(h.level),
                inlines_to_runs(&h.children),
            );
        }

        BlockNode::Paragraph(p) => {
            let style = if ctx.in_quote {
                ParagraphStyle::Quote
            } else {
                ParagraphStyle::Normal
            };
            out.add_styled_paragraph(style, inlines_to_runs(&p.children));
        }

        // Each source line becomes its own monospace `Code` paragraph, preserving
        // line breaks. A single trailing newline (common on fenced blocks) is
        // stripped so it doesn't add a spurious blank paragraph.
        BlockNode::CodeBlock(c) => {
            let body = c.value.strip_suffix('\n').unwrap_or(&c.value);
            for line in body.split('\n') {
                out.add_styled_paragraph(ParagraphStyle::Code, vec![Run::plain(line).mono()]);
            }
        }

        // Child blocks render inside the quote context (their paragraphs take the
        // Quote style). Blockquotes can nest and hold any blocks.
        BlockNode::Blockquote(b) => {
            render_blocks(
                &b.children,
                out,
                Ctx {
                    in_quote: true,
                    ..deeper
                },
            );
        }

        BlockNode::List(l) => render_list(l, out, deeper),

        // A stray list item outside a List — render its blocks as a one-item list
        // at the current depth (defensive; a well-formed AST wraps items in List).
        BlockNode::ListItem(item) => render_item_blocks(&item.children, out, deeper, "• "),
        BlockNode::TaskItem(item) => {
            let mark = if item.checked { "☑ " } else { "☐ " };
            render_item_blocks(&item.children, out, deeper, mark);
        }

        // A horizontal rule — the writer has no paragraph borders, so we render a
        // visible box-drawing rule (a documented rendering choice).
        BlockNode::ThematicBreak(_) => {
            out.add_styled_paragraph(ParagraphStyle::Normal, vec![Run::plain("────────")]);
        }

        BlockNode::Table(t) => render_table(t, out),

        // Raw HTML is dropped, never injected into the .docx (MD02 §6).
        BlockNode::RawBlock(_) => {}

        // Table rows/cells never appear as top-level blocks in a well-formed AST;
        // ignore them defensively rather than panic.
        BlockNode::TableRow(_) | BlockNode::TableCell(_) => {}
    }
}

/// Render a list: each item becomes one paragraph, prefixed with a bullet (`• `)
/// or its number (`1. `, `2. `…, honouring the list's `start`), and indented by
/// its nesting depth. Nested lists recurse with a deeper `list_depth`.
fn render_list(list: &ListNode, out: &mut Document, ctx: Ctx) {
    if ctx.depth > MAX_DEPTH {
        return;
    }
    let mut number = list.start.unwrap_or(1);
    for child in &list.children {
        match child {
            ListChildNode::ListItem(item) => {
                let prefix = if list.ordered {
                    format!("{number}. ")
                } else {
                    "• ".to_string()
                };
                render_item_blocks(&item.children, out, ctx, &prefix);
                number = number.saturating_add(1);
            }
            ListChildNode::TaskItem(item) => {
                let mark = if item.checked { "☑ " } else { "☐ " };
                render_item_blocks(&item.children, out, ctx, mark);
            }
        }
    }
}

/// Render a list item's blocks. The item's FIRST paragraph gets the bullet/number
/// prefix + indentation; any further blocks (a nested list, a second paragraph)
/// render normally at a deeper list depth. This keeps `- a\n  - b` readable
/// (`• a` then an indented `• b`) without native Word numbering.
fn render_item_blocks(blocks: &[BlockNode], out: &mut Document, ctx: Ctx, prefix: &str) {
    if ctx.depth > MAX_DEPTH {
        return;
    }
    let indent: String = "    ".repeat(ctx.list_depth);
    let inner = Ctx {
        list_depth: ctx.list_depth + 1,
        depth: ctx.depth + 1,
        ..ctx
    };
    let mut first_para_done = false;
    for block in blocks {
        match block {
            // The item's leading paragraph carries the marker.
            BlockNode::Paragraph(p) if !first_para_done => {
                first_para_done = true;
                let mut runs = vec![Run::plain(&format!("{indent}{prefix}"))];
                runs.extend(inlines_to_runs(&p.children));
                out.add_styled_paragraph(ParagraphStyle::List, runs);
            }
            // Nested lists deepen the indentation.
            BlockNode::List(l) => render_list(l, out, inner),
            // Any other block in the item renders at the item's depth.
            other => render_block(other, out, inner),
        }
    }
}

/// Render a GFM table via the writer's plain-text table. Each cell is its inlines
/// flattened to text (no per-cell formatting in v1, so the header row isn't
/// bolded — a documented follow-up). An empty table yields an empty `<w:tbl>`.
fn render_table(table: &TableNode, out: &mut Document) {
    let rows: Vec<Vec<String>> = table
        .children
        .iter()
        .map(|row: &TableRowNode| {
            row.children
                .iter()
                .map(|c| inlines_to_plain(&c.children))
                .collect()
        })
        .collect();
    out.add_table(&rows);
}

// ===========================================================================
// Inline rendering
// ===========================================================================

/// The character formatting carried down the inline tree. Nested `Strong` /
/// `Emphasis` / `CodeSpan` combine (a code span inside a `**bold**` span is a
/// bold monospace run).
#[derive(Clone, Copy, Default)]
struct Fmt {
    bold: bool,
    italic: bool,
    mono: bool,
}

impl Fmt {
    /// A run of `text` carrying this formatting.
    fn run(self, text: &str) -> Run {
        let mut r = Run::plain(text);
        r.bold = self.bold;
        r.italic = self.italic;
        r.mono = self.mono;
        r
    }
}

/// Flatten a sequence of inline nodes into formatted runs (top-level, no ambient
/// formatting).
fn inlines_to_runs(inlines: &[InlineNode]) -> Vec<Run> {
    let mut runs = Vec::new();
    for node in inlines {
        push_inline(node, Fmt::default(), 0, &mut runs);
    }
    runs
}

/// Append the runs for one inline node, carrying `fmt` down into children.
///
/// `depth` bounds the inline recursion the same way the block walker is bounded:
/// nested emphasis/strong/link inlines come from untrusted Markdown with no
/// upstream cap (`***…*` nests arbitrarily), so past [`MAX_DEPTH`] we drop the
/// over-deep node rather than overflow the stack.
fn push_inline(node: &InlineNode, fmt: Fmt, depth: usize, runs: &mut Vec<Run>) {
    if depth > MAX_DEPTH {
        return;
    }
    let d = depth + 1;
    match node {
        InlineNode::Text(t) => runs.push(fmt.run(&t.value)),
        InlineNode::Strong(s) => push_children(&s.children, Fmt { bold: true, ..fmt }, d, runs),
        InlineNode::Emphasis(e) => push_children(
            &e.children,
            Fmt {
                italic: true,
                ..fmt
            },
            d,
            runs,
        ),
        // No strike run flag in the writer — render the text plainly (MD02 §6).
        InlineNode::Strikethrough(s) => push_children(&s.children, fmt, d, runs),
        InlineNode::CodeSpan(c) => runs.push(Fmt { mono: true, ..fmt }.run(&c.value)),
        // Link: its child text, then the URL in parentheses (no clickable hyperlink v1).
        InlineNode::Link(l) => {
            push_children(&l.children, fmt, d, runs);
            runs.push(fmt.run(&format!(" ({})", l.destination)));
        }
        // Image: "[alt] (url)" as plain text (no embedded media v1).
        InlineNode::Image(i) => {
            runs.push(fmt.run(&format!("[{}]", i.alt)));
            runs.push(fmt.run(&format!(" ({})", i.destination)));
        }
        InlineNode::Autolink(a) => runs.push(fmt.run(&a.destination)),
        // Hard/soft breaks both become a space (the writer has no <w:br/>).
        InlineNode::HardBreak(_) | InlineNode::SoftBreak(_) => runs.push(fmt.run(" ")),
        // Raw inline HTML is dropped, never injected.
        InlineNode::RawInline(_) => {}
    }
}

/// Push the runs for a child inline sequence under `fmt` at recursion depth `depth`.
fn push_children(children: &[InlineNode], fmt: Fmt, depth: usize, runs: &mut Vec<Run>) {
    for node in children {
        push_inline(node, fmt, depth, runs);
    }
}

/// Flatten inlines to a single plain-text string (for table cells) — formatting
/// is dropped, breaks become spaces, raw inline is dropped, images become their
/// alt text.
fn inlines_to_plain(inlines: &[InlineNode]) -> String {
    let mut s = String::new();
    for node in inlines {
        plain_inline(node, 0, &mut s);
    }
    s
}

/// Depth-bounded (see [`push_inline`]) plain-text flattening.
fn plain_inline(node: &InlineNode, depth: usize, s: &mut String) {
    if depth > MAX_DEPTH {
        return;
    }
    let d = depth + 1;
    match node {
        InlineNode::Text(t) => s.push_str(&t.value),
        InlineNode::CodeSpan(c) => s.push_str(&c.value),
        InlineNode::Autolink(a) => s.push_str(&a.destination),
        InlineNode::Strong(n) => n.children.iter().for_each(|c| plain_inline(c, d, s)),
        InlineNode::Emphasis(n) => n.children.iter().for_each(|c| plain_inline(c, d, s)),
        InlineNode::Strikethrough(n) => n.children.iter().for_each(|c| plain_inline(c, d, s)),
        InlineNode::Link(n) => n.children.iter().for_each(|c| plain_inline(c, d, s)),
        InlineNode::Image(i) => s.push_str(&i.alt),
        InlineNode::HardBreak(_) | InlineNode::SoftBreak(_) => s.push(' '),
        InlineNode::RawInline(_) => {}
    }
}

#[cfg(test)]
mod tests;

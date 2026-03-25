//! CommonMark 0.31.2 Parser
//!
//! Parses Markdown source text into a Document AST — the format-agnostic IR
//! defined in the `document-ast` crate. The result is a `DocumentNode`
//! ready for any back-end renderer (HTML, PDF, plain text, …).
//!
//! The parse is two-phase:
//!   Phase 1 — Block structure: headings, lists, code blocks, blockquotes, …
//!   Phase 2 — Inline content: emphasis, links, images, code spans, …
//!
//! # Quick Start
//!
//! ```rust
//! use commonmark_parser::parse;
//!
//! let doc = parse("# Hello\n\nWorld *with* emphasis.\n");
//! assert_eq!(doc.children.len(), 2);
//! ```

mod block_parser;
pub mod entities;
pub(crate) mod entities_table;
mod inline_parser;
pub mod scanner;

use document_ast::DocumentNode;
pub use block_parser::{LinkRefMap, LinkReference, FinalBlock, parse as parse_blocks};

/// Parse a CommonMark Markdown string into a `DocumentNode` AST.
///
/// The result conforms to the Document AST spec (TE00) — a format-agnostic IR
/// with all link references resolved and all inline markup parsed.
///
/// # Arguments
///
/// * `markdown` — The Markdown source string.
///
/// # Returns
///
/// The root `DocumentNode`.
///
/// # Examples
///
/// ```rust
/// use commonmark_parser::parse;
///
/// let doc = parse("## Heading\n\n- item 1\n- item 2\n");
/// assert_eq!(doc.children.len(), 2);
/// ```
pub fn parse(markdown: &str) -> DocumentNode {
    // Phase 1: Block parsing — builds the structural skeleton
    let (blocks, link_refs) = block_parser::parse(markdown);

    // Phase 2: Inline parsing — fills in emphasis, links, code spans, etc.
    inline_parser::resolve_document(blocks, &link_refs)
}

pub const VERSION: &str = "0.1.0";

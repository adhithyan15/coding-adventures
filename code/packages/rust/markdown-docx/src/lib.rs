//! # `markdown-docx` — native Markdown → `.docx`
//!
//! Turn Markdown into a real Word `.docx`, over the repo's own zero-dependency
//! stack, by routing it through the shared **Document AST**. This is the
//! terminal convenience crate of the `MD02` pipeline: it just composes two
//! already-tested stages —
//!
//! ```text
//!   Markdown  ──commonmark-parser::parse──▶  document_ast::DocumentNode
//!             ──document_ast_to_docx::to_docx_bytes──▶  .docx bytes
//! ```
//!
//! — so there is nothing format-specific here beyond wiring. All the block/inline
//! mapping (headings → `Heading N`, `**bold**` → bold runs, lists → prefixed
//! paragraphs, tables → `<w:tbl>`, raw-HTML dropped, …) and the recursion-depth
//! DoS guard live in [`document_ast_to_docx`]; the Markdown parsing lives in
//! [`commonmark_parser`] / [`gfm_parser`]. See `code/specs/MD02-markdown-to-docx.md`.
//!
//! ## Example
//!
//! ```
//! let docx = markdown_docx::markdown_to_docx("# Hi\n\nA **bold** word.\n");
//! assert_eq!(&docx[..2], b"PK"); // a .docx is a ZIP/OPC package
//! // std::fs::write("out.docx", docx)?; — opens in Word.
//! ```
//!
//! ## CommonMark vs GFM
//!
//! [`markdown_to_docx`] uses the strict CommonMark parser. [`gfm_to_docx`] uses
//! the GitHub-Flavored parser, which additionally recognizes pipe **tables**,
//! **task lists** (`- [x]`), **strikethrough**, and autolinks — all of which the
//! Document AST models and this pipeline renders.
//!
//! ## Why native (not pandoc)
//!
//! Same rationale as the rest of the repo: pandoc is a large external Haskell
//! binary; this is zero-dependency Rust that already reads and writes real Office
//! files. Routing through `document-ast` means every prose format the repo learns
//! reuses one IR and one set of bridges.

#![forbid(unsafe_code)]

use document_ast_to_docx::to_docx_bytes;

/// Convert **CommonMark** Markdown to the bytes of a valid `.docx`.
///
/// `markdown_to_docx(md) == document_ast_to_docx::to_docx_bytes(&commonmark_parser::parse(md))`.
/// Total and panic-free on any input (the depth guard in `document-ast-to-docx`
/// bounds adversarially nested Markdown); an empty string yields a valid empty
/// `.docx`.
pub fn markdown_to_docx(markdown: &str) -> Vec<u8> {
    to_docx_bytes(&commonmark_parser::parse(markdown))
}

/// Convert **GitHub-Flavored Markdown** to the bytes of a valid `.docx`.
///
/// Like [`markdown_to_docx`] but via the GFM parser, so pipe tables, task lists
/// (`- [x]` / `- [ ]`), strikethrough, and autolinks are recognized and rendered.
pub fn gfm_to_docx(markdown: &str) -> Vec<u8> {
    to_docx_bytes(&gfm_parser::parse(markdown))
}

#[cfg(test)]
mod tests;

//! # coding_adventures_document_ast_sanitizer
//!
//! Policy-driven AST sanitization for the Document IR pipeline.
//!
//! This crate performs a **pure, immutable tree transformation** on a
//! `DocumentNode` value, producing a sanitized copy according to a caller-
//! supplied `SanitizationPolicy`. The input is never mutated.
//!
//! ## Pipeline position
//!
//! ```text
//! parse(markdown)          ← TE01 — CommonMark Parser
//!       ↓
//! sanitize(doc, policy)    ← TE02 — document-ast-sanitizer (this crate)
//!       ↓
//! to_html(doc)             ← TE00 — document-ast-to-html
//!       ↓
//! final output
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use document_ast::{DocumentNode, BlockNode, ParagraphNode, InlineNode,
//!                    LinkNode, TextNode};
//! use coding_adventures_document_ast_sanitizer::{sanitize, strict};
//!
//! // Build a document with a javascript: link
//! let doc = DocumentNode {
//!     children: vec![
//!         BlockNode::Paragraph(ParagraphNode {
//!             children: vec![
//!                 InlineNode::Link(LinkNode {
//!                     destination: "javascript:alert(1)".to_string(),
//!                     title: None,
//!                     children: vec![InlineNode::Text(TextNode {
//!                         value: "click me".to_string(),
//!                     })],
//!                 }),
//!             ],
//!         }),
//!     ],
//! };
//!
//! let safe = sanitize(&doc, &strict());
//!
//! // The destination has been neutralised
//! if let BlockNode::Paragraph(p) = &safe.children[0] {
//!     if let InlineNode::Link(l) = &p.children[0] {
//!         assert_eq!(l.destination, "");
//!     }
//! }
//! ```
//!
//! ## Policy customisation
//!
//! The three named presets (`strict`, `relaxed`, `passthrough`) cover the most
//! common cases. For fine-grained control, build a `SanitizationPolicy`
//! directly using struct update syntax:
//!
//! ```rust
//! use coding_adventures_document_ast_sanitizer::policy::{SanitizationPolicy, relaxed};
//!
//! // Reserve h1 for the page title; all other RELAXED rules apply.
//! let custom = SanitizationPolicy {
//!     min_heading_level: 2,
//!     ..relaxed()
//! };
//! ```

pub mod policy;
pub mod sanitizer;
pub mod url_utils;

// Re-export the most common items so callers don't need to reach into submodules.

pub use policy::{
    passthrough, relaxed, strict, MaxHeadingLevel, RawFormatPolicy, SanitizationPolicy,
};
pub use sanitizer::sanitize;

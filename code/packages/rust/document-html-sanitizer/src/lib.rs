//! # coding_adventures_document_html_sanitizer
//!
//! Regex-based HTML string sanitizer — strips dangerous elements, attributes,
//! and URLs from an opaque HTML string with **no dependency on document-ast**.
//!
//! This crate is stage 2 of the TE02 Document Sanitization pipeline. Unlike
//! the AST sanitizer (stage 1), which operates on structured document nodes,
//! this crate accepts any HTML string and returns a sanitized string.
//!
//! ## When to use this crate
//!
//! - As a **safety net** after the AST sanitizer + renderer pipeline
//! - When HTML arrives from **external systems** (CMS APIs, user paste, etc.)
//! - When you need to sanitize HTML that was **not produced by your pipeline**
//!
//! ## Pipeline position
//!
//! ```text
//! sanitize(doc, policy)    ← TE02 stage 1 — document-ast-sanitizer
//!       ↓
//! to_html(doc)             ← TE00 — document-ast-to-html
//!       ↓
//! sanitize_html(html, pol) ← TE02 stage 2 — document-html-sanitizer (this crate)
//!       ↓
//! final output
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use coding_adventures_document_html_sanitizer::{sanitize_html, html_strict};
//!
//! let safe = sanitize_html(
//!     "<p>Hello</p><script>alert(1)</script>",
//!     &html_strict(),
//! );
//! assert_eq!(safe, "<p>Hello</p>");
//! ```

pub mod html_sanitizer;
pub mod policy;
pub mod url_utils;

pub use html_sanitizer::sanitize_html;
pub use policy::{html_passthrough, html_relaxed, html_strict, HtmlSanitizationPolicy};

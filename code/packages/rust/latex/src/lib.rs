//! # latex — a full-fidelity LaTeX parser
//!
//! A standalone parser for LaTeX **documents and math** (not just a math subset). It turns
//! LaTeX source into a faithful AST that any consumer can use — a reasoning engine, a
//! computer-algebra system, a renderer. It is the first frontend of the pluggable
//! [`math-frontend`](https://docs.rs/) framework (it will implement `MathFrontend` in a
//! later layer) and is also useful on its own.
//!
//! ## Honest scope
//!
//! LaTeX rests on TeX, whose macro layer is Turing-complete. This crate parses the full
//! LaTeX **surface** and supports the macro mechanisms authors actually use; the
//! programmable TeX tail (runtime `\catcode` reassignment, `\expandafter`/`\csname`,
//! `\if…` programming, external `\input`) is the documented asymptote and is surfaced as
//! an explicit "unsupported" node rather than mis-parsed. See `code/specs/LTX01-full-latex-parser.md`.
//!
//! ## Layers (built incrementally)
//!
//! 1. [`tokenize`] — a catcode-driven, **text-mode-primary** state machine from source to
//!    a flat [`Token`] stream. **Implemented (this release).**
//! 2. `parse` / `parse_math` — the structural and math parsers. *(Later layers.)*
//!
//! ## Example
//!
//! ```
//! use latex::{tokenize, TokenKind};
//! let toks = tokenize(r"Let $x$ be.").unwrap();
//! assert_eq!(toks[0].kind, TokenKind::Char('L'));
//! assert!(toks.iter().any(|t| matches!(t.kind, TokenKind::MathOn { .. })));
//! ```

pub mod catcode;
mod error;
mod lexer;
mod token;

pub use catcode::{catcode, Catcode};
pub use error::LexError;
pub use lexer::tokenize;
pub use token::{Span, Token, TokenKind};

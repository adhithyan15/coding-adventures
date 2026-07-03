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
//!    a flat [`Token`] stream.
//! 2. [`parse`] — the **structural** document parser: text, groups, commands
//!    (`\cmd[opt]{arg}`), environments (`\begin…\end`), and raw math islands → a [`Node`]
//!    tree, with [`Node::to_latex`] round-tripping. **Implemented (this release).**
//! 3. [`parse_math`] — the **math grammar** over each island's raw content: fractions,
//!    roots, scripts, big operators, functions, fenced groups, relations, and **math
//!    environments** (`matrix`/`pmatrix`/`cases`/`aligned`/… with `&` and `\\`) →
//!    [`MathNode`], with precedence and [`MathNode::to_latex`] round-tripping.
//!    [`Node::parsed_math`] parses a [`Node::Math`] island on demand. **Implemented
//!    (this release).**
//! 4. [`expand`] — an opt-in **macro pass** over the document tree: registers
//!    `\newcommand`/`\renewcommand`/`\providecommand` (positional `#1`..`#9`) and replaces
//!    uses by their bounded, recursively-expanded bodies. **Implemented (L4a).**
//! 5. **Verbatim** — `\verb`/`\verb*` ([`Node::Verb`]) and the `verbatim`/`verbatim*`
//!    environment ([`Node::VerbatimEnv`]) read their bodies **raw** (catcodes suspended).
//!    **Implemented (L5a/L5b).**
//! 6. [`recognize_accents`] — an opt-in pass folding text accents (`\'e`, `\c{c}`) into
//!    [`Node::Accent`]. **Implemented (L5c).**
//! 7. [`recognize_structure`] — an opt-in pass classifying generic commands into structure
//!    nodes: sectioning ([`Node::Section`]), cross-refs/citations ([`Node::CrossRef`]),
//!    preamble directives ([`Node::Preamble`]), and argument-form font commands
//!    ([`Node::Styled`]). **Implemented (L5d).**
//! 8. [`LatexMath`] — the `math-frontend` adapter: implements `math_frontend::MathFrontend`,
//!    lifting a math island's [`MathNode`] into the neutral `MathExpr`, so LaTeX plugs into the
//!    pluggable-frontend registry ([`registry`] / [`register_latex`]). **Implemented (L6).**
//!    Gated behind the default-on `frontend` cargo feature; `--no-default-features` keeps
//!    L0–L5 dependency-free.
//!
//! ## Example
//!
//! ```
//! use latex::{parse, parse_math, Node, MathNode};
//! let doc = parse(r"Let $x$ be \textbf{bold}.").unwrap();
//! assert!(matches!(doc[0], Node::Text(_)));
//! assert!(doc.iter().any(|n| matches!(n, Node::Math { .. })));
//!
//! let m = parse_math(r"\frac{12 \times 15}{3}").unwrap();
//! assert!(matches!(m, MathNode::Frac(..)));
//!
//! let grid = parse_math(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").unwrap();
//! assert!(matches!(grid, MathNode::Matrix { .. }));
//! ```

pub mod catcode;
mod ast;
mod error;
mod lexer;
mod macros;
mod math;
mod parser;
mod structure;
mod text;
mod token;

#[cfg(feature = "frontend")]
mod frontend;

#[cfg(feature = "frontend")]
pub use frontend::{register_latex, registry, LatexMath};

pub use ast::{document_to_latex, Node, SectionLevel};
pub use catcode::{catcode, Catcode};
pub use error::{LexError, ParseError};
pub use lexer::tokenize;
pub use macros::expand;
pub use math::{parse_math, MBinOp, MRelOp, MUnOp, MathNode};
pub use parser::parse;
pub use structure::recognize_structure;
pub use text::recognize_accents;
pub use token::{Span, Token, TokenKind};

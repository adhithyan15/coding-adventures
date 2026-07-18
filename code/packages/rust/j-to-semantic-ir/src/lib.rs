//! # j-to-semantic-ir
//!
//! J CST → narrow-waist Semantic IR, **v0.1.0** — task **MA-6e**, the last
//! remaining rollout item for J (see
//! [`MA06`](../../../specs/MA06-j-language.md) §6/§7 and
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)'s standing
//! convention that every math-language frontend also emits SIR from day
//! one, "built in this same wave rather than as a later retrofit" — MA06
//! §5's own words).
//!
//! J is APL's ASCII-respelled descendant, and this crate is built directly
//! on [`apl-to-semantic-ir`]'s design: it reuses the same SIR22 base cut
//! and SIR22 "APL addendum" (`Reduce`/`Scan`/`Shape`/`Reshape`/
//! `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`) that crate already
//! consumes, walking the [`parser::grammar_parser::GrammarASTNode`] CST
//! produced by `coding-adventures-j-parser`. J adds exactly one genuinely
//! new production APL never had — **trains** (`(f g)` hooks, `(f g h)`/
//! `(n g h)` forks, and `f@g` compose) — which lower to *nested
//! applications* of the same node types, needing no new SIR node of their
//! own (MA06 §5's own explicit instruction). J also has no in-scope outer
//! product (unlike APL), and adds two primitives with no APL analogue at
//! all (`#` tally/replicate, `^` exponential/power).
//!
//! ## Pipeline
//!
//! ```text
//! J source
//!    │
//!    ▼  coding_adventures_j_parser::try_parse_j(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  j_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR22 + SIR22 addendum)
//! ```
//!
//! ## Public API
//!
//! ```
//! use j_to_semantic_ir::compile_source;
//! let module = compile_source("a=.3+4\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope (v0.1.0)
//!
//! See `lower.rs`'s module doc comment for the exact per-construct lowering
//! table (including the full J-verb-to-SIR-node mapping table), the train
//! (hook/fork/compose) lowering design and its dedicated combinator-depth
//! guard, and the handful of deliberately-rejected constructs.

mod lower;
pub use lower::{compile, JLowerError};

/// Parse `source` as J and lower it into a [`semantic_ir::Module`] in one
/// step, mirroring every other `-to-semantic-ir` frontend's `compile_source`
/// convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, JLowerError> {
    let tree = coding_adventures_j_parser::try_parse_j(source).map_err(|msg| JLowerError {
        message: format!("parse error: {msg}"),
        line: 1,
        column: 1,
    })?;
    compile(&tree, module_name)
}

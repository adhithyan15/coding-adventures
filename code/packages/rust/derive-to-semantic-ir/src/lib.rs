//! # derive-to-semantic-ir
//!
//! Derive CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! This is the **third** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
//! `wolfram-to-semantic-ir` (the first) and `macsyma-to-semantic-ir` (the
//! second). It consumes the generic `GrammarASTNode` CST produced by the
//! `coding-adventures-derive-parser` crate and emits a
//! [`semantic_ir::Module`]. See `lower.rs`'s module doc comment for the
//! full scope, the "retarget `derive-runtime`, not start from scratch"
//! design note, and the disclosed no-pattern-matching scope boundary
//! (Derive's grammar has no pattern or rewrite-rule syntax at all).
//!
//! ## Pipeline
//!
//! ```text
//! Derive source
//!    │
//!    ▼  coding_adventures_derive_parser::try_parse_derive(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  derive_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use derive_to_semantic_ir::compile_source;
//! let module = compile_source("1 + 2\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, DeriveLowerError};

/// Parse `source` as Derive and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
///
/// Unlike `wolfram-to-semantic-ir::compile_source` (which spawns an
/// enlarged-stack worker thread because Wolfram's 20-rule precedence
/// cascade makes its own parser's `MAX_RULE_DEPTH` unsafe on a bare
/// stack), this function needs no worker thread at all:
/// `coding_adventures_derive_parser`'s own `MAX_RULE_DEPTH` (200) is
/// already documented — see that crate's `src/lib.rs` doc comment — as
/// safe on a bare default (~2 MiB) stack with comfortable margin. That doc
/// comment's own measured numbers make the case even more strongly than
/// `macsyma-parser`'s: `derive-parser`'s bare-stack crash floor sits at
/// **298** `parse_rule` frames (vs. `macsyma-parser`'s/`wolfram-parser`'s
/// own ~275-278), with `MAX_RULE_DEPTH` set to 200 — about 33% below that
/// floor — and its own test suite (`test_opt_in_cap_trips_before_overflow_
/// on_default_stack`) directly confirms 5,000 levels of nesting past the
/// cap still fails cleanly with a `Result::Err` on a bare default-stack
/// thread, never a crash. So this crate mirrors `macsyma-to-semantic-ir`'s
/// (and `matlab-to-semantic-ir`'s / `apl-to-semantic-ir`'s) simple,
/// worker-thread-free `compile_source` shape rather than
/// `wolfram-to-semantic-ir`'s.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, DeriveLowerError> {
    let tree =
        coding_adventures_derive_parser::try_parse_derive(source).map_err(|msg| DeriveLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        })?;
    compile(&tree, module_name)
}

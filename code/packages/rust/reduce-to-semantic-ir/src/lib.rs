//! # reduce-to-semantic-ir
//!
//! Reduce CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! This is the **fourth** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
//! `wolfram-to-semantic-ir` (the first), `macsyma-to-semantic-ir` (the
//! second), and `derive-to-semantic-ir` (the third). It consumes the
//! generic `GrammarASTNode` CST produced by the
//! `coding-adventures-reduce-parser` crate and emits a
//! [`semantic_ir::Module`]. See `lower.rs`'s module doc comment for the
//! full scope, the "retarget `reduce-runtime`, not start from scratch"
//! design note, the arithmetic-head-naming divergence from MA08 §3's own
//! prose, the disclosed missing-handler gap this crate structurally
//! mirrors but does not paper over, and the no-pattern-matching scope
//! boundary (Reduce's grammar has no pattern or rewrite-rule syntax in
//! this subset).
//!
//! ## Pipeline
//!
//! ```text
//! Reduce source
//!    │
//!    ▼  coding_adventures_reduce_parser::try_parse_reduce(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  reduce_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use reduce_to_semantic_ir::compile_source;
//! let module = compile_source("1 + 2;\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, ReduceLowerError};

/// Parse `source` as Reduce and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
///
/// Like `derive-to-semantic-ir::compile_source` and
/// `macsyma-to-semantic-ir::compile_source` (and unlike
/// `wolfram-to-semantic-ir::compile_source`, which spawns an
/// enlarged-stack worker thread because Wolfram's 20-rule precedence
/// cascade makes its own parser's `MAX_RULE_DEPTH` unsafe on a bare
/// stack), this function needs no worker thread at all:
/// `coding_adventures_reduce_parser`'s own `MAX_RULE_DEPTH` (128) is
/// already documented — see that crate's `src/lib.rs` doc comment — as
/// safe on a bare default (~2 MiB) stack with comfortable margin. That doc
/// comment measures FIVE independent recursion shapes (parenthesised
/// nesting, a `:=` chain, an `if`/`else` chain, a cons (`.`) chain, and a
/// power (`^`) chain) in *rule-frame* terms and finds the binding
/// constraint is the cons chain's floor of 179 rule frames — `128` sits
/// about 28.5% below that floor, a comparable margin to
/// `derive-parser`'s own ~33% and `macsyma-parser`'s/`wolfram-parser`'s
/// ~275-278-frame floors. Its own test suite directly confirms input
/// thousands of levels/links past the cap still fails cleanly with a
/// `Result::Err` on a bare default-stack thread, never a crash. So this
/// crate mirrors `macsyma-to-semantic-ir`'s (and `derive-to-semantic-ir`'s
/// / `matlab-to-semantic-ir`'s / `apl-to-semantic-ir`'s) simple,
/// worker-thread-free `compile_source` shape rather than
/// `wolfram-to-semantic-ir`'s.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, ReduceLowerError> {
    let tree =
        coding_adventures_reduce_parser::try_parse_reduce(source).map_err(|msg| ReduceLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        })?;
    compile(&tree, module_name)
}

//! # maple-to-semantic-ir
//!
//! Maple CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! This is the **fifth and final** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
//! `wolfram-to-semantic-ir` (the first), `macsyma-to-semantic-ir` (the
//! second), `derive-to-semantic-ir` (the third), and
//! `reduce-to-semantic-ir` (the fourth). It consumes the generic
//! `GrammarASTNode` CST produced by the `coding-adventures-maple-parser`
//! crate and emits a [`semantic_ir::Module`]. See `lower.rs`'s module doc
//! comment for the full scope, the "retarget `maple-runtime`, not start
//! from scratch" design note, the statement-vs-expression dispatch split
//! `maple.grammar` forces (unlike Reduce's unified `expr` production), the
//! new `Set` head, and the no-pattern-matching scope boundary (Maple's
//! grammar has no pattern or rewrite-rule syntax in this subset).
//!
//! ## Pipeline
//!
//! ```text
//! Maple source
//!    │
//!    ▼  coding_adventures_maple_parser::try_parse_maple(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  maple_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use maple_to_semantic_ir::compile_source;
//! let module = compile_source("1 + 2;\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, MapleLowerError};

/// Parse `source` as Maple and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
///
/// Like `derive-to-semantic-ir::compile_source`,
/// `macsyma-to-semantic-ir::compile_source`, and
/// `reduce-to-semantic-ir::compile_source` (and unlike
/// `wolfram-to-semantic-ir::compile_source`, which spawns an
/// enlarged-stack worker thread because Wolfram's 20-rule precedence
/// cascade makes its own parser's `MAX_RULE_DEPTH` unsafe on a bare
/// stack), this function needs no worker thread at all:
/// `coding_adventures_maple_parser`'s own `MAX_RULE_DEPTH` (150) is
/// already documented — see that crate's `src/lib.rs` doc comment — as
/// safe on a bare default (~2 MiB) stack with comfortable margin. That doc
/// comment measures SIX independent recursion shapes (parenthesised
/// nesting, list/set-literal nesting, a `not`-prefix chain, a unary-minus
/// chain, a power (`^`) chain, and a nested `if`/`end if` chain) in
/// *rule-frame* terms and finds the binding constraint is the `not`-chain's
/// floor of 218 rule frames — `150` sits about 31.2% below that floor, a
/// comparable margin to `reduce-parser`'s own ~28.5%, `apl-parser`'s
/// ~26.5%, `j-parser`'s ~30%, and `derive-parser`'s ~33%. Its own test
/// suite directly confirms input thousands of levels/links past the cap
/// still fails cleanly with a `Result::Err` on a bare default-stack
/// thread, never a crash. So this crate mirrors
/// `macsyma-to-semantic-ir`'s (and `derive-to-semantic-ir`'s /
/// `reduce-to-semantic-ir`'s) simple, worker-thread-free `compile_source`
/// shape rather than `wolfram-to-semantic-ir`'s.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, MapleLowerError> {
    let tree =
        coding_adventures_maple_parser::try_parse_maple(source).map_err(|msg| MapleLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        })?;
    compile(&tree, module_name)
}

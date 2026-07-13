//! # macsyma-to-semantic-ir
//!
//! Macsyma CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! This is the **second** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — sibling to
//! `wolfram-to-semantic-ir`, the first. It consumes the generic
//! `GrammarASTNode` CST produced by the `coding-adventures-macsyma-parser`
//! crate and emits a [`semantic_ir::Module`]. See `lower.rs`'s module doc
//! comment for the full scope, the "retarget `macsyma-compiler`, not start
//! from scratch" design note, and the disclosed no-pattern-matching scope
//! boundary (Macsyma's currently-implemented grammar has no pattern or
//! rewrite-rule syntax at all).
//!
//! ## Pipeline
//!
//! ```text
//! Macsyma source
//!    │
//!    ▼  coding_adventures_macsyma_parser::create_macsyma_parser(src).parse()
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  macsyma_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR23)
//! ```
//!
//! ## Public API
//!
//! ```
//! use macsyma_to_semantic_ir::compile_source;
//! let module = compile_source("1 + 2$\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```

mod lower;
pub use lower::{compile, MacsymaLowerError};

/// Parse `source` as Macsyma and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
///
/// Unlike `wolfram-to-semantic-ir::compile_source` (which spawns an
/// enlarged-stack worker thread because Wolfram's 20-rule precedence
/// cascade makes its own parser's `MAX_RULE_DEPTH` unsafe on a bare
/// stack), this function needs no worker thread at all:
/// `coding_adventures_macsyma_parser`'s own `MAX_RULE_DEPTH` (200) is
/// already documented — see that crate's `src/lib.rs` doc comment — as
/// safe on a bare default (~2 MiB) stack with comfortable margin (measured
/// crash floor ~275-278 `parse_rule` frames on that stack size; 200 sits
/// ~28% below it). This mirrors `matlab-to-semantic-ir`'s and
/// `apl-to-semantic-ir`'s identically simple `compile_source` shape rather
/// than `wolfram-to-semantic-ir`'s worker-thread pattern, since this
/// crate's own parser has the same bare-stack-safety property those two
/// crates' parsers do (and `wolfram-parser`'s does not).
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, MacsymaLowerError> {
    let mut parser = coding_adventures_macsyma_parser::create_macsyma_parser(source);
    let tree = parser.parse().map_err(|err| MacsymaLowerError {
        message: format!("parse error: {}", err.message),
        line: err.token.line,
        column: err.token.column,
    })?;
    compile(&tree, module_name)
}

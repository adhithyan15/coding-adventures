//! # apl-to-semantic-ir
//!
//! APL CST → narrow-waist Semantic IR, **v0.1.0** — task **MA-4f**, the last
//! remaining rollout item for APL (see
//! [`MA05`](../../../specs/MA05-apl-language.md) §5/§6 and
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)'s standing
//! convention that every math-language frontend also emits SIR from day
//! one).
//!
//! This is the first frontend to actually *consume* the SIR22 addendum
//! (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/
//! `IndexOf`/`Ravel`/`Catenate`, plus the `Max`/`Min`/`Eq`/`Ne`/`Lt`/`Le`/
//! `Ge`/`Gt` `ElementwiseOpKind` variants) that `semantic-ir` shipped
//! specifically for APL ahead of this crate landing — see that addendum's
//! own doc comment in `semantic-ir/src/nodes.rs`. It walks the
//! [`parser::grammar_parser::GrammarASTNode`] CST produced by
//! `coding-adventures-apl-parser` and emits a [`semantic_ir::Module`].
//!
//! ## Pipeline
//!
//! ```text
//! APL source
//!    │
//!    ▼  coding_adventures_apl_parser::try_parse_apl(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  apl_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR22 + SIR22 addendum)
//! ```
//!
//! ## Public API
//!
//! ```
//! use apl_to_semantic_ir::compile_source;
//! let module = compile_source("A←3+4\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope (v0.1.0)
//!
//! APL's grammar (per MA05 §4, as implemented by `apl-parser`/`apl-runtime`)
//! has **no control flow and no user-defined functions** in this cut — just
//! straight-line assignment and value expressions. That makes this frontend
//! genuinely simpler than `matlab-to-semantic-ir` in two concrete ways:
//!
//! 1. Every one of APL's 12 scalar dyadic atoms (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`)
//!    lowers **unconditionally** to [`semantic_ir::Expr::ElementwiseOp`] —
//!    there is no MATLAB-`*`-style non-elementwise alternate reading to
//!    disambiguate, so this frontend has no scalar-vs-array heuristic to
//!    write at all (contrast `matlab-to-semantic-ir`'s `expr_is_known_scalar`).
//! 2. The whole program lowers into a single `main` [`semantic_ir::Function`]
//!    — there are no separate named functions to collect in a first pass the
//!    way MATLAB's `func_def`s require.
//!
//! See `lower.rs`'s module doc comment for the exact per-construct lowering
//! table, the chained-assignment unrolling design, the auto-print
//! convention, and the handful of deliberately-rejected constructs (reduce/
//! scan used dyadically, outer product used monadically, `⍴`/`⍳`/`,`
//! decorated with an operator) — each is syntactically constructible by the
//! grammar but semantically invalid, and gets a clean [`AplLowerError`]
//! rather than a silent misinterpretation.

mod lower;
pub use lower::{compile, AplLowerError};

/// Parse `source` as APL and lower it into a [`semantic_ir::Module`] in one
/// step, mirroring every other `-to-semantic-ir` frontend's `compile_source`
/// convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, AplLowerError> {
    let tree = coding_adventures_apl_parser::try_parse_apl(source).map_err(|msg| AplLowerError {
        message: format!("parse error: {msg}"),
        line: 1,
        column: 1,
    })?;
    compile(&tree, module_name)
}

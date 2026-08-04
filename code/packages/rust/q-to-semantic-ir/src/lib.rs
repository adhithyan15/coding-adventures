//! # q-to-semantic-ir
//!
//! Q (kdb+'s scripting language) CST → narrow-waist Semantic IR, **v0.1.0**
//! — task **MA-11e**, per [`MA11`](../../../specs/MA11-q-language.md) §5/§6:
//! built alongside the runtime in this same wave rather than as a later
//! retrofit, mirroring APL's/J's/Scilab's own precedent (per
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md) §2's amended
//! per-language pattern).
//!
//! Q is APL/J's second-generation descendant (MA11 §1) and reuses their
//! exact two-nonterminal, right-to-left, no-precedence grammar shape (MA11
//! §3) — so this crate is built directly on [`j-to-semantic-ir`]'s design,
//! the most structurally similar prior frontend (same `noun_expr`/`term`/
//! `verb_expr` shape, same primitive-verb dispatch pattern), reusing SIR22
//! plus the "APL/J addendum" (`Reduce`/`Scan`/`Ravel`/`Catenate`/
//! `IndexGenerator`) those two crates already established. Q adds exactly
//! one genuinely new concept neither APL nor J ever needed: a real
//! user-defined function literal, `{[x;y] stmt; stmt; ...}` — see
//! `lower.rs`'s module doc comment for the full design (why it lowers to an
//! ordinary SIR [`semantic_ir::Function`] plus [`semantic_ir::Expr::MakeClosure`]/
//! [`semantic_ir::Expr::DirectCall`]/[`semantic_ir::Expr::IndirectCall`],
//! not a brand-new SIR node kind).
//!
//! ## Pipeline
//!
//! ```text
//! Q source
//!    │
//!    ▼  coding_adventures_q_parser::try_parse_q(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  q_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR22 + SIR22 addendum)
//! ```
//!
//! ## Public API
//!
//! ```
//! use q_to_semantic_ir::compile_source;
//! let module = compile_source("f:{x+y}\n2 f 3\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! // The user-defined function `f` lowers to its OWN top-level
//! // `semantic_ir::Function` (params `x`, `y`), not a bare closure value --
//! // see `lower.rs`'s module doc comment for why.
//! assert!(module.functions.len() >= 2);
//! ```
//!
//! ## Scope (v0.1.0)
//!
//! See `lower.rs`'s module doc comment for the exact per-construct lowering
//! table (the full Q-primitive-to-SIR-node mapping, including which five
//! primitives needed genuinely new `BuiltinCall` names with no APL/J
//! precedent), the function-literal lowering design (the one new lowering
//! surface this crate adds relative to its `j-to-semantic-ir` model), and
//! the handful of deliberately-rejected constructs.

mod lower;
pub use lower::{compile, QLowerError};

/// Parse `source` as Q and lower it into a [`semantic_ir::Module`] in one
/// step, mirroring every other `-to-semantic-ir` frontend's `compile_source`
/// convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, QLowerError> {
    let tree = coding_adventures_q_parser::try_parse_q(source).map_err(|msg| QLowerError {
        message: format!("parse error: {msg}"),
        line: 1,
        column: 1,
    })?;
    compile(&tree, module_name)
}

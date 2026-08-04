//! # idl-to-semantic-ir
//!
//! IDL (Interactive Data Language) CST → narrow-waist Semantic IR,
//! **v0.1.0** — task **MA-12e**, the final item in IDL's Wave-6 rollout per
//! [`MA12`](../../../specs/MA12-idl-language.md) §6, and front3 of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)'s "every math
//! language also builds a `<lang>-to-semantic-ir` frontend" pipeline stage.
//!
//! Targets [SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md), the
//! array/matrix domain extension of the SIR10 narrow-waist IR, plus KW1's
//! keyword-argument vocabulary — the same domains
//! `matlab-to-semantic-ir`/`scilab-to-semantic-ir`/`apl-to-semantic-ir`/
//! `j-to-semantic-ir`/`q-to-semantic-ir` already target. It consumes the
//! generic `GrammarASTNode` CST produced by the
//! `coding-adventures-idl-parser` crate and emits a [`semantic_ir::Module`].
//!
//! Per MA12 §5/§6, this crate is built *alongside*
//! `idl-runtime`/`idl-repl` in the same rollout wave (not bolted on
//! afterward) — but it is a wholly separate, ahead-of-time lowering pass
//! over the same CST shape `idl-runtime`'s tree-walking evaluator walks,
//! not a consumer of that evaluator (this crate has no dependency on
//! `coding-adventures-idl-runtime` at all, mirroring every sibling
//! `-to-semantic-ir` frontend's identical choice).
//!
//! ## Pipeline
//!
//! ```text
//! IDL source
//!    │
//!    ▼  coding_adventures_idl_parser::try_parse_idl(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  coding_adventures_idl_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR16 + SIR22 + KW1)
//! ```
//!
//! ## Public API
//!
//! ```
//! use coding_adventures_idl_to_semantic_ir::compile_source;
//! let module = compile_source("x = 1 + 2\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope (v0.1.0)
//!
//! See `lower.rs`'s module doc comment for the exact supported-construct
//! list and the documented, deliberate scope limits — most notably the two
//! semantic details this task's own brief flags as the exact class of bug
//! this session's `idl-runtime` review already caught once: the `#`-vs-`##`
//! matrix-product operand order, and the 2-D subscript column/row swap
//! between IDL's own `[column, row]` source order and SIR's `[row, column]`
//! `IndexGet`/`IndexSet` convention. Both are verified directly against
//! `idl-runtime`'s own (already-fixed) source, not re-derived independently
//! — see `lower.rs`'s own dedicated sections for the full account.

mod lower;
pub use lower::{compile, IdlLowerError};

/// Parse `source` as IDL and lower it into a [`semantic_ir::Module`] in one
/// step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, IdlLowerError> {
    let tree =
        coding_adventures_idl_parser::try_parse_idl(source).map_err(|msg| IdlLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        })?;
    compile(&tree, module_name)
}

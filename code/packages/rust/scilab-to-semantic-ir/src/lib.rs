//! # scilab-to-semantic-ir
//!
//! Scilab CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! Targets [SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md), the
//! array/matrix domain extension of the SIR10 narrow-waist IR (see
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)) — the same
//! domain `matlab-to-semantic-ir`/`apl-to-semantic-ir`/`j-to-semantic-ir`
//! already target. It consumes the generic `GrammarASTNode` CST produced by
//! the `coding-adventures-scilab-parser` crate and emits a
//! [`semantic_ir::Module`].
//!
//! Per [`MA10`](../../../specs/MA10-scilab-language.md) §5/§6, this crate is
//! built *alongside* `scilab-runtime`/`scilab-repl` in the same rollout wave
//! (MA-10e, the last item), not bolted on afterward — but it is a wholly
//! separate, ahead-of-time lowering pass over the same CST shape
//! `scilab-runtime`'s tree-walking evaluator walks, not a consumer of that
//! evaluator (this crate has no dependency on `coding-adventures-scilab-runtime`
//! at all, mirroring `apl-to-semantic-ir`/`j-to-semantic-ir`'s identical
//! choice).
//!
//! ## Pipeline
//!
//! ```text
//! Scilab source
//!    │
//!    ▼  coding_adventures_scilab_parser::try_parse_scilab(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  scilab_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR16 + SIR22)
//! ```
//!
//! ## Public API
//!
//! ```
//! use scilab_to_semantic_ir::compile_source;
//! let module = compile_source("x = 1 + 2;\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope (v0.1.0)
//!
//! Scilab is close to MATLAB in grammar shape but not in language (MA10
//! §1); this first cut covers MA10 §4's own in-scope surface and returns a
//! clean [`ScilabLowerError`] — rather than silently mis-lowering — for
//! anything outside it. See `lower.rs`'s module doc comment for the exact
//! supported-construct list and the documented, deliberate scope limits
//! (`$`-relative indexing, `%i`, multi-output functions, `break`/`continue`,
//! stepped/non-range `for`, matrix division, cell arrays/`list`/`tlist`/
//! `mlist`, and any operator over string literals).

mod lower;
pub use lower::{compile, ScilabLowerError};

/// Parse `source` as Scilab and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, ScilabLowerError> {
    let tree = coding_adventures_scilab_parser::try_parse_scilab(source).map_err(|msg| {
        ScilabLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        }
    })?;
    compile(&tree, module_name)
}

//! # matlab-to-semantic-ir
//!
//! MATLAB CST → narrow-waist Semantic IR, **v0.1.0**.
//!
//! This is the first frontend to target [SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md),
//! the array/matrix domain extension of the SIR10 narrow-waist IR (see
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)). It consumes the
//! generic `GrammarASTNode` CST produced by the `coding-adventures-matlab-parser`
//! crate and emits a [`semantic_ir::Module`].
//!
//! ## Pipeline
//!
//! ```text
//! MATLAB source
//!    │
//!    ▼  coding_adventures_matlab_parser::try_parse_matlab(src)
//! parser::grammar_parser::GrammarASTNode   (generic CST)
//!    │
//!    ▼  matlab_to_semantic_ir::compile
//! semantic_ir::Module                      (per SIR10 + SIR16 + SIR22)
//! ```
//!
//! ## Public API
//!
//! ```
//! use matlab_to_semantic_ir::compile_source;
//! let module = compile_source("x = 1 + 2;\n", "demo").unwrap();
//! assert!(module.functions.iter().any(|f| f.name == "main"));
//! ```
//!
//! ## Scope (v0.1.0)
//!
//! MATLAB is a large language; this first cut covers a well-defined subset
//! and returns a clean [`MatlabLowerError`] — rather than silently
//! mis-lowering — for anything outside it. See `lower.rs`'s module doc
//! comment for the exact supported-construct list and the documented,
//! deliberate scope limits (stepped/matrix-valued `for` loops, matrix
//! division `/`/`\`, matrix power, multi-output functions, `end`-relative
//! indexing, `break`/`continue`/`return`, `switch`/`try`/cell arrays/
//! lambdas/`global`).

mod lower;
pub use lower::{compile, MatlabLowerError};

/// Parse `source` as MATLAB and lower it into a [`semantic_ir::Module`] in
/// one step, mirroring every other `-to-semantic-ir` frontend's
/// `compile_source` convenience wrapper.
pub fn compile_source(
    source: &str,
    module_name: &str,
) -> Result<semantic_ir::Module, MatlabLowerError> {
    let tree = coding_adventures_matlab_parser::try_parse_matlab(source).map_err(|msg| {
        MatlabLowerError {
            message: format!("parse error: {msg}"),
            line: 1,
            column: 1,
        }
    })?;
    compile(&tree, module_name)
}

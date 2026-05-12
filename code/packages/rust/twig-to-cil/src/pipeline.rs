//! # pipeline — the four-stage Twig → CLR CIL compilation pipeline.
//!
//! This module wires the four compilation stages into a single `run_pipeline`
//! function.  Each stage transforms the intermediate representation one step
//! closer to CLR CIL bytecode:
//!
//! ```text
//! Twig source (str)
//!     │
//!     ▼  Stage 1 — twig_ir_compiler::compile_source
//! IIRModule   (type_hint = "any" everywhere; dynamically typed)
//!     │
//!     ▼  Stage 2 — iir_type_checker::infer_and_check  (mutates module)
//! IIRModule   (type_hint = "i64", "bool", … where statically inferrable)
//!     │
//!     ▼  Stage 3 — iir_builtin_lowering::lower_builtins  (mutates module)
//! IIRModule   (call_builtin "+", "-", … → add, sub, …)
//!     │
//!     ▼  Stage 4 — iir_to_cil_bytecode::lower_iir_to_cil
//! CILProgramArtifact  (structured multi-method CLR artifact)
//! ```
//!
//! ## Stage 1 — Twig frontend
//!
//! `twig_ir_compiler::compile_source` lexes, parses, and compiles the Twig
//! source string into an `IIRModule`.  All instructions carry `type_hint =
//! "any"` because Twig is dynamically typed; subsequent passes fill this in.
//!
//! ## Stage 2 — type inference + checking
//!
//! `iir_type_checker::infer_and_check` runs an SSA-propagation pass that
//! fills in `type_hint` for instructions whose type can be determined from
//! constants or arithmetic.  For `(+ 1 2)` this gives `i64` to the constants
//! and the `add` result.
//!
//! The return type is `TypeCheckReport` (not `Result`).  We inspect
//! `report.ok()` and abort the pipeline if the report contains fatal errors.
//!
//! ## Stage 3 — builtin lowering
//!
//! `iir_builtin_lowering::lower_builtins` rewrites `call_builtin "+"` →
//! `add`, `call_builtin "<"` → `lt`, etc.  This pass is infallible — builtins
//! not in the lowering table are left as `call_builtin` for the VM to dispatch.
//!
//! ## Stage 4 — CLR backend
//!
//! `iir_to_cil_bytecode::lower_iir_to_cil` validates the module for CLR
//! constraints, then emits CIL bytecode per function.  The result is a
//! `CILProgramArtifact` — a structured multi-method artifact ready for the
//! CLR simulator or a PE-file packager.

use iir_builtin_lowering::lower_builtins;
use iir_to_cil_bytecode::{lower_iir_to_cil, validate_iir_for_clr, IIRClrConfig, CILProgramArtifact};
use iir_type_checker::infer_and_check;
use interpreter_ir::IIRModule;
use twig_ir_compiler::compile_source;

use crate::error::TwigToCilError;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the full Twig → CLR CIL compilation pipeline.
///
/// This is the internal workhorse called by `compile_twig_to_cil` in
/// `lib.rs`.  Exposed here so callers who need a custom [`IIRClrConfig`]
/// (e.g. a non-default assembly name) can access the plumbing directly.
///
/// # Arguments
///
/// * `source` — Twig source code string.
/// * `module_name` — Name attached to the `IIRModule` (used in error messages
///   and the artifact's assembly label).
/// * `config` — CLR backend configuration (assembly name).
///
/// # Returns
///
/// A [`CILProgramArtifact`] with one [`CILMethodArtifact`] per Twig function.
/// Each method artifact carries a `body: Vec<u8>` with the raw CIL bytecode
/// for that method.
///
/// # Errors
///
/// Returns [`TwigToCilError`] on any pipeline failure.  See the error module
/// for variants.
///
/// # Example
///
/// ```rust,no_run
/// use twig_to_cil::pipeline::run_pipeline;
/// use iir_to_cil_bytecode::IIRClrConfig;
///
/// let config = IIRClrConfig::new("Demo");
/// // The result may be Ok or a CLR-stage error.
/// let result = run_pipeline("(+ 1 2)", "demo", config);
/// println!("{}", result.is_ok());
/// ```
pub fn run_pipeline(
    source: &str,
    module_name: &str,
    config: IIRClrConfig,
) -> Result<CILProgramArtifact, TwigToCilError> {
    // ── Stage 1: Twig source → IIRModule ─────────────────────────────────
    //
    // The frontend lexes, parses, and compiles the source.  All IIR
    // instructions have `type_hint = "any"` at this point.
    let mut module = compile_source(source, module_name)?;

    // ── Stage 2: type inference + checking ───────────────────────────────
    //
    // SSA-propagation fills in concrete types for constants and arithmetic.
    // `infer_and_check` is not fallible by `Result` — we check `report.ok()`.
    let report = infer_and_check(&mut module);
    if !report.ok() {
        let messages: Vec<String> = report
            .errors
            .iter()
            .map(|e| format!("{e}"))
            .collect();
        return Err(TwigToCilError::TypeCheck(messages));
    }

    // ── Stage 3: builtin lowering ─────────────────────────────────────────
    //
    // Rewrites `call_builtin "+"` → `add`, `call_builtin "<"` → `lt`, etc.
    // Must run after type-checker so the type_hint is concrete.  Infallible.
    lower_builtins(&mut module);

    // ── Stage 4: CLR backend ──────────────────────────────────────────────
    //
    // Pre-flight validation first, so we get a structured error message
    // rather than a panic from the lowering pass.
    let validation_errors = validate_iir_for_clr(&module);
    if !validation_errors.is_empty() {
        return Err(TwigToCilError::ClrValidation(validation_errors));
    }

    let artifact = lower_iir_to_cil(&module, &config)?;

    Ok(artifact)
}

// ---------------------------------------------------------------------------
// Entry point for pre-built typed IIR
// ---------------------------------------------------------------------------

/// Run stages 3-4 of the pipeline on a pre-built, already-typed `IIRModule`.
///
/// Useful for tests and callers that construct fully-typed IIR directly,
/// bypassing the Twig frontend.  The module must have concrete type hints
/// on all instructions; `"any"` type hints will cause validation to fail.
///
/// ## Stages run
///
/// - Stage 3 (builtin lowering) — rewrites `call_builtin` ops in place.
/// - Stage 4 (CLR backend) — validates and lowers to `CILProgramArtifact`.
///
/// # Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
/// use twig_to_cil::pipeline::run_pipeline_from_iir;
/// use twig_to_cil::IIRClrConfig;
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let mut module = IIRModule::new("test", "tetrad");
/// module.entry_point = Some("main".into());
/// module.add_or_replace(fn_);
///
/// let artifact = run_pipeline_from_iir(module, IIRClrConfig::default()).unwrap();
/// assert!(!artifact.methods.is_empty());
/// assert!(!artifact.methods[0].body.is_empty());
/// ```
pub fn run_pipeline_from_iir(
    mut module: IIRModule,
    config: IIRClrConfig,
) -> Result<CILProgramArtifact, TwigToCilError> {
    // Stage 3: builtin lowering.
    lower_builtins(&mut module);

    // Stage 4: CLR backend.
    let validation_errors = validate_iir_for_clr(&module);
    if !validation_errors.is_empty() {
        return Err(TwigToCilError::ClrValidation(validation_errors));
    }

    let artifact = lower_iir_to_cil(&module, &config)?;
    Ok(artifact)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> IIRClrConfig {
        IIRClrConfig::new("TestAssembly")
    }

    /// `(+ 1 2)` — reaches the CLR stage without a frontend error.
    #[test]
    fn pipeline_reaches_clr_stage() {
        let result = run_pipeline("(+ 1 2)", "test", default_config());
        // Must not be a Compile or TypeCheck error — valid Twig.
        assert!(!matches!(result, Err(TwigToCilError::Compile(_))));
        assert!(!matches!(result, Err(TwigToCilError::TypeCheck(_))));
    }

    /// `run_pipeline_from_iir` with typed IIR produces a CILProgramArtifact.
    #[test]
    fn pipeline_from_typed_iir_succeeds() {
        use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
            "i32",
            vec![
                IIRInstr::new("add", Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
            ],
        );
        let mut module = IIRModule::new("test", "tetrad");
        module.entry_point = Some("add".into());
        module.add_or_replace(fn_);
        let artifact = run_pipeline_from_iir(module, default_config()).unwrap();
        assert!(!artifact.methods.is_empty());
        assert!(!artifact.methods[0].body.is_empty());
    }

    #[test]
    fn pipeline_rejects_broken_source() {
        let result = run_pipeline("(+ 1", "bad", default_config());
        assert!(
            matches!(result, Err(TwigToCilError::Compile(_))),
            "expected Compile error for broken source"
        );
    }
}

//! # pipeline — the four-stage Twig → JVM compilation pipeline.
//!
//! This module wires the four compilation stages into a single
//! `run_pipeline` function.  Each stage transforms the IR one step
//! closer to JVM bytecode:
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
//!     ▼  Stage 4 — iir_to_jvm_class_file::lower_iir_to_jvm
//! JvmClassFile  (structured multi-method JVM class file)
//! ```
//!
//! ## Stage 1 — Twig frontend
//!
//! `twig_ir_compiler::compile_source` lexes, parses, and compiles the Twig
//! source string into an `IIRModule`.  All instructions carry `type_hint =
//! "any"` because Twig is dynamically typed; the type-checker in Stage 2
//! propagates concrete types where it can.
//!
//! ## Stage 2 — type inference + checking
//!
//! `iir_type_checker::infer_and_check` runs an SSA-propagation pass that
//! fills in `type_hint` for instructions whose type can be determined from
//! constants or arithmetic.  It then validates the enriched module.
//!
//! The return type is `TypeCheckReport`, not `Result`.  We inspect
//! `report.ok()` and abort the pipeline if the report contains fatal errors.
//!
//! Note: for Twig programs, most instructions remain `"any"` because Twig
//! is dynamically typed and the JVM backend validates the module before
//! lowering (it rejects `"any"` type_hint on arithmetic ops).  For simple
//! programs like `(+ 1 2)` the type-checker infers `i64` for constants and
//! results.
//!
//! ## Stage 3 — builtin lowering
//!
//! `iir_builtin_lowering::lower_builtins` rewrites `call_builtin "+"` →
//! `add`, `call_builtin "<"` → `lt`, etc.  This pass is infallible — builtins
//! not in the table are left as `call_builtin` for the VM to handle.
//!
//! ## Stage 4 — JVM backend
//!
//! `iir_to_jvm_class_file::lower_iir_to_jvm` translates the IIR into a
//! `JvmClassFile`.  The backend validates the module first and returns
//! `IIRJvmError` if any unsupported opcode or `"any"` type_hint appears.

use iir_builtin_lowering::lower_builtins;
use iir_to_jvm_class_file::{lower_iir_to_jvm, validate_for_jvm, IIRJvmConfig, JvmClassFile};
use iir_type_checker::infer_and_check;
use interpreter_ir::IIRModule;
use twig_ir_compiler::compile_source;

use crate::error::TwigToJvmError;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the full Twig → JVM compilation pipeline.
///
/// This is the internal workhorse called by `compile_twig_to_jvm` in
/// `lib.rs`.  Exposed here so callers who want to supply a custom
/// `IIRJvmConfig` (e.g. a non-default class name) can access the
/// plumbing directly.
///
/// # Arguments
///
/// * `source` — Twig source code string.
/// * `module_name` — Name attached to the `IIRModule` (used in error
///   messages and debug output).
/// * `config` — JVM backend configuration (class name, target version, …).
///
/// # Returns
///
/// A `JvmClassFile` ready for use by the CLR simulator, a test, or any
/// downstream tool.  The class file is *structured* — individual methods
/// can be inspected without parsing raw bytes.
///
/// # Errors
///
/// Returns `TwigToJvmError` on any pipeline failure.  See [`TwigToJvmError`]
/// for the variants.
///
/// # Example
///
/// ```rust,no_run
/// use twig_to_jvm::pipeline::run_pipeline;
/// use iir_to_jvm_class_file::IIRJvmConfig;
///
/// let config = IIRJvmConfig::new("Demo");
/// // The result may be Ok or a JVM-stage error depending on type inference.
/// let result = run_pipeline("(+ 1 2)", "demo", config);
/// println!("{result:?}");
/// ```
pub fn run_pipeline(
    source: &str,
    module_name: &str,
    config: IIRJvmConfig,
) -> Result<JvmClassFile, TwigToJvmError> {
    // ── Stage 1: Twig source → IIRModule ─────────────────────────────────
    //
    // `compile_source` lexes, parses, and compiles.  All instructions have
    // `type_hint = "any"` at this point (Twig is dynamically typed).
    let mut module = compile_source(source, module_name)?;

    // ── Stage 2: type inference + checking ───────────────────────────────
    //
    // Runs SSA-propagation to fill in concrete types where possible.
    // For Twig programs the constants (Int/Bool) get i64/bool; arithmetic
    // results propagate from there.
    //
    // `infer_and_check` is infallible in the sense that it always returns a
    // TypeCheckReport rather than a Result.  We treat fatal type errors as a
    // pipeline failure, but we don't abort on warnings.
    let report = infer_and_check(&mut module);
    if !report.ok() {
        let messages: Vec<String> = report
            .errors
            .iter()
            .map(|e| format!("{e}"))
            .collect();
        return Err(TwigToJvmError::TypeCheck(messages));
    }

    // ── Stage 3: builtin lowering ─────────────────────────────────────────
    //
    // Rewrites `call_builtin "+"` → `add`, `call_builtin "<"` → `lt`, etc.
    // Must run after type-checker so the destination type_hint is concrete.
    // This function is infallible — unrecognised builtins are left in place.
    lower_builtins(&mut module);

    // ── Stage 4: JVM backend ──────────────────────────────────────────────
    //
    // Validates the module for JVM constraints (no `"any"` type_hint on
    // arithmetic, no unsupported builtins, …) then emits a JvmClassFile.
    //
    // Pre-flight validation: `validate_for_jvm` is separate from lowering so
    // we can give a clear error message without a half-constructed output.
    let validation_errors = validate_for_jvm(&module);
    if !validation_errors.is_empty() {
        // Wrap validation errors as a JvmBackend error.  We pick the first
        // error for the IIRJvmError message since IIRJvmError is a structured
        // enum; the full list is visible in the Debug representation.
        return Err(TwigToJvmError::JvmValidation(validation_errors));
    }

    let class_file = lower_iir_to_jvm(&module, &config)?;

    Ok(class_file)
}

// ---------------------------------------------------------------------------
// Entry point for pre-built IIR
// ---------------------------------------------------------------------------

/// Run stages 3-4 of the pipeline on a pre-built, already-typed `IIRModule`.
///
/// This entry point is for callers that have already produced a typed
/// `IIRModule` (e.g. from a language that emits typed IIR, or from a test
/// that constructs IIR directly) and want to run just the backend stages:
///
/// - Stage 3 (builtin lowering) — rewrites `call_builtin` ops in place.
/// - Stage 4 (JVM backend) — validates and lowers to `JvmClassFile`.
///
/// Use this when the IIR is known to be fully typed and free of unsupported
/// builtins.
///
/// # Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
/// use twig_to_jvm::pipeline::run_pipeline_from_iir;
/// use twig_to_jvm::IIRJvmConfig;
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let module = IIRModule {
///     name: "test".into(),
///     functions: vec![fn_],
///     entry_point: Some("main".into()),
///     language: "test".into(),
/// };
///
/// let class_file = run_pipeline_from_iir(module, IIRJvmConfig::new("Test")).unwrap();
/// assert_eq!(class_file.methods.len(), 1);
/// ```
pub fn run_pipeline_from_iir(
    mut module: IIRModule,
    config: IIRJvmConfig,
) -> Result<JvmClassFile, TwigToJvmError> {
    // Stage 3: builtin lowering (idempotent for already-typed IIR).
    lower_builtins(&mut module);

    // Stage 4: JVM backend (validates internally).
    let validation_errors = validate_for_jvm(&module);
    if !validation_errors.is_empty() {
        return Err(TwigToJvmError::JvmValidation(validation_errors));
    }

    let class_file = lower_iir_to_jvm(&module, &config)?;
    Ok(class_file)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> IIRJvmConfig {
        IIRJvmConfig::new("TestClass")
    }

    /// `(+ 1 2)` — reaches the JVM stage (may succeed or produce a JVM-stage error).
    #[test]
    fn pipeline_simple_addition_reaches_jvm_stage() {
        let result = run_pipeline("(+ 1 2)", "test", default_config());
        // The pipeline must not produce a Compile or TypeCheck error for valid Twig.
        assert!(
            !matches!(result, Err(TwigToJvmError::Compile(_))),
            "valid Twig must not produce a Compile error"
        );
        assert!(
            !matches!(result, Err(TwigToJvmError::TypeCheck(_))),
            "valid Twig must not produce a TypeCheck error"
        );
        // Either success or JVM-stage error is acceptable.
    }

    /// `run_pipeline_from_iir` with typed IIR produces a JvmClassFile.
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
        let module = IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("add".into()),
            language: "test".into(),
        };
        let config = IIRJvmConfig::new("AddTest");
        let cf = run_pipeline_from_iir(module, config).unwrap();
        assert_eq!(cf.this_class_name, "AddTest");
        assert!(!cf.methods.is_empty());
    }

    /// `run_pipeline_from_iir` class name flows through config.
    #[test]
    fn pipeline_from_iir_configured_class_name() {
        use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};
        let fn_ = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        );
        let module = IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "test".into(),
        };
        let config = IIRJvmConfig::new("MyApp");
        let cf = run_pipeline_from_iir(module, config).unwrap();
        assert_eq!(cf.this_class_name, "MyApp");
    }

    #[test]
    fn pipeline_rejects_broken_source() {
        let result = run_pipeline("(+ 1", "bad", default_config());
        assert!(
            matches!(result, Err(TwigToJvmError::Compile(_))),
            "expected Compile error, got {result:?}"
        );
    }
}

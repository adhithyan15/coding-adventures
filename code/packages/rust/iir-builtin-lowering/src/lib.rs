//! # iir-builtin-lowering — Phase 1 numeric builtin lowering pass.
//!
//! This crate implements the first phase of the LANG31 lowering pipeline.
//! It transforms `call_builtin` instructions for arithmetic and comparison
//! operations into the typed IIR opcodes that the `iir-to-*` backends
//! (`iir-to-beam`, `iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`)
//! can directly lower to target bytecode.
//!
//! ## Pipeline position
//!
//! ```text
//! twig-ir-compiler  →  iir-type-checker  →  iir-builtin-lowering  →  iir-to-<target>
//! ```
//!
//! **This pass MUST run after `iir-type-checker`.**  The type checker promotes
//! `"any"` type hints to concrete types (e.g. `"i64"`, `"bool"`).  The backends
//! need concrete types on arithmetic instructions.  If this pass runs before the
//! type checker, it would produce `add`, `sub`, etc. instructions with `"any"`
//! type hints — which the backends reject.
//!
//! ## What this pass does (Phase 1)
//!
//! For each `call_builtin` instruction whose builtin name appears in the numeric
//! lowering table, the instruction is rewritten in place:
//!
//! ```text
//! BEFORE:  %r0 = call_builtin("+", %a, %b) : i64
//! AFTER:   %r0 = add(%a, %b) : i64
//! ```
//!
//! The `type_hint`, `dest`, and all profiling fields (`observed_slot`,
//! `observed_type`, `observation_count`, `deopt_anchor`, `ic_slot`) are
//! preserved from the original instruction.  Only `op`, `srcs`, and `may_alloc`
//! change.
//!
//! ## Lowering table (18 numeric builtins)
//!
//! | Builtin   | Arity | IIR op emitted |
//! |-----------|:-----:|----------------|
//! | `+`       | 2     | `add`          |
//! | `-`       | 2     | `sub`          |
//! | `*`       | 2     | `mul`          |
//! | `/`       | 2     | `div`          |
//! | `%`       | 2     | `mod`          |
//! | `neg`     | 1     | `neg`          |
//! | `=`       | 2     | `cmp_eq`       |
//! | `!=`      | 2     | `cmp_ne`       |
//! | `<`       | 2     | `cmp_lt`       |
//! | `<=`      | 2     | `cmp_le`       |
//! | `>`       | 2     | `cmp_gt`       |
//! | `>=`      | 2     | `cmp_ge`       |
//! | `and`     | 2     | `and`          |
//! | `or`      | 2     | `or`           |
//! | `not`     | 1     | `not`          |
//! | `shl`     | 2     | `shl`          |
//! | `shr`     | 2     | `shr`          |
//! | `xor`     | 2     | `xor`          |
//!
//! ## What this pass does NOT touch (Phase 1)
//!
//! - `call_builtin "cons"`, `"car"`, `"cdr"`, `"null?"` — Phase 2 (LANG31).
//! - `call_builtin "make_closure"`, `"apply_closure"` — BEAM02 / CLR02.
//! - `call_builtin "global_set"`, `"global_get"`, `"print"` — LANG27.
//! - Any other unrecognised builtin name — left entirely unchanged.
//!
//! ## Error handling
//!
//! `lower_builtins` returns a `Vec<BuiltinLoweringError>` collecting all
//! problems found across all functions.  This is intentionally non-short-
//! circuiting: accumulating all errors gives better diagnostics.
//!
//! ## Quick start
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_builtin_lowering::lower_builtins;
//!
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
//!     "i64",
//!     vec![
//!         IIRInstr::new(
//!             "call_builtin",
//!             Some("%r0".into()),
//!             vec![
//!                 Operand::Var("+".into()),
//!                 Operand::Var("a".into()),
//!                 Operand::Var("b".into()),
//!             ],
//!             "i64",
//!         ),
//!         IIRInstr::new("ret", None, vec![Operand::Var("%r0".into())], "i64"),
//!     ],
//! );
//! let mut module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("add".into()),
//!     language: "twig".into(),
//! };
//!
//! let errors = lower_builtins(&mut module);
//! assert!(errors.is_empty());
//!
//! let instr = &module.functions[0].instructions[0];
//! assert_eq!(instr.op, "add");
//! assert_eq!(instr.srcs.len(), 2);
//! assert_eq!(instr.type_hint, "i64");
//! ```

pub mod error;
pub mod numeric;

// We keep the `lower` module for backward compatibility with any code that
// already imports from it, but the canonical implementation is now in
// `numeric.rs`.  The public API exported here is the primary interface.
pub mod lower;

// Re-export the public API at the crate root.
pub use error::BuiltinLoweringError;

use interpreter_ir::IIRModule;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Lower all recognised `call_builtin` instructions in `module` to typed IIR
/// ops, mutating `module` in place.
///
/// Returns a (possibly empty) list of errors.  If the list is non-empty, some
/// instructions could not be lowered; they are left in the state they were in
/// at the time the error was detected.
///
/// **Ordering invariant:** this function must be called after `iir-type-checker`
/// has promoted all `"any"` type hints to concrete types.  Calling it on an
/// un-checked module will produce `BuiltinLoweringError::UntypedBuiltin` errors.
pub fn lower_builtins(module: &mut IIRModule) -> Vec<BuiltinLoweringError> {
    let mut all_errors = Vec::new();
    for fn_ in &mut module.functions {
        let errors = numeric::lower_function(fn_);
        all_errors.extend(errors);
    }
    all_errors
}

/// Same as [`lower_builtins`], but returns a fresh `IIRModule` and leaves the
/// original untouched.
///
/// Useful for tooling (debugger, LSP, coverage instrumentation) that needs to
/// display the pre-lowering module alongside the post-lowering module.
///
/// Returns a tuple `(lowered_module, errors)`.
///
/// # Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use iir_builtin_lowering::lower_builtins_cloned;
///
/// let fn_ = IIRFunction::new(
///     "f",
///     vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
///     "i64",
///     vec![
///         IIRInstr::new(
///             "call_builtin",
///             Some("%r0".into()),
///             vec![
///                 Operand::Var("+".into()),
///                 Operand::Var("a".into()),
///                 Operand::Var("b".into()),
///             ],
///             "i64",
///         ),
///     ],
/// );
/// let original = IIRModule {
///     name: "test".into(),
///     functions: vec![fn_],
///     entry_point: None,
///     language: "twig".into(),
/// };
///
/// let (lowered, errors) = lower_builtins_cloned(&original);
/// assert!(errors.is_empty());
///
/// // Original unchanged — still call_builtin.
/// assert_eq!(original.functions[0].instructions[0].op, "call_builtin");
/// // Lowered copy has the typed op.
/// assert_eq!(lowered.functions[0].instructions[0].op, "add");
/// ```
pub fn lower_builtins_cloned(module: &IIRModule) -> (IIRModule, Vec<BuiltinLoweringError>) {
    let mut cloned = module.clone();
    let errors = lower_builtins(&mut cloned);
    (cloned, errors)
}

/// Same as `lower_builtins` but returns `Err` if any error occurs.
///
/// Useful in pipeline crates where any lowering failure is fatal (the source
/// is already type-checked, so any error indicates a pipeline bug).
pub fn lower_builtins_checked(module: &mut IIRModule) -> Result<(), Vec<BuiltinLoweringError>> {
    let errors = lower_builtins(module);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

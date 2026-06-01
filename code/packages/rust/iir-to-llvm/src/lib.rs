//! # iir-to-llvm — IIR → textual LLVM IR backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `String` containing valid
//! LLVM textual IR (a `.ll` source file).
//!
//! ## Why a new crate?
//!
//! The existing IIR backends (wasm / JVM / CLR / BEAM) all target *managed*
//! runtimes that own register allocation, memory layout, GC, and exception
//! handling.  LLVM is a different beast: it is an AOT-native target whose
//! output runs on the bare metal of whatever CPU LLVM ships a backend for,
//! with the user's choice of LLVM optimization quality (`opt -O0` …
//! `opt -O3`) in front of it.
//!
//! Adding LLVM as a backend gives us:
//!
//! 1. A **second AOT path** alongside our hand-rolled aarch64 / x86_64
//!    emitters.  The hand-rolled emitters give us full encoding control (for
//!    the debugger story); LLVM gives us world-class O2 optimization for
//!    free.
//! 2. A **direct comparison axis** — same IIR, two AOT-native code
//!    generators, what does each do well?
//! 3. A **bridge to every CPU LLVM ships a backend for** (Apple Silicon,
//!    x86_64, RISC-V, MIPS, PowerPC, …) without writing per-CPU encoders.
//!
//! ## Why textual LLVM IR (not `llvm-sys`)?
//!
//! - **Zero build-time dep.**  We emit a `String`; CI does not need LLVM
//!   installed.  `cargo install` ships a tiny crate.
//! - **Debuggability.**  The output IS the human-readable form.  No FFI ABI
//!   drift, no opaque builder API — just strings we can `assert!` on.
//! - **Forward-compat.**  If we later want JIT execution via `llvm-sys`, we
//!   can add a second emitter alongside the textual one without breaking
//!   callers.
//!
//! ## Pipeline
//!
//! ```text
//! IIRModule
//!   → validate_for_llvm()     pre-flight, returns Vec<String>
//!   → lower_iir_to_llvm()     two-pass, returns String (the .ll source)
//!   → (optional) llc / opt    user runs these — out of scope for this crate
//!   → object file → linker → native executable
//! ```
//!
//! ## Scope of v0.1.0 (LLVM01)
//!
//! This release is a **skeleton**: it emits a valid empty LLVM module —
//! a `; ModuleID` comment and a `target triple` directive — but does **not**
//! lower any IIR instructions.  Instruction lowering arrives in v0.2.0+
//! (LLVM02–04, see [`code/specs/MULTILANG-BACKEND-PLAN.md`][plan]).
//!
//! [plan]: ../../specs/MULTILANG-BACKEND-PLAN.md
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_llvm::{validate_for_llvm, lower_iir_to_llvm, IIRLlvmConfig};
//!
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![],
//!     entry_point: None,
//!     language: "demo".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! // v0.1.0: validator always returns empty (no rules yet).
//! assert!(validate_for_llvm(&module).is_empty());
//!
//! let ll = lower_iir_to_llvm(&module, &IIRLlvmConfig::default())
//!     .expect("lowering should succeed");
//! assert!(ll.contains("target triple ="));
//! ```

use interpreter_ir::IIRModule;
use std::fmt;

// ---------------------------------------------------------------------------
// IIRLlvmConfig
// ---------------------------------------------------------------------------

/// Configuration for the IIR → LLVM textual IR lowering pass.
///
/// `target_triple` is what LLVM calls the CPU+OS+ABI triple that downstream
/// `llc` will assume.  We default to `"x86_64-unknown-linux-gnu"` — a fixed
/// string — so test output is deterministic across machines.  Override via
/// [`IIRLlvmConfig::with_target`] when you want to actually run the `.ll`.
///
/// We deliberately do NOT call out to `rustc -vV` or any host-detection
/// helper at build time: that would make doctests host-dependent and create
/// a cross-compilation footgun.  Better to make the override explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRLlvmConfig {
    /// Module name — emitted in the `; ModuleID = '<name>'` comment.
    pub module_name: String,
    /// LLVM target triple — emitted in `target triple = "<triple>"`.
    pub target_triple: String,
}

impl IIRLlvmConfig {
    /// Build a config with a custom module name; keeps the default triple.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            ..Self::default()
        }
    }

    /// Override the LLVM target triple.  Use when you actually intend to run
    /// `llc` on the output for a non-default architecture.
    pub fn with_target(mut self, triple: impl Into<String>) -> Self {
        self.target_triple = triple.into();
        self
    }
}

impl Default for IIRLlvmConfig {
    /// Fixed-host default — picks `x86_64-unknown-linux-gnu` for determinism.
    ///
    /// This is NOT the running host's triple — it's a deliberate fixed value
    /// so that test output is byte-identical no matter where the tests run.
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// IIRLlvmError
// ---------------------------------------------------------------------------

/// Errors that can occur during IIR → LLVM IR lowering.
///
/// Every variant carries at minimum the function name where the error
/// occurred, except for `ValidationFailed` which aggregates pre-flight errors
/// before any function-specific work has started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRLlvmError {
    /// The module failed pre-flight validation (see [`validate_for_llvm`]).
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0 does not
    /// lower any instructions, so any non-empty function body returns this.
    UnsupportedOp { function: String, op: String },
    /// A type hint that this backend does not know how to lower.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRLlvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationFailed(errs) => {
                write!(f, "validation failed:\n  {}", errs.join("\n  "))
            }
            Self::UnsupportedOp { function, op } => {
                write!(f, "unsupported op in function {function:?}: {op}")
            }
            Self::UnsupportedType { function, type_hint } => {
                write!(
                    f,
                    "unsupported type hint in function {function:?}: {type_hint}"
                )
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "invalid operand in function {function:?}: {detail}")
            }
        }
    }
}

impl std::error::Error for IIRLlvmError {}

// ---------------------------------------------------------------------------
// validate_for_llvm
// ---------------------------------------------------------------------------

/// Pre-flight validation for IIR → LLVM lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no validation
/// rules yet because no instructions are lowered.  Future versions will add
/// rules as instructions come online (see `MULTILANG-BACKEND-PLAN.md` §LLVM).
///
/// This mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam}` so callers can switch backends without
/// changing their pre-flight logic.
pub fn validate_for_llvm(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// lower_iir_to_llvm
// ---------------------------------------------------------------------------

/// Lower an [`IIRModule`] to a `String` containing LLVM textual IR.
///
/// **v0.1.0 scope**: emits a minimal but valid empty module — a `; ModuleID`
/// comment and a `target triple` directive.  Does not lower any instructions
/// yet; functions in the input module are simply not reflected in the output.
/// Instruction lowering arrives in v0.2.0+ (LLVM02 et al.).
///
/// # Output shape
///
/// ```text
/// ; ModuleID = '<module_name>'
/// target triple = "<target_triple>"
/// ```
///
/// # Why this shape?
///
/// `; ModuleID` is the conventional first line of every LLVM `.ll` file as
/// emitted by `clang -S -emit-llvm`.  `target triple` is required by `llc`
/// (without it, llc falls back to the build-machine default, which is
/// nondeterministic across CI runners).  Together they form the smallest
/// LLVM module that round-trips through `opt` and `llc` without warnings.
pub fn lower_iir_to_llvm(
    module: &IIRModule,
    cfg: &IIRLlvmConfig,
) -> Result<String, IIRLlvmError> {
    // ── Pre-flight validation ────────────────────────────────────────────
    //
    // Even though the v0.1.0 validator is a stub, we run it unconditionally
    // so the contract is established: callers can rely on
    // `lower_iir_to_llvm` returning `ValidationFailed` for any rule the
    // validator catches.  Wiring it now means later versions can add rules
    // without changing the API.
    let errors = validate_for_llvm(module);
    if !errors.is_empty() {
        return Err(IIRLlvmError::ValidationFailed(errors));
    }

    // ── Emit header ──────────────────────────────────────────────────────
    //
    // Note: `; ModuleID` uses the module's *configured* name, NOT
    // `module.name`.  This lets callers pin the emitted module identity
    // (useful when bundling several IIR modules into one .ll).  If we ever
    // need to surface the IIR-side name, we can add a second comment line.
    let mut out = String::with_capacity(128);
    out.push_str(&format!("; ModuleID = '{}'\n", cfg.module_name));
    out.push_str(&format!("target triple = \"{}\"\n", cfg.target_triple));

    Ok(out)
}

//! # iir-to-llvm — IIR → textual LLVM IR backend.
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `String` containing valid
//! LLVM textual IR (a `.ll` source file).
//!
//! ## Why a new crate?
//!
//! The existing IIR backends (wasm / JVM / CLR / BEAM) all target *managed*
//! runtimes that own register allocation, memory layout, GC, and exception
//! handling.  LLVM is a different beast: an AOT-native target whose output
//! runs on the bare metal of whatever CPU LLVM ships a backend for, with the
//! user's choice of LLVM optimization quality (`opt -O0` … `opt -O3`) in
//! front of it.
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
//! ## Scope of v0.2.0 (LLVM02)
//!
//! Function signatures + four instructions:
//!
//! | IIR op     | Lowering strategy                                      |
//! |------------|--------------------------------------------------------|
//! | `const`    | tracked in a name→operand map, no LLVM line emitted    |
//! | `mov`      | aliases dest to source's operand, no LLVM line emitted |
//! | `ret_void` | `  ret void`                                           |
//! | `ret`      | `  ret <ty> <operand>`                                 |
//!
//! Tracking constants and moves in a side map (rather than emitting
//! `%dest = add 0, src` no-ops) keeps the output looking like what
//! `opt -mem2reg` would produce — short, idiomatic, easy to eyeball-verify.
//!
//! Everything else is `UnsupportedOp` / `UnsupportedType`.  v0.3.0 (LLVM03)
//! adds typed arithmetic + comparisons + branches.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_llvm::{lower_iir_to_llvm, IIRLlvmConfig};
//!
//! let fn_ = IIRFunction::new(
//!     "answer",
//!     vec![],
//!     "i64",
//!     vec![
//!         IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i64"),
//!         IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i64"),
//!     ],
//! );
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("answer".into()),
//!     language: "test".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! let ll = lower_iir_to_llvm(&module, &IIRLlvmConfig::default())
//!     .expect("lowering should succeed");
//! assert!(ll.contains("define i64 @answer()"));
//! assert!(ll.contains("ret i64 42"));
//! ```

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// IIRLlvmConfig
// ===========================================================================

/// Configuration for the IIR → LLVM textual IR lowering pass.
///
/// `target_triple` defaults to a fixed string (`"x86_64-unknown-linux-gnu"`)
/// for deterministic test output.  Override via [`IIRLlvmConfig::with_target`]
/// when you actually intend to run `llc` for a non-default architecture.
///
/// We deliberately do NOT detect the host triple at build time: that would
/// make doctests host-dependent and create a cross-compilation footgun.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRLlvmConfig {
    pub module_name: String,
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

    /// Override the LLVM target triple.
    pub fn with_target(mut self, triple: impl Into<String>) -> Self {
        self.target_triple = triple.into();
        self
    }
}

impl Default for IIRLlvmConfig {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
        }
    }
}

// ===========================================================================
// IIRLlvmError
// ===========================================================================

/// Errors that can occur during IIR → LLVM IR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRLlvmError {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any LLVM type this backend handles.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape (e.g. `Int` where `Var` expected).
    InvalidOperand { function: String, detail: String },
    /// A `Var` operand references a name never defined in this function.
    UndefinedVariable { function: String, name: String },
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
                write!(f, "unsupported type in function {function:?}: {type_hint}")
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "invalid operand in function {function:?}: {detail}")
            }
            Self::UndefinedVariable { function, name } => {
                write!(f, "undefined variable {name:?} in function {function:?}")
            }
        }
    }
}

impl std::error::Error for IIRLlvmError {}

// ===========================================================================
// Type mapping — IIR type-hint → LLVM type name
// ===========================================================================
//
// LLVM types have no signedness — `i32` covers both `i32` and `u32`.  The
// signed-ness shows up in the *opcode* (`sdiv` vs `udiv`, `slt` vs `ult`)
// rather than the type, which is why arithmetic lowering in v0.3.0+ has to
// remember both pieces of information.
//
// Float and double map to LLVM's `float` and `double` respectively.
//
// Anything else (refs, str, bool, polymorphic) is rejected; v0.2.0 deals
// only in numeric scalars.
fn llvm_type_for(type_hint: &str, function: &str) -> Result<&'static str, IIRLlvmError> {
    match type_hint {
        "void" => Ok("void"),
        "i8"  | "u8"  => Ok("i8"),
        "i16" | "u16" => Ok("i16"),
        "i32" | "u32" => Ok("i32"),
        "i64" | "u64" => Ok("i64"),
        "f32" => Ok("float"),
        "f64" => Ok("double"),
        other => Err(IIRLlvmError::UnsupportedType {
            function: function.to_string(),
            type_hint: other.to_string(),
        }),
    }
}

// ===========================================================================
// validate_for_llvm
// ===========================================================================

/// Supported instruction opcodes in v0.2.0 (LLVM02).
///
/// Adding an opcode here requires also handling it in
/// [`lower_instr`] — the validator and the lowerer must stay in lockstep.
const SUPPORTED_OPS: &[&str] = &["const", "mov", "ret", "ret_void"];

/// Pre-flight validation for IIR → LLVM lowering.
///
/// Returns a `Vec<String>` of human-readable error messages.  An empty vector
/// means the module is safe to pass to [`lower_iir_to_llvm`].
///
/// # Checks
///
/// 1. Every instruction's `op` is in [`SUPPORTED_OPS`].
/// 2. Every instruction's `type_hint` maps to an LLVM type (see
///    [`llvm_type_for`]).
/// 3. Every function's return type maps to an LLVM type.
///
/// These mirror the post-hoc checks the lowerer would do anyway, but
/// surfaced up-front so callers can fail-fast and aggregate all errors.
pub fn validate_for_llvm(module: &IIRModule) -> Vec<String> {
    let mut errors = Vec::new();

    for func in &module.functions {
        // Return type check.
        if let Err(e) = llvm_type_for(&func.return_type, &func.name) {
            errors.push(format!(
                "UnsupportedType: function {:?}, return type {:?} not supported by LLVM backend",
                func.name, func.return_type
            ));
            // Don't bail — keep collecting so the caller sees everything.
            drop(e);
        }
        // Per-param type check.
        for (pname, pty) in &func.params {
            if llvm_type_for(pty, &func.name).is_err() {
                errors.push(format!(
                    "UnsupportedType: function {:?}, param {:?} type {:?} not supported",
                    func.name, pname, pty
                ));
            }
        }
        // Per-instruction checks.
        for instr in &func.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} not in LLVM backend's v0.2.0 whitelist (supported: {:?})",
                    func.name, instr.op, SUPPORTED_OPS
                ));
            }
            // `ret_void` carries type_hint "void"; everything else carries a
            // real type.  Both go through `llvm_type_for`.
            if llvm_type_for(&instr.type_hint, &func.name).is_err() {
                errors.push(format!(
                    "UnsupportedType: function {:?}, instr {:?} type_hint {:?} not supported",
                    func.name, instr.op, instr.type_hint
                ));
            }
        }
    }

    errors
}

// ===========================================================================
// Lowering
// ===========================================================================

/// Lower an [`IIRModule`] to a `String` containing LLVM textual IR.
///
/// # Output shape
///
/// ```text
/// ; ModuleID = '<module_name>'
/// target triple = "<target_triple>"
///
/// define <ret_ty> @<fn_name>(<param_ty> %<param>, ...) {
///   ret <ty> <value>
/// }
/// ```
pub fn lower_iir_to_llvm(
    module: &IIRModule,
    cfg: &IIRLlvmConfig,
) -> Result<String, IIRLlvmError> {
    // ── Pre-flight ────────────────────────────────────────────────────────
    let errors = validate_for_llvm(module);
    if !errors.is_empty() {
        return Err(IIRLlvmError::ValidationFailed(errors));
    }

    // ── Header ────────────────────────────────────────────────────────────
    let mut out = String::with_capacity(256);
    out.push_str(&format!("; ModuleID = '{}'\n", cfg.module_name));
    out.push_str(&format!("target triple = \"{}\"\n", cfg.target_triple));

    // ── Function bodies ───────────────────────────────────────────────────
    for func in &module.functions {
        out.push('\n');
        lower_function(func, &mut out)?;
    }

    Ok(out)
}

/// Emit one LLVM `define` block for one IIR function.
fn lower_function(func: &IIRFunction, out: &mut String) -> Result<(), IIRLlvmError> {
    // ── Header line: `define <ret> @<name>(<params>) {`
    let ret_ty = llvm_type_for(&func.return_type, &func.name)?;
    out.push_str(&format!("define {ret_ty} @{}(", func.name));
    for (i, (pname, pty)) in func.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let llvm_pty = llvm_type_for(pty, &func.name)?;
        out.push_str(&format!("{llvm_pty} %{pname}"));
    }
    out.push_str(") {\n");

    // ── Local-name → emitted-LLVM-operand map ─────────────────────────────
    //
    // We seed this with the parameters (each param `a` is referenced in the
    // body as `%a` in our scheme).  As we walk the body:
    //
    //   * `const` adds a literal mapping (`dest → "42"`).
    //   * `mov`   adds an alias mapping  (`dest → operand_of(src)`).
    //   * `ret`   looks up the source's operand and emits a single line.
    //
    // This sidesteps the "no-op SSA assignment" pattern that LLVM verifiers
    // and `opt` would otherwise have to clean up — the result is tighter
    // and closer to what hand-written `.ll` looks like.
    let mut env: HashMap<String, String> = HashMap::new();
    for (pname, _) in &func.params {
        env.insert(pname.clone(), format!("%{pname}"));
    }

    // ── Body ──────────────────────────────────────────────────────────────
    for instr in &func.instructions {
        lower_instr(instr, &func.name, &mut env, out)?;
    }

    out.push_str("}\n");
    Ok(())
}

/// Emit (or record state for) one IIR instruction.
fn lower_instr(
    instr: &IIRInstr,
    fn_name: &str,
    env: &mut HashMap<String, String>,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    match instr.op.as_str() {
        // ── const: tracked, not emitted ──────────────────────────────────
        //
        // `srcs[0]` carries the literal payload (Int / Float / Bool); `dest`
        // names the IIR var that should be bound to it.  We render the
        // literal in LLVM textual form and remember it for later use sites.
        "const" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRLlvmError::InvalidOperand {
                function: fn_name.into(),
                detail: "const requires a dest".into(),
            })?;
            let lit = render_literal(instr.srcs.first(), &instr.type_hint, fn_name)?;
            env.insert(dest.to_string(), lit);
            Ok(())
        }

        // ── mov: alias, not emitted ──────────────────────────────────────
        //
        // `mov dest, src` aliases dest to whatever operand src already
        // resolves to.  For const-source it becomes a literal pass-through;
        // for var-source it becomes a register alias.  Either way no LLVM
        // line is needed — the alias lives in `env`.
        "mov" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRLlvmError::InvalidOperand {
                function: fn_name.into(),
                detail: "mov requires a dest".into(),
            })?;
            let src_operand = resolve_operand(instr.srcs.first(), env, &instr.type_hint, fn_name)?;
            env.insert(dest.to_string(), src_operand);
            Ok(())
        }

        // ── ret_void ────────────────────────────────────────────────────
        "ret_void" => {
            out.push_str("  ret void\n");
            Ok(())
        }

        // ── ret <var> ───────────────────────────────────────────────────
        "ret" => {
            let ty = llvm_type_for(&instr.type_hint, fn_name)?;
            let operand = resolve_operand(instr.srcs.first(), env, &instr.type_hint, fn_name)?;
            out.push_str(&format!("  ret {ty} {operand}\n"));
            Ok(())
        }

        other => Err(IIRLlvmError::UnsupportedOp {
            function: fn_name.into(),
            op: other.into(),
        }),
    }
}

/// Resolve an `Operand` to its LLVM textual form using `env` for variables
/// or rendering literals directly.
fn resolve_operand(
    op: Option<&Operand>,
    env: &HashMap<String, String>,
    type_hint: &str,
    fn_name: &str,
) -> Result<String, IIRLlvmError> {
    match op {
        Some(Operand::Var(name)) => env.get(name).cloned().ok_or_else(|| {
            IIRLlvmError::UndefinedVariable {
                function: fn_name.into(),
                name: name.clone(),
            }
        }),
        Some(other) => render_literal(Some(other), type_hint, fn_name),
        None => Err(IIRLlvmError::InvalidOperand {
            function: fn_name.into(),
            detail: "operand missing".into(),
        }),
    }
}

/// Render an `Operand` literal as LLVM textual form.
///
/// For floats we use `{:e}`-style formatting so the output is unambiguous
/// (LLVM parses `1.500000e+00` as a `double`/`float` literal directly).
/// Integers are decimal; bools are `0`/`1` in their declared int type.
fn render_literal(
    op: Option<&Operand>,
    type_hint: &str,
    fn_name: &str,
) -> Result<String, IIRLlvmError> {
    match op {
        Some(Operand::Int(n)) => Ok(n.to_string()),
        Some(Operand::Float(v)) => {
            // LLVM accepts both 1.500000e+00 and 0x3FF8000000000000 for
            // doubles; we use scientific notation because it round-trips
            // through f64::to_string for finite values.
            Ok(format!("{v:e}"))
        }
        Some(Operand::Bool(b)) => Ok(if *b { "1".into() } else { "0".into() }),
        Some(Operand::Var(name)) => {
            // A literal slot containing a `Var` is invalid — that means the
            // caller asked us to render `%foo` as a constant, which is a
            // type error in IIR.
            Err(IIRLlvmError::InvalidOperand {
                function: fn_name.into(),
                detail: format!("expected literal, got Var({name:?})"),
            })
        }
        // Any other Operand variant (e.g. Str) is not yet supported by the
        // LLVM backend.  Strings would need a global constant + GEP dance
        // that's out of scope for v0.2.0.
        Some(other) => Err(IIRLlvmError::InvalidOperand {
            function: fn_name.into(),
            detail: format!("unsupported operand variant: {other:?}"),
        }),
        None => Err(IIRLlvmError::InvalidOperand {
            function: fn_name.into(),
            detail: format!("missing literal for type {type_hint}"),
        }),
    }
}

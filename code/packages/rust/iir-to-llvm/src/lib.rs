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
        // i1 is LLVM's boolean — added in LLVM03 so comparison results can
        // be requested at i1 width without a redundant zext+trunc round-trip.
        "i1"  | "bool" => Ok("i1"),
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

/// Supported instruction opcodes through v0.3.0 (LLVM03).
///
/// Adding an opcode here requires also handling it in
/// [`lower_instr`] — the validator and the lowerer must stay in lockstep.
///
/// LLVM02 added: `const`, `mov`, `ret`, `ret_void`.
/// LLVM03 added: arithmetic (`add`/`sub`/`mul`/`div`/`rem`), comparison
/// (`eq`/`ne`/`lt`/`le`/`gt`/`ge` plus their `cmp_`-prefixed aliases per
/// gap G1 in the multi-language backend plan), and control flow
/// (`label`/`jmp`/`jmp_if_true`/`jmp_if_false`).
const SUPPORTED_OPS: &[&str] = &[
    // LLVM02
    "const", "mov", "ret", "ret_void",
    // LLVM03 — arithmetic
    "add", "sub", "mul", "div", "rem",
    // LLVM03 — comparison (both naked and cmp_-prefixed; see G1)
    "eq", "ne", "lt", "le", "gt", "ge",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
    // LLVM03 — control flow
    "label", "jmp", "jmp_if_true", "jmp_if_false",
    // LLVM04 — calls
    "call", "call_builtin",
];

/// Builtins the LLVM backend knows how to lower.
///
/// Each entry maps to an extern declaration emitted once per module (at the
/// top, after the header) and a `call` site at each use point.  Today only
/// `print_i64` is supported — the LLVM counterpart to wasm's
/// `env.__print_i64` import (iir-to-wasm v0.8.0), JVM's
/// `env/BasicRuntime.println(J)V` (iir-to-jvm-class-file v0.7.0), and CLR's
/// `env.BasicRuntime::PrintI64(int64)` (iir-to-cil-bytecode v0.7.0).
///
/// Convention: BASIC's PRINT lowers to `@__print_i64` in textual LLVM IR.
/// We pick that name (rather than `@env.__print_i64` etc.) because LLVM
/// global names commonly use `__` for runtime / launcher symbols, and the
/// downstream linker resolves the host implementation.
const SUPPORTED_BUILTINS: &[&str] = &["print_i64"];

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
        if llvm_type_for(&func.return_type, &func.name).is_err() {
            errors.push(format!(
                "UnsupportedType: function {:?}, return type {:?} not supported by LLVM backend",
                func.name, func.return_type
            ));
            // Don't bail — keep collecting so the caller sees everything.
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
                    "UnsupportedOp: function {:?}, op {:?} not in LLVM backend's whitelist (supported: {:?})",
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

    // ── Build the callee signature table ──────────────────────────────────
    //
    // Every `call <name>(...)` site needs the param types of `<name>` to
    // emit a well-typed LLVM call.  IIR's `call` instruction carries the
    // return type in its `type_hint` but not the param types, so we
    // pre-scan the module once and stash a `name → FnSig` map.  This is
    // O(n) in the number of functions and avoids re-scanning per call.
    let mut callee_sigs: HashMap<String, FnSig> = HashMap::new();
    for f in &module.functions {
        let ret = llvm_type_for(&f.return_type, &f.name)?;
        let mut params = Vec::with_capacity(f.params.len());
        for (_, pty) in &f.params {
            params.push(llvm_type_for(pty, &f.name)?);
        }
        callee_sigs.insert(f.name.clone(), FnSig {
            param_types: params,
            return_type: ret,
        });
    }

    // ── Header ────────────────────────────────────────────────────────────
    let mut out = String::with_capacity(256);
    out.push_str(&format!("; ModuleID = '{}'\n", cfg.module_name));
    out.push_str(&format!("target triple = \"{}\"\n", cfg.target_triple));

    // ── Extern declarations for builtins actually used ────────────────────
    //
    // Pre-scan to find every `call_builtin "<name>"` that appears in any
    // function body, and emit one `declare` line per used builtin.  We do
    // this up-front (rather than incrementally as we lower) so the output
    // has the canonical LLVM shape: header → declares → defines.
    //
    // Currently the only supported builtin is `print_i64` (LLVM convention
    // chosen for BASIC's PRINT — see `SUPPORTED_BUILTINS` doc above).
    let mut used_print_i64 = false;
    for f in &module.functions {
        for i in &f.instructions {
            if i.op == "call_builtin" {
                if let Some(Operand::Var(name)) = i.srcs.first() {
                    if name == "print_i64" {
                        used_print_i64 = true;
                    }
                }
            }
        }
    }
    if used_print_i64 {
        out.push('\n');
        out.push_str("declare void @__print_i64(i64)\n");
    }

    // ── Function bodies ───────────────────────────────────────────────────
    for func in &module.functions {
        out.push('\n');
        lower_function(func, &callee_sigs, &mut out)?;
    }

    Ok(out)
}

/// A user-defined function's signature, captured at the start of module
/// lowering so each `call` site knows the param types of its callee.
struct FnSig {
    /// LLVM type names for each parameter, in order.
    param_types: Vec<&'static str>,
    /// LLVM return type name.
    return_type: &'static str,
}

/// Per-function lowering state.
///
/// Splitting this out keeps `lower_instr`'s signature manageable now that
/// LLVM03 needs both an SSA env, a sidecar i1-form env (for comparisons
/// consumed by `jmp_if_*`), and a fresh-name counter for synthesized
/// fallthrough basic blocks; LLVM04 adds a reference to a module-wide map
/// of callee signatures so `call` knows each arg's type, and a flag set
/// when the function uses any extern builtin (so the module-level emitter
/// can emit `declare` lines).
struct FnState<'a> {
    /// IIR var name → emitted LLVM operand (e.g. `%v0` or `42`).
    env: HashMap<String, String>,
    /// IIR var name → its LLVM i1 form, when the var was produced by a
    /// comparison.  `jmp_if_true` / `jmp_if_false` consume this directly
    /// without an extra `trunc` round-trip.
    env_i1: HashMap<String, String>,
    /// Per-function counter for synthesized SSA names — used both for
    /// post-cmp zext'd values and for fallthrough block labels.
    counter: u32,
    /// The function name (for error messages).
    fn_name: &'a str,
    /// Module-wide map of every user-defined function's signature.  Built
    /// up-front by `lower_iir_to_llvm` before any function body is lowered.
    callee_sigs: &'a HashMap<String, FnSig>,
}

impl FnState<'_> {
    fn fresh(&mut self, hint: &str) -> String {
        self.counter += 1;
        format!("%__{}{}", hint, self.counter)
    }
}

/// Emit one LLVM `define` block for one IIR function.
fn lower_function(
    func: &IIRFunction,
    callee_sigs: &HashMap<String, FnSig>,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
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

    // ── Per-function state ───────────────────────────────────────────────
    //
    // Seed the env with parameters (each param `a` is referenced in the
    // body as `%a`).  As we walk the body:
    //
    //   * `const` adds a literal mapping (`dest → "42"`) — no LLVM line.
    //   * `mov`   adds an alias mapping (`dest → operand_of(src)`) — no LLVM line.
    //   * `add`/`sub`/etc. emit `%dest = <op> <ty> <a>, <b>`, dest → "%dest".
    //   * `eq`/`lt`/etc. emit `%dest.i1 = icmp <op> <ty> <a>, <b>`; if the
    //     IIR type_hint is wider than i1, zext to that width.  env_i1 keeps
    //     the i1 form for downstream `jmp_if_*`.
    //   * `label`/`jmp`/`jmp_if_*` emit basic-block headers and terminators.
    //
    // This side-map trick (rather than emitting `%dest = add 0, x` no-ops
    // for const/mov) keeps output close to what `opt -mem2reg` would
    // produce — short, idiomatic, easy to eyeball-verify.
    let mut state = FnState {
        env: HashMap::new(),
        env_i1: HashMap::new(),
        counter: 0,
        fn_name: &func.name,
        callee_sigs,
    };
    for (pname, _) in &func.params {
        state.env.insert(pname.clone(), format!("%{pname}"));
    }

    // ── Body ──────────────────────────────────────────────────────────────
    for instr in &func.instructions {
        lower_instr(instr, &mut state, out)?;
    }

    out.push_str("}\n");
    Ok(())
}

/// Emit (or record state for) one IIR instruction.
fn lower_instr(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let fn_name = state.fn_name;
    match instr.op.as_str() {
        // ── const: tracked, not emitted ──────────────────────────────────
        "const" => {
            let dest = require_dest(instr, "const", fn_name)?;
            let lit = render_literal(instr.srcs.first(), &instr.type_hint, fn_name)?;
            state.env.insert(dest.to_string(), lit);
            Ok(())
        }

        // ── mov: alias, not emitted ──────────────────────────────────────
        "mov" => {
            let dest = require_dest(instr, "mov", fn_name)?;
            let src_operand =
                resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, fn_name)?;
            state.env.insert(dest.to_string(), src_operand);
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
            let operand =
                resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, fn_name)?;
            out.push_str(&format!("  ret {ty} {operand}\n"));
            Ok(())
        }

        // ── arithmetic ──────────────────────────────────────────────────
        //
        // Two-operand integer or float operation.  Signedness comes from
        // the type_hint prefix (`i*` = signed, `u*` = unsigned), which only
        // matters for `div` and `rem` (LLVM splits these into `sdiv`/`udiv`
        // and `srem`/`urem`; `add`/`sub`/`mul` are signedness-agnostic).
        "add" | "sub" | "mul" | "div" | "rem" => {
            lower_arith(instr.op.as_str(), instr, state, out)
        }

        // ── comparison ──────────────────────────────────────────────────
        //
        // LLVM `icmp` and `fcmp` always return i1.  IIR's type_hint on a
        // comparison is the *operand* type (matching the wasm convention
        // where cmps produce i32 0/1).  If the type_hint is wider than i1
        // we zext the i1 to that width; either way we remember the i1 form
        // in `env_i1` so a downstream `jmp_if_*` can use it directly.
        //
        // Both naked (`eq`) and `cmp_`-prefixed (`cmp_eq`) opcodes work —
        // the latter were introduced in gap G1 for the wasm backend and
        // we accept them here for consistency.
        "eq" | "ne" | "lt" | "le" | "gt" | "ge"
        | "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
            let bare = instr.op.strip_prefix("cmp_").unwrap_or(instr.op.as_str());
            lower_cmp(bare, instr, state, out)
        }

        // ── label "<name>": open a new basic block ──────────────────────
        //
        // LLVM requires every basic block to begin with a label (except the
        // implicit entry block).  The name comes from srcs[0] as a Var.
        "label" => {
            let name = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRLlvmError::InvalidOperand {
                    function: fn_name.into(),
                    detail: "label requires srcs[0] = Operand::Var(name)".into(),
                }),
            };
            out.push_str(&format!("{name}:\n"));
            Ok(())
        }

        // ── jmp "<name>": unconditional branch ──────────────────────────
        "jmp" => {
            let target = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRLlvmError::InvalidOperand {
                    function: fn_name.into(),
                    detail: "jmp requires srcs[0] = Operand::Var(name)".into(),
                }),
            };
            out.push_str(&format!("  br label %{target}\n"));
            Ok(())
        }

        // ── jmp_if_true <cond>, "<name>" ────────────────────────────────
        //
        // LLVM conditional branches require *both* arms.  IIR's
        // `jmp_if_true` only names the true target; the false arm is the
        // implicit fallthrough to the next IIR instruction.  We synthesize
        // a fresh block label `%__fall<N>` and immediately emit it after
        // the branch, so the next instruction lands in a valid block.
        //
        // `jmp_if_false` is the same with arms swapped.
        "jmp_if_true" => lower_jmp_if(instr, state, out, /*true_first=*/ true),
        "jmp_if_false" => lower_jmp_if(instr, state, out, /*true_first=*/ false),

        // ── call <name>(<args>...) — user-defined function call ─────────
        //
        // Layout: srcs = [Var(callee), Var(arg1), Var(arg2), ...].  dest is
        // Some(name) for non-void return, None for void.  The IIR
        // type_hint is the callee's return type.  Per-arg types come from
        // the pre-built `callee_sigs` map; without it we cannot emit
        // well-typed LLVM (`call` requires explicit per-arg types).
        "call" => lower_call(instr, state, out),

        // ── call_builtin "<name>"(<args>...) — extern host call ─────────
        //
        // Layout: srcs = [Var("<builtin_name>"), Var(arg1), ...].  Today
        // only `print_i64` is supported (lowers to
        // `call void @__print_i64(i64 %v)`).  The matching `declare` line
        // is emitted once at the module top by `lower_iir_to_llvm`.
        "call_builtin" => lower_call_builtin(instr, state, out),

        other => Err(IIRLlvmError::UnsupportedOp {
            function: fn_name.into(),
            op: other.into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Helpers for LLVM03
// ---------------------------------------------------------------------------

fn require_dest<'a>(
    instr: &'a IIRInstr,
    op: &str,
    fn_name: &str,
) -> Result<&'a str, IIRLlvmError> {
    instr.dest.as_deref().ok_or_else(|| IIRLlvmError::InvalidOperand {
        function: fn_name.into(),
        detail: format!("{op} requires a dest"),
    })
}

fn is_float_type(s: &str) -> bool {
    s == "f32" || s == "f64"
}

fn is_unsigned_type(s: &str) -> bool {
    s.starts_with('u')
}

/// Pick the LLVM opcode for a binary arithmetic instruction.
///
/// The result is signedness-aware for `div` and `rem` (split into
/// `sdiv`/`udiv` and `srem`/`urem`), and operand-type-aware for floats
/// (use `f*` variants).  `add`/`sub`/`mul` share opcodes between signed
/// and unsigned because in two's-complement they produce the same bits.
fn llvm_arith_op(iir_op: &str, type_hint: &str) -> &'static str {
    let f = is_float_type(type_hint);
    let u = is_unsigned_type(type_hint);
    match iir_op {
        "add" => if f { "fadd" } else { "add" },
        "sub" => if f { "fsub" } else { "sub" },
        "mul" => if f { "fmul" } else { "mul" },
        "div" => {
            if f { "fdiv" } else if u { "udiv" } else { "sdiv" }
        }
        "rem" => {
            if f { "frem" } else if u { "urem" } else { "srem" }
        }
        _ => unreachable!("llvm_arith_op called with non-arith op {iir_op}"),
    }
}

fn lower_arith(
    iir_op: &str,
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, iir_op, state.fn_name)?.to_string();
    let ty = llvm_type_for(&instr.type_hint, state.fn_name)?;
    let a = resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, state.fn_name)?;
    let b = resolve_operand(instr.srcs.get(1), &state.env, &instr.type_hint, state.fn_name)?;
    let llvm_op = llvm_arith_op(iir_op, &instr.type_hint);
    out.push_str(&format!("  %{dest} = {llvm_op} {ty} {a}, {b}\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Pick the LLVM `icmp`/`fcmp` predicate for a comparison.
///
/// Equality predicates (`eq`/`ne`) are signedness-agnostic for integers.
/// Inequality predicates split by signedness (`slt`/`ult` etc.).  Float
/// comparisons use `o<pred>` (ordered) — meaning NaN compares false — to
/// match the most common language-level expectation.
fn llvm_cmp_predicate(bare_op: &str, type_hint: &str) -> Result<&'static str, IIRLlvmError> {
    let f = is_float_type(type_hint);
    let u = is_unsigned_type(type_hint);
    Ok(match bare_op {
        "eq" => if f { "oeq" } else { "eq" },
        "ne" => if f { "one" } else { "ne" },
        "lt" => if f { "olt" } else if u { "ult" } else { "slt" },
        "le" => if f { "ole" } else if u { "ule" } else { "sle" },
        "gt" => if f { "ogt" } else if u { "ugt" } else { "sgt" },
        "ge" => if f { "oge" } else if u { "uge" } else { "sge" },
        _ => unreachable!("llvm_cmp_predicate called with non-cmp op {bare_op}"),
    })
}

fn lower_cmp(
    bare_op: &str,
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, bare_op, state.fn_name)?.to_string();
    let operand_ty = llvm_type_for(&instr.type_hint, state.fn_name)?;
    let a = resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, state.fn_name)?;
    let b = resolve_operand(instr.srcs.get(1), &state.env, &instr.type_hint, state.fn_name)?;
    let pred = llvm_cmp_predicate(bare_op, &instr.type_hint)?;
    let icmp_or_fcmp = if is_float_type(&instr.type_hint) { "fcmp" } else { "icmp" };

    // i1 form: always synthesized.  Lives in env_i1 for downstream jmp_if_*.
    let i1_name = format!("%{dest}.i1");
    out.push_str(&format!(
        "  {i1_name} = {icmp_or_fcmp} {pred} {operand_ty} {a}, {b}\n"
    ));
    state.env_i1.insert(dest.clone(), i1_name.clone());

    // If the IIR type_hint is i1 (width 1), use the i1 form directly.
    // Otherwise zext to the wider type so subsequent arithmetic stays
    // typed-correctly.  We approximate "is i1" as type_hint == "i1" since
    // IIR doesn't currently emit a literal "i1" — most callers use the
    // operand type as the result type (wasm convention).
    if instr.type_hint == "i1" {
        state.env.insert(dest, i1_name);
    } else {
        out.push_str(&format!(
            "  %{dest} = zext i1 {i1_name} to {operand_ty}\n"
        ));
        state.env.insert(dest.clone(), format!("%{dest}"));
    }
    Ok(())
}

fn lower_jmp_if(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
    true_first: bool,
) -> Result<(), IIRLlvmError> {
    // Operand layout: srcs = [Var(cond), Var(target_label)].
    let cond_name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "jmp_if_* requires srcs[0] = Operand::Var(cond)".into(),
        }),
    };
    let target = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "jmp_if_* requires srcs[1] = Operand::Var(target_label)".into(),
        }),
    };

    // Prefer the i1 form when the cond was produced by a comparison; else
    // truncate the env operand back to i1.  type_hint on jmp_if_* carries
    // the cond's type (typically same as the producing cmp).
    let cond_i1 = if let Some(i1) = state.env_i1.get(&cond_name).cloned() {
        i1
    } else {
        let cond_op = state.env.get(&cond_name).cloned().ok_or_else(|| {
            IIRLlvmError::UndefinedVariable {
                function: state.fn_name.into(),
                name: cond_name.clone(),
            }
        })?;
        // Truncate to i1.  Need to know the cond's current type for the
        // trunc — use the instr's type_hint as the operand type.
        let cond_ty = llvm_type_for(&instr.type_hint, state.fn_name)?;
        let i1 = state.fresh("trunc");
        out.push_str(&format!("  {i1} = trunc {cond_ty} {cond_op} to i1\n"));
        i1
    };

    let fallthrough = format!("__fall{}", {
        state.counter += 1;
        state.counter
    });
    let (t_label, f_label) = if true_first {
        (target.clone(), fallthrough.clone())
    } else {
        (fallthrough.clone(), target.clone())
    };
    out.push_str(&format!(
        "  br i1 {cond_i1}, label %{t_label}, label %{f_label}\n"
    ));
    out.push_str(&format!("{fallthrough}:\n"));
    Ok(())
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

// ---------------------------------------------------------------------------
// LLVM04 — call / call_builtin
// ---------------------------------------------------------------------------

/// Lower an IIR `call` of a user-defined function.
///
/// Layout: `srcs = [Var(callee), Var(arg1), Var(arg2), ...]` and `dest =
/// Some(name)` for non-void return, `None` for void.  The IIR `type_hint`
/// is the callee's return type.
///
/// Output:
///
/// ```llvm
/// %dest = call <ret_ty> @<callee>(<arg_ty> <arg>, ...)   ; non-void
///         call void     @<callee>(<arg_ty> <arg>, ...)   ; void
/// ```
fn lower_call(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    // srcs[0] is the callee name.
    let callee = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "call requires srcs[0] = Operand::Var(callee_name)".into(),
        }),
    };
    let sig = state.callee_sigs.get(&callee).ok_or_else(|| {
        IIRLlvmError::UndefinedVariable {
            function: state.fn_name.into(),
            name: format!("callee {callee:?} not in module"),
        }
    })?;

    // Validate arg count matches signature so we fail fast rather than
    // emitting malformed LLVM that `opt` would reject later.
    let arg_srcs = &instr.srcs[1..];
    if arg_srcs.len() != sig.param_types.len() {
        return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!(
                "call to {callee:?}: arg-count mismatch (got {}, want {})",
                arg_srcs.len(),
                sig.param_types.len()
            ),
        });
    }

    // Resolve each operand and pair it with its declared param type.
    let mut arg_parts = Vec::with_capacity(arg_srcs.len());
    for (src, pty) in arg_srcs.iter().zip(sig.param_types.iter()) {
        let op = match src {
            Operand::Var(name) => state.env.get(name).cloned().ok_or_else(|| {
                IIRLlvmError::UndefinedVariable {
                    function: state.fn_name.into(),
                    name: name.clone(),
                }
            })?,
            other => render_literal(Some(other), pty, state.fn_name)?,
        };
        arg_parts.push(format!("{pty} {op}"));
    }
    let args_joined = arg_parts.join(", ");

    let ret_ty = sig.return_type;
    if let Some(dest) = &instr.dest {
        out.push_str(&format!(
            "  %{dest} = call {ret_ty} @{callee}({args_joined})\n"
        ));
        state.env.insert(dest.clone(), format!("%{dest}"));
    } else {
        // Void return — no dest binding.  Per LLVM IR, a void `call` must
        // not be on the LHS of an assignment.
        out.push_str(&format!("  call {ret_ty} @{callee}({args_joined})\n"));
    }
    Ok(())
}

/// Lower an IIR `call_builtin "<name>"(...)` to an extern call.
///
/// Today only `print_i64` is supported (the LLVM counterpart to wasm's
/// `env.__print_i64`, JVM's `env/BasicRuntime.println(J)V`, and CLR's
/// `env.BasicRuntime::PrintI64(int64)`).  Layout:
///
/// ```text
/// srcs = [Var("print_i64"), Var(val: i64)]
/// dest = None
/// ```
///
/// emits:
///
/// ```llvm
/// call void @__print_i64(i64 <val>)
/// ```
///
/// The matching `declare void @__print_i64(i64)` is emitted at the module
/// top by `lower_iir_to_llvm` if any function uses the builtin.
fn lower_call_builtin(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "call_builtin requires srcs[0] = Operand::Var(builtin_name)".into(),
        }),
    };
    if !SUPPORTED_BUILTINS.contains(&name.as_str()) {
        return Err(IIRLlvmError::UnsupportedOp {
            function: state.fn_name.into(),
            op: format!("call_builtin {name:?}: not in LLVM backend whitelist"),
        });
    }
    match name.as_str() {
        "print_i64" => {
            let val = match instr.srcs.get(1) {
                Some(Operand::Var(s)) => state.env.get(s).cloned().ok_or_else(|| {
                    IIRLlvmError::UndefinedVariable {
                        function: state.fn_name.into(),
                        name: s.clone(),
                    }
                })?,
                Some(other) => render_literal(Some(other), "i64", state.fn_name)?,
                None => return Err(IIRLlvmError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "call_builtin \"print_i64\" requires srcs[1] = Operand::Var(val:i64)".into(),
                }),
            };
            out.push_str(&format!("  call void @__print_i64(i64 {val})\n"));
            Ok(())
        }
        _ => unreachable!("SUPPORTED_BUILTINS guard above prevents this"),
    }
}

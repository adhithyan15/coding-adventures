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

use interpreter_ir::opcodes::array_elem_type;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use std::cmp::Ordering;
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
// Anything else (refs, str, polymorphic) is rejected in this scalar layer.
fn llvm_type_for(type_hint: &str, function: &str) -> Result<&'static str, IIRLlvmError> {
    match type_hint {
        "void" => Ok("void"),
        // i1 is LLVM's boolean — added in LLVM03 so comparison results can
        // be requested at i1 width without a redundant zext+trunc round-trip.
        "i1"  | "bool" => Ok("i1"),
        // u4 (Nib's 4-bit nibble) has no native LLVM width; it rides in an i8
        // and the E2 wrap mask (`and i64 …, 0xF`) enforces the 4-bit range.
        "u4"  => Ok("i8"),
        "i8"  | "u8"  => Ok("i8"),
        "i16" | "u16" => Ok("i16"),
        "i32" | "u32" => Ok("i32"),
        "i64" | "u64" => Ok("i64"),
        "f32" => Ok("float"),
        "f64" => Ok("double"),
        // McCarthy W12b (tagged-word lisp): a lisp value is a tagged 64-bit word
        // (the C runtime's `LispyValue`), and the polymorphic `any` flows as the
        // same word. A lisp heap reference (`ref<LispyPair>`, and any future
        // `ref<Lispy…>`) is likewise carried as a tagged `i64` — the runtime owns
        // the cell layout, the backend only moves words and calls `__dyn_*`.
        // NON-lisp references (`ref<Foo>`) remain unsupported (no value model).
        "any" => Ok("i64"),
        t if t.starts_with("ref<Lispy") => Ok("i64"),
        // McCarthy W13 (F6): an interned symbol is a tagged 64-bit immediate.
        "symbol" => Ok("i64"),
        // E4-dyn (E4d-2b): a `str` VALUE is carried as an i64 **handle** — the
        // address of a `[i64 len][bytes…]` block.  Mapping it here lets a string
        // flow through a function boundary (a `str` parameter or return type) and
        // a `call` result, so an ALGOL `string procedure`'s returned string is a
        // first-class value.  String *operations* still dispatch on the `"str"`
        // type_hint separately (`lower_str_*`); this only governs the LLVM value
        // type at those boundaries.
        "str" => Ok("i64"),
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
/// LLVM03 added: arithmetic (`add`/`sub`/`mul`/`div`/`mod`/`rem`), comparison
/// (`eq`/`ne`/`lt`/`le`/`gt`/`ge` plus their `cmp_`-prefixed aliases per
/// gap G1 in the multi-language backend plan), and control flow
/// (`label`/`jmp`/`jmp_if_true`/`jmp_if_false`).
const SUPPORTED_OPS: &[&str] = &[
    // LLVM02
    "const", "mov", "ret", "ret_void",
    // LLVM03 — arithmetic and bitwise/logical scalar ops
    "add", "sub", "mul", "div", "mod", "rem",
    "and", "or", "xor",
    // bitwise NOT — synthesised as `xor x, -1` (LLVM has no `not`); unlocks
    // Nib N3-`~` and Oct O2-`~`.
    "not",
    // unary negation — `fneg` for floats, `sub 0, x` for integers.  Used by
    // BASIC ABS/SGN inline conditionals and any frontend that emits unary `-`.
    "neg",
    // LLVM03 — comparison (both naked and cmp_-prefixed; see G1)
    "eq", "ne", "lt", "le", "gt", "ge",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
    // LLVM03 — control flow
    "label", "jmp", "jmp_if_true", "jmp_if_false",
    // LLVM04 — calls
    "call", "call_builtin",
    // LLVM05 — byte-tape memory (LANG-MATRIX LM-L Brainfuck). `alloc_bytes`
    // mallocs a zero-filled tape, `load_byte`/`store_byte` index it at byte
    // width (zero-extend on load, truncate on store). The same op trio the
    // x86_64 / native AOT backend already supports (LANG76); we add the LLVM
    // lowering so Brainfuck — which builds an implicit byte tape — compiles.
    "alloc_bytes", "load_byte", "store_byte",
    // LANG-FULL E5 — bounds-checked arrays (the *static* representation:
    // length-prefixed flat `calloc` block + an explicit compare/trap, vs the
    // JVM/CLR managed-array native check). `alloc_array` allocates `[i64 len]
    // [elems…]`; `array_get`/`array_set` bounds-check the index then GEP+load/
    // store; `array_len` reads the header.
    "alloc_array", "array_get", "array_set", "array_len",
    // LANG-FULL E6 (layer 1) — typed module globals. A function reads/writes a
    // module-level variable via `global_load`/`global_store`; each distinct
    // global name becomes a module-level `@__twig_global_N = internal global i64`
    // (the LLVM analogue of the native `_twig_globals` slots / JVM-CLR static
    // fields). The name is a string literal, never a register.
    "global_load", "global_store",
    // LANG-FULL E8 — numeric conversions integer↔real. `int_to_real` is
    // `sitofp i64 → double`; `real_to_int_trunc`/`real_to_int_floor` round
    // (`@llvm.trunc.f64`/`@llvm.floor.f64`), range-check (trap on
    // NaN/∞/out-of-i64-range, matching the VM), then `fptosi double → i64`.
    "int_to_real", "real_to_int_trunc", "real_to_int_floor",
    // AL8 sqrt + transcendentals — IEEE-754 ops via LLVM intrinsics / libm.
    "f64_sqrt", "f64_sin", "f64_cos", "f64_ln", "f64_exp", "f64_atan", "f64_tan",
    // BA-pow — two-argument pow(base, exp) via libm `@pow`.
    "f64_pow",
    // LANG-FULL E4 — string literal foothold for the static LLVM column.
    // `str_const` materialises a length-prefixed private constant. `str_index`,
    // `str_concat`, `str_slice`, `str_len`, `str_eq`, and `str_cmp` read literal metadata,
    // and `print_str` calls the generic C runtime. Richer dynamic byte-string
    // ops remain unsupported.
    "str_const", "str_index", "str_concat", "str_slice", "str_len", "str_eq", "str_cmp",
    "print_str",
];

/// Builtins the LLVM backend knows how to lower.
///
/// Each entry maps to an extern declaration emitted once per module (at the
/// top, after the header) and a `call` site at each use point.  Today only
/// `print_i64` is supported — the LLVM counterpart to wasm's
/// `env.__print_i64` import (iir-to-wasm v0.8.0), JVM's
/// `env/BasicRuntime.println(J)V` (iir-to-jvm-class-file v0.7.0), and CLR's
/// `env.BasicRuntime::PrintI64(int64)` (iir-to-cil-bytecode v0.7.0). E4 also
/// adds `print_str`, which lowers to a separate `@__print_str(ptr, i64)` runtime
/// call from the dedicated `print_str` opcode rather than through `call_builtin`.
///
/// Convention: BASIC's PRINT lowers to `@__print_i64` in textual LLVM IR.
/// We pick that name (rather than `@env.__print_i64` etc.) because LLVM
/// global names commonly use `__` for runtime / launcher symbols, and the
/// downstream linker resolves the host implementation.
///
/// LLVM05 (LANG-MATRIX LM-L Brainfuck) adds `putchar` / `getchar` — Brainfuck's
/// `.` and `,`. These lower directly to the libc `@putchar(i32)` / `@getchar()`
/// the C standard library already provides, so no host-runtime shim is needed
/// (unlike `print_i64`); `clang` links libc by default.
/// `input_i64` — BASIC's `INPUT X` — lowers to `@__twig_input_i64()` from the
/// AOT runtime archive, which reads a line from stdin and parses it as `int64_t`.
/// `input_str` — BASIC's string `INPUT A$` (E4-dyn) — lowers to
/// `@__twig_input_str()`, which reads a whole line and returns an i64 handle to a
/// `[i64 len][bytes]` heap block (the runtime-string repr `print_str` reads).
const SUPPORTED_BUILTINS: &[&str] =
    &["print_i64", "putchar", "getchar", "input_i64", "input_str"];

#[derive(Debug, Clone)]
struct LlvmStringLiteralDef {
    symbol: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct LlvmStringLiteralRef {
    symbol: String,
    len: usize,
}

/// McCarthy W12b — the **tagged-word lisp** builtins the LLVM backend lowers to
/// `call`s into the shared C runtime (`twig-aot/runtime/dynval_runtime.c`), the
/// same runtime the native AOT backend links. Each entry is
/// `(iir_name, runtime_symbol, arity)`; every lisp value is a tagged 64-bit word
/// so the signature is always `i64 (i64 × arity)`. The `lispy_*` IIR names are
/// produced by `iir_builtin_lowering::lower_heap_builtins_runtime` /
/// `lower_lisp_repr`; they map to the runtime's `__dyn_*` symbols.
///
/// | iir name         | runtime symbol            | McCarthy primitive            |
/// |------------------|---------------------------|-------------------------------|
/// | `lispy_cons`     | `__dyn_cons`       | `CONS` — build a pair `[a|b]`  |
/// | `lispy_car`/`cdr`| `__dyn_car`/`cdr`  | `CAR`/`CDR`                    |
/// | `lispy_pair_p`   | `__dyn_pair_p`     | `pair?` (→ `ATOM`)            |
/// | `lispy_equal`    | `__dyn_equal`      | `EQ`                          |
/// | `lispy_not`      | `__dyn_not`        | logical `not`                 |
/// | `lispy_truthy`   | `__dyn_truthy`     | `COND` clause test            |
/// | `lispy_box_int`  | `__dyn_box_int`    | int → tagged word             |
/// | `lispy_unbox_int`| `__dyn_unbox_int`  | tagged word → int (result)    |
/// | `lispy_nil`      | `__dyn_nil`        | `()` / nil                    |
const LISPY_BUILTINS: &[(&str, &str, usize)] = &[
    ("lispy_cons", "__dyn_cons", 2),
    ("lispy_car", "__dyn_car", 1),
    ("lispy_cdr", "__dyn_cdr", 1),
    ("lispy_pair_p", "__dyn_pair_p", 1),
    ("lispy_equal", "__dyn_equal", 2),
    ("lispy_not", "__dyn_not", 1),
    ("lispy_truthy", "__dyn_truthy", 1),
    ("lispy_box_int", "__dyn_box_int", 1),
    ("lispy_unbox_int", "__dyn_unbox_int", 1),
    ("lispy_to_exit_code", "__dyn_to_exit_code", 1),
    ("lispy_nil", "__dyn_nil", 0),
];

/// Look up a `lispy_*` builtin by its IIR name.
fn lispy_builtin(name: &str) -> Option<&'static (&'static str, &'static str, usize)> {
    LISPY_BUILTINS.iter().find(|(n, _, _)| *n == name)
}

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
            if instr.op == "str_const" {
                validate_str_const(func, instr, &mut errors);
                continue;
            }
            if instr.op == "str_concat" {
                validate_str_concat(func, instr, &mut errors);
                continue;
            }
            if instr.op == "str_slice" {
                validate_str_slice(func, instr, &mut errors);
                continue;
            }
            if instr.op == "str_index" {
                validate_str_index(func, instr, &mut errors);
                continue;
            }
            if instr.op == "str_len" {
                validate_str_len(func, instr, &mut errors);
                continue;
            }
            if instr.op == "str_eq" {
                validate_str_eq(func, instr, &mut errors);
                continue;
            }
            if instr.op == "str_cmp" {
                validate_str_cmp(func, instr, &mut errors);
                continue;
            }
            if instr.op == "print_str" {
                validate_print_str(func, instr, &mut errors);
                continue;
            }
            // `ret_void` carries type_hint "void"; everything else carries a
            // real type.  Both go through `llvm_type_for`. An `alloc_array`
            // carries an `array<T>` hint (LANG-FULL E5) — not a scalar LLVM type,
            // so validate its *element* `T` instead (the handle itself is a `ptr`).
            let type_ok = match array_elem_type(&instr.type_hint) {
                Some(elem) => llvm_type_for(&elem, &func.name).is_ok(),
                None => llvm_type_for(&instr.type_hint, &func.name).is_ok(),
            };
            if !type_ok {
                errors.push(format!(
                    "UnsupportedType: function {:?}, instr {:?} type_hint {:?} not supported",
                    func.name, instr.op, instr.type_hint
                ));
            }
        }
    }

    errors
}

fn validate_str_const(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_const requires a dest",
            func.name
        ));
    }
    if instr.type_hint != "str" {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_const type_hint {:?} must be \"str\"",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Str(s)] if is_printable_ascii_str(s) => {}
        [Operand::Str(_)] => errors.push(format!(
            "InvalidOperand: function {:?}, str_const only supports printable ASCII plus tab/newline/carriage-return",
            func.name
        )),
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_const requires exactly one Operand::Str literal",
            func.name
        )),
    }
}

fn validate_print_str(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_some() {
        errors.push(format!(
            "InvalidOperand: function {:?}, print_str must not have a dest",
            func.name
        ));
    }
    if instr.type_hint != "void" {
        errors.push(format!(
            "UnsupportedType: function {:?}, print_str type_hint {:?} must be \"void\"",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, print_str requires exactly one Operand::Var",
            func.name
        )),
    }
}

fn validate_str_len(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_len requires a dest",
            func.name
        ));
    }
    if llvm_type_for(&instr.type_hint, &func.name).is_err() {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_len result type {:?} is not supported",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_len requires exactly one Operand::Var",
            func.name
        )),
    }
}

fn validate_str_concat(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_concat requires a dest",
            func.name
        ));
    }
    if instr.type_hint != "str" {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_concat result type {:?} must be \"str\"",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_), Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_concat requires exactly two Operand::Var sources",
            func.name
        )),
    }
}

fn validate_str_slice(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_slice requires a dest",
            func.name
        ));
    }
    if instr.type_hint != "str" {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_slice result type {:?} must be \"str\"",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_), Operand::Var(_), Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_slice requires string, start, and end Operand::Var sources",
            func.name
        )),
    }
}

fn validate_str_index(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_index requires a dest",
            func.name
        ));
    }
    if llvm_type_for(&instr.type_hint, &func.name).is_err() {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_index result type {:?} is not supported",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_), Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_index requires exactly two Operand::Var sources",
            func.name
        )),
    }
}

fn validate_str_eq(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_eq requires a dest",
            func.name
        ));
    }
    if llvm_type_for(&instr.type_hint, &func.name).is_err() {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_eq result type {:?} is not supported",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_), Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_eq requires exactly two Operand::Var sources",
            func.name
        )),
    }
}

fn validate_str_cmp(func: &IIRFunction, instr: &IIRInstr, errors: &mut Vec<String>) {
    if instr.dest.is_none() {
        errors.push(format!(
            "InvalidOperand: function {:?}, str_cmp requires a dest",
            func.name
        ));
    }
    if llvm_type_for(&instr.type_hint, &func.name).is_err() {
        errors.push(format!(
            "UnsupportedType: function {:?}, str_cmp result type {:?} is not supported",
            func.name, instr.type_hint
        ));
    }
    match instr.srcs.as_slice() {
        [Operand::Var(_), Operand::Var(_)] => {}
        _ => errors.push(format!(
            "InvalidOperand: function {:?}, str_cmp requires exactly two Operand::Var sources",
            func.name
        )),
    }
}

fn is_printable_ascii_str(s: &str) -> bool {
    s.bytes()
        .all(|b| matches!(b, b'\n' | b'\r' | b'\t' | 0x20..=0x7e))
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
    let mut used_print_str = false;
    // LLVM05 — Brainfuck I/O + tape: libc `putchar`/`getchar` and the
    // allocator behind `alloc_bytes`. Declared once each, when used.
    let mut used_putchar = false;
    let mut used_getchar = false;
    // BA-INPUT: BASIC's `INPUT X` lowers to `@__twig_input_i64()` from the AOT
    // runtime archive (reads a line, parses as int64_t; 0 on EOF/parse failure).
    let mut used_input_i64 = false;
    // E4-dyn: BASIC string `INPUT A$` lowers to `@__twig_input_str()` (reads a
    // line, returns an i64 handle to a `[i64 len][bytes]` heap block).
    let mut used_input_str = false;
    let mut used_alloc_bytes = false;
    // LANG-FULL E5: any array op needs `@calloc` (the allocation) and `@llvm.trap`
    // (the out-of-bounds trap). `is_array_op` covers alloc_array/array_*.
    let mut used_arrays = false;
    // LANG-FULL E4: direct-literal `str_index` can emit `@llvm.trap` when a
    // compile-known index is out of bounds.
    let mut used_str_index = false;
    // E4-dyn runtime string concatenation over non-literal operands lowers to a
    // call to `@__twig_str_concat` (from the AOT archive). Set whenever any
    // `str_concat` op appears; if it turns out to fold to a compile-time literal,
    // the declare is simply unused (a declare with no call site is legal LLVM and
    // creates no undefined-symbol reference at link time).
    let mut used_str_concat = false;
    // E4-dyn runtime string equality over non-literal operands lowers to a call to
    // `@__twig_str_eq`. Set whenever any `str_eq` op appears; an unused declare is
    // legal LLVM (same rationale as `used_str_concat`).
    let mut used_str_eq = false;
    // LANG-FULL E8: the `real_to_int_*` conversions need `@llvm.trap` (the
    // out-of-range trap) plus the `@llvm.floor.f64`/`@llvm.trunc.f64` rounding
    // intrinsics. `is_conversion` covers int_to_real / real_to_int_{trunc,floor}.
    let mut used_conversions = false;
    // AL8 sqrt/trig: `f64_sqrt`/`f64_sin`/`f64_cos`/`f64_ln`/`f64_exp` each
    // lower to a single LLVM intrinsic call — one `declare` per used op.
    // `f64_atan`/`f64_tan` use direct libm declarations (no LLVM intrinsic).
    let mut used_f64_sqrt = false;
    let mut used_f64_sin  = false;
    let mut used_f64_cos  = false;
    let mut used_f64_ln   = false;
    let mut used_f64_exp  = false;
    let mut used_f64_atan = false;
    let mut used_f64_tan  = false;
    // BA-pow: `f64_pow` lowers to `call double @pow(double, double)` — libm.
    let mut used_f64_pow = false;
    // McCarthy W12b: collect the tagged-word lisp builtins actually used, in
    // first-seen order, so each gets exactly one `declare i64 @__dyn_*`.
    let mut used_lispy: Vec<&'static (&'static str, &'static str, usize)> = Vec::new();
    for f in &module.functions {
        for i in &f.instructions {
            if i.op == "alloc_bytes" {
                used_alloc_bytes = true;
            }
            if i.op == "print_str" {
                used_print_str = true;
            }
            if interpreter_ir::opcodes::is_array_op(&i.op) {
                used_arrays = true;
            }
            if i.op == "str_index" {
                used_str_index = true;
            }
            if i.op == "str_concat" {
                used_str_concat = true;
            }
            if i.op == "str_eq" {
                used_str_eq = true;
            }
            if interpreter_ir::opcodes::is_conversion(&i.op) {
                used_conversions = true;
            }
            if i.op == "f64_sqrt" { used_f64_sqrt = true; }
            if i.op == "f64_sin"  { used_f64_sin  = true; }
            if i.op == "f64_cos"  { used_f64_cos  = true; }
            if i.op == "f64_ln"   { used_f64_ln   = true; }
            if i.op == "f64_exp"  { used_f64_exp  = true; }
            if i.op == "f64_atan" { used_f64_atan = true; }
            if i.op == "f64_tan"  { used_f64_tan  = true; }
            if i.op == "f64_pow"  { used_f64_pow  = true; }
            if i.op == "call_builtin" {
                if let Some(Operand::Var(name)) = i.srcs.first() {
                    match name.as_str() {
                        "print_i64" => used_print_i64 = true,
                        "putchar" => used_putchar = true,
                        "getchar" => used_getchar = true,
                        "input_i64" => used_input_i64 = true,
                        "input_str" => used_input_str = true,
                        _ => {
                            if let Some(b) = lispy_builtin(name) {
                                if !used_lispy.iter().any(|(n, _, _)| n == &b.0) {
                                    used_lispy.push(b);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if used_print_i64 {
        out.push('\n');
        out.push_str("declare void @__print_i64(i64)\n");
    }
    if used_print_str {
        out.push('\n');
        out.push_str("declare void @__print_str(ptr, i64)\n");
    }
    // LLVM05 — libc / allocator declarations for the Brainfuck tape & I/O.
    // `calloc(nmemb, size)` returns a zero-filled buffer (BF cells start at 0);
    // `putchar`/`getchar` are the libc character I/O the BF `.`/`,` map to.
    if used_f64_sqrt || used_f64_sin || used_f64_cos || used_f64_ln || used_f64_exp
        || used_f64_atan || used_f64_tan {
        out.push('\n');
    }
    if used_f64_sqrt {
        // `@llvm.sqrt.f64` is an IEEE-754 hardware sqrt intrinsic — maps to
        // `sqrtsd` on x86_64 and `fsqrt` on aarch64.  No trap needed: NaN
        // propagates and negatives return NaN per IEEE-754.
        out.push_str("declare double @llvm.sqrt.f64(double)\n");
    }
    // AL8 transcendentals: LLVM maps `@llvm.sin/cos/log/exp.f64` to libm calls
    // (or hardware intrinsics where available).  `log` is the natural log.
    if used_f64_sin  { out.push_str("declare double @llvm.sin.f64(double)\n"); }
    if used_f64_cos  { out.push_str("declare double @llvm.cos.f64(double)\n"); }
    if used_f64_ln   { out.push_str("declare double @llvm.log.f64(double)\n"); }
    if used_f64_exp  { out.push_str("declare double @llvm.exp.f64(double)\n"); }
    // `f64_atan`/`f64_tan` — LLVM has no `@llvm.atan`/`@llvm.tan` standard
    // intrinsics; call libm directly.  libm is pre-linked on both platforms.
    if used_f64_atan { out.push_str("declare double @atan(double)\n"); }
    if used_f64_tan  { out.push_str("declare double @tan(double)\n"); }
    if used_f64_pow {
        out.push('\n');
        // `@pow` is the C99 libm two-argument power function.  There is no
        // LLVM intrinsic for pow; direct libm call is the canonical approach.
        out.push_str("declare double @pow(double, double)\n");
    }
    if used_alloc_bytes || used_arrays || used_conversions || used_str_index || used_putchar || used_getchar
        || used_input_i64 || used_input_str || used_str_concat || used_str_eq {
        out.push('\n');
        if used_alloc_bytes || used_arrays {
            out.push_str("declare ptr @calloc(i64, i64)\n");
        }
        if used_arrays || used_conversions || used_str_index {
            // The trap target — out-of-bounds for arrays (LANG-FULL E5) and
            // `str_index`, and out-of-range for `real_to_int_*` (LANG-FULL E8).
            // `llvm.trap` is an intrinsic — declaring it is harmless and keeps
            // the module explicit. Declared once even when several users are present.
            out.push_str("declare void @llvm.trap()\n");
        }
        if used_conversions {
            // Rounding intrinsics for `real_to_int_floor` (toward −∞) and
            // `real_to_int_trunc` (toward zero). `int_to_real` needs no declare
            // (`sitofp` is a core instruction, not an intrinsic).
            out.push_str("declare double @llvm.floor.f64(double)\n");
            out.push_str("declare double @llvm.trunc.f64(double)\n");
        }
        if used_putchar {
            out.push_str("declare i32 @putchar(i32)\n");
        }
        if used_getchar {
            out.push_str("declare i32 @getchar()\n");
        }
        if used_input_i64 {
            // `@__twig_input_i64` is provided by `twig_runtime.c` in the AOT archive:
            // reads one line from stdin and parses it as `int64_t`; returns 0 on
            // EOF or parse failure (V1 permissive contract).
            out.push_str("declare i64 @__twig_input_i64()\n");
        }
        if used_input_str {
            // `@__twig_input_str` (E4-dyn) is provided by `twig_runtime.c`: reads one
            // line and returns an i64 handle to a `[i64 len][bytes]` heap block — the
            // runtime-string repr `print_str` reads the length from at run time.
            out.push_str("declare i64 @__twig_input_str()\n");
        }
        if used_str_concat {
            // `@__twig_str_concat` (E4-dyn) is provided by `twig_runtime.c`: reads both
            // operands' `[i64 len][bytes]` headers and returns an i64 handle to a fresh
            // joined block. Used when `str_concat` has a non-literal (runtime) operand.
            out.push_str("declare i64 @__twig_str_concat(i64, i64)\n");
        }
        if used_str_eq {
            // `@__twig_str_eq` (E4-dyn) is provided by `twig_runtime.c`: compares both
            // operands' `[i64 len][bytes]` headers then `memcmp`s the bytes, returning
            // 1/0. Used when `str_eq` has a non-literal (runtime) operand.
            out.push_str("declare i64 @__twig_str_eq(i64, i64)\n");
        }
    }
    if !used_lispy.is_empty() {
        out.push('\n');
        for (_iir_name, symbol, arity) in &used_lispy {
            let params = vec!["i64"; *arity].join(", ");
            out.push_str(&format!("declare i64 @{symbol}({params})\n"));
        }
    }

    // ── String literal constants (LANG-FULL E4 literal-output foothold) ─────
    //
    // Static backends use the unmanaged string layout from `lang-full-e4-strings`:
    // an `i64` byte-length header followed by the bytes. `str_const` binds a
    // pointer to this header, `str_len`/`str_eq`/`str_cmp` materialise literal metadata,
    // and `print_str` passes `header+8,len` to the C runtime. Richer ops
    // (`str_index`, `str_concat`) remain literal-only in this slice, but this
    // representation leaves the header in place for those later loads/checks.
    let (string_defs, string_literals) = collect_string_literals(module);
    if !string_defs.is_empty() {
        out.push('\n');
        for def in &string_defs {
            let len = def.bytes.len();
            let bytes = llvm_c_bytes(&def.bytes);
            out.push_str(&format!(
                "{} = private unnamed_addr constant {{ i64, [{} x i8] }} {{ i64 {}, [{} x i8] c\"{}\" }}, align 8\n",
                def.symbol, len, len, len, bytes
            ));
        }
    }

    // ── Module globals (LANG-FULL E6 layer 1) ─────────────────────────────
    //
    // Collect every distinct global name read/written by a `global_load`/
    // `global_store` (in first-seen order) and assign each an index-based LLVM
    // symbol `@__twig_global_N`. Index-based (not name-based) so an arbitrary
    // source identifier can never produce an invalid or colliding LLVM symbol —
    // the same lazy-slot discipline the native `_twig_globals` backend uses.
    // Each is `internal global i64 0` (zero-initialised, matching every other
    // backend's never-written-global-reads-0 convention).
    let globals = collect_global_syms(module);
    if !globals.is_empty() {
        out.push('\n');
        // Emit in symbol order (0,1,2,…) for stable, readable output.
        let mut defs: Vec<(&String, &String)> = globals.iter().collect();
        defs.sort_by(|a, b| a.1.cmp(b.1));
        for (_name, sym) in defs {
            out.push_str(&format!("{sym} = internal global i64 0\n"));
        }
    }

    // ── Function bodies ───────────────────────────────────────────────────
    for func in &module.functions {
        out.push('\n');
        lower_function(func, &callee_sigs, &globals, &string_literals, &mut out)?;
    }

    Ok(out)
}

fn collect_string_literals(
    module: &IIRModule,
) -> (Vec<LlvmStringLiteralDef>, HashMap<String, LlvmStringLiteralRef>) {
    let mut defs = Vec::new();
    let mut map = HashMap::new();
    fn intern_string_literal(
        text: &str,
        defs: &mut Vec<LlvmStringLiteralDef>,
        map: &mut HashMap<String, LlvmStringLiteralRef>,
    ) {
        if map.contains_key(text) {
            return;
        }
        let symbol = format!("@__twig_str_{}", defs.len());
        let bytes = text.as_bytes().to_vec();
        map.insert(text.to_string(), LlvmStringLiteralRef {
            symbol: symbol.clone(),
            len: bytes.len(),
        });
        defs.push(LlvmStringLiteralDef { symbol, bytes });
    }
    for func in &module.functions {
        let mut fn_values: HashMap<String, String> = HashMap::new();
        let mut fn_ints: HashMap<String, i64> = HashMap::new();
        for instr in &func.instructions {
            match instr.op.as_str() {
                "const" => {
                    if let (Some(dest), Some(Operand::Int(value))) =
                        (instr.dest.as_ref(), instr.srcs.first())
                    {
                        fn_ints.insert(dest.clone(), *value);
                    }
                }
                "mov" => {
                    let (Some(dest), Some(Operand::Var(src))) =
                        (instr.dest.as_ref(), instr.srcs.first())
                    else {
                        continue;
                    };
                    if let Some(value) = fn_values.get(src).cloned() {
                        fn_values.insert(dest.clone(), value);
                    }
                    if let Some(value) = fn_ints.get(src).copied() {
                        fn_ints.insert(dest.clone(), value);
                    }
                }
                "add" | "sub" | "mul" | "div" => {
                    let Some(dest) = instr.dest.as_ref() else {
                        continue;
                    };
                    let left = instr.srcs.first().and_then(|src| match src {
                        Operand::Int(value) => Some(*value),
                        Operand::Var(name) => fn_ints.get(name).copied(),
                        _ => None,
                    });
                    let right = instr.srcs.get(1).and_then(|src| match src {
                        Operand::Int(value) => Some(*value),
                        Operand::Var(name) => fn_ints.get(name).copied(),
                        _ => None,
                    });
                    let value = match (instr.op.as_str(), left, right) {
                        ("add", Some(left), Some(right)) => left.checked_add(right),
                        ("sub", Some(left), Some(right)) => left.checked_sub(right),
                        ("mul", Some(left), Some(right)) => left.checked_mul(right),
                        ("div", Some(left), Some(right)) if right != 0 => left.checked_div(right),
                        _ => None,
                    };
                    if let Some(value) = value {
                        fn_ints.insert(dest.clone(), value);
                    }
                }
                "str_const" => {
                    let Some(Operand::Str(s)) = instr.srcs.first() else {
                        continue;
                    };
                    intern_string_literal(s, &mut defs, &mut map);
                    if let Some(dest) = instr.dest.as_ref() {
                        fn_values.insert(dest.clone(), s.clone());
                    }
                }
                "str_concat" => {
                    let (Some(dest), Some(Operand::Var(left)), Some(Operand::Var(right))) = (
                        instr.dest.as_ref(),
                        instr.srcs.first(),
                        instr.srcs.get(1),
                    ) else {
                        continue;
                    };
                    let (Some(left), Some(right)) = (fn_values.get(left), fn_values.get(right))
                    else {
                        continue;
                    };
                    let value = format!("{left}{right}");
                    intern_string_literal(&value, &mut defs, &mut map);
                    fn_values.insert(dest.clone(), value);
                }
                "str_slice" => {
                    let (
                        Some(dest),
                        Some(Operand::Var(src)),
                        Some(Operand::Var(start)),
                        Some(Operand::Var(end)),
                    ) = (
                        instr.dest.as_ref(),
                        instr.srcs.first(),
                        instr.srcs.get(1),
                        instr.srcs.get(2),
                    ) else {
                        continue;
                    };
                    let (Some(source), Some(start), Some(end)) = (
                        fn_values.get(src),
                        fn_ints.get(start).copied(),
                        fn_ints.get(end).copied(),
                    ) else {
                        continue;
                    };
                    if start < 0 || end < start || end as usize > source.len() {
                        continue;
                    }
                    let bytes = source.as_bytes()[start as usize..end as usize].to_vec();
                    let Ok(value) = String::from_utf8(bytes) else {
                        continue;
                    };
                    intern_string_literal(&value, &mut defs, &mut map);
                    fn_values.insert(dest.clone(), value);
                }
                "str_len" => {
                    let (Some(dest), Some(Operand::Var(src))) =
                        (instr.dest.as_ref(), instr.srcs.first())
                    else {
                        continue;
                    };
                    if let Some(value) = fn_values.get(src) {
                        fn_ints.insert(dest.clone(), value.len() as i64);
                    }
                }
                _ => {}
            }
        }
    }
    (defs, map)
}

fn llvm_c_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for b in bytes {
        out.push('\\');
        out.push_str(&format!("{:02X}", *b));
    }
    out
}

/// Collect every distinct module-global name (read or written) into a map
/// `name → "@__twig_global_N"`, numbered in first-seen order across all
/// functions (LANG-FULL E6 layer 1).
fn collect_global_syms(module: &IIRModule) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut next = 0usize;
    for f in &module.functions {
        for i in &f.instructions {
            if i.op == "global_load" || i.op == "global_store" {
                if let Some(Operand::Str(name)) = i.srcs.first() {
                    map.entry(name.clone()).or_insert_with(|| {
                        let s = format!("@__twig_global_{next}");
                        next += 1;
                        s
                    });
                }
            }
        }
    }
    map
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
    /// IIR string var name → byte length. The pointer itself lives in `env`;
    /// this sidecar lets `print_str` pass `(base+8,len)` without inventing a
    /// general string value model for LLVM locals.
    str_lens: HashMap<String, usize>,
    /// IIR string var name → literal text for literal-only E4 folds (`str_eq`).
    str_values: HashMap<String, String>,
    /// Per-function counter for synthesized SSA names — used both for
    /// post-cmp zext'd values and for fallthrough block labels.
    counter: u32,
    /// The function name (for error messages).
    fn_name: &'a str,
    /// Module-wide map of every user-defined function's signature.  Built
    /// up-front by `lower_iir_to_llvm` before any function body is lowered.
    callee_sigs: &'a HashMap<String, FnSig>,
    /// Module-wide map: global variable name → its LLVM global symbol
    /// (`@__twig_global_N`). Built once by `lower_iir_to_llvm` so the same name
    /// resolves to the same symbol across every function (LANG-FULL E6).
    globals: &'a HashMap<String, String>,
    /// Module-wide map of source literal text → LLVM private constant metadata.
    string_literals: &'a HashMap<String, LlvmStringLiteralRef>,
    /// Is the current LLVM basic block still **open** (no terminator yet)?
    /// LLVM requires every block to end in a terminator (`br`/`ret`/…). IIR
    /// blocks fall through to the next `label` implicitly, and a block whose
    /// body is all tracked-not-emitted `const`/`mov` emits nothing — so two
    /// `label`s can land back-to-back. Before emitting a `label` while a block
    /// is open we synthesize an explicit `br` fallthrough. `false` after
    /// `jmp`/`ret`; `true` at entry and after every `label`/`jmp_if_*`.
    /// (McCarthy W12b-3 — needed once `COND`'s clause blocks appeared.)
    block_open: bool,
    /// Variables that are assigned in **more than one place** (e.g. a `COND`
    /// result var written in each clause block). The `const`/`mov` side-map
    /// trick only models straight-line SSA, so a cross-block variable collapses
    /// to its last assignment. We instead give each such variable a stack slot
    /// (`alloca`): every assignment becomes a `store`, every read a `load`. This
    /// is the naive-frontend / `opt -mem2reg` pattern (McCarthy W12b-3, F5).
    slots: std::collections::HashSet<String>,
    /// The LLVM stack-slot type for each promoted variable — `"i64"` for the
    /// usual integer/word values, `"double"` for an `f64` variable (LANG-FULL
    /// enabler E3). A slot's `alloca`/`load`/`store` all use this type so a
    /// `real` local stores a `double` into a `double` slot instead of the old
    /// invalid `store i64 <double>`. Any slot not present here defaults to
    /// `i64`. (See [`collect_slot_types`].)
    slot_types: std::collections::HashMap<String, &'static str>,
}

impl FnState<'_> {
    fn fresh(&mut self, hint: &str) -> String {
        self.counter += 1;
        format!("%__{}{}", hint, self.counter)
    }

    /// The LLVM type of a promoted variable's stack slot (`"i64"` by default,
    /// `"double"` for an `f64` slot).
    fn slot_ty(&self, name: &str) -> &'static str {
        self.slot_types.get(name).copied().unwrap_or("i64")
    }

    /// Like [`fresh`](Self::fresh) but without the leading `%` — a *bare* SSA
    /// name suitable for use as an `IIRInstr::dest` (the lowering helpers add
    /// the `%` themselves). Used to give a promoted stack-slot's assignment a
    /// unique SSA name so a variable written more than once does not emit
    /// `%v = …` twice (LLVM rejects "multiple definition of local value").
    fn fresh_bare(&mut self, hint: &str) -> String {
        self.counter += 1;
        format!("__{}{}", hint, self.counter)
    }
}

/// Emit one LLVM `define` block for one IIR function.
fn lower_function(
    func: &IIRFunction,
    callee_sigs: &HashMap<String, FnSig>,
    globals: &HashMap<String, String>,
    string_literals: &HashMap<String, LlvmStringLiteralRef>,
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
    // McCarthy W12b-3: any variable assigned in 2+ instructions is promoted to a
    // stack slot (an `alloca` in the entry block). Most slots are `i64` (every
    // tagged-word / scalar value), but an `f64` local gets a `double` slot
    // (LANG-FULL E3) — `collect_slot_types` decides per slot.
    let slots = collect_slot_vars(func);
    let slot_types = collect_slot_types(func, &slots);
    for slot in &slots {
        let ty = slot_types.get(slot).copied().unwrap_or("i64");
        out.push_str(&format!("  %{slot}.slot = alloca {ty}\n"));
    }

    // Initialise the slot of any **promoted parameter** (a parameter that is
    // reassigned in the body) from its incoming SSA argument, so the first
    // `load` sees the caller's value. Narrow integer / `i1` params are zero-
    // extended to the i64 slot width. Non-slot params stay pure SSA (seeded
    // into `env` below) and are unaffected.
    for (pname, pty) in &func.params {
        // Only initialise slots we can represent in the i64 slot model. A
        // non-compatible param (e.g. a `float` reassigned 2+ times) can still be
        // in `slots` via the body-`dest` count, but the i64 load/store protocol
        // already can't model it — so we must NOT emit a bogus `zext double …`
        // here. That float-in-slot case is pre-existing and tracked under E3.
        if !slots.contains(pname) || !param_slot_compatible(pty) {
            continue;
        }
        let pllvm = llvm_type_for(pty, &func.name)?;
        let init = if pllvm == "i64" {
            format!("%{pname}")
        } else {
            // i1 / i8 / i16 / i32 → widen to the i64 slot.
            let widened = format!("%{pname}.init");
            out.push_str(&format!("  {widened} = zext {pllvm} %{pname} to i64\n"));
            widened
        };
        out.push_str(&format!("  store i64 {init}, ptr %{pname}.slot\n"));
    }

    let mut state = FnState {
        env: HashMap::new(),
        env_i1: HashMap::new(),
        str_lens: HashMap::new(),
        str_values: HashMap::new(),
        counter: 0,
        fn_name: &func.name,
        callee_sigs,
        globals,
        string_literals,
        block_open: true, // the entry block is open until its first terminator.
        slots,
        slot_types,
    };
    for (pname, pty) in &func.params {
        let llvm_val = format!("%{pname}");
        state.env.insert(pname.clone(), llvm_val.clone());
        // Bool/i1 parameters arrive as i1 SSA values — seed env_i1 so
        // `lower_bitwise` can use them directly without `trunc i64 → i1`.
        if matches!(pty.as_str(), "bool" | "i1") {
            state.env_i1.insert(pname.clone(), llvm_val);
        }
    }

    // ── Body ──────────────────────────────────────────────────────────────
    for instr in &func.instructions {
        lower_instr_with_slots(instr, &mut state, out)?;
    }

    out.push_str("}\n");
    Ok(())
}

/// Collect the variables that are the `dest` of **two or more** instructions in
/// `func`. Such a variable cannot be modelled by the `const`/`mov` compile-time
/// side-map (which keeps only the latest binding and so is valid only within a
/// single straight-line block); it needs a stack slot so each assignment is a
/// real `store` and each read a real `load` (McCarthy W12b-3, F5 — `COND`).
///
/// A **parameter** counts as already having one assignment — the incoming
/// argument binding. So a parameter that is reassigned even *once* in the body
/// (e.g. `x = x + 1`, the common shape of a loop accumulator) must also become a
/// stack slot: across a loop back-edge the straight-line side-map is invalid and
/// would silently drop the update, producing wrong LLVM IR (LANG-FULL — LLVM is
/// first-class). We seed each i64-slot-compatible parameter with a count of 1 so
/// that a single later reassignment crosses the `>= 2` promotion threshold.
fn collect_slot_vars(func: &IIRFunction) -> std::collections::HashSet<String> {
    use std::collections::HashMap as Map;
    let mut counts: Map<&str, usize> = Map::new();
    for (pname, pty) in &func.params {
        if param_slot_compatible(pty) {
            counts.insert(pname.as_str(), 1);
        }
    }
    // E4-dyn: a `str` variable is promoted to a slot only when it is assigned in
    // **more than one basic block** — i.e. its value is chosen by control flow
    // (a branch), so the compile-time last-write-wins string tracking can no
    // longer resolve it to a single literal and it must carry a runtime `i64`
    // **handle** through memory.  A str reassigned twice *straight-line*
    // (`s := "OK"; s := "NO"`) keeps the literal fast path — the linear tracking
    // is exactly right there.  Basic-block boundaries: a `label` starts a new
    // block, and a terminator (`jmp*`/`ret*`) ends one.
    let mut str_blocks: Map<&str, std::collections::HashSet<usize>> = Map::new();
    let mut block: usize = 0;
    for instr in &func.instructions {
        let op = instr.op.as_str();
        if op == "label" {
            block += 1;
        }
        if let Some(dest) = &instr.dest {
            if instr.type_hint == "str" {
                str_blocks.entry(dest.as_str()).or_default().insert(block);
            } else {
                *counts.entry(dest.as_str()).or_insert(0) += 1;
            }
        }
        if matches!(op, "jmp" | "jmp_if_false" | "jmp_if_true" | "ret" | "ret_void") {
            block += 1;
        }
    }
    let mut slots: std::collections::HashSet<String> = counts
        .into_iter()
        .filter(|&(_, n)| n >= 2)
        .map(|(name, _)| name.to_string())
        .collect();
    for (name, blocks) in str_blocks {
        if blocks.len() >= 2 {
            slots.insert(name.to_string());
        }
    }
    slots
}

/// Decide the LLVM stack-slot type for each promoted variable in `slots`.
///
/// A slot is `"double"` if it ever holds an `f64` value — i.e. some
/// instruction whose `dest` is that slot carries a float `type_hint` (a
/// `const f64`, an `f64` arithmetic op, an `f64` `mov`, …). Otherwise it is the
/// default `"i64"` word slot. (Float *parameters* are not promoted to slots —
/// `param_slot_compatible` excludes them — so every slot we see here is a body
/// local; a real local seeded with `const … : f64` then reassigned is the
/// shape `ScalarType::Real` produces.) Enabler E3 (real arithmetic): without
/// this, an `f64` variable was given an `i64` slot and `store i64 <double>`
/// produced invalid IR that `clang` rejected.
fn collect_slot_types(
    func: &IIRFunction,
    slots: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, &'static str> {
    let mut types = std::collections::HashMap::new();
    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            if slots.contains(dest) && is_float_type(&instr.type_hint) {
                types.insert(dest.clone(), "double");
            }
        }
    }
    types
}

/// Whether a parameter of IIR type `pty` can be promoted to a stack slot.
/// Slots are `i64`, so only values that already flow as a 64-bit word qualify:
/// every integer width, `bool`, `any`, `symbol`, and the lisp heap references.
/// Floats/doubles do not fit the i64 slot model, so a reassigned float parameter
/// is left as SSA (that path is a separate concern, tracked under enabler E3 —
/// real arithmetic — and is no worse than before this change).
fn param_slot_compatible(pty: &str) -> bool {
    matches!(
        pty,
        "bool" | "i1" | "u4" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64"
            | "any" | "symbol"
    ) || pty.starts_with("ref<Lispy")
}

/// Wrap [`lower_instr`] with the slot (`alloca`/`load`/`store`) protocol:
///
/// 1. **Pre-load:** for every `Var` source operand that is a slot, emit
///    `%t = load i64, ptr %v.slot` and temporarily rebind it in `env` so the
///    instruction reads the loaded value.
/// 2. Lower the instruction normally.
/// 3. **Post-store:** if `dest` is a slot, emit `store i64 <value>, ptr %v.slot`
///    (the value the instruction left in `env[dest]` — a literal for `const`/`mov`
///    or an SSA name for an emitted op) and then drop `env[dest]` so the variable
///    is only ever read back through its slot.
/// 4. Restore the env bindings overridden in step 1.
///
/// Redundant loads/stores are fine: `opt -mem2reg` collapses them, and an
/// un-optimized `clang -O0` build runs them correctly.
fn lower_instr_with_slots(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    if state.slots.is_empty() {
        return lower_instr(instr, state, out); // fast path — no promoted vars.
    }

    // 1. Pre-load slot source operands.
    let mut saved: Vec<(String, Option<String>)> = Vec::new();
    for op in &instr.srcs {
        if let Operand::Var(name) = op {
            if state.slots.contains(name) {
                let ty = state.slot_ty(name);
                let fresh = state.fresh("ld");
                out.push_str(&format!("  {fresh} = load {ty}, ptr %{name}.slot\n"));
                let old = state.env.insert(name.clone(), fresh);
                saved.push((name.clone(), old));
            }
        }
    }

    // 2. Lower the instruction.
    //
    // If `dest` is a slot, rename it to a *fresh* SSA name first. A
    // value-producing op (`add`/`zext`/`load_byte`/…) emits `%<dest> = …`
    // using the dest name verbatim; a slot variable is by definition assigned
    // 2+ times, so reusing its name would emit `%v = …` twice — which LLVM
    // rejects ("multiple definition of local value named 'v'"). Lowering a
    // clone with a unique dest name sidesteps that; the post-store below then
    // writes the produced value into the *original* variable's slot. (`const`
    // and `mov` slot-dests emit no `%dest = …` line, so they were always fine —
    // but routing them through the same rename is harmless and keeps the code
    // uniform.) This is the LANG-MATRIX LM-L-Brainfuck trigger: BF's `ptr`/`v`
    // are the first slot variables to be the dest of real arithmetic.
    let slot_dest = instr
        .dest
        .as_ref()
        .filter(|d| state.slots.contains(*d))
        .cloned();
    if let Some(orig) = &slot_dest {
        let fresh = state.fresh_bare("slot");
        let mut renamed = instr.clone();
        renamed.dest = Some(fresh.clone());
        lower_instr(&renamed, state, out)?;

        // 3a. Post-store the produced value into the original slot.
        if let Some(val) = state.env.get(&fresh).cloned() {
            let ty = state.slot_ty(orig);
            // E4-dyn: a `str` value is carried in `env` as a global-symbol
            // pointer (e.g. `@.str.3`), but its slot stores the runtime *handle*
            // (an `i64` block address).  Convert the symbol to an integer first —
            // a literal string's `{i64 len, [N x i8]}` global address IS a valid
            // string handle, so a str slot and a runtime heap string are the same
            // representation.  (Non-`@` values — SSA registers, integer literals —
            // store directly.)
            if ty == "i64" && val.starts_with('@') {
                let h = state.fresh("strh");
                out.push_str(&format!("  {h} = ptrtoint ptr {val} to i64\n"));
                out.push_str(&format!("  store i64 {h}, ptr %{orig}.slot\n"));
            } else {
                out.push_str(&format!("  store {ty} {val}, ptr %{orig}.slot\n"));
            }
        }
        state.env.remove(&fresh); // the fresh temp is not referenced again.
        state.env.remove(orig); // future reads of `orig` go through its slot load.
    } else {
        lower_instr(instr, state, out)?;
    }

    // 4. Restore the env bindings we overrode for the pre-loads.
    for (name, old) in saved {
        match old {
            Some(v) => {
                state.env.insert(name, v);
            }
            None => {
                state.env.remove(&name);
            }
        }
    }
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
            state.block_open = false; // `ret` terminates the block.
            Ok(())
        }

        // ── ret <var> ───────────────────────────────────────────────────
        "ret" => {
            let ty = llvm_type_for(&instr.type_hint, fn_name)?;
            let mut operand =
                resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, fn_name)?;
            // E4-dyn (E4d-2b): a `str` is returned as an i64 handle. A
            // single-assignment string is tracked as its literal GLOBAL POINTER
            // (`@__twig_str_N`), so returning it directly would emit
            // `ret i64 @global` — a type error. A literal's `{i64 len,[N×i8]}`
            // address IS a valid handle, so convert the pointer with `ptrtoint`
            // first (branch-selected / call-result strings already carry an i64).
            if instr.type_hint == "str" && operand.starts_with('@') {
                let h = state.fresh("reth");
                out.push_str(&format!("  {h} = ptrtoint ptr {operand} to i64\n"));
                operand = h;
            }
            out.push_str(&format!("  ret {ty} {operand}\n"));
            state.block_open = false; // `ret` terminates the block.
            Ok(())
        }

        // ── arithmetic ──────────────────────────────────────────────────
        //
        // Two-operand integer or float operation.  Signedness comes from
        // the type_hint prefix (`i*` = signed, `u*` = unsigned), which only
        // matters for `div` and `mod`/`rem` (LLVM splits these into `sdiv`/`udiv`
        // and `srem`/`urem`; `add`/`sub`/`mul` are signedness-agnostic).
        "add" | "sub" | "mul" | "div" | "mod" | "rem" => {
            lower_arith(instr.op.as_str(), instr, state, out)
        }

        // ── bitwise / logical ───────────────────────────────────────────
        //
        // On `bool`/`i1` this is logical `and`/`or`/`xor`; on integer widths
        // it is the usual bitwise operation.  The same IIR opcodes are used
        // by WASM/JVM/CLR/BEAM, so LLVM accepts them too.
        "and" | "or" | "xor" => lower_bitwise(instr.op.as_str(), instr, state, out),

        // ── bitwise NOT ─────────────────────────────────────────────────
        //
        // LLVM has no `not` instruction; bitwise complement is `xor x, -1`
        // (flip every bit). For a narrow unsigned width the E2 mask brings it
        // back into range (`~0u8 = 255`). Used by Nib/Oct unary `~`.
        "not" => lower_not(instr, state, out),

        // ── unary negation ───────────────────────────────────────────────
        //
        // `fneg` for floats; `sub 0, x` for integers.  BASIC ABS/SGN emit
        // this to negate a real value inside an inline conditional.
        "neg" => lower_neg(instr, state, out),

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
            // If the current block never got a terminator (its body was all
            // tracked-not-emitted `const`/`mov`), LLVM would see two labels
            // back-to-back — an invalid empty block. Synthesize the implicit
            // fallthrough as an explicit `br` to this label (McCarthy W12b-3).
            if state.block_open {
                out.push_str(&format!("  br label %{name}\n"));
            }
            out.push_str(&format!("{name}:\n"));
            state.block_open = true;
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
            state.block_open = false; // unconditional branch terminates the block.
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

        // ── byte-tape memory (LLVM05 — LANG-MATRIX LM-L Brainfuck) ───────
        "alloc_bytes" => lower_alloc_bytes(instr, state, out),
        "load_byte" => lower_load_byte(instr, state, out),
        "store_byte" => lower_store_byte(instr, state, out),
        "alloc_array" => lower_alloc_array(instr, state, out),
        "array_get" => lower_array_get(instr, state, out),
        "array_set" => lower_array_set(instr, state, out),
        "array_len" => lower_array_len(instr, state, out),

        "global_load" => lower_global_load(instr, state, out),
        "global_store" => lower_global_store(instr, state, out),

        // ── numeric conversions integer↔real (LANG-FULL E8) ───────────────
        "int_to_real" => lower_int_to_real(instr, state, out),
        "real_to_int_trunc" => lower_real_to_int(instr, state, out, /*floor=*/ false),
        "real_to_int_floor" => lower_real_to_int(instr, state, out, /*floor=*/ true),
        // AL8 sqrt + transcendentals — via LLVM intrinsics.
        "f64_sqrt" => lower_f64_sqrt(instr, state, out),
        "f64_sin"  => lower_f64_intrinsic(instr, state, out, "f64_sin",  "@llvm.sin.f64"),
        "f64_cos"  => lower_f64_intrinsic(instr, state, out, "f64_cos",  "@llvm.cos.f64"),
        "f64_ln"   => lower_f64_intrinsic(instr, state, out, "f64_ln",   "@llvm.log.f64"),
        "f64_exp"  => lower_f64_intrinsic(instr, state, out, "f64_exp",  "@llvm.exp.f64"),
        // AL8-arctan: direct libm calls (no LLVM intrinsic for atan/tan).
        "f64_atan" => lower_f64_intrinsic(instr, state, out, "f64_atan", "@atan"),
        "f64_tan"  => lower_f64_intrinsic(instr, state, out, "f64_tan",  "@tan"),
        // BA-pow — two-argument pow(base, exp) via libm `@pow`.
        "f64_pow" => lower_f64_pow(instr, state, out),

        // ── strings (LANG-FULL E4 literal-output foothold) ─────────────────
        "str_const" => lower_str_const(instr, state),
        "str_concat" => lower_str_concat(instr, state, out),
        "str_slice" => lower_str_slice(instr, state, out),
        "str_index" => lower_str_index(instr, state, out),
        "str_len" => lower_str_len(instr, state, out),
        "str_eq" => lower_str_eq(instr, state, out),
        "str_cmp" => lower_str_cmp(instr, state),
        "print_str" => lower_print_str(instr, state, out),

        other => Err(IIRLlvmError::UnsupportedOp {
            function: fn_name.into(),
            op: other.into(),
        }),
    }
}

fn lower_str_const(
    instr: &IIRInstr,
    state: &mut FnState,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_const", state.fn_name)?.to_string();
    let literal = match instr.srcs.first() {
        Some(Operand::Str(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_const requires srcs[0] = Operand::Str(literal)".into(),
            });
        }
    };
    let info = state.string_literals.get(literal).ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_const literal {literal:?} was not collected"),
        }
    })?;
    state.env.insert(dest.clone(), info.symbol.clone());
    state.str_lens.insert(dest.clone(), info.len);
    state.str_values.insert(dest, literal.clone());
    Ok(())
}

fn lower_print_str(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    if instr.dest.is_some() {
        return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "print_str must not have a dest".into(),
        });
    }
    let src = match instr.srcs.first() {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "print_str requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };

    // E4-dyn runtime path: any str value WITHOUT a compile-time length is a
    // runtime i64-**handle** — a branch-selected slot (E4d-2), OR a value that
    // arrives as a function **return value / call result** or a **parameter**
    // (E4d-2b).  In every case `env[src]` holds the handle (the slot protocol
    // pre-loads it; a `call` binds its i64 result; a param is its argument), so
    // read the length from the block header (`[i64 len][bytes…]`) at run time
    // rather than from compile-time metadata.  `inttoptr` recovers the pointer.
    // (A slot is never in `str_lens` — its `str_const` lowers through a renamed
    // temp — so this single check subsumes the old `slots.contains` test.)
    if !state.str_lens.contains_key(src) {
        let handle = state.env.get(src).cloned().ok_or_else(|| {
            IIRLlvmError::UndefinedVariable {
                function: state.fn_name.into(),
                name: src.clone(),
            }
        })?;
        let p = state.fresh("strp");
        out.push_str(&format!("  {p} = inttoptr i64 {handle} to ptr\n"));
        let len = state.fresh("strn");
        out.push_str(&format!("  {len} = load i64, ptr {p}\n"));
        let bytes = state.fresh("strb");
        out.push_str(&format!(
            "  {bytes} = getelementptr inbounds i8, ptr {p}, i64 8\n"
        ));
        out.push_str(&format!(
            "  call void @__print_str(ptr {bytes}, i64 {len})\n"
        ));
        return Ok(());
    }

    // Literal fast path: a single-assignment string folds to a known global +
    // compile-time length.
    let base = state.env.get(src).cloned().ok_or_else(|| {
        IIRLlvmError::UndefinedVariable {
            function: state.fn_name.into(),
            name: src.clone(),
        }
    })?;
    let len = state.str_lens.get(src).copied().ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("print_str source {src:?} is not a string literal value"),
        }
    })?;
    let bytes = state.fresh("str");
    out.push_str(&format!(
        "  {bytes} = getelementptr inbounds i8, ptr {base}, i64 8\n"
    ));
    out.push_str(&format!("  call void @__print_str(ptr {bytes}, i64 {len})\n"));
    Ok(())
}

fn lower_str_len(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_len", state.fn_name)?.to_string();
    let src = match instr.srcs.first() {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_len requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };
    // Literal fast path: a single-assignment string has a compile-time length.
    if let Some(len) = state.str_lens.get(src).copied() {
        state.env.insert(dest, len.to_string());
        return Ok(());
    }

    // E4-dyn runtime path (E4d-2b): a runtime string (slot / call result / param)
    // has no compile-time length — read it from the block header at run time.
    // `env[src]` is the i64 handle; the length is the leading `i64` of the
    // `[i64 len][bytes…]` block.
    let handle = state.env.get(src).cloned().ok_or_else(|| {
        IIRLlvmError::UndefinedVariable {
            function: state.fn_name.into(),
            name: src.clone(),
        }
    })?;
    let p = state.fresh("slp");
    out.push_str(&format!("  {p} = inttoptr i64 {handle} to ptr\n"));
    let len = state.fresh("sln");
    out.push_str(&format!("  {len} = load i64, ptr {p}\n"));
    state.env.insert(dest, len);
    Ok(())
}

fn lower_str_concat(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_concat", state.fn_name)?.to_string();
    let left = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_concat requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };
    let right = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s.clone(),
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_concat requires srcs[1] = Operand::Var(str)".into(),
            });
        }
    };

    // Literal fast path: when BOTH operands are compile-time string VALUES, the
    // collection pre-pass already interned the joined literal — reuse its symbol so
    // the result stays a known literal (downstream `str_len`/`str_index` keep folding).
    if let (Some(left_value), Some(right_value)) =
        (state.str_values.get(&left).cloned(), state.str_values.get(&right).cloned())
    {
        let value = format!("{left_value}{right_value}");
        let info = state.string_literals.get(&value).ok_or_else(|| {
            IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: format!("str_concat value {value:?} was not collected"),
            }
        })?;
        state.env.insert(dest.clone(), info.symbol.clone());
        state.str_lens.insert(dest.clone(), info.len);
        state.str_values.insert(dest, value);
        return Ok(());
    }

    // E4-dyn runtime path: at least one operand is a runtime string handle (an
    // `INPUT` result, a call result, a branch-selected string) with no compile-time
    // value. `env[operand]` holds the i64 handle to that operand's `[i64 len][bytes]`
    // block; call `@__twig_str_concat`, which reads both headers and returns a handle
    // to a fresh joined block. The result is a runtime string — stored ONLY in `env`,
    // with no `str_lens`/`str_values` entry — so `str_len`/`print_str` on it read the
    // length header at run time rather than folding a length that isn't known.
    let left_handle = state.env.get(&left).cloned().ok_or_else(|| {
        IIRLlvmError::UndefinedVariable {
            function: state.fn_name.into(),
            name: left.clone(),
        }
    })?;
    let right_handle = state.env.get(&right).cloned().ok_or_else(|| {
        IIRLlvmError::UndefinedVariable {
            function: state.fn_name.into(),
            name: right.clone(),
        }
    })?;
    let res = state.fresh("scc");
    out.push_str(&format!(
        "  {res} = call i64 @__twig_str_concat(i64 {left_handle}, i64 {right_handle})\n"
    ));
    state.env.insert(dest, res);
    Ok(())
}

fn lower_str_slice(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_slice", state.fn_name)?.to_string();
    let src = match instr.srcs.first() {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_slice requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };
    let start = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_slice requires srcs[1] = Operand::Var(start)".into(),
            });
        }
    };
    let end = match instr.srcs.get(2) {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_slice requires srcs[2] = Operand::Var(end)".into(),
            });
        }
    };
    let literal = state.str_values.get(src).ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_slice source {src:?} is not a string literal value"),
        }
    })?;
    let start_value = state
        .env
        .get(start)
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_slice start {start:?} is not a constant integer value"),
        })?;
    let end_value = state
        .env
        .get(end)
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_slice end {end:?} is not a constant integer value"),
        })?;
    if start_value < 0 || end_value < start_value || end_value as usize > literal.len() {
        out.push_str("  call void @llvm.trap()\n");
        state.env.insert(dest.clone(), "null".to_string());
        state.str_lens.insert(dest.clone(), 0);
        state.str_values.insert(dest, String::new());
        return Ok(());
    }
    let value = String::from_utf8(
        literal.as_bytes()[start_value as usize..end_value as usize].to_vec(),
    )
    .map_err(|_| IIRLlvmError::InvalidOperand {
        function: state.fn_name.into(),
        detail: format!("str_slice range {start_value}..{end_value} does not preserve UTF-8"),
    })?;
    let info = state.string_literals.get(&value).ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_slice value {value:?} was not collected"),
        }
    })?;
    state.env.insert(dest.clone(), info.symbol.clone());
    state.str_lens.insert(dest.clone(), info.len);
    state.str_values.insert(dest, value);
    Ok(())
}

fn lower_str_index(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_index", state.fn_name)?.to_string();
    let src = match instr.srcs.first() {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_index requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };
    let idx = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_index requires srcs[1] = Operand::Var(idx)".into(),
            });
        }
    };
    let literal = state.str_values.get(src).ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_index source {src:?} is not a string literal value"),
        }
    })?;
    let idx_value = state
        .env
        .get(idx)
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_index index {idx:?} is not a constant integer value"),
        })?;
    let Some(byte) = usize::try_from(idx_value)
        .ok()
        .and_then(|idx| literal.as_bytes().get(idx))
        .copied()
    else {
        out.push_str("  call void @llvm.trap()\n");
        state.env.insert(dest, "0".to_string());
        return Ok(());
    };
    state.env.insert(dest, byte.to_string());
    Ok(())
}

fn lower_str_eq(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_eq", state.fn_name)?.to_string();
    let left = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_eq requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };
    let right = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s.clone(),
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_eq requires srcs[1] = Operand::Var(str)".into(),
            });
        }
    };

    // Literal fast path: BOTH operands are compile-time string VALUES, so the
    // comparison folds to a constant 1/0 with no runtime call.
    if let (Some(lv), Some(rv)) =
        (state.str_values.get(&left).cloned(), state.str_values.get(&right).cloned())
    {
        state.env.insert(dest, if lv == rv { "1" } else { "0" }.into());
        return Ok(());
    }

    // E4-dyn runtime path: at least one operand is a runtime string handle (a param,
    // call result, branch-selected string, or `str_concat`/`str_slice` result). Both
    // must be i64 handles for `@__twig_str_eq(a, b)`, which reads the `[i64 len][bytes]`
    // headers and `memcmp`s. Each operand resolves to `env[op]`, which is either a
    // literal's GLOBAL POINTER (`@__twig_str_N` — convert with `ptrtoint`) or an i64
    // handle already (param / call-result / concat-result). Returns i64 1/0.
    let lop = state.env.get(&left).cloned().ok_or_else(|| IIRLlvmError::UndefinedVariable {
        function: state.fn_name.into(),
        name: left.clone(),
    })?;
    let lh = if lop.starts_with('@') {
        let h = state.fresh("seqh");
        out.push_str(&format!("  {h} = ptrtoint ptr {lop} to i64\n"));
        h
    } else {
        lop
    };
    let rop = state.env.get(&right).cloned().ok_or_else(|| IIRLlvmError::UndefinedVariable {
        function: state.fn_name.into(),
        name: right.clone(),
    })?;
    let rh = if rop.starts_with('@') {
        let h = state.fresh("seqh");
        out.push_str(&format!("  {h} = ptrtoint ptr {rop} to i64\n"));
        h
    } else {
        rop
    };
    let res = state.fresh("seq");
    out.push_str(&format!("  {res} = call i64 @__twig_str_eq(i64 {lh}, i64 {rh})\n"));
    state.env.insert(dest, res);
    Ok(())
}

fn lower_str_cmp(instr: &IIRInstr, state: &mut FnState) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "str_cmp", state.fn_name)?.to_string();
    let left = match instr.srcs.first() {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_cmp requires srcs[0] = Operand::Var(str)".into(),
            });
        }
    };
    let right = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "str_cmp requires srcs[1] = Operand::Var(str)".into(),
            });
        }
    };
    let left_value = state.str_values.get(left).ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_cmp left source {left:?} is not a string literal value"),
        }
    })?;
    let right_value = state.str_values.get(right).ok_or_else(|| {
        IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("str_cmp right source {right:?} is not a string literal value"),
        }
    })?;
    let value = match left_value.as_bytes().cmp(right_value.as_bytes()) {
        Ordering::Less => "-1",
        Ordering::Equal => "0",
        Ordering::Greater => "1",
    };
    state.env.insert(dest, value.into());
    Ok(())
}

/// Lower `int_to_real dest <- x` — widen an `i64` to `f64` with `sitofp`
/// (IEEE-754 round-to-nearest-even). The dest slot is already typed `double` by
/// `collect_slot_types` (the op's `type_hint` is `f64`).
fn lower_int_to_real(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "int_to_real", state.fn_name)?.to_string();
    let a = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    out.push_str(&format!("  %{dest} = sitofp i64 {a} to double\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `real_to_int_trunc` / `real_to_int_floor dest <- x` — round a `real` to
/// an integer, trapping (fail-closed, exactly like the VM and the E5 array
/// bounds) on a NaN/±∞/out-of-`i64`-range operand.
///
/// To match the VM's `real_to_i64_checked(f.floor()/f.trunc())` **exactly**, we
/// (1) round first with `@llvm.floor.f64` (toward −∞, `entier`) or
/// `@llvm.trunc.f64` (toward zero, `INT()`), (2) range-check the *rounded*
/// value, then (3) `fptosi` — which on an already-integral, in-range `double`
/// is exact and never the UB that a bare `fptosi` of an out-of-range value would
/// be.
fn lower_real_to_int(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
    floor: bool,
) -> Result<(), IIRLlvmError> {
    let opname = if floor { "real_to_int_floor" } else { "real_to_int_trunc" };
    let dest = require_dest(instr, opname, state.fn_name)?.to_string();
    let a = resolve_operand(instr.srcs.first(), &state.env, "f64", state.fn_name)?;

    let intrinsic = if floor { "@llvm.floor.f64" } else { "@llvm.trunc.f64" };
    let rounded = state.fresh("rrnd");
    out.push_str(&format!("  {rounded} = call double {intrinsic}(double {a})\n"));

    emit_real_range_check(&rounded, state, out);

    out.push_str(&format!("  %{dest} = fptosi double {rounded} to i64\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `f64_sqrt dest <- x` — IEEE-754 hardware square root.
///
/// Maps to LLVM's `@llvm.sqrt.f64` intrinsic, which lowers to `sqrtsd` on
/// x86_64 and `fsqrt` on aarch64 — no libm call, no trap.  NaN propagates
/// naturally; `sqrt(negative)` returns NaN per IEEE-754 (same as Rust's
/// `f64::sqrt`).  The ALGOL 60 spec §3.2.4 defines `sqrt` only for non-negative
/// arguments; the NaN result for negative inputs is the "undefined" outcome.
fn lower_f64_sqrt(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "f64_sqrt", state.fn_name)?.to_string();
    let a = resolve_operand(instr.srcs.first(), &state.env, "f64", state.fn_name)?;
    out.push_str(&format!("  %{dest} = call double @llvm.sqrt.f64(double {a})\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Generic helper for `f64 → f64` LLVM intrinsic calls.
///
/// Used for `sin`/`cos`/`ln`/`exp` — each maps to `call double @llvm.<op>.f64(double)`.
/// `ln` maps to `@llvm.log.f64` (LLVM uses `log` for natural log).
fn lower_f64_intrinsic(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
    iir_op: &str,
    llvm_fn: &str,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, iir_op, state.fn_name)?.to_string();
    let a = resolve_operand(instr.srcs.first(), &state.env, "f64", state.fn_name)?;
    out.push_str(&format!("  %{dest} = call double {llvm_fn}(double {a})\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `f64_pow dest <- base, exp` — two-argument IEEE-754 power via libm `@pow`.
///
/// LLVM has no `@llvm.pow.f64` intrinsic; the standard approach is a direct
/// call to the C library `pow(double, double)`.  The linker resolves `@pow`
/// from libm (`-lm`).  NaN / ±inf propagate per IEEE-754.
fn lower_f64_pow(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "f64_pow", state.fn_name)?.to_string();
    let base = resolve_operand(instr.srcs.first(), &state.env, "f64", state.fn_name)?;
    let exp_ = resolve_operand(instr.srcs.get(1), &state.env, "f64", state.fn_name)?;
    out.push_str(&format!("  %{dest} = call double @pow(double {base}, double {exp_})\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Emit the finiteness + `i64`-range check shared by the `real_to_int_*` ops:
/// trap (`@llvm.trap` + `unreachable`) unless the already-rounded operand is in
/// `[-2⁶³, 2⁶³)`. The comparisons are **ordered** (`fcmp oge`/`olt`), which are
/// `false` for NaN — so NaN and ±∞ fall through the same single check and trap.
/// The bounds are the `double` hex literals for −2⁶³ and +2⁶³ (`i64::MAX` =
/// 2⁶³−1 is unrepresentable as `double`, so the upper bound is the exact `< 2⁶³`
/// the VM uses). Mirrors `emit_bounds_check`; leaves the cursor in the "ok"
/// block.
fn emit_real_range_check(operand: &str, state: &mut FnState, out: &mut String) {
    let ge = state.fresh("rge");
    let lt = state.fresh("rlt");
    let inr = state.fresh("rin");
    state.counter += 1;
    let trap = format!("__rtrap{}", state.counter);
    state.counter += 1;
    let ok = format!("__rok{}", state.counter);
    // -2^63 = 0xC3E0000000000000 ; +2^63 = 0x43E0000000000000 (LLVM double hex).
    out.push_str(&format!("  {ge} = fcmp oge double {operand}, 0xC3E0000000000000\n"));
    out.push_str(&format!("  {lt} = fcmp olt double {operand}, 0x43E0000000000000\n"));
    out.push_str(&format!("  {inr} = and i1 {ge}, {lt}\n"));
    out.push_str(&format!("  br i1 {inr}, label %{ok}, label %{trap}\n"));
    out.push_str(&format!("{trap}:\n"));
    out.push_str("  call void @llvm.trap()\n");
    out.push_str("  unreachable\n");
    out.push_str(&format!("{ok}:\n"));
}

// ---------------------------------------------------------------------------
// LLVM05 — byte-tape memory ops (Brainfuck's implicit tape)
// ---------------------------------------------------------------------------

/// Lower `alloc_bytes dest <- size` — allocate a `size`-byte, zero-filled tape.
///
/// ```llvm
/// %dest = call ptr @calloc(i64 <size>, i64 1)
/// ```
///
/// `calloc` zero-initialises (Brainfuck cells start at 0). `dest` (the tape
/// base) is bound in `env` as an LLVM `ptr`; it is written exactly once by the
/// `lower_brainfuck_for_aot` preamble, so it is never a promoted stack slot —
/// later `load_byte`/`store_byte` read it straight from `env`.
///
/// The `size` operand is a compile-time constant from `lower_brainfuck_for_aot`
/// (30000); we emit it verbatim. A hostile hand-built IIR could pass a huge or
/// negative literal, but that only makes `calloc` return null at runtime (a
/// crash on first store, not a compile-time or memory-safety defect in this
/// emitter), so no extra guard is warranted here — exactly the contract the
/// native `alloc_bytes` lowering already relies on.
fn lower_alloc_bytes(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "alloc_bytes", state.fn_name)?.to_string();
    let size = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    out.push_str(&format!("  %{dest} = call ptr @calloc(i64 {size}, i64 1)\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `load_byte dest <- base, idx` — read one tape cell, zero-extended.
///
/// ```llvm
/// %p = getelementptr i8, ptr <base>, i64 <idx>
/// %b = load i8, ptr %p
/// %dest = zext i8 %b to i64
/// ```
///
/// The 8-bit cell becomes an `i64` register (the uniform BF value width); the
/// `zext` is the load half of the "byte width only at the tape boundary"
/// contract. `dest` may be a promoted slot — the slot wrapper stores the
/// `i64` value we leave in `env[dest]`.
fn lower_load_byte(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "load_byte", state.fn_name)?.to_string();
    let base = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    let idx = resolve_operand(instr.srcs.get(1), &state.env, "i64", state.fn_name)?;
    let p = state.fresh("gep");
    let b = state.fresh("byte");
    out.push_str(&format!("  {p} = getelementptr i8, ptr {base}, i64 {idx}\n"));
    out.push_str(&format!("  {b} = load i8, ptr {p}\n"));
    out.push_str(&format!("  %{dest} = zext i8 {b} to i64\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `store_byte base, idx, val` (no dest) — write the low byte of `val`.
///
/// ```llvm
/// %p = getelementptr i8, ptr <base>, i64 <idx>
/// %t = trunc i64 <val> to i8
/// store i8 %t, ptr %p
/// ```
///
/// The `trunc` is the store half of the tape-boundary contract; it is exactly
/// what makes Brainfuck's 8-bit cell wrap-around (`255 + 1 == 0`) fall out
/// even though the arithmetic ran at `i64` width.
fn lower_store_byte(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    if instr.dest.is_some() {
        return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "store_byte must not have a dest".into(),
        });
    }
    let base = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    let idx = resolve_operand(instr.srcs.get(1), &state.env, "i64", state.fn_name)?;
    let val = resolve_operand(instr.srcs.get(2), &state.env, "i64", state.fn_name)?;
    let p = state.fresh("gep");
    let t = state.fresh("trunc");
    out.push_str(&format!("  {p} = getelementptr i8, ptr {base}, i64 {idx}\n"));
    out.push_str(&format!("  {t} = trunc i64 {val} to i8\n"));
    out.push_str(&format!("  store i8 {t}, ptr {p}\n"));
    Ok(())
}

// ---------------------------------------------------------------------------
// LANG-FULL E6 (layer 1) — typed module globals
// ---------------------------------------------------------------------------

/// Resolve the `@__twig_global_N` symbol for the string-literal global name at
/// `instr.srcs[0]`. The name must be an `Operand::Str` (never a register), and
/// it must have been collected into the module's global map.
fn global_symbol<'a>(
    instr: &IIRInstr,
    state: &'a FnState,
    op: &str,
) -> Result<&'a str, IIRLlvmError> {
    let name = match instr.srcs.first() {
        Some(Operand::Str(s)) => s,
        _ => {
            return Err(IIRLlvmError::InvalidOperand {
                function: state.fn_name.into(),
                detail: format!("{op} expects a string-literal global name at srcs[0]"),
            })
        }
    };
    state.globals.get(name).map(String::as_str).ok_or_else(|| IIRLlvmError::InvalidOperand {
        function: state.fn_name.into(),
        detail: format!("{op}: global {name:?} was not collected (internal error)"),
    })
}

/// Lower `global_load dest <- "g"` — read the module global `g` (an `i64`).
///
/// ```llvm
/// %dest = load i64, ptr @__twig_global_N
/// ```
///
/// `dest` may be a promoted slot; the slot wrapper stores the `i64` we leave in
/// `env[dest]`.
fn lower_global_load(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "global_load", state.fn_name)?.to_string();
    let sym = global_symbol(instr, state, "global_load")?.to_string();
    out.push_str(&format!("  %{dest} = load i64, ptr {sym}\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `global_store "g", val` (no dest) — write the module global `g`.
///
/// ```llvm
/// store i64 <val>, ptr @__twig_global_N
/// ```
fn lower_global_store(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    if instr.dest.is_some() {
        return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "global_store must not have a dest".into(),
        });
    }
    let sym = global_symbol(instr, state, "global_store")?.to_string();
    let val = resolve_operand(instr.srcs.get(1), &state.env, "i64", state.fn_name)?;
    out.push_str(&format!("  store i64 {val}, ptr {sym}\n"));
    Ok(())
}

// ---------------------------------------------------------------------------
// LANG-FULL E5 — bounds-checked arrays (static / length-prefixed model)
// ---------------------------------------------------------------------------
//
// An array is a single `calloc` block laid out as a **length header followed by
// the elements**:
//
//   base ──► [ i64 length | element 0 | element 1 | … ]   (zero-filled)
//            └─ 8 bytes ──┘
//
// The IIR *handle* is a pointer to the **payload** (`base + 8`), so element
// access is a plain typed GEP (`getelementptr <T>, ptr handle, i64 idx`) and the
// length lives at `handle − 8`. Unlike the JVM/CLR managed arrays (whose runtime
// bounds-checks `*aload`/`ldelem` for free), the native/LLVM target has no such
// check, so every `array_get`/`array_set` emits an **explicit** compare against
// the stored length and branches to a trap (`llvm.trap`) on an out-of-range
// index — the static-backend realisation of E5's "OOB → trap" rule. A single
// **unsigned** compare (`icmp uge`) catches both a `>= len` index and a negative
// one (a negative `i64` is a huge unsigned value).

/// The LLVM element type + its byte size for an array element type hint.
fn array_elem_llvm(elem: &str, fn_name: &str) -> Result<(&'static str, u32), IIRLlvmError> {
    let ty = llvm_type_for(elem, fn_name)?;
    let size = match ty {
        "i1" | "i8" => 1,
        "i16" => 2,
        "i32" | "float" => 4,
        "i64" | "double" => 8,
        other => {
            return Err(IIRLlvmError::UnsupportedType {
                function: fn_name.into(),
                type_hint: format!("array element {other}"),
            })
        }
    };
    Ok((ty, size))
}

/// Lower `alloc_array dest <- count : array<T>`.
///
/// ```llvm
/// %sz    = mul i64 <count>, <elemsize>
/// %total = add i64 %sz, 8
/// %base  = call ptr @calloc(i64 %total, i64 1)
/// store i64 <count>, ptr %base                 ; length header
/// %dest  = getelementptr i8, ptr %base, i64 8  ; handle = payload
/// ```
///
/// **Trust boundary (size overflow).** `count` is a *compiler-produced* operand
/// (a constant or a bounded length expression from a frontend), not an
/// end-user-controlled value, so the size `mul`/`add` is left as plain wrapping
/// i64 arithmetic. A hostile *hand-built* IIR could pass `count ≈ 2⁶¹` so
/// `count*elemsize + 8` wraps to a small `calloc` while the stored `len` stays
/// huge — letting later in-bounds-looking indices overrun the undersized block at
/// runtime. This is the same trust contract the existing `alloc_bytes` lowering
/// already relies on (its `size` is likewise unchecked), and the array path is
/// strictly *safer*: it adds the per-access bounds check `alloc_bytes` lacks. A
/// future hardening could gate the size on `@llvm.umul.with.overflow.i64` and
/// branch to the same trap; unneeded for the trusted-frontend threat model.
fn lower_alloc_array(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "alloc_array", state.fn_name)?.to_string();
    let elem = array_elem_type(&instr.type_hint).ok_or_else(|| IIRLlvmError::InvalidOperand {
        function: state.fn_name.into(),
        detail: format!("alloc_array type_hint must be array<T>, got {:?}", instr.type_hint),
    })?;
    let (_, elem_size) = array_elem_llvm(&elem, state.fn_name)?;
    let count = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    let sz = state.fresh("asz");
    let total = state.fresh("atot");
    let base = state.fresh("abase");
    out.push_str(&format!("  {sz} = mul i64 {count}, {elem_size}\n"));
    out.push_str(&format!("  {total} = add i64 {sz}, 8\n"));
    out.push_str(&format!("  {base} = call ptr @calloc(i64 {total}, i64 1)\n"));
    out.push_str(&format!("  store i64 {count}, ptr {base}\n"));
    out.push_str(&format!("  %{dest} = getelementptr i8, ptr {base}, i64 8\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Emit the bounds check shared by `array_get`/`array_set`: load the length from
/// the header at `handle − 8`, compare the index unsigned, and branch to a trap
/// block on out-of-range. Leaves the cursor in a fresh "ok" block (still open).
fn emit_bounds_check(
    handle: &str,
    idx: &str,
    state: &mut FnState,
    out: &mut String,
) {
    let hdr = state.fresh("ahdr");
    let len = state.fresh("alen");
    let oob = state.fresh("aoob");
    state.counter += 1;
    let trap = format!("__atrap{}", state.counter);
    state.counter += 1;
    let ok = format!("__aok{}", state.counter);
    out.push_str(&format!("  {hdr} = getelementptr i8, ptr {handle}, i64 -8\n"));
    out.push_str(&format!("  {len} = load i64, ptr {hdr}\n"));
    out.push_str(&format!("  {oob} = icmp uge i64 {idx}, {len}\n"));
    out.push_str(&format!("  br i1 {oob}, label %{trap}, label %{ok}\n"));
    out.push_str(&format!("{trap}:\n"));
    out.push_str("  call void @llvm.trap()\n");
    out.push_str("  unreachable\n");
    out.push_str(&format!("{ok}:\n"));
}

/// Lower `array_get dest <- handle, idx : T` — bounds-checked element load.
fn lower_array_get(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "array_get", state.fn_name)?.to_string();
    let (elem_ty, _) = array_elem_llvm(&instr.type_hint, state.fn_name)?;
    let handle = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    let idx = resolve_operand(instr.srcs.get(1), &state.env, "i64", state.fn_name)?;
    emit_bounds_check(&handle, &idx, state, out);
    let ep = state.fresh("aep");
    out.push_str(&format!("  {ep} = getelementptr {elem_ty}, ptr {handle}, i64 {idx}\n"));
    out.push_str(&format!("  %{dest} = load {elem_ty}, ptr {ep}\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `array_set handle, idx, val : T` (no dest) — bounds-checked element store.
fn lower_array_set(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    if instr.dest.is_some() {
        return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "array_set must not have a dest".into(),
        });
    }
    let (elem_ty, _) = array_elem_llvm(&instr.type_hint, state.fn_name)?;
    let handle = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    let idx = resolve_operand(instr.srcs.get(1), &state.env, "i64", state.fn_name)?;
    let val = resolve_operand(instr.srcs.get(2), &state.env, elem_ty, state.fn_name)?;
    // E4d-BA-arr: a folded `str` literal stored into an `array<str>` element is
    // tracked as its `{i64 len,[N×i8]}` GLOBAL POINTER (`@__twig_str_N`), so
    // storing it directly would emit `store i64 @global` — a `ptr` constant in an
    // i64 slot (invalid IR). The literal's address IS a valid handle, so `ptrtoint`
    // it to i64 first — the exact mirror of the call-arg (line ~3209) and `ret`
    // guards. A runtime str element (branch-selected / read from another array_get)
    // already carries an i64, so the guard is scoped to the `@__twig_str` global.
    let val = if elem_ty == "i64" && val.starts_with("@__twig_str") {
        let h = state.fresh("aeh");
        out.push_str(&format!("  {h} = ptrtoint ptr {val} to i64\n"));
        h
    } else {
        val
    };
    emit_bounds_check(&handle, &idx, state, out);
    let ep = state.fresh("aep");
    out.push_str(&format!("  {ep} = getelementptr {elem_ty}, ptr {handle}, i64 {idx}\n"));
    out.push_str(&format!("  store {elem_ty} {val}, ptr {ep}\n"));
    Ok(())
}

/// Lower `array_len dest <- handle` — read the `i64` length header at `handle − 8`.
fn lower_array_len(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "array_len", state.fn_name)?.to_string();
    let handle = resolve_operand(instr.srcs.first(), &state.env, "i64", state.fn_name)?;
    let hdr = state.fresh("ahdr");
    out.push_str(&format!("  {hdr} = getelementptr i8, ptr {handle}, i64 -8\n"));
    out.push_str(&format!("  %{dest} = load i64, ptr {hdr}\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
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
/// The result is signedness-aware for `div` and `mod`/`rem` (split into
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
        "mod" | "rem" => {
            if f { "frem" } else if u { "urem" } else { "srem" }
        }
        _ => unreachable!("llvm_arith_op called with non-arith op {iir_op}"),
    }
}

/// The bit-mask for a narrow **unsigned** integer width, or `None` if the
/// width is the full machine word (`i64`/`u64`) or a signed/float/ref type.
///
/// LANG-FULL **E2 — register width & wrap**, LLVM column. Every IIR value flows
/// through a 64-bit slot in this backend (see the module header — arithmetic
/// operands are `i64` SSA values, never the narrow type), so a `u8`/`u16`/… op
/// must NOT be typed at its narrow LLVM width: `add i8 %a, %b` over two `i64`
/// SSA values is invalid IR that `clang` rejects. Instead we compute the op at
/// `i64` and AND-mask the result back into the declared width — the exact
/// "compute wide, mask the value" shape the VM, JIT, wasm, JVM, and CLR
/// backends already use (and the LLVM byte-tape `store_byte` already does at the
/// memory boundary; this generalises it to register arithmetic):
///
/// ```llvm
///   %nwN = add i64 %a, %b        ; 200 + 100 = 300  (wide)
///   %dst = and i64 %nwN, 255     ; 300 & 0xFF = 44  ✓ wrapped to u8
/// ```
///
/// | type_hint | mask         | example                       |
/// |-----------|--------------|-------------------------------|
/// | `u4`      | `0xF`        | `15u4 + 1u4` → `0`            |
/// | `u8`      | `0xFF`       | `200u8 + 100u8` → `44`        |
/// | `u16`     | `0xFFFF`     | `~0u16` → `65535`            |
/// | `u32`     | `0xFFFFFFFF` | wraps mod-2³²                 |
/// | `u64`,`i*`,`f*` | —      | full word / signed / float: unchanged |
///
/// Signed narrow widths (`i8`/`i16`/`i32`) are intentionally left alone — E2
/// models unsigned wrap; a signed wrap needs `trunc`+`sext`, out of scope here.
fn narrow_unsigned_width_mask(type_hint: &str) -> Option<i64> {
    match type_hint {
        "u4" => Some(0xF),
        "u8" => Some(0xFF),
        "u16" => Some(0xFFFF),
        "u32" => Some(0xFFFF_FFFF),
        _ => None,
    }
}

/// Emit a narrow-unsigned binary op as an `i64` computation followed by an
/// `and i64 %tmp, <mask>` that wraps the result into its declared width, then
/// bind `dest` to the masked value. Shared by [`lower_arith`] and
/// [`lower_bitwise`]; see [`narrow_unsigned_width_mask`] for the rationale.
fn emit_narrow_wrapped(
    llvm_op: &str,
    a: &str,
    b: &str,
    dest: &str,
    mask: i64,
    state: &mut FnState,
    out: &mut String,
) {
    let tmp = state.fresh("nw");
    out.push_str(&format!("  {tmp} = {llvm_op} i64 {a}, {b}\n"));
    out.push_str(&format!("  %{dest} = and i64 {tmp}, {mask}\n"));
    state.env.insert(dest.to_string(), format!("%{dest}"));
}

fn fold_i64_arith(iir_op: &str, a: &str, b: &str) -> Option<i64> {
    let a = a.parse::<i64>().ok()?;
    let b = b.parse::<i64>().ok()?;
    match iir_op {
        "add" => a.checked_add(b),
        "sub" => a.checked_sub(b),
        "mul" => a.checked_mul(b),
        "div" if b != 0 => a.checked_div(b),
        "mod" | "rem" if b != 0 => a.checked_rem(b),
        _ => None,
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
    if instr.type_hint == "i64" {
        if let Some(value) = fold_i64_arith(iir_op, &a, &b) {
            state.env.insert(dest, value.to_string());
            return Ok(());
        }
    }
    // E2: a narrow unsigned op (u4/u8/u16/u32) flows through i64 slots, so
    // compute at i64 then mask the result into its width (200u8+100u8=44).
    if let Some(mask) = narrow_unsigned_width_mask(&instr.type_hint) {
        emit_narrow_wrapped(llvm_op, &a, &b, &dest, mask, state, out);
        return Ok(());
    }
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

    // If the operand type maps to i1 (`i1` or `bool`), use the i1 form
    // directly. Otherwise zext to a wider integer so the boolean result can be
    // consumed as a value. A comparison ALWAYS yields a boolean — never a
    // float — so a *float* comparison (`operand_ty == "double"`) must still
    // zext to an integer (`i64`), not to `double` (`zext i1 to double` is
    // invalid IR clang rejects). Integer comparisons keep zext'ing to their
    // own operand width.
    if operand_ty == "i1" {
        state.env.insert(dest.clone(), i1_name.clone());
    } else {
        let result_ty = if is_float_type(&instr.type_hint) { "i64" } else { operand_ty };
        out.push_str(&format!(
            "  %{dest} = zext i1 {i1_name} to {result_ty}\n"
        ));
        state.env.insert(dest.clone(), format!("%{dest}"));
    }
    Ok(())
}

fn lower_bitwise(
    iir_op: &str,
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    if is_float_type(&instr.type_hint) {
        return Err(IIRLlvmError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("{iir_op} does not support floating-point operands"),
        });
    }
    let dest = require_dest(instr, iir_op, state.fn_name)?.to_string();
    let ty = llvm_type_for(&instr.type_hint, state.fn_name)?;

    // E2: a narrow unsigned bitwise op (u4/u8/u16/u32) flows through i64 slots —
    // compute at i64 and mask the result into its width (matches lower_arith and
    // the register backends). `and`/`or`/`xor` of in-range operands is unchanged
    // by the mask; the mask matters once `not`/`shl` widen the result.
    // Note: narrow_unsigned_width_mask("bool") is None, so bool ops skip this path.
    if let Some(mask) = narrow_unsigned_width_mask(&instr.type_hint) {
        let a = resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, state.fn_name)?;
        let b = resolve_operand(instr.srcs.get(1), &state.env, &instr.type_hint, state.fn_name)?;
        emit_narrow_wrapped(iir_op, &a, &b, &dest, mask, state, out);
        return Ok(());
    }

    // When the target type is i1 (`bool` or `i1` type_hint), comparison results
    // are stored in `env` as zext'd i64 values — using them directly in `and i1`
    // would be a type error.  Prefer the i1 forms from `env_i1` (set by
    // `lower_cmp`), with a trunc fallback for other i64 sources.  Mirrors the
    // operand-lifting logic in `lower_jmp_if`.
    let (a, b) = if ty == "i1" {
        let lift_a = match instr.srcs.first() {
            Some(Operand::Var(name)) => {
                if let Some(i1) = state.env_i1.get(name).cloned() {
                    i1
                } else {
                    let wide = state.env.get(name).cloned().ok_or_else(|| {
                        IIRLlvmError::UndefinedVariable {
                            function: state.fn_name.into(),
                            name: name.clone(),
                        }
                    })?;
                    let t = state.fresh("tobool");
                    out.push_str(&format!("  {t} = trunc i64 {wide} to i1\n"));
                    t
                }
            }
            other => render_literal(other, "bool", state.fn_name)?,
        };
        let lift_b = match instr.srcs.get(1) {
            Some(Operand::Var(name)) => {
                if let Some(i1) = state.env_i1.get(name).cloned() {
                    i1
                } else {
                    let wide = state.env.get(name).cloned().ok_or_else(|| {
                        IIRLlvmError::UndefinedVariable {
                            function: state.fn_name.into(),
                            name: name.clone(),
                        }
                    })?;
                    let t = state.fresh("tobool");
                    out.push_str(&format!("  {t} = trunc i64 {wide} to i1\n"));
                    t
                }
            }
            other => render_literal(other, "bool", state.fn_name)?,
        };
        (lift_a, lift_b)
    } else {
        (
            resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, state.fn_name)?,
            resolve_operand(instr.srcs.get(1), &state.env, &instr.type_hint, state.fn_name)?,
        )
    };

    out.push_str(&format!("  %{dest} = {iir_op} {ty} {a}, {b}\n"));
    let value = format!("%{dest}");
    state.env.insert(dest.clone(), value.clone());
    if ty == "i1" {
        state.env_i1.insert(dest, value);
    }
    Ok(())
}

/// Lower a bitwise NOT (`not dest, src`).
///
/// LLVM has no `not` instruction — bitwise complement is `xor x, -1` (every bit
/// flipped). For a narrow unsigned width (`u4`/`u8`/`u16`/`u32`) we reuse the E2
/// "compute wide, mask the value" path: `xor i64 src, -1` then `and i64 …, <mask>`,
/// so `~0u8` is `255` (`-1 & 0xFF`), not the i64 all-ones. Unlocks Nib N3-`~` and
/// Oct O2-`~` (whose `compile_unary` lowers `~` to this op).
fn lower_not(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "not", state.fn_name)?.to_string();
    let a = resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, state.fn_name)?;
    if let Some(mask) = narrow_unsigned_width_mask(&instr.type_hint) {
        // `xor i64 a, -1` then mask to width — reuses the E2 binary helper with
        // `-1` as the second operand.
        emit_narrow_wrapped("xor", &a, "-1", &dest, mask, state, out);
        return Ok(());
    }
    let ty = llvm_type_for(&instr.type_hint, state.fn_name)?;
    out.push_str(&format!("  %{dest} = xor {ty} {a}, -1\n"));
    state.env.insert(dest.clone(), format!("%{dest}"));
    Ok(())
}

/// Lower `neg dest <- src` — unary arithmetic negation.
///
/// LLVM IR uses `fneg` for floating-point and `sub 0, x` for integers.
/// The `sub 0, x` form is idiomatic and matches every existing backend's
/// negation pattern (WASM `f64.neg`, native `fneg`/`neg r`).
fn lower_neg(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut String,
) -> Result<(), IIRLlvmError> {
    let dest = require_dest(instr, "neg", state.fn_name)?.to_string();
    let ty = llvm_type_for(&instr.type_hint, state.fn_name)?;
    let a = resolve_operand(instr.srcs.first(), &state.env, &instr.type_hint, state.fn_name)?;
    if is_float_type(&instr.type_hint) {
        out.push_str(&format!("  %{dest} = fneg {ty} {a}\n"));
    } else {
        out.push_str(&format!("  %{dest} = sub {ty} 0, {a}\n"));
    }
    state.env.insert(dest.clone(), format!("%{dest}"));
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
        // Truncate to i1 unless the condition is already a boolean/i1 value.
        // Need to know the cond's current type for the trunc — use the
        // instr's type_hint as the operand type.
        let cond_ty = llvm_type_for(&instr.type_hint, state.fn_name)?;
        if cond_ty == "i1" {
            cond_op
        } else if cond_ty == "void" {
            // McCarthy W12b-3: a `COND` whose clause test is a tagged-word lisp
            // predicate carries NO operand type on `jmp_if_*` (type_hint "void");
            // the condition is the `i64` 0/1 from `lispy_truthy`. Compare it
            // against zero to get the `i1` — `trunc void …` would be invalid.
            let i1 = state.fresh("tobool");
            out.push_str(&format!("  {i1} = icmp ne i64 {cond_op}, 0\n"));
            i1
        } else {
            let i1 = state.fresh("trunc");
            out.push_str(&format!("  {i1} = trunc {cond_ty} {cond_op} to i1\n"));
            i1
        }
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
    // The conditional branch terminated the prior block; we are now in the
    // freshly-opened fallthrough block (needs its own terminator later).
    state.block_open = true;
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
            // Render an `f64` as LLVM's **hexadecimal** double literal — `0x`
            // followed by the 16 hex digits of the IEEE-754 bit pattern.
            //
            // This is exact (no rounding — a `real` constant round-trips
            // bit-for-bit) *and* always parses. The previous `{:e}` form was
            // neither: Rust's scientific notation emits `2e0` / `0e0` for round
            // numbers (no decimal point), and LLVM's assembler rejects a
            // floating literal without a `.` — `store double 0e0` fails with
            // "integer constant must have integer type". The hex form sidesteps
            // decimal-point and precision pitfalls entirely. (Every float the
            // frontends emit is an `f64`/`double`; `llvm_type_for` maps the
            // matching `f64` type_hint to `double`, so the literal's type
            // agrees with its use site.)
            Ok(format!("0x{:016X}", v.to_bits()))
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
        let mut op = match src {
            Operand::Var(name) => state.env.get(name).cloned().ok_or_else(|| {
                IIRLlvmError::UndefinedVariable {
                    function: state.fn_name.into(),
                    name: name.clone(),
                }
            })?,
            other => render_literal(Some(other), pty, state.fn_name)?,
        };
        // E4-dyn: a `str` argument is passed as an i64 handle. A single-assignment
        // string literal is tracked as its `{i64 len,[N×i8]}` GLOBAL POINTER
        // (`@__twig_str_N`), so passing it directly would emit `call ...(i64
        // @global)` — a type error (a `ptr` constant in an `i64` slot). The literal's
        // address IS a valid handle, so `ptrtoint` it to i64 first — the exact mirror
        // of the `ret` path. The callee reads the length header via `inttoptr`+`load`.
        // (Branch-selected / call-result / param strings already carry an i64.)
        // Guard on the param slot being `i64`: a `@__twig_str` global only ever fills
        // an i64 (str-handle) slot today, but requiring it keeps the `ptrtoint … to
        // i64` correct if some future param type ever lowers to `ptr` instead.
        if *pty == "i64" && op.starts_with("@__twig_str") {
            let h = state.fresh("argh");
            out.push_str(&format!("  {h} = ptrtoint ptr {op} to i64\n"));
            op = h;
        }
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
    // McCarthy W12b: a tagged-word lisp builtin lowers to a `call` into the C
    // runtime. Every arg + the result is an `i64` (tagged word); the dest gets a
    // fresh SSA name registered in the env so later instructions can use it.
    if let Some((_iir_name, symbol, arity)) = lispy_builtin(&name) {
        let dest = require_dest(instr, &name, state.fn_name)?.to_string();
        let mut args = Vec::with_capacity(*arity);
        for k in 0..*arity {
            // srcs[0] is the builtin name; the operands start at srcs[1].
            let a = resolve_operand(instr.srcs.get(1 + k), &state.env, "i64", state.fn_name)?;
            args.push(format!("i64 {a}"));
        }
        out.push_str(&format!("  %{dest} = call i64 @{symbol}({})\n", args.join(", ")));
        state.env.insert(dest.clone(), format!("%{dest}"));
        return Ok(());
    }
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
        // ── putchar(v) — Brainfuck `.` (LLVM05) ─────────────────────────
        //
        // BF's value register is i64; libc `putchar` takes an `int` (i32), so
        // truncate to the low 32 bits (the cell already lives in the low 8).
        // The returned int is discarded.
        //
        //   srcs = [Var("putchar"), Var(v: i64)]  →  call i32 @putchar(i32 %t)
        "putchar" => {
            let val = resolve_operand(instr.srcs.get(1), &state.env, "i64", state.fn_name)?;
            let t = state.fresh("pc");
            out.push_str(&format!("  {t} = trunc i64 {val} to i32\n"));
            out.push_str(&format!("  call i32 @putchar(i32 {t})\n"));
            Ok(())
        }
        // ── getchar() -> v — Brainfuck `,` (LLVM05) ─────────────────────
        //
        // libc `getchar` returns an `int` (the byte, or -1 at EOF). We
        // sign-extend to the i64 cell register; a subsequent `store_byte`
        // truncates to the 8-bit cell, so EOF (-1) lands as 0xFF — the
        // conventional Brainfuck "leave 255 on EOF" behaviour.
        //
        //   srcs = [Var("getchar")], dest = v  →  %g = call i32 @getchar()
        "getchar" => {
            let dest = require_dest(instr, "getchar", state.fn_name)?.to_string();
            let g = state.fresh("gc");
            out.push_str(&format!("  {g} = call i32 @getchar()\n"));
            out.push_str(&format!("  %{dest} = sext i32 {g} to i64\n"));
            state.env.insert(dest.clone(), format!("%{dest}"));
            Ok(())
        }
        // ── input_i64() -> v — BASIC `INPUT X` (BA-INPUT) ───────────────
        //
        // `@__twig_input_i64()` (in `twig_runtime.c`) reads one line from
        // stdin and parses it as `int64_t`; returns 0 on EOF / parse
        // failure (V1 permissive contract). The result goes straight into
        // the dest register — no conversion needed since the return type
        // is already i64.
        //
        //   srcs = [Var("input_i64")], dest = v  →  %v = call i64 @__twig_input_i64()
        "input_i64" => {
            let dest = require_dest(instr, "input_i64", state.fn_name)?.to_string();
            out.push_str(&format!("  %{dest} = call i64 @__twig_input_i64()\n"));
            state.env.insert(dest.clone(), format!("%{dest}"));
            Ok(())
        }
        // ── input_str() -> v — BASIC string `INPUT A$` (E4-dyn) ─────────
        //
        // `@__twig_input_str()` (in `twig_runtime.c`) reads one line and returns
        // an i64 **handle** — the base address of a `[i64 len][bytes]` heap
        // block, the same runtime-string repr `print_str` reads the length from
        // at run time. The handle goes straight into the dest register (no
        // conversion — a str value is carried as its i64 handle on this backend,
        // exactly like the E4-dyn branch-selected / call-result strings). A
        // later `mov`/`print_str` consuming this dest has no compile-time length
        // metadata, so it takes the runtime header-read path (E4d-2b).
        //
        //   srcs = [Var("input_str")], dest = v  →  %v = call i64 @__twig_input_str()
        "input_str" => {
            let dest = require_dest(instr, "input_str", state.fn_name)?.to_string();
            out.push_str(&format!("  %{dest} = call i64 @__twig_input_str()\n"));
            state.env.insert(dest.clone(), format!("%{dest}"));
            Ok(())
        }
        _ => unreachable!("SUPPORTED_BUILTINS guard above prevents this"),
    }
}

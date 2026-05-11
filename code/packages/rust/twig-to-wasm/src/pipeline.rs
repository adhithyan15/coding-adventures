//! # pipeline — the Twig → WASM compilation chain
//!
//! This module wires together five stages into one function call:
//!
//! ```text
//! Source text
//!   │
//!   ▼ twig_ir_compiler::compile_source
//! IIRModule   (type_hint = "any" on every instruction — Twig is dynamically typed)
//!   │
//!   ▼ pre_lower_builtins   [pipeline-local, unconditional]
//! IIRModule   (call_builtin "+" → add, "=" → cmp_eq, etc., even with "any" type)
//!   │
//!   ▼ iir_type_checker::infer_and_check
//! IIRModule   (add/sub/cmp_eq now have concrete types: "i64", "bool", …)
//!   │
//!   ▼ fixup_control_flow_types   [pipeline-local]
//! IIRModule   (ret/call/jmp_if* "any" hints repaired: "void" or propagated type)
//!   │
//!   ▼ iir_to_wasm::lower_iir_to_wasm
//! WasmModule
//!   │
//!   ▼ iir_to_wasm::encode_module
//! Vec<u8>     ← WASM binary, starts with b"\x00asm"
//! ```
//!
//! ## Why pre-lower before type inference?
//!
//! See the `twig-to-beam` pipeline module for the full explanation.  Short
//! version: the type-checker's inference rules fire on `add`/`sub`/`cmp_eq`
//! (which it knows are arithmetic), NOT on `call_builtin "+"`.  So we must
//! convert `call_builtin "+"` → `add` before running inference.
//!
//! ## The control-flow type fixup
//!
//! After inference, `add`/`sub`/`cmp_eq` have concrete types.  But `ret`,
//! `call`, `jmp_if_*`, and `label` are control-flow instructions that the
//! inference rules do not cover.  The WASM validator rejects `"any"` type
//! hints on ANY instruction.  The fixup pass:
//!
//! - `ret_void`, `label`, `jmp`, `jmp_if_true`, `jmp_if_false` → `"void"`.
//! - `ret <src>` → type of `src` from the SSA env (or `"void"`).
//! - `call <fn> ...` → type of the dest register from the SSA env (or `"void"`).
//!
//! Setting control-flow type hints to `"void"` is semantically correct in the
//! WASM model: these instructions consume operands but produce nothing on the
//! WASM value stack at the instruction level (the return value is handled by
//! the `local.get + return` pattern in the lowering pass).

use std::collections::HashMap;

use iir_to_wasm::{encode_module, lower_iir_to_wasm, validate_for_wasm, IIRWasmConfig};
use iir_type_checker::infer_and_check;
use interpreter_ir::{IIRFunction, IIRModule, IIRInstr, Operand};
use twig_ir_compiler::compile_source;

use crate::error::TwigToWasmError;

// ---------------------------------------------------------------------------
// Builtin lowering table (same as twig-to-beam)
// ---------------------------------------------------------------------------
//
// Twig builtin name → IIR op name.
// Only numeric and comparison builtins are lowered here.
//
// Comparison ops use the canonical IIR names:
//   `cmp_eq`, `cmp_lt`, `cmp_gt`, `cmp_le`, `cmp_ge`
//
// Note: the WASM backend's validate.rs only checks for "call_builtin" in its
// UNSUPPORTED_OPS list (not for `cmp_eq`, `cmp_lt`, etc.).  The lowering
// pass handles `cmp_*` ops by emitting the appropriate comparison opcodes
// (i32.eq, i32.lt_s, etc.) based on the type hint.

const BUILTIN_MAP: &[(&str, &str)] = &[
    ("+",  "add"),
    ("-",  "sub"),
    ("*",  "mul"),
    ("/",  "div"),
    // Comparison builtins.  The WASM backend lower.rs handles these ops as
    // `"eq"`, `"lt"`, `"gt"`, `"le"`, `"ge"` (no `cmp_` prefix) — they
    // match the WASM comparison opcode naming convention (i64.eq, i64.lt_s,
    // etc.).  Unlike the BEAM backend which uses `cmp_eq`/`cmp_lt`/etc., the
    // WASM backend matches the raw two-letter form.
    ("=",  "eq"),
    ("<",  "lt"),
    (">",  "gt"),
    ("<=", "le"),
    (">=", "ge"),
    ("not", "lnot"),
    // `_move` is emitted by the Twig compiler for `if` expression arm unification.
    // It copies one register to another — maps to the IIR `mov` op.
    ("_move", "mov"),
];

// ---------------------------------------------------------------------------
// compile_twig_to_wasm — the public pipeline function
// ---------------------------------------------------------------------------

/// Compile a Twig source string to WebAssembly 1.0 bytecode in one step.
///
/// # Arguments
///
/// - `source` — the complete Twig program text.
/// - `module_name` — the WASM module identifier.
///
/// # Returns
///
/// On success, a `Vec<u8>` containing the complete WASM binary.  The bytes
/// start with `b"\x00asm"` followed by version `[0x01, 0x00, 0x00, 0x00]`.
///
/// # Errors
///
/// - [`TwigToWasmError::CompileError`] — Twig syntax error or unbound name.
/// - [`TwigToWasmError::WasmError`] — IIR → WASM validation or lowering failed.
/// - [`TwigToWasmError::EncodeError`] — WASM binary encoding failed.
///
/// # Example
///
/// ```rust
/// use twig_to_wasm::compile_twig_to_wasm;
///
/// let bytes = compile_twig_to_wasm(
///     "(define (add a b) (+ a b)) (add 1 2)",
///     "arith",
/// ).unwrap();
/// assert!(bytes.starts_with(b"\x00asm"));
/// ```
pub fn compile_twig_to_wasm(
    source: &str,
    module_name: &str,
) -> Result<Vec<u8>, TwigToWasmError> {
    // ── Stage 1: Twig source → IIR ───────────────────────────────────────────
    let mut iir = compile_source(source, module_name)?;

    // ── Stage 2: Unconditional builtin pre-lowering ──────────────────────────
    //
    // Convert arithmetic `call_builtin` to typed IIR ops BEFORE inference.
    // This lets the type-checker see `add`, `sub`, `cmp_eq` (which it has
    // inference rules for) instead of `call_builtin "+"` (which it does not).
    pre_lower_builtins(&mut iir);

    // ── Stage 3: Type inference ──────────────────────────────────────────────
    //
    // After pre-lowering, `add v1 v2 : any` (where v1, v2 are i64) becomes
    // `add v1 v2 : i64`.  `cmp_eq v1 v2 : any` → `cmp_eq v1 v2 : bool`.
    let _report = infer_and_check(&mut iir);

    // ── Stage 4: Control-flow type fixup ─────────────────────────────────────
    //
    // Repair `"any"` type hints on `ret`, `call`, `jmp*`, `label`.
    // The WASM validator rejects "any" on any instruction.
    fixup_control_flow_types(&mut iir);

    // ── Stage 5a: Validate ───────────────────────────────────────────────────
    let validation_errors = validate_for_wasm(&iir);
    if !validation_errors.is_empty() {
        return Err(TwigToWasmError::WasmError(
            iir_to_wasm::IIRWasmError::ValidationFailed(validation_errors),
        ));
    }

    // ── Stage 5b: IIR → WasmModule ──────────────────────────────────────────
    let config = IIRWasmConfig::new(module_name);
    let wasm_module = lower_iir_to_wasm(&iir, &config)?;

    // ── Stage 5c: WasmModule → Vec<u8> ──────────────────────────────────────
    let bytes = encode_module(&wasm_module)
        .map_err(|e| TwigToWasmError::EncodeError(e.to_string()))?;

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// pre_lower_builtins — unconditional, type-agnostic builtin lowering
// ---------------------------------------------------------------------------

/// Lower arithmetic/comparison `call_builtin` instructions to typed IIR ops,
/// regardless of whether the current type hint is `"any"` or concrete.
///
/// This is a pipeline-local function distinct from `iir_builtin_lowering::lower_builtins`:
/// the crate version rejects `"any"` type hints (to catch ordering bugs); this
/// version is intentionally permissive because it runs before inference.
pub(crate) fn pre_lower_builtins(module: &mut IIRModule) {
    for function in &mut module.functions {
        pre_lower_function(function);
    }
}

fn pre_lower_function(func: &mut IIRFunction) {
    let old_instrs = std::mem::take(&mut func.instructions);
    let mut new_instrs = Vec::with_capacity(old_instrs.len());

    for instr in old_instrs {
        if instr.op != "call_builtin" {
            new_instrs.push(instr);
            continue;
        }

        let builtin_name = match instr.srcs.first() {
            Some(Operand::Var(name)) => name.clone(),
            _ => {
                new_instrs.push(instr);
                continue;
            }
        };

        let iir_op = BUILTIN_MAP.iter().find(|(b, _)| *b == builtin_name.as_str());

        match iir_op {
            None => {
                new_instrs.push(instr);
            }
            Some((_, op)) => {
                let args: Vec<Operand> = instr.srcs[1..].to_vec();
                let new_instr = IIRInstr::new(*op, instr.dest.clone(), args, &instr.type_hint);
                new_instrs.push(new_instr);
            }
        }
    }

    func.instructions = new_instrs;
}

// ---------------------------------------------------------------------------
// fixup_control_flow_types — pipeline-local pass
// ---------------------------------------------------------------------------

/// Fix up `"any"` type hints on control-flow instructions.
///
/// The WASM validator requires every instruction to have a concrete type hint
/// (not `"any"`).  Inference handles arithmetic; this pass handles control-flow.
pub(crate) fn fixup_control_flow_types(module: &mut IIRModule) {
    for function in &mut module.functions {
        fixup_function(function);
    }
}

fn fixup_function(func: &mut IIRFunction) {
    // Pass 1: SSA env from all concretely-typed instructions.
    //
    // Seed function parameters as "i64" (Twig's integer runtime type).
    // This lets arithmetic ops that take param variables propagate a
    // concrete type, even though the Twig compiler emits "any" for params.
    let mut env: HashMap<String, String> = HashMap::new();

    for (param_name, _) in &func.params {
        env.insert(param_name.clone(), "i64".to_string());
    }

    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            let ty = &instr.type_hint;
            if ty != "any" && ty != "polymorphic" {
                env.insert(dest.clone(), ty.clone());
            }
        }
    }

    // Pass 2: fix up "any" on control-flow, arithmetic, and mov.
    for instr in &mut func.instructions {
        if instr.type_hint != "any" {
            continue;
        }
        let fixed: String = match instr.op.as_str() {
            // ── Pure control-flow: no stack value produced ──────────────────
            "ret_void" | "label" | "jmp" | "jmp_if_true" | "jmp_if_false" => {
                "void".to_string()
            }

            // ── ret <src>: type = type of src ────────────────────────────────
            "ret" => {
                if let Some(Operand::Var(src)) = instr.srcs.first() {
                    env.get(src).cloned().unwrap_or_else(|| "void".to_string())
                } else if let Some(Operand::Int(_)) = instr.srcs.first() {
                    "i64".to_string()
                } else {
                    "void".to_string()
                }
            }

            // ── call: type = dest type or "i64" default ──────────────────────
            "call" => {
                if let Some(dest) = &instr.dest {
                    env.get(dest).cloned().unwrap_or_else(|| "i64".to_string())
                } else {
                    "void".to_string()
                }
            }

            // ── mov: passthrough ─────────────────────────────────────────────
            "mov" => {
                if let Some(Operand::Var(src)) = instr.srcs.first() {
                    env.get(src).cloned().unwrap_or_else(|| "i64".to_string())
                } else {
                    "i64".to_string()
                }
            }

            // ── Arithmetic: default to "i64" ──────────────────────────────────
            //
            // Function parameters are "any" in Twig, so `add a, b` may have
            // "any" type even after inference.  Default to "i64" — WASM will
            // emit `i64.add`, which is correct for Twig integer arithmetic.
            "add" | "sub" | "mul" | "div" | "mod" | "neg" | "not" => {
                let from_ops: Option<String> = instr.srcs.iter().find_map(|src| {
                    if let Operand::Var(name) = src {
                        env.get(name).cloned()
                    } else {
                        None
                    }
                });
                from_ops.unwrap_or_else(|| "i64".to_string())
            }

            // ── Comparison: always bool ───────────────────────────────────────
            //
            // The WASM backend uses the short names `"eq"`, `"ne"`, `"lt"`,
            // `"le"`, `"gt"`, `"ge"` (matching WASM opcode naming).  These
            // produce an i32 result (0 or 1) which the WASM type system sees
            // as `bool` in the IIR type hint.
            "eq" | "ne" | "lt" | "le" | "gt" | "ge" => {
                "bool".to_string()
            }

            // ── lnot: bool ───────────────────────────────────────────────────
            "lnot" => "bool".to_string(),

            // ── All else: leave for the validator to report ───────────────────
            _ => "any".to_string(),
        };
        if fixed != "any" {
            instr.type_hint = fixed.clone();
            if let Some(dest) = &instr.dest {
                env.insert(dest.clone(), fixed);
            }
        }
    }
}

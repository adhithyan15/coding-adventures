//! # pipeline — the Twig → BEAM compilation chain
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
//! IIRModule   (add/sub/… now get concrete types: "i64", "bool", …)
//!   │
//!   ▼ fixup_control_flow_types   [pipeline-local]
//! IIRModule   (ret/call/jmp_if* "any" hints repaired)
//!   │
//!   ▼ iir_to_beam::lower_iir_to_beam
//! BEAMModule
//!   │
//!   ▼ iir_to_beam::encode_beam
//! Vec<u8>     ← BEAM binary, starts with b"FOR1"
//! ```
//!
//! ## Why pre-lower before type inference?
//!
//! The Twig compiler emits `call_builtin "+"` for addition — not `add`.  The
//! type-checker's inference rules (R5: `add/sub/…` with concrete-typed operands
//! → same type) only fire on `add`, `sub`, etc.  They do NOT fire on
//! `call_builtin "+"` because that is a function-call opcode, not an arithmetic
//! opcode.
//!
//! Consequently, if we run inference before lowering, `const 1 : i64` and
//! `const 2 : i64` are correctly inferred, but `call_builtin "+" v1 v2 : any`
//! stays `"any"` because no rule propagates from constants through `call_builtin`.
//!
//! The fix: lower first (convert `call_builtin "+"` → `add`), then infer.
//! After lowering, `add v1 v2 : any` becomes `add v1 v2 : i64` via rule R5.
//!
//! ## The strict `iir-builtin-lowering` crate
//!
//! The `iir-builtin-lowering` crate's `lower_builtins` is intentionally strict:
//! it rejects `call_builtin` with `"any"` type hints to catch pipeline-ordering
//! bugs.  We bypass this strictness by using a local unconditional pre-lower pass
//! instead.  The strict version is still available and is used in tools that
//! guarantee the type-checker ran first.
//!
//! ## BEAM binary format
//!
//! The resulting bytes follow the BEAM IFF format:
//!
//! ```text
//! FOR1 <size> BEAM
//!   AtU8 <atoms>
//!   Code <instructions>
//!   ExpT <exports>
//!   ...
//! ```

use std::collections::HashMap;

use iir_to_beam::{encode_beam, lower_iir_to_beam, validate_for_beam, IIRBeamConfig};
use iir_type_checker::infer_and_check;
use interpreter_ir::{IIRFunction, IIRModule, IIRInstr, Operand};
use twig_ir_compiler::compile_source;

use crate::error::TwigToBeamError;

// ---------------------------------------------------------------------------
// compile_twig_to_beam — the public pipeline function
// ---------------------------------------------------------------------------

/// Compile a Twig source string to BEAM bytecode in one step.
///
/// # Arguments
///
/// - `source` — the complete Twig program text.
/// - `module_name` — the BEAM module atom.  Must be a valid Erlang atom
///   identifier (lower-case start, letters/digits/underscores).
///
/// # Returns
///
/// On success, a `Vec<u8>` containing the complete BEAM binary.  The bytes
/// start with `b"FOR1"` (the BEAM IFF magic number).
///
/// # Errors
///
/// - [`TwigToBeamError::CompileError`] — Twig source has a syntax error or
///   references an unbound name.
/// - [`TwigToBeamError::BeamError`] — the IIR module could not be lowered to
///   BEAM (unsupported operation, unresolved type, etc.).
///
/// # Example
///
/// ```rust
/// use twig_to_beam::compile_twig_to_beam;
///
/// let bytes = compile_twig_to_beam("(+ 1 2)", "arith").unwrap();
/// assert!(bytes.starts_with(b"FOR1"), "should produce a BEAM binary");
/// ```
pub fn compile_twig_to_beam(
    source: &str,
    module_name: &str,
) -> Result<Vec<u8>, TwigToBeamError> {
    // ── Stage 1: Twig source → IIR ───────────────────────────────────────────
    //
    // The compiler returns an IIRModule with `type_hint = "any"` on every
    // instruction.  Entry point is always `"main"`.
    let mut iir = compile_source(source, module_name)?;

    // ── Stage 2: Unconditional builtin pre-lowering ──────────────────────────
    //
    // Convert `call_builtin "+"` → `add`, `"="` → `cmp_eq`, etc., regardless
    // of the current type hints.  We do this BEFORE inference so that inference
    // can see `add`/`sub`/`cmp_eq` (which it has rules for) rather than
    // `call_builtin` (which it does not).
    //
    // This is a local unconditional pass; see module docs for rationale.
    pre_lower_builtins(&mut iir);

    // ── Stage 3: Type inference ──────────────────────────────────────────────
    //
    // Inference now sees `add v1 v2 : any` where v1, v2 have type `"i64"`.
    // Rule R5 fires: `add` with i64-typed operands → type_hint becomes `"i64"`.
    // Rule R4 fires for `cmp_eq`, `cmp_lt`, … → type_hint becomes `"bool"`.
    //
    // After this pass, all arithmetic and comparison instructions should be
    // concretely typed.  Control-flow instructions (`ret`, `call`, `jmp*`,
    // `label`) still have `"any"`.
    let _report = infer_and_check(&mut iir);

    // ── Stage 4: Control-flow type fixup ─────────────────────────────────────
    //
    // Repair `"any"` type hints on control-flow instructions:
    // - `ret_void`, `label`, `jmp`, `jmp_if_true`, `jmp_if_false` → `"void"`
    // - `ret <src>` → type of `src` (from the SSA env), or `"void"`
    // - `call <fn> ...` → type of the dest register, or `"void"`
    fixup_control_flow_types(&mut iir);

    // ── Stage 5a: Validate ───────────────────────────────────────────────────
    let validation_errors = validate_for_beam(&iir);
    if !validation_errors.is_empty() {
        return Err(TwigToBeamError::BeamError(
            iir_to_beam::IIRBeamError::ValidationFailed(validation_errors),
        ));
    }

    // ── Stage 5b: IIR → BEAMModule ──────────────────────────────────────────
    let config = IIRBeamConfig::new(module_name);
    let beam_module = lower_iir_to_beam(&iir, &config)?;

    // ── Stage 5c: BEAMModule → Vec<u8> ──────────────────────────────────────
    let bytes = encode_beam(&beam_module);

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// pre_lower_builtins — unconditional builtin lowering
// ---------------------------------------------------------------------------
//
// # Builtin lowering table
//
// Maps Twig builtin names (as they appear in `call_builtin` instruction
// source operands) to their IIR opcode equivalents.
//
// The table is intentionally small — only numeric and comparison operations
// that map 1-to-1 to IIR typed ops.  Everything else (make_nil, cons,
// make_closure, global_get, etc.) is left as `call_builtin`.
//
// The comparison ops use the BEAM/WASM canonical names: `cmp_eq`, `cmp_lt`,
// etc. (NOT `eq` / `lt` — those are different ops).
//
// | Twig builtin | Arity | IIR op emitted |
// |-------------|-------|----------------|
// | `+`         | 2     | `add`          |
// | `-`         | 2     | `sub`          |
// | `*`         | 2     | `mul`          |
// | `/`         | 2     | `div`          |
// | `=`         | 2     | `cmp_eq`       |
// | `<`         | 2     | `cmp_lt`       |
// | `>`         | 2     | `cmp_gt`       |
// | `<=`        | 2     | `cmp_le`       |
// | `>=`        | 2     | `cmp_ge`       |
// | `not`       | 1     | `lnot`         |

/// Builtin name → IIR op name.
const BUILTIN_MAP: &[(&str, &str)] = &[
    ("+",  "add"),
    ("-",  "sub"),
    ("*",  "mul"),
    ("/",  "div"),
    ("=",  "cmp_eq"),
    ("<",  "cmp_lt"),
    (">",  "cmp_gt"),
    ("<=", "cmp_le"),
    (">=", "cmp_ge"),
    ("not", "lnot"),
    // `_move` is emitted by the Twig compiler for `if` expression arm unification.
    // It copies one register to another — maps to `load_reg` in the BEAM backend
    // (which emits `move {x,src} {x,dest}`).
    ("_move", "load_reg"),
];

/// Lower arithmetic/comparison `call_builtin` instructions to typed IIR ops.
///
/// Unlike the strict `iir-builtin-lowering` crate, this pass is unconditional:
/// it does NOT check whether the type hint is concrete before lowering.  This
/// allows inference to run *after* lowering and fill in the types on the
/// already-lowered `add`/`sub`/`cmp_eq` instructions.
pub(crate) fn pre_lower_builtins(module: &mut IIRModule) {
    for function in &mut module.functions {
        pre_lower_function(function);
    }
}

/// Apply the pre-lower pass to a single function.
fn pre_lower_function(func: &mut IIRFunction) {
    let old_instrs = std::mem::take(&mut func.instructions);
    let mut new_instrs = Vec::with_capacity(old_instrs.len());

    for instr in old_instrs {
        if instr.op != "call_builtin" {
            new_instrs.push(instr);
            continue;
        }

        // Extract the builtin name from the first source operand.
        let builtin_name = match instr.srcs.first() {
            Some(Operand::Var(name)) => name.clone(),
            _ => {
                new_instrs.push(instr);
                continue;
            }
        };

        // Look up the builtin in the map.
        let iir_op = BUILTIN_MAP.iter().find(|(b, _)| *b == builtin_name.as_str());

        match iir_op {
            None => {
                // Not a numeric builtin — leave as call_builtin.
                new_instrs.push(instr);
            }
            Some((_, op)) => {
                // Argument operands are instr.srcs[1..] (skip the builtin name).
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
/// After inference, `add`/`sub`/`cmp_eq` have concrete types, but `ret`,
/// `call`, `jmp`, `label` still have `"any"`.  This pass:
///
/// 1. Builds an SSA env: `{dest_var → type}` from all concretely-typed instrs.
/// 2. For each remaining `"any"` instruction:
///    - Pure control-flow (no result): `"void"`.
///    - `ret <src>`: look up `src` in env, use its type or `"void"`.
///    - `call <fn> ...`: use dest's type from env, or `"void"`.
pub(crate) fn fixup_control_flow_types(module: &mut IIRModule) {
    for function in &mut module.functions {
        fixup_function(function);
    }
}

fn fixup_function(func: &mut IIRFunction) {
    // Pass 1: build SSA env from all instructions with concrete types.
    //
    // Also seed function parameters: Twig parameters are "any" in the IR,
    // but for the purposes of BEAM compilation we treat them as "i64" (the
    // default Twig integer type).  This lets arithmetic ops that use
    // parameters propagate a concrete type.
    //
    // This is correct semantically: the BEAM backend uses `gc_bif2 erlang:+/2`
    // for all arithmetic, which is dynamically dispatched at the C level.
    // The `type_hint` on `add` instructions is used only to route to that
    // gc_bif2 call — and any BEAM-native integer type routes to the same path.
    let mut env: HashMap<String, String> = HashMap::new();

    // Seed params as i64 (Twig's primary integer runtime type).
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

    // Pass 2: fix up "any" on control-flow and arithmetic.
    for instr in &mut func.instructions {
        if instr.type_hint != "any" {
            continue;
        }
        let fixed: String = match instr.op.as_str() {
            // ── Pure control-flow: no stack value produced ──────────────────
            "ret_void" | "label" | "jmp" | "jmp_if_true" | "jmp_if_false" => {
                "void".to_string()
            }

            // ── ret <src>: type = type of src, or "void" if not found ───────
            "ret" => {
                if let Some(Operand::Var(src)) = instr.srcs.first() {
                    env.get(src).cloned().unwrap_or_else(|| "void".to_string())
                } else if let Some(Operand::Int(_)) = instr.srcs.first() {
                    "i64".to_string()
                } else {
                    "void".to_string()
                }
            }

            // ── call: type = type of dest register, or "i64" default ────────
            //
            // Twig user-function calls return "any" from the compiler.  At
            // the BEAM level, functions return a tagged value in x0.  We
            // default to "i64" — numeric programs always return integers.
            "call" => {
                if let Some(dest) = &instr.dest {
                    env.get(dest)
                        .cloned()
                        .unwrap_or_else(|| "i64".to_string())
                } else {
                    "void".to_string()
                }
            }

            // ── load_reg / mov: register-to-register copy ───────────────────
            //
            // `load_reg` is the BEAM backend's register-copy op (lowered from
            // `_move` which the Twig compiler emits for `if`-expression arm
            // unification).  Its type is the type of the source register.
            "load_reg" | "mov" => {
                if let Some(Operand::Var(src)) = instr.srcs.first() {
                    env.get(src).cloned().unwrap_or_else(|| "i64".to_string())
                } else {
                    "i64".to_string()
                }
            }

            // ── Arithmetic ops with "any" typed operands ─────────────────────
            //
            // When function parameters are used in arithmetic, the inference
            // pass cannot determine the type (parameters are "any").  Default
            // to "i64" — this is always correct for Twig because the runtime
            // BEAM BIFs dispatch dynamically anyway.
            "add" | "sub" | "mul" | "div" | "mod" | "neg" | "not" => {
                // Try to infer from SSA env first, then default to i64.
                let from_ops: Option<String> = instr.srcs.iter().find_map(|src| {
                    if let Operand::Var(name) = src {
                        env.get(name).cloned()
                    } else {
                        None
                    }
                });
                from_ops.unwrap_or_else(|| "i64".to_string())
            }

            // ── Comparison ops ────────────────────────────────────────────────
            //
            // cmp_* always returns bool; this should have been inferred by the
            // type checker (rule R4), but guard here for completeness.
            "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                "bool".to_string()
            }

            // ── lnot ──────────────────────────────────────────────────────────
            "lnot" => "bool".to_string(),

            // ── Anything else: leave as-is for the validator to catch ─────────
            _ => "any".to_string(),
        };
        if fixed != "any" {
            instr.type_hint = fixed.clone();
            // Update SSA env if this instruction has a dest.
            if let Some(dest) = &instr.dest {
                env.insert(dest.clone(), fixed);
            }
        }
    }
}

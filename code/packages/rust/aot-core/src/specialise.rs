//! AOT specialization pass: `IIRFunction` + inferred types → `Vec<CIRInstr>`.
//!
//! This pass is the AOT analog of `jit-core`'s `specialise()` function.
//! The two are structurally identical; the only difference is how the
//! **specialization type** is determined:
//!
//! - **JIT**: consults `IIRInstr.observed_type` from the runtime profiler.
//! - **AOT**: consults the `inferred` type map produced by
//!   [`infer_types`](crate::infer::infer_types).
//!
//! In both cases, `type_hint` takes priority when it is concrete (not `"any"`).
//!
//! # How the spec type is chosen
//!
//! 1. If `instr.type_hint != "any"` and it is in the ALLOWED_TYPES allowlist
//!    → use it directly (statically typed source).
//! 2. Elif the instruction has a `dest` in `inferred` and its type is not
//!    `"any"` → use the inferred type.
//! 3. Elif the instruction is `ret` and its first src is a `Var` name in
//!    `inferred` → use that type.
//! 4. Otherwise → `"any"` → generic runtime-call path.
//!
//! # Guard emission
//!
//! Type guards (`type_assert`) are emitted the same way as in jit-core:
//! only when `type_hint == "any"` (statically untyped instruction) and the
//! specialization type is concrete.  For AOT the backend is responsible for
//! handling guard failures (typically a trap / abort rather than a JIT deopt).
//!
//! # Security
//!
//! Type strings are validated against `ALLOWED_TYPES` before being embedded in
//! CIR mnemonics.  An adversarial IIR with a malicious `type_hint` (e.g. one
//! containing spaces or underscores that would confuse backends) falls back to
//! `"any"` rather than being embedded verbatim.
//!
//! # Example
//!
//! ```
//! use std::collections::HashMap;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use aot_core::specialise::aot_specialise;
//!
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
//!     "u8",
//!     vec![
//!         IIRInstr::new("add", Some("v0".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
//!     ],
//! );
//! let mut env = HashMap::new();
//! env.insert("a".into(), "u8".into());
//! env.insert("b".into(), "u8".into());
//! env.insert("v0".into(), "u8".into());
//!
//! let cir = aot_specialise(&fn_, Some(&env));
//! let add = cir.iter().find(|i| i.op.starts_with("add_")).unwrap();
//! assert_eq!(add.op, "add_u8");
//! ```

use std::collections::HashMap;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use jit_core::cir::{CIRInstr, CIROperand};

// ---------------------------------------------------------------------------
// ALLOWED_TYPES allowlist
// ---------------------------------------------------------------------------

/// The same allowlist used by jit-core's `spec_type`.
///
/// Only types in this set are embedded in CIR mnemonics.  Unknown types fall
/// back to `"any"` (the generic runtime-call path).
const ALLOWED_TYPES: &[&str] = &[
    "u8",  "u16", "u32",  "u64",
    "i8",  "i16", "i32",  "i64",
    "f32", "f64",
    "bool", "str", "any",  "void",
];

// ---------------------------------------------------------------------------
// Op classification (mirrors jit-core)
// ---------------------------------------------------------------------------

fn is_binary_op(op: &str) -> bool {
    matches!(
        op,
        "add" | "sub" | "mul" | "div" | "mod"
        | "and" | "or" | "xor" | "shl" | "shr"
        | "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

fn is_unary_op(op: &str) -> bool {
    matches!(op, "neg" | "not")
}

/// Ops whose CIR form is identical to the IIR form — no specialization needed.
fn is_passthrough_op(op: &str) -> bool {
    matches!(
        op,
        "label" | "jmp" | "jmp_if_true" | "jmp_if_false"
        | "call" | "call_builtin"
        | "cast" | "type_assert"
        | "load_reg" | "store_reg" | "load_mem" | "store_mem"
        | "io_in" | "io_out"
    )
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Translate `fn_`'s instructions into typed `CIRInstr` objects using the
/// pre-computed `inferred` type environment.
///
/// # Parameters
///
/// - `fn_` — the `IIRFunction` to specialise.
/// - `inferred` — type map from [`infer_types`](crate::infer::infer_types).
///   `None` means only `type_hint` fields are consulted; untyped instructions
///   fall back to the generic `call_runtime` path.
///
/// # Returns
///
/// A flat `Vec<CIRInstr>` ready for [`CIROptimizer`](jit_core::optimizer::CIROptimizer)
/// and backend compilation.
pub fn aot_specialise(
    fn_: &IIRFunction,
    inferred: Option<&HashMap<String, String>>,
) -> Vec<CIRInstr> {
    let empty = HashMap::new();
    let env = inferred.unwrap_or(&empty);
    let mut result = Vec::new();
    for instr in &fn_.instructions {
        result.extend(translate(instr, env));
    }
    result
}

// ---------------------------------------------------------------------------
// Per-instruction translation
// ---------------------------------------------------------------------------

fn translate(instr: &IIRInstr, env: &HashMap<String, String>) -> Vec<CIRInstr> {
    let op = instr.op.as_str();

    // --- const ---
    if op == "const" {
        return vec![translate_const(instr)];
    }

    // --- ret_void ---
    if op == "ret_void" {
        return vec![CIRInstr::new("ret_void", None::<&str>, vec![], "void")];
    }

    // --- ret ---
    if op == "ret" {
        return vec![translate_ret(instr, env)];
    }

    // --- call_builtin: try to lower operator builtins to typed ops ---
    //
    // The IR compiler emits primitive operators (`+`, `-`, `*`, `/`,
    // `=`, `<`, `<=`, `>`, `>=`, `!=`, `_move`) as `call_builtin`
    // because the IIR is language-agnostic and treats them as runtime
    // helpers.  When the AOT pipeline can prove all operands have a
    // concrete type, lowering these to native typed ops (`add_<ty>`,
    // `cmp_eq_<ty>`, `mov_<ty>`) eliminates the runtime call and lets
    // a native backend emit a single CPU instruction.
    if op == "call_builtin" {
        if let Some(lowered) = try_specialize_builtin(instr, env) {
            return vec![lowered];
        }
        // Fall through to passthrough for unrecognised builtins.
    }

    // --- passthrough ---
    if is_passthrough_op(op) {
        let sp = spec_type(instr, env);
        return vec![CIRInstr::new(
            op,
            instr.dest.clone(),
            lift_srcs(&instr.srcs),
            sp,
        )];
    }

    // --- binary ops ---
    if is_binary_op(op) {
        return translate_binary(instr, env);
    }

    // --- unary ops ---
    if is_unary_op(op) {
        return translate_unary(instr, env);
    }

    // --- fallback: emit as generic ---
    let sp = spec_type(instr, env);
    vec![CIRInstr::new(op, instr.dest.clone(), lift_srcs(&instr.srcs), sp)]
}

fn translate_const(instr: &IIRInstr) -> CIRInstr {
    let src = instr.srcs.first();
    let t = if instr.type_hint != "any" && ALLOWED_TYPES.contains(&instr.type_hint.as_str()) {
        instr.type_hint.clone()
    } else {
        crate::infer::literal_type(src)
    };
    let cir_src = src.map(CIROperand::from).unwrap_or(CIROperand::Int(0));
    CIRInstr::new(format!("const_{t}"), instr.dest.clone(), vec![cir_src], t)
}

fn translate_ret(instr: &IIRInstr, env: &HashMap<String, String>) -> CIRInstr {
    let sp = spec_type(instr, env);
    CIRInstr::new(
        format!("ret_{sp}"),
        None::<&str>,
        lift_srcs(&instr.srcs),
        sp,
    )
}

fn translate_binary(instr: &IIRInstr, env: &HashMap<String, String>) -> Vec<CIRInstr> {
    let sp = spec_type(instr, env);
    let mut result = Vec::new();

    if sp == "any" {
        // Generic path — emit call_runtime "generic_{op}".
        let runtime_name = format!("generic_{}", instr.op);
        let mut srcs = vec![CIROperand::Var(runtime_name)];
        srcs.extend(lift_srcs(&instr.srcs));
        result.push(CIRInstr::new("call_runtime", instr.dest.clone(), srcs, "any"));
        return result;
    }

    // Special (op, type) override: string concatenation.
    if instr.op == "add" && sp == "str" {
        let mut srcs = vec![CIROperand::Var("str_concat".into())];
        srcs.extend(lift_srcs(&instr.srcs));
        result.push(CIRInstr::new("call_runtime", instr.dest.clone(), srcs, sp));
        return result;
    }

    // jmp_if_true/false + bool → specialized branch ops.
    match (instr.op.as_str(), sp.as_str()) {
        ("jmp_if_false", "bool") => {
            result.push(CIRInstr::new("br_false_bool", instr.dest.clone(), lift_srcs(&instr.srcs), sp));
            return result;
        }
        ("jmp_if_true", "bool") => {
            result.push(CIRInstr::new("br_true_bool", instr.dest.clone(), lift_srcs(&instr.srcs), sp));
            return result;
        }
        _ => {}
    }

    // Concrete type — emit guards (only for untyped source instructions).
    if instr.type_hint == "any" {
        let deopt = instr.deopt_anchor.unwrap_or(0);
        for src in &instr.srcs {
            if let Operand::Var(name) = src {
                result.push(CIRInstr::new_with_deopt(
                    "type_assert",
                    None::<&str>,
                    vec![CIROperand::Var(name.clone()), CIROperand::Var(sp.clone())],
                    "void",
                    deopt,
                ));
            }
        }
    }

    let cir_op = format!("{}_{}", instr.op, sp);
    result.push(CIRInstr::new(cir_op, instr.dest.clone(), lift_srcs(&instr.srcs), sp));
    result
}

fn translate_unary(instr: &IIRInstr, env: &HashMap<String, String>) -> Vec<CIRInstr> {
    let sp = spec_type(instr, env);
    let mut result = Vec::new();

    if sp == "any" {
        let mut srcs = vec![CIROperand::Var(format!("generic_{}", instr.op))];
        srcs.extend(lift_srcs(&instr.srcs));
        result.push(CIRInstr::new("call_runtime", instr.dest.clone(), srcs, "any"));
        return result;
    }

    if instr.type_hint == "any" {
        let deopt = instr.deopt_anchor.unwrap_or(0);
        for src in &instr.srcs {
            if let Operand::Var(name) = src {
                result.push(CIRInstr::new_with_deopt(
                    "type_assert",
                    None::<&str>,
                    vec![CIROperand::Var(name.clone()), CIROperand::Var(sp.clone())],
                    "void",
                    deopt,
                ));
            }
        }
    }

    let cir_op = format!("{}_{}", instr.op, sp);
    result.push(CIRInstr::new(cir_op, instr.dest.clone(), lift_srcs(&instr.srcs), sp));
    result
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Determine the specialization type for an instruction.
///
/// Decision tree (mirrors Python `_spec_type`):
///
/// 1. `type_hint` concrete and in ALLOWED_TYPES → use it.
/// 2. `dest` in `inferred` with a concrete type → use it.
/// 3. `op == "ret"` and first src is a `Var` in `inferred` → use it.
/// 4. → `"any"`.
pub fn spec_type(instr: &IIRInstr, env: &HashMap<String, String>) -> String {
    // 1. Explicit type hint beats everything.
    if instr.type_hint != "any" {
        if ALLOWED_TYPES.contains(&instr.type_hint.as_str()) {
            return instr.type_hint.clone();
        }
        // Malformed type hint — fall through to "any".
    }

    // 2. Check inferred environment for the destination register.
    if let Some(dest) = &instr.dest {
        if let Some(t) = env.get(dest) {
            if t != "any" && ALLOWED_TYPES.contains(&t.as_str()) {
                return t.clone();
            }
        }
    }

    // 3. `ret` has no dest — check the return value's type.
    if instr.op == "ret" {
        if let Some(Operand::Var(name)) = instr.srcs.first() {
            if let Some(t) = env.get(name) {
                if t != "any" && ALLOWED_TYPES.contains(&t.as_str()) {
                    return t.clone();
                }
            }
        }
    }

    "any".into()
}

/// Lift a slice of `Operand` into `Vec<CIROperand>`.
fn lift_srcs(srcs: &[Operand]) -> Vec<CIROperand> {
    srcs.iter().map(CIROperand::from).collect()
}

// ---------------------------------------------------------------------------
// `call_builtin` operator lowering
// ---------------------------------------------------------------------------
//
// The IIR's `call_builtin "<op>" arg1 arg2 …` is the universal way to
// invoke runtime helpers, including primitive arithmetic (because the
// IIR is intentionally language-agnostic).  The AOT specialiser lowers
// these to typed CIR ops when all operands have concrete types — the
// native backend then emits a single CPU instruction instead of a
// runtime call.

/// Map `call_builtin "<name>"` to a typed CIR mnemonic family when both
/// operands carry the same concrete type.
///
/// Returns `Some(cir_instr)` when the lowering succeeded, or `None`
/// when the builtin is unrecognised or operand types are unknown — in
/// which case the caller falls back to the passthrough path.
fn try_specialize_builtin(
    instr: &IIRInstr,
    env: &HashMap<String, String>,
) -> Option<CIRInstr> {
    // First src is `Var("<op_name>")` — the builtin's name string.
    let op_name = instr.srcs.first().and_then(|s| match s {
        Operand::Var(n) => Some(n.as_str()),
        _ => None,
    })?;

    // Inspect the *actual* arguments (everything after the name).
    let arg_srcs = &instr.srcs[1..];

    match op_name {
        // ---- Binary arithmetic ---------------------------------------
        "+" | "-" | "*" | "/" => {
            if arg_srcs.len() != 2 { return None; }
            let ty = pick_binary_ty(arg_srcs, env)?;
            let cir_op = match op_name {
                "+" => "add", "-" => "sub", "*" => "mul", "/" => "div",
                _ => unreachable!(),
            };
            Some(CIRInstr::new(
                format!("{cir_op}_{ty}"),
                instr.dest.clone(),
                lift_srcs(arg_srcs),
                ty,
            ))
        }

        // ---- Binary comparisons (result is `bool`) -------------------
        "=" | "==" | "!=" | "<" | "<=" | ">" | ">=" => {
            if arg_srcs.len() != 2 { return None; }
            let ty = pick_binary_ty(arg_srcs, env)?;
            let cir_op = match op_name {
                "=" | "==" => "cmp_eq",
                "!="       => "cmp_ne",
                "<"        => "cmp_lt",
                "<="       => "cmp_le",
                ">"        => "cmp_gt",
                ">="       => "cmp_ge",
                _ => unreachable!(),
            };
            Some(CIRInstr::new(
                format!("{cir_op}_{ty}"),
                instr.dest.clone(),
                lift_srcs(arg_srcs),
                "bool".to_string(),
            ))
        }

        // ---- Unary move (`if`'s phi-node implementation) -------------
        //
        // The IR compiler emits `call_builtin "_move" src` to assign
        // `src` to a destination register, typically when both arms of
        // an `if` need to write to the same `_ifv*` slot.  Lower as a
        // typed move so the backend just copies the value.
        "_move" => {
            if arg_srcs.len() != 1 { return None; }
            let ty = pick_unary_ty(&arg_srcs[0], env)?;
            Some(CIRInstr::new(
                format!("mov_{ty}"),
                instr.dest.clone(),
                lift_srcs(arg_srcs),
                ty,
            ))
        }

        _ => None,
    }
}

/// Pick a single concrete type for a 2-arg builtin.
///
/// Strategy:
/// 1. Prefer the type of any `Var` operand that resolves in `env` to a
///    type in `ALLOWED_TYPES` (excluding "any" and "void").
/// 2. Fall back to the literal type of any `Int`/`Bool` operand.
/// 3. Returns `None` if no operand has a usable type — the caller
///    will leave the instruction as `call_builtin`.
fn pick_binary_ty(args: &[Operand], env: &HashMap<String, String>) -> Option<String> {
    for a in args {
        if let Some(t) = operand_concrete_ty(a, env) {
            return Some(t);
        }
    }
    None
}

fn pick_unary_ty(arg: &Operand, env: &HashMap<String, String>) -> Option<String> {
    operand_concrete_ty(arg, env)
}

fn operand_concrete_ty(op: &Operand, env: &HashMap<String, String>) -> Option<String> {
    match op {
        Operand::Var(name) => {
            let t = env.get(name)?;
            if t != "any" && t != "void" && ALLOWED_TYPES.contains(&t.as_str()) {
                Some(t.clone())
            } else {
                None
            }
        }
        Operand::Int(_)   => Some("u8".to_string()), // small int default
        Operand::Bool(_)  => Some("bool".to_string()),
        Operand::Float(_) => Some("f64".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_fn(instrs: Vec<IIRInstr>) -> IIRFunction {
        IIRFunction::new("test", vec![], "any", instrs)
    }

    fn env_from(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    // ------------------------------------------------------------------
    // spec_type()
    // ------------------------------------------------------------------

    #[test]
    fn spec_type_uses_type_hint() {
        let instr = IIRInstr::new("add", Some("v".into()), vec![], "u8");
        let env = HashMap::new();
        assert_eq!(spec_type(&instr, &env), "u8");
    }

    #[test]
    fn spec_type_uses_env_dest() {
        let instr = IIRInstr::new("add", Some("v".into()), vec![], "any");
        let env = env_from(&[("v", "u16")]);
        assert_eq!(spec_type(&instr, &env), "u16");
    }

    #[test]
    fn spec_type_ret_uses_src_env() {
        let instr = IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any");
        let env = env_from(&[("v0", "u32")]);
        assert_eq!(spec_type(&instr, &env), "u32");
    }

    #[test]
    fn spec_type_falls_back_to_any() {
        let instr = IIRInstr::new("add", Some("v".into()), vec![], "any");
        let env = HashMap::new();
        assert_eq!(spec_type(&instr, &env), "any");
    }

    #[test]
    fn spec_type_rejects_unknown_type_hint() {
        let instr = IIRInstr::new("add", Some("v".into()), vec![], "evil_type");
        let env = HashMap::new();
        assert_eq!(spec_type(&instr, &env), "any");
    }

    // ------------------------------------------------------------------
    // aot_specialise()
    // ------------------------------------------------------------------

    #[test]
    fn const_i32_emitted() {
        let fn_ = make_fn(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "i32"),
        ]);
        let cir = aot_specialise(&fn_, None);
        assert_eq!(cir.len(), 1);
        assert_eq!(cir[0].op, "const_i32");
    }

    #[test]
    fn const_inferred_from_literal() {
        let fn_ = make_fn(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(10)], "any"),
        ]);
        let cir = aot_specialise(&fn_, None);
        assert_eq!(cir[0].op, "const_u8");
    }

    #[test]
    fn ret_void_emitted() {
        let fn_ = make_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let cir = aot_specialise(&fn_, None);
        assert_eq!(cir[0].op, "ret_void");
        assert_eq!(cir[0].ty, "void");
    }

    #[test]
    fn add_u8_typed() {
        let fn_ = make_fn(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        let env = env_from(&[("a", "u8"), ("b", "u8"), ("v", "u8")]);
        let cir = aot_specialise(&fn_, Some(&env));
        // Should have two type_assert guards + one add_u8
        let add = cir.iter().find(|i| i.op.starts_with("add_")).unwrap();
        assert_eq!(add.op, "add_u8");
    }

    #[test]
    fn add_any_emits_call_runtime() {
        let fn_ = make_fn(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        let cir = aot_specialise(&fn_, None);
        assert_eq!(cir[0].op, "call_runtime");
    }

    #[test]
    fn passthrough_label() {
        let fn_ = make_fn(vec![
            IIRInstr::new("label", None, vec![Operand::Var("loop_start".into())], "any"),
        ]);
        let cir = aot_specialise(&fn_, None);
        assert_eq!(cir[0].op, "label");
    }

    #[test]
    fn ret_inferred_from_src_env() {
        let fn_ = make_fn(vec![
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
        ]);
        let env = env_from(&[("v0", "u64")]);
        let cir = aot_specialise(&fn_, Some(&env));
        assert_eq!(cir[0].op, "ret_u64");
    }

    #[test]
    fn neg_typed() {
        let fn_ = make_fn(vec![
            IIRInstr::new("neg", Some("v".into()), vec![Operand::Var("x".into())], "any"),
        ]);
        let env = env_from(&[("x", "i32"), ("v", "i32")]);
        let cir = aot_specialise(&fn_, Some(&env));
        let neg = cir.iter().find(|i| i.op.starts_with("neg_")).unwrap();
        assert_eq!(neg.op, "neg_i32");
    }

    #[test]
    fn add_str_str_emits_call_runtime_str_concat() {
        let fn_ = make_fn(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        let env = env_from(&[("a", "str"), ("b", "str"), ("v", "str")]);
        let cir = aot_specialise(&fn_, Some(&env));
        assert_eq!(cir.last().unwrap().op, "call_runtime");
    }

    #[test]
    fn cmp_eq_typed() {
        // The result of cmp_eq is always "bool", so spec_type returns "bool"
        // from the env (infer_types sets dest → "bool" for comparison ops).
        // This is correct: the CIR op encodes the RESULT type, not the
        // operand type.  For comparing two u8 values, the CIR is
        // `cmp_eq_bool` because the dest register holds a bool.
        let fn_ = make_fn(vec![
            IIRInstr::new("cmp_eq", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        let env = env_from(&[("a", "u8"), ("b", "u8"), ("v", "bool")]);
        let cir = aot_specialise(&fn_, Some(&env));
        let cmp = cir.iter().find(|i| i.op.starts_with("cmp_")).unwrap();
        assert_eq!(cmp.op, "cmp_eq_bool");
    }

    #[test]
    fn cmp_eq_explicit_type_hint() {
        // When type_hint is set explicitly to a concrete non-bool type,
        // spec_type uses it verbatim (for frontends that encode operand type).
        let fn_ = make_fn(vec![
            IIRInstr::new("cmp_eq", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
        ]);
        let cir = aot_specialise(&fn_, None);
        let cmp = cir.iter().find(|i| i.op.starts_with("cmp_")).unwrap();
        assert_eq!(cmp.op, "cmp_eq_u8");
    }

    #[test]
    fn guards_emitted_for_untyped_instr() {
        let fn_ = make_fn(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        let env = env_from(&[("a", "u8"), ("b", "u8"), ("v", "u8")]);
        let cir = aot_specialise(&fn_, Some(&env));
        let guards: Vec<_> = cir.iter().filter(|i| i.op == "type_assert").collect();
        assert_eq!(guards.len(), 2);
    }

    #[test]
    fn no_guards_for_statically_typed_instr() {
        let fn_ = make_fn(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
        ]);
        let env = env_from(&[("a", "u8"), ("b", "u8"), ("v", "u8")]);
        let cir = aot_specialise(&fn_, Some(&env));
        let guards: Vec<_> = cir.iter().filter(|i| i.op == "type_assert").collect();
        assert!(guards.is_empty());
    }
}

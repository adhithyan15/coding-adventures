//! Specialisation pass: `IIRFunction` → `Vec<CIRInstr>`.
//!
//! The specialisation pass is the heart of jit-core.  It walks the flat list
//! of [`interpreter_ir::instr::IIRInstr`] objects in an
//! [`interpreter_ir::function::IIRFunction`], consults the type-feedback slots
//! filled by `vm-core`'s profiler, and emits typed [`CIRInstr`] objects.
//!
//! # How the pass works
//!
//! For each `IIRInstr`:
//!
//! 1. **Determine the specialisation type** using [`spec_type`]:
//!    - If `type_hint` is concrete (not `"any"`): use it directly.
//!    - Elif `observed_type` is concrete, not polymorphic, and
//!      `observation_count >= min_observations`: use it.
//!    - Otherwise: fall back to `"any"` → generic runtime call.
//!
//! 2. **Emit typed CIR**:
//!    - Typed arithmetic/bitwise/comparison: emit type guards for each variable
//!      source, then the specialised instruction (e.g. `add_u8`).
//!    - Generic (type = `"any"` or polymorphic): emit `call_runtime` with a
//!      `"generic_{op}"` argument for binary / unary ops.
//!    - Control-flow (`label`, `jmp`, `jmp_if_true`, `jmp_if_false`): pass
//!      through unchanged.
//!    - `const`: emit `const_{type}` using the literal value's natural type.
//!    - `ret` / `ret_void`: emit `ret_{type}` / `ret_void`.
//!    - `call` / `call_builtin` / `cast` / `type_assert` / memory ops: pass
//!      through unchanged.
//!
//! # Type guard emission
//!
//! Guards are only emitted when:
//! - `type_hint == "any"` (statically-typed instructions don't need guards), AND
//! - the specialisation type is concrete, AND
//! - the source operand is a variable name (guards on literals are vacuous).
//!
//! Each guard is:
//! ```text
//! type_assert  [x, "u8"]  [void]  [deopt→anchor]
//! ```
//!
//! # Special-case op mappings
//!
//! Some `(op, type)` combinations map to non-trivial CIR ops:
//!
//! | IIR op | Type | CIR op |
//! |---|---|---|
//! | `add` | `str` | `call_runtime str_concat` |
//! | `jmp_if_false` | `bool` | `br_false_bool` |
//! | `jmp_if_true` | `bool` | `br_true_bool` |
//! | `neg` | any concrete | `neg_{type}` (unary) |
//! | `not` | any concrete | `not_{type}` (unary) |

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::opcodes::POLYMORPHIC_TYPE;

use crate::cir::{CIRInstr, CIROperand};

// ---------------------------------------------------------------------------
// Operation sets
// ---------------------------------------------------------------------------

/// Binary ops that map to `{op}_{type}` by default.
fn is_binary_op(op: &str) -> bool {
    matches!(
        op,
        "add" | "sub" | "mul" | "div" | "mod"
        | "and" | "or" | "xor" | "shl" | "shr"
        | "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

/// Unary ops that map to `{op}_{type}`.
fn is_unary_op(op: &str) -> bool {
    matches!(op, "neg" | "not")
}

/// Ops whose CIR form is identical to the IIR form — no specialisation needed.
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

/// Translate `fn_`'s instructions into typed `CIRInstr` objects.
///
/// # Parameters
///
/// - `fn_` — the `IIRFunction` to specialise.  Its `IIRInstr` objects should
///   have been populated with type-feedback by `vm-core`'s profiler.
/// - `min_observations` — minimum number of times an `"any"`-typed instruction
///   must have been profiled before its observed type is trusted.  Lower values
///   produce more aggressive specialisation but riskier guards.  Default: 5.
///
/// # Returns
///
/// A flat `Vec<CIRInstr>` ready for [`CIROptimizer`](crate::optimizer::CIROptimizer)
/// and backend compilation.
///
/// # Example
///
/// ```
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use jit_core::specialise::specialise;
///
/// let fn_ = IIRFunction::new(
///     "add",
///     vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
///     "u8",
///     vec![
///         IIRInstr::new("add", Some("v0".into()), vec![
///             Operand::Var("a".into()), Operand::Var("b".into()),
///         ], "u8"),
///         IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
///     ],
/// );
/// let cir = specialise(&fn_, 0);
/// assert!(!cir.is_empty());
/// let add = cir.iter().find(|i| i.op.starts_with("add_")).unwrap();
/// assert_eq!(add.op, "add_u8");
/// ```
pub fn specialise(fn_: &IIRFunction, min_observations: u32) -> Vec<CIRInstr> {
    let mut result = Vec::new();
    for instr in &fn_.instructions {
        result.extend(translate(instr, min_observations));
    }
    result
}

// ---------------------------------------------------------------------------
// Per-instruction translation
// ---------------------------------------------------------------------------

fn translate(instr: &IIRInstr, min_obs: u32) -> Vec<CIRInstr> {
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
        return vec![translate_ret(instr, min_obs)];
    }

    // --- passthrough ---
    if is_passthrough_op(op) {
        let sp_type = spec_type(instr, min_obs);
        return vec![CIRInstr::new(
            op,
            instr.dest.clone(),
            lift_srcs(&instr.srcs),
            sp_type,
        )];
    }

    // --- binary ops ---
    if is_binary_op(op) {
        return translate_binary(instr, min_obs);
    }

    // --- unary ops ---
    if is_unary_op(op) {
        return translate_unary(instr, min_obs);
    }

    // --- fallback: emit as generic ---
    let sp_type = spec_type(instr, min_obs);
    vec![CIRInstr::new(
        op,
        instr.dest.clone(),
        lift_srcs(&instr.srcs),
        sp_type,
    )]
}

fn translate_const(instr: &IIRInstr) -> CIRInstr {
    let src = instr.srcs.first();
    let t = if instr.type_hint != "any" {
        instr.type_hint.clone()
    } else {
        literal_type(src)
    };
    let cir_src = src.map(CIROperand::from).unwrap_or(CIROperand::Int(0));
    CIRInstr::new(format!("const_{t}"), instr.dest.clone(), vec![cir_src], t)
}

fn translate_ret(instr: &IIRInstr, min_obs: u32) -> CIRInstr {
    let sp_type = spec_type(instr, min_obs);
    CIRInstr::new(
        format!("ret_{sp_type}"),
        None::<&str>,
        lift_srcs(&instr.srcs),
        sp_type,
    )
}

fn translate_binary(instr: &IIRInstr, min_obs: u32) -> Vec<CIRInstr> {
    let sp_type = spec_type(instr, min_obs);
    let mut result = Vec::new();

    if sp_type == "any" {
        // Generic path — emit call_runtime "generic_{op}".
        let runtime_name = format!("generic_{}", instr.op);
        let mut srcs = vec![CIROperand::Var(runtime_name)];
        srcs.extend(lift_srcs(&instr.srcs));
        result.push(CIRInstr::new("call_runtime", instr.dest.clone(), srcs, "any"));
        return result;
    }

    // Check for special (op, type) overrides.
    match (instr.op.as_str(), sp_type.as_str()) {
        ("add", "str") => {
            let mut srcs = vec![CIROperand::Var("str_concat".into())];
            srcs.extend(lift_srcs(&instr.srcs));
            result.push(CIRInstr::new("call_runtime", instr.dest.clone(), srcs, sp_type));
            return result;
        }
        ("jmp_if_false", "bool") => {
            result.push(CIRInstr::new("br_false_bool", instr.dest.clone(), lift_srcs(&instr.srcs), sp_type));
            return result;
        }
        ("jmp_if_true", "bool") => {
            result.push(CIRInstr::new("br_true_bool", instr.dest.clone(), lift_srcs(&instr.srcs), sp_type));
            return result;
        }
        _ => {}
    }

    // Concrete type path — emit guards then specialised op.
    if instr.type_hint == "any" {
        // Guards needed only when source was untyped.
        let deopt = instr.deopt_anchor.unwrap_or(0);
        for src in &instr.srcs {
            if let Operand::Var(name) = src {
                result.push(CIRInstr::new_with_deopt(
                    "type_assert",
                    None::<&str>,
                    vec![CIROperand::Var(name.clone()), CIROperand::Var(sp_type.clone())],
                    "void",
                    deopt,
                ));
            }
        }
    }

    let cir_op = format!("{}_{}", instr.op, sp_type);
    result.push(CIRInstr::new(cir_op, instr.dest.clone(), lift_srcs(&instr.srcs), sp_type));
    result
}

fn translate_unary(instr: &IIRInstr, min_obs: u32) -> Vec<CIRInstr> {
    let sp_type = spec_type(instr, min_obs);
    let mut result = Vec::new();

    if sp_type == "any" {
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
                    vec![CIROperand::Var(name.clone()), CIROperand::Var(sp_type.clone())],
                    "void",
                    deopt,
                ));
            }
        }
    }

    let cir_op = format!("{}_{}", instr.op, sp_type);
    result.push(CIRInstr::new(cir_op, instr.dest.clone(), lift_srcs(&instr.srcs), sp_type));
    result
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Known concrete type strings that the specialiser can embed in CIR mnemonics.
///
/// Only types in this allowlist are used to form typed mnemonics like
/// `add_u8` or `const_f64`.  Any unknown type string from `type_hint` or
/// `observed_type` falls back to `"any"` (the generic path), preventing
/// attacker-controlled IIR from injecting arbitrary strings into CIR
/// operation names.
///
/// # Security
///
/// `type_hint` and `observed_type` originate from language frontends and
/// are propagated through IIR deserialization.  If the IIRModule came from
/// untrusted input, these fields must not be blindly embedded in CIR
/// mnemonics — a malicious type string containing spaces, newlines, or
/// underscore sequences could confuse backends that parse the mnemonic by
/// splitting on `_`.  The allowlist ensures only safe, known types are used.
const ALLOWED_TYPES: &[&str] = &[
    "u8",  "u16", "u32",  "u64",
    "i8",  "i16", "i32",  "i64",
    "f32", "f64",
    "bool", "str", "any",  "void",
];

/// Return the specialisation type for `instr`, or `"any"` for the generic path.
///
/// Decision tree (mirrors the Python `_spec_type` function verbatim):
///
/// 1. If `type_hint != "any"` and `type_hint` is in the allowlist → use it.
/// 2. If `observed_type` is set, is not `"polymorphic"`, is in the allowlist,
///    and `observation_count >= min_obs` → use the observed type.
/// 3. Otherwise → `"any"`.
///
/// # Security
///
/// Type strings are validated against [`ALLOWED_TYPES`] before being
/// embedded in CIR mnemonics.  Unknown types fall back to `"any"` so the
/// generic path is taken instead.
pub fn spec_type(instr: &IIRInstr, min_obs: u32) -> String {
    if instr.type_hint != "any" {
        // Validate against allowlist — reject unknown type strings.
        if ALLOWED_TYPES.contains(&instr.type_hint.as_str()) {
            return instr.type_hint.clone();
        }
        // Unknown type_hint: fall through to "any" rather than embedding it.
    }
    if let Some(ot) = &instr.observed_type {
        if ot != POLYMORPHIC_TYPE
            && instr.observation_count >= min_obs
            && ALLOWED_TYPES.contains(&ot.as_str())
        {
            return ot.clone();
        }
    }
    "any".to_string()
}

/// Infer a type string from an IIR literal operand.
///
/// Mirrors the Python `_literal_type` function.
///
/// | Rust literal | Returned type |
/// |---|---|
/// | `Bool(_)` | `"bool"` |
/// | `Int(0..=255)` | `"u8"` |
/// | `Int(256..=65535)` | `"u16"` |
/// | `Int(65536..=4294967295)` | `"u32"` |
/// | `Int(_)` | `"u64"` |
/// | `Float(_)` | `"f64"` |
/// | `Var(_)` (string literal via name) | `"str"` |
/// | `None` | `"any"` |
pub fn literal_type(op: Option<&Operand>) -> String {
    match op {
        None => "any".into(),
        Some(Operand::Bool(_)) => "bool".into(),
        Some(Operand::Int(n)) => {
            let u = *n as u64;
            if u <= 255 { "u8" }
            else if u <= 65_535 { "u16" }
            else if u <= 4_294_967_295 { "u32" }
            else { "u64" }
        }.into(),
        Some(Operand::Float(_)) => "f64".into(),
        Some(Operand::Var(_)) => "str".into(),
    }
}

/// Lift a slice of `Operand` into `Vec<CIROperand>`.
fn lift_srcs(srcs: &[Operand]) -> Vec<CIROperand> {
    srcs.iter().map(CIROperand::from).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::function::IIRFunction;
    use crate::cir::CIROperand;

    fn make_fn(instructions: Vec<IIRInstr>) -> IIRFunction {
        IIRFunction::new("test", vec![], "any", instructions)
    }

    fn add_instr(type_hint: &str) -> IIRInstr {
        IIRInstr::new(
            "add",
            Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            type_hint,
        )
    }

    // ------------------------------------------------------------------
    // spec_type tests
    // ------------------------------------------------------------------

    #[test]
    fn spec_type_uses_type_hint_when_concrete() {
        let instr = add_instr("u8");
        assert_eq!(spec_type(&instr, 5), "u8");
    }

    #[test]
    fn spec_type_any_with_no_observation() {
        let instr = add_instr("any");
        assert_eq!(spec_type(&instr, 5), "any");
    }

    #[test]
    fn spec_type_uses_observed_when_enough_obs() {
        let mut instr = add_instr("any");
        for _ in 0..5 { instr.record_observation("u16"); }
        assert_eq!(spec_type(&instr, 5), "u16");
    }

    #[test]
    fn spec_type_any_when_too_few_obs() {
        let mut instr = add_instr("any");
        instr.record_observation("u16");
        assert_eq!(spec_type(&instr, 5), "any");
    }

    #[test]
    fn spec_type_any_when_polymorphic() {
        let mut instr = add_instr("any");
        instr.record_observation("u8");
        instr.record_observation("u16");
        assert_eq!(spec_type(&instr, 2), "any");
    }

    // ------------------------------------------------------------------
    // literal_type tests
    // ------------------------------------------------------------------

    #[test]
    fn literal_type_bool() {
        assert_eq!(literal_type(Some(&Operand::Bool(true))), "bool");
    }

    #[test]
    fn literal_type_small_int() {
        assert_eq!(literal_type(Some(&Operand::Int(42))), "u8");
    }

    #[test]
    fn literal_type_u16_range() {
        assert_eq!(literal_type(Some(&Operand::Int(1000))), "u16");
    }

    #[test]
    fn literal_type_u32_range() {
        assert_eq!(literal_type(Some(&Operand::Int(100_000))), "u32");
    }

    #[test]
    fn literal_type_float() {
        assert_eq!(literal_type(Some(&Operand::Float(3.14))), "f64");
    }

    // ------------------------------------------------------------------
    // specialise() end-to-end tests
    // ------------------------------------------------------------------

    #[test]
    fn specialise_typed_add() {
        let fn_ = make_fn(vec![add_instr("u8")]);
        let cir = specialise(&fn_, 0);
        let add = cir.iter().find(|i| i.op.starts_with("add_")).unwrap();
        assert_eq!(add.op, "add_u8");
        assert_eq!(add.ty, "u8");
    }

    #[test]
    fn specialise_const_instr() {
        let fn_ = make_fn(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        ]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir.len(), 1);
        assert_eq!(cir[0].op, "const_u8");
        assert_eq!(cir[0].srcs[0], CIROperand::Int(10));
    }

    #[test]
    fn specialise_ret_void() {
        let fn_ = make_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "ret_void");
    }

    #[test]
    fn specialise_ret_typed() {
        let fn_ = make_fn(vec![
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
        ]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "ret_u8");
    }

    #[test]
    fn specialise_binary_generic_fallback() {
        let fn_ = make_fn(vec![add_instr("any")]);
        let cir = specialise(&fn_, 5);
        // Not enough observations → generic fallback.
        assert_eq!(cir[0].op, "call_runtime");
        assert_eq!(cir[0].srcs[0], CIROperand::Var("generic_add".into()));
    }

    #[test]
    fn specialise_binary_with_observed_type() {
        let mut instr = add_instr("any");
        for _ in 0..5 { instr.record_observation("u32"); }
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 5);
        // No call_runtime — specialised to add_u32.
        let add = cir.iter().find(|i| i.op.starts_with("add_")).unwrap();
        assert_eq!(add.op, "add_u32");
    }

    #[test]
    fn specialise_guard_emitted_for_any_typed_instr() {
        // Observed type exists → specialise; since type_hint is "any" → emit guards.
        let mut instr = add_instr("any");
        for _ in 0..5 { instr.record_observation("u8"); }
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 5);
        // Two guards (one per variable source) + one add_u8.
        let guards: Vec<_> = cir.iter().filter(|i| i.op == "type_assert").collect();
        assert_eq!(guards.len(), 2);
    }

    #[test]
    fn specialise_no_guard_for_statically_typed() {
        // type_hint = "u8" → no guards.
        let fn_ = make_fn(vec![add_instr("u8")]);
        let cir = specialise(&fn_, 0);
        assert!(cir.iter().all(|i| i.op != "type_assert"));
    }

    #[test]
    fn specialise_str_add_emits_call_runtime() {
        let instr = IIRInstr::new(
            "add",
            Some("r".into()),
            vec![Operand::Var("s1".into()), Operand::Var("s2".into())],
            "str",
        );
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "call_runtime");
        assert_eq!(cir[0].srcs[0], CIROperand::Var("str_concat".into()));
    }

    #[test]
    fn specialise_jmp_if_false_passthrough() {
        // `jmp_if_false` is in _PASSTHROUGH_OPS — it passes through unchanged.
        // The special-op mapping (jmp_if_false, bool) → br_false_bool is only
        // reachable via _translate_binary, which is never called for passthrough
        // ops.  The instruction stays as "jmp_if_false".
        let instr = IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var("cond".into()), Operand::Var("label_end".into())],
            "bool",
        );
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "jmp_if_false");
    }

    #[test]
    fn specialise_jmp_if_true_passthrough() {
        // Same reasoning as jmp_if_false — passthrough, stays as "jmp_if_true".
        let instr = IIRInstr::new(
            "jmp_if_true",
            None,
            vec![Operand::Var("cond".into()), Operand::Var("label_loop".into())],
            "bool",
        );
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "jmp_if_true");
    }

    #[test]
    fn specialise_neg_unary() {
        let instr = IIRInstr::new(
            "neg",
            Some("r".into()),
            vec![Operand::Var("x".into())],
            "i32",
        );
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir.last().unwrap().op, "neg_i32");
    }

    #[test]
    fn specialise_not_unary() {
        let instr = IIRInstr::new(
            "not",
            Some("r".into()),
            vec![Operand::Var("x".into())],
            "bool",
        );
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir.last().unwrap().op, "not_bool");
    }

    #[test]
    fn specialise_passthrough_label() {
        let instr = IIRInstr::new("label", None, vec![Operand::Var("loop_start".into())], "any");
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "label");
    }

    #[test]
    fn specialise_passthrough_jmp() {
        let instr = IIRInstr::new("jmp", None, vec![Operand::Var("exit".into())], "any");
        let fn_ = make_fn(vec![instr]);
        let cir = specialise(&fn_, 0);
        assert_eq!(cir[0].op, "jmp");
    }

    #[test]
    fn specialise_full_function() {
        // Compile add(a:u8, b:u8) → u8.
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
            "u8",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
            ],
        );
        let cir = specialise(&fn_, 0);
        assert!(cir.iter().any(|i| i.op == "add_u8"));
        assert!(cir.iter().any(|i| i.op == "ret_u8"));
    }
}

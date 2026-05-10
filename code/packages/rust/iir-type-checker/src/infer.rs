//! Type-inference pass — populates `type_hint` on `IIRInstr` objects
//! that currently carry `"any"`.
//!
//! # Design
//!
//! The inference pass is a **mutating SSA propagation** algorithm.  It
//! works entirely at the `IIRModule` level, which means it sees the full
//! definition of every function and can use SSA variable information from
//! earlier instructions in the same function body.
//!
//! ## Inference rules (in precedence order)
//!
//! | Rule | Trigger | Inferred type |
//! |------|---------|---------------|
//! | **R1 const-int** | `const dest, Int(n)` with `type_hint = "any"` | `"i64"` |
//! | **R2 const-float** | `const dest, Float(f)` with `type_hint = "any"` | `"f64"` |
//! | **R3 const-bool** | `const dest, Bool(b)` with `type_hint = "any"` | `"bool"` |
//! | **R4 cmp-bool** | `cmp_* dest, [a, b]` with `type_hint = "any"` | `"bool"` (comparisons always return bool) |
//! | **R5 arith-same** | `add/sub/mul/div/mod dest, [a, b]` where both `a` and `b` are the same concrete type | same type |
//! | **R6 bitwise-same** | `and/or/xor/shl/shr dest, [a, b]` with same-typed operands | same type |
//! | **R7 unary-passthrough** | `neg/not dest, [a]` where `a` has a concrete type | same type as `a` |
//! | **R8 ssa-copy** | `dest = src` (explicit copy or alias) where `src` is concrete | same type as `src` |
//!
//! ## Multi-pass propagation
//!
//! Rules R5–R8 depend on source types that may themselves be inferred.
//! A single forward pass is therefore not sufficient — e.g. if `a` is
//! inferred after the `add` that reads `a`.  The pass repeats until no
//! new types are inferred (fixed-point iteration).  Typical programs
//! converge in 2–3 passes.
//!
//! ## Returned map
//!
//! `infer_types_mut` returns a `HashMap<String, String>` of every variable
//! it successfully annotated (dest name → inferred type).  This is
//! included verbatim in the `inferred_types` field of
//! [`TypeCheckReport`](crate::report::TypeCheckReport).

use std::collections::HashMap;

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use interpreter_ir::opcodes::{is_concrete_type, DYNAMIC_TYPE};

// ---------------------------------------------------------------------------
// Op category helpers
// ---------------------------------------------------------------------------

fn is_cmp_op(op: &str) -> bool {
    matches!(
        op,
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

fn is_arith_op(op: &str) -> bool {
    matches!(op, "add" | "sub" | "mul" | "div" | "mod")
}

fn is_bitwise_op(op: &str) -> bool {
    matches!(op, "and" | "or" | "xor" | "shl" | "shr")
}

fn is_unary_op(op: &str) -> bool {
    matches!(op, "neg" | "not")
}

// ---------------------------------------------------------------------------
// Per-function inference
// ---------------------------------------------------------------------------

/// Infer types for one function's instructions.
///
/// Returns a `HashMap<dest_var, inferred_type>` for every instruction
/// this call annotated (across all iterations).  The function's
/// `IIRInstr` objects are mutated in place.
fn infer_function(func: &mut IIRFunction) -> HashMap<String, String> {
    // SSA map: dest var → concrete type.  Populated incrementally.
    let mut env: HashMap<String, String> = HashMap::new();
    let mut newly_inferred: HashMap<String, String> = HashMap::new();

    // Seed env with already-typed instructions.
    for instr in func.instructions.iter() {
        if let Some(dest) = &instr.dest {
            if is_concrete_type(instr.type_hint.as_str()) {
                env.insert(dest.clone(), instr.type_hint.clone());
            }
        }
    }

    // Fixed-point: keep passing until no new inferences.
    loop {
        let mut changed = false;

        for instr in func.instructions.iter_mut() {
            // Only consider instructions that are currently untyped.
            if instr.type_hint != DYNAMIC_TYPE {
                continue;
            }

            let Some(dest) = instr.dest.as_ref() else {
                continue; // void-result instruction — nothing to infer
            };

            let inferred = infer_single(instr, &env);

            if let Some(ty) = inferred {
                instr.type_hint = ty.clone();
                env.insert(dest.clone(), ty.clone());
                newly_inferred.insert(dest.clone(), ty);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    newly_inferred
}

/// Try to infer the type for a single instruction.
///
/// Returns `Some(type_string)` when a concrete type can be determined,
/// `None` when the instruction remains unresolvable.
fn infer_single(instr: &IIRInstr, env: &HashMap<String, String>) -> Option<String> {
    let op = instr.op.as_str();

    // R1–R3: constant literals ────────────────────────────────────────────
    if op == "const" {
        if let Some(src) = instr.srcs.first() {
            return match src {
                Operand::Int(_) => Some("i64".into()),
                Operand::Float(_) => Some("f64".into()),
                Operand::Bool(_) => Some("bool".into()),
                Operand::Var(_) => None, // symbolic constant — can't infer
            };
        }
    }

    // R4: comparisons always produce bool ─────────────────────────────────
    if is_cmp_op(op) {
        return Some("bool".into());
    }

    // R5 / R6: binary arithmetic / bitwise — infer from same-typed srcs ──
    if (is_arith_op(op) || is_bitwise_op(op)) && instr.srcs.len() >= 2 {
        let t0 = resolve_operand_type(&instr.srcs[0], env);
        let t1 = resolve_operand_type(&instr.srcs[1], env);
        if let (Some(a), Some(b)) = (t0, t1) {
            if a == b {
                return Some(a.to_string());
            }
        }
    }

    // R7: unary passthrough ───────────────────────────────────────────────
    if is_unary_op(op) {
        if let Some(src) = instr.srcs.first() {
            if let Some(ty) = resolve_operand_type(src, env) {
                return Some(ty.to_string());
            }
        }
    }

    // R8: SSA copy (load_reg / store_reg with a variable source) ──────────
    if op == "load_reg" || op == "store_reg" {
        if let Some(Operand::Var(name)) = instr.srcs.first() {
            if let Some(ty) = env.get(name.as_str()) {
                return Some(ty.clone());
            }
        }
    }

    None
}

/// Resolve the concrete type of an operand using the SSA env.
///
/// - `Var(name)` → looks up `name` in env.
/// - Immediates: `Int` → `"i64"`, `Float` → `"f64"`, `Bool` → `"bool"`.
///
/// Returns `None` when the variable is not yet typed.
fn resolve_operand_type<'a>(
    operand: &Operand,
    env: &'a HashMap<String, String>,
) -> Option<&'a str> {
    match operand {
        Operand::Var(name) => env.get(name.as_str()).map(String::as_str),
        // Immediate literals have well-known types.
        // We return static strings here; the caller converts to owned if needed.
        Operand::Int(_) => Some("i64"),
        Operand::Float(_) => Some("f64"),
        Operand::Bool(_) => Some("bool"),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the inference pass over every function in `module`, mutating
/// `type_hint` on instructions that were previously `"any"`.
///
/// Returns a `HashMap<dest_var, inferred_type>` of **all** type hints
/// that were added (across all functions in the module).  Variable names
/// from different functions may collide; the map reflects the last write.
/// Callers that need per-function granularity should call
/// `infer_types_mut` themselves on individual functions.
///
/// # Example
///
/// ```
/// use interpreter_ir::module::IIRModule;
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use iir_type_checker::infer::infer_types_mut;
///
/// let fn_ = IIRFunction::new(
///     "main", vec![], "void",
///     vec![
///         IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any"),
///         IIRInstr::new("ret_void", None, vec![], "void"),
///     ],
/// );
/// let mut module = IIRModule::new("test", "tetrad");
/// module.functions.push(fn_);
///
/// let inferred = infer_types_mut(&mut module);
/// assert_eq!(inferred.get("x").map(String::as_str), Some("i64"));
/// // The module itself was mutated:
/// assert_eq!(module.functions[0].instructions[0].type_hint, "i64");
/// ```
pub fn infer_types_mut(module: &mut IIRModule) -> HashMap<String, String> {
    let mut all_inferred: HashMap<String, String> = HashMap::new();
    for func in module.functions.iter_mut() {
        let inferred = infer_function(func);
        all_inferred.extend(inferred);
    }
    all_inferred
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        let mut m = IIRModule::new("test", "tetrad");
        m.functions.push(fn_);
        m
    }

    // R1–R3: constant literals

    #[test]
    fn infer_const_int() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any"),
        ]);
        let inf = infer_types_mut(&mut m);
        assert_eq!(inf.get("x").map(String::as_str), Some("i64"));
        assert_eq!(m.functions[0].instructions[0].type_hint, "i64");
    }

    #[test]
    fn infer_const_float() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("f".into()), vec![Operand::Float(3.14)], "any"),
        ]);
        infer_types_mut(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "f64");
    }

    #[test]
    fn infer_const_bool() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("b".into()), vec![Operand::Bool(false)], "any"),
        ]);
        infer_types_mut(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "bool");
    }

    // R4: comparisons

    #[test]
    fn infer_cmp_eq_produces_bool() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "any"),
            IIRInstr::new("cmp_eq", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        infer_types_mut(&mut m);
        assert_eq!(m.functions[0].instructions[2].type_hint, "bool");
    }

    // R5: arithmetic propagation

    #[test]
    fn infer_add_from_same_typed_srcs() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "any"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "any"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        infer_types_mut(&mut m);
        // a and b both infer as i64, so c should also be i64
        assert_eq!(m.functions[0].instructions[2].type_hint, "i64");
    }

    #[test]
    fn infer_does_not_change_already_typed() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "u8"),
        ]);
        infer_types_mut(&mut m);
        // Should not override explicitly-set u8
        assert_eq!(m.functions[0].instructions[0].type_hint, "u8");
    }

    #[test]
    fn infer_mismatched_arith_stays_any() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Float(1.0)], "any"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]);
        infer_types_mut(&mut m);
        // a infers i64, b infers f64 — they differ, so c stays "any"
        assert_eq!(m.functions[0].instructions[2].type_hint, "any");
    }

    // R7: unary passthrough

    #[test]
    fn infer_neg_from_src_type() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "any"),
            IIRInstr::new("neg", Some("b".into()), vec![Operand::Var("a".into())], "any"),
        ]);
        infer_types_mut(&mut m);
        assert_eq!(m.functions[0].instructions[1].type_hint, "i64");
    }

    // Multi-pass (SSA chain)

    #[test]
    fn infer_chain_two_levels_deep() {
        // a → b → c: only const a is typed directly from literal
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("neg", Some("b".into()), vec![Operand::Var("a".into())], "any"),
            IIRInstr::new("neg", Some("c".into()), vec![Operand::Var("b".into())], "any"),
        ]);
        infer_types_mut(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "i64");
        assert_eq!(m.functions[0].instructions[1].type_hint, "i64");
        assert_eq!(m.functions[0].instructions[2].type_hint, "i64");
    }
}

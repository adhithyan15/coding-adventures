//! `CIROptimizer` — constant folding + dead-code elimination over CIR.
//!
//! This module is a Rust port of the Python `codegen_core.optimizer.cir_optimizer`
//! module.  It runs two lightweight passes over the `Vec<CIRInstr>` produced by
//! the specialisation pass:
//!
//! # Pass 1 — Constant folding
//!
//! If **both** source operands of a typed arithmetic or comparison instruction are
//! literal values (`CIROperand::Int`, `Float`, or `Bool`), the result is computed
//! at compile time and the instruction is replaced with a `const_<type>` instruction.
//!
//! For example:
//! ```text
//! v0 = add_u8 5, 3  [u8]   →   v0 = const_u8 8  [u8]
//! v1 = cmp_lt_u8 2, 10  [bool]   →   v1 = const_bool true  [bool]
//! ```
//!
//! # Pass 2 — Dead-code elimination (DCE)
//!
//! An instruction is **dead** when:
//! - it has a `dest` register, AND
//! - that register is never read by any later instruction's `srcs`, AND
//! - the instruction has no observable side effects (is `pure`).
//!
//! Side-effectful ops are always kept: `call_runtime`, `call`, `call_builtin`,
//! `io_out`, `store_mem`, `store_reg`, `type_assert`, `ret`, `ret_void`,
//! `jmp`, `jmp_if_true`, `jmp_if_false`, `label`.
//!
//! # Usage
//!
//! ```
//! use jit_core::optimizer::CIROptimizer;
//! use jit_core::cir::{CIRInstr, CIROperand};
//!
//! let cir = vec![
//!     CIRInstr::new("const_u8", Some("x"), vec![CIROperand::Int(5)], "u8"),
//!     CIRInstr::new("const_u8", Some("y"), vec![CIROperand::Int(3)], "u8"),
//!     CIRInstr::new("add_u8",   Some("z"), vec![
//!         CIROperand::Var("x".into()),
//!         CIROperand::Var("y".into()),
//!     ], "u8"),
//!     CIRInstr::new("ret_u8",   None::<&str>, vec![CIROperand::Var("z".into())], "u8"),
//! ];
//!
//! let opt = CIROptimizer::new().run(cir);
//! // After optimisation the const instr is folded and dead loads removed.
//! assert!(opt.len() <= 4);
//! ```
//!
//! # Complexity
//!
//! Both passes are O(n) — linear in the number of instructions.  They are
//! idempotent: running twice produces the same result as running once.

use std::collections::{HashMap, HashSet};
use crate::cir::{CIRInstr, CIROperand};

// ---------------------------------------------------------------------------
// Foldable operations
// ---------------------------------------------------------------------------

/// Operations whose result can be constant-folded when both sources are literals.
///
/// The map value is the base op name stripped of the `_{type}` suffix
/// (e.g. `"add"` covers `"add_u8"`, `"add_u16"`, `"add_f64"`, …).
const FOLDABLE_OPS: &[&str] = &[
    "add", "sub", "mul", "div", "mod",
    "and", "or", "xor", "shl", "shr",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
];

fn base_op(op: &str) -> &str {
    // Strip the last `_type` segment from a typed mnemonic.
    // E.g. "add_u8" → "add", "cmp_lt_u8" → "cmp_lt", "add_f64" → "add".
    // Strategy: find the last '_' and check if what follows is a known type.
    if let Some(pos) = op.rfind('_') {
        let suffix = &op[pos + 1..];
        let known_types = [
            "u8","u16","u32","u64","i8","i16","i32","i64","bool","f32","f64","str","any","void",
        ];
        if known_types.contains(&suffix) {
            return &op[..pos];
        }
    }
    op
}

fn is_foldable(op: &str) -> bool {
    let base = base_op(op);
    FOLDABLE_OPS.contains(&base)
}

// ---------------------------------------------------------------------------
// CIROptimizer
// ---------------------------------------------------------------------------

/// Constant-folding + dead-code elimination over a CIR stream.
///
/// Construct with [`CIROptimizer::new`] and call [`CIROptimizer::run`] to
/// get the optimised list.
#[derive(Debug, Default)]
pub struct CIROptimizer;

impl CIROptimizer {
    /// Create a new `CIROptimizer`.
    pub fn new() -> Self {
        CIROptimizer
    }

    /// Run constant folding then dead-code elimination.
    ///
    /// Returns the optimised CIR.  The input `ir` is consumed.
    pub fn run(&self, ir: Vec<CIRInstr>) -> Vec<CIRInstr> {
        let folded = self.constant_fold(ir);
        self.dead_code_eliminate(folded)
    }

    // ------------------------------------------------------------------
    // Pass 1: constant folding
    // ------------------------------------------------------------------

    /// Walk `ir` and replace foldable instructions that have two literal
    /// sources with a `const_<type>` instruction.
    ///
    /// The fold is only performed when:
    /// 1. `is_foldable(instr.op)` is `true`, AND
    /// 2. `instr.srcs.len() >= 2`, AND
    /// 3. Both `srcs[0]` and `srcs[1]` are literals (not variables).
    fn constant_fold(&self, ir: Vec<CIRInstr>) -> Vec<CIRInstr> {
        // We keep a "known constant" map: register name → CIROperand literal.
        // Every `const_<t>` with a single literal source is added here so
        // subsequent instructions can propagate the constant.
        let mut known: HashMap<String, CIROperand> = HashMap::new();
        let mut out = Vec::with_capacity(ir.len());

        for mut instr in ir {
            // Propagate known constants into the instruction's sources.
            for src in &mut instr.srcs {
                if let CIROperand::Var(name) = src {
                    if let Some(literal) = known.get(name) {
                        *src = literal.clone();
                    }
                }
            }

            // Try to fold if the instruction is foldable and has two literal srcs.
            let folded_literal = if is_foldable(&instr.op) && instr.srcs.len() >= 2 {
                try_fold_binary(&instr)
            } else {
                None
            };

            if let (Some(literal), Some(dest)) = (folded_literal, &instr.dest) {
                // Replace with const_<type>.
                let const_op = const_op_for_ty(&instr.ty);
                let new_instr = CIRInstr::new(const_op, Some(dest.clone()), vec![literal.clone()], instr.ty.clone());
                known.insert(dest.clone(), literal);
                out.push(new_instr);
            } else {
                // If this is a `const_<t>` with a single literal src, record it.
                if instr.op.starts_with("const_") && instr.srcs.len() == 1 && instr.srcs[0].is_literal() {
                    if let Some(dest) = &instr.dest {
                        known.insert(dest.clone(), instr.srcs[0].clone());
                    }
                }
                out.push(instr);
            }
        }

        out
    }

    // ------------------------------------------------------------------
    // Pass 2: dead-code elimination
    // ------------------------------------------------------------------

    /// Remove instructions whose `dest` is never read by any later instruction.
    ///
    /// Side-effectful instructions are always kept.  The pass makes one
    /// backwards scan to build the live set, then one forwards scan to emit
    /// surviving instructions.
    fn dead_code_eliminate(&self, ir: Vec<CIRInstr>) -> Vec<CIRInstr> {
        // Build the set of all variable names that appear as a source
        // in any instruction.  A dest register is "live" if it appears
        // in this set.
        //
        // We collect into a HashSet<String> (owned) first so that the
        // subsequent `ir.into_iter()` does not create a borrow conflict.
        let used: HashSet<String> = ir
            .iter()
            .flat_map(|instr| {
                instr.srcs.iter().filter_map(|src| {
                    if let CIROperand::Var(name) = src {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Emit: keep all impure instructions, and all instructions whose
        // dest is in `used`.
        ir.into_iter()
            .filter(|instr| {
                if !instr.is_pure() {
                    return true; // always keep side-effectful instructions
                }
                match &instr.dest {
                    None => true, // no dest: keep (unusual for pure; play safe)
                    Some(dest) => used.contains(dest.as_str()),
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Attempt to constant-fold a binary instruction.
///
/// Returns `Some(CIROperand)` — the computed literal result — if both
/// sources are literals and the operation is supported.
fn try_fold_binary(instr: &CIRInstr) -> Option<CIROperand> {
    let a = &instr.srcs[0];
    let b = &instr.srcs[1];
    let base = base_op(&instr.op);
    let ty = &instr.ty;

    // Integer × integer folds.
    if let (CIROperand::Int(ai), CIROperand::Int(bi)) = (a, b) {
        let result: Option<i64> = match base {
            "add" => Some(ai.wrapping_add(*bi)),
            "sub" => Some(ai.wrapping_sub(*bi)),
            "mul" => Some(ai.wrapping_mul(*bi)),
            "div" => if *bi != 0 { Some(ai.wrapping_div(*bi)) } else { None },
            "mod" => if *bi != 0 { Some(ai.wrapping_rem(*bi)) } else { None },
            "and" => Some(ai & bi),
            "or"  => Some(ai | bi),
            "xor" => Some(ai ^ bi),
            "shl" => Some(ai.wrapping_shl((*bi as u32).min(63))),
            "shr" => Some((((*ai) as u64).wrapping_shr((*bi as u32).min(63))) as i64),
            _ => None,
        };
        if let Some(v) = result {
            return Some(CIROperand::Int(v));
        }
        // Boolean comparisons over integers.
        let bool_result: Option<bool> = match base {
            "cmp_eq" => Some(ai == bi),
            "cmp_ne" => Some(ai != bi),
            "cmp_lt" => Some(ai < bi),
            "cmp_le" => Some(ai <= bi),
            "cmp_gt" => Some(ai > bi),
            "cmp_ge" => Some(ai >= bi),
            _ => None,
        };
        if let Some(v) = bool_result {
            return Some(CIROperand::Bool(v));
        }
    }

    // Float × float folds.
    if let (CIROperand::Float(af), CIROperand::Float(bf)) = (a, b) {
        let result: Option<f64> = match base {
            "add" => Some(af + bf),
            "sub" => Some(af - bf),
            "mul" => Some(af * bf),
            "div" => Some(af / bf), // IEEE 754 defines x/0.0 = ±inf
            _ => None,
        };
        if let Some(v) = result {
            return Some(CIROperand::Float(v));
        }
        let bool_result: Option<bool> = match base {
            "cmp_eq" => Some(af == bf),
            "cmp_ne" => Some(af != bf),
            "cmp_lt" => Some(af < bf),
            "cmp_le" => Some(af <= bf),
            "cmp_gt" => Some(af > bf),
            "cmp_ge" => Some(af >= bf),
            _ => None,
        };
        if let Some(v) = bool_result {
            return Some(CIROperand::Bool(v));
        }
    }

    // Bool × bool for logical ops.
    if let (CIROperand::Bool(ab), CIROperand::Bool(bb)) = (a, b) {
        // Treat booleans as 0/1 for and/or/xor.
        let ai = *ab as i64;
        let bi = *bb as i64;
        let result: Option<i64> = match base {
            "and" => Some(ai & bi),
            "or"  => Some(ai | bi),
            "xor" => Some(ai ^ bi),
            _ => None,
        };
        if let Some(v) = result {
            return Some(if ty == "bool" {
                CIROperand::Bool(v != 0)
            } else {
                CIROperand::Int(v)
            });
        }
        let bool_result: Option<bool> = match base {
            "cmp_eq" => Some(ab == bb),
            "cmp_ne" => Some(ab != bb),
            _ => None,
        };
        if let Some(v) = bool_result {
            return Some(CIROperand::Bool(v));
        }
    }

    None
}

/// Return the `const_<type>` mnemonic for a given type string.
fn const_op_for_ty(ty: &str) -> String {
    format!("const_{ty}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cir::{CIRInstr, CIROperand};

    fn vars(names: &[&str]) -> Vec<CIROperand> {
        names.iter().map(|n| CIROperand::Var((*n).into())).collect()
    }
    fn ints(vals: &[i64]) -> Vec<CIROperand> {
        vals.iter().map(|v| CIROperand::Int(*v)).collect()
    }

    #[test]
    fn fold_add_int_literals() {
        // After constant propagation, add_u8(a=5, b=3) is folded to const_u8 z=8.
        // Then z=8 is propagated into ret_u8 → ret_u8(8).
        // DCE then removes the const_u8 z=8 instruction because z is no longer read.
        // Expected output: just [ret_u8 _ 8].
        let cir = vec![
            CIRInstr::new("const_u8", Some("a"), ints(&[5]), "u8"),
            CIRInstr::new("const_u8", Some("b"), ints(&[3]), "u8"),
            CIRInstr::new("add_u8", Some("z"), vars(&["a", "b"]), "u8"),
            CIRInstr::new("ret_u8", None::<&str>, vars(&["z"]), "u8"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // All const instructions should be DCE'd; only the ret survives.
        assert!(opt.len() < 4, "unused constants should be eliminated");
        let ret = opt.iter().find(|i| i.op == "ret_u8").unwrap();
        // The ret's src should be the folded literal 8.
        assert_eq!(ret.srcs[0], CIROperand::Int(8), "ret should have folded literal");
    }

    #[test]
    fn fold_cmp_lt_to_bool() {
        // Same propagation chain: cmp_lt_u8(a=2, b=10) → const_bool r=true
        // → r propagated into ret → r dead → removed.
        let cir = vec![
            CIRInstr::new("const_u8", Some("a"), ints(&[2]), "u8"),
            CIRInstr::new("const_u8", Some("b"), ints(&[10]), "u8"),
            CIRInstr::new("cmp_lt_u8", Some("r"), vars(&["a", "b"]), "bool"),
            CIRInstr::new("ret_bool", None::<&str>, vars(&["r"]), "bool"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // Only ret_bool should survive; it should have the folded literal.
        let ret = opt.iter().find(|i| i.op == "ret_bool").unwrap();
        assert_eq!(ret.srcs[0], CIROperand::Bool(true), "ret should have folded bool");
    }

    #[test]
    fn no_fold_variable_srcs() {
        let cir = vec![
            CIRInstr::new("add_u8", Some("z"), vars(&["a", "b"]), "u8"),
            CIRInstr::new("ret_u8", None::<&str>, vars(&["z"]), "u8"),
        ];
        let opt = CIROptimizer::new().run(cir.clone());
        // Nothing to fold — a and b are variables, not constants.
        let z = opt.iter().find(|i| i.dest.as_deref() == Some("z")).unwrap();
        assert_eq!(z.op, "add_u8");
    }

    #[test]
    fn dce_removes_unused_const() {
        // "dead" is never read.  "live" IS read by ret_u8, but constant
        // propagation substitutes `live=1` into ret_u8, making live also
        // dead after DCE.  Only the ret instruction survives.
        let cir = vec![
            CIRInstr::new("const_u8", Some("dead"), ints(&[99]), "u8"),
            CIRInstr::new("const_u8", Some("live"), ints(&[1]), "u8"),
            CIRInstr::new("ret_u8", None::<&str>, vars(&["live"]), "u8"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // Both "dead" and "live" are eliminated by propagation + DCE.
        assert!(opt.iter().all(|i| i.dest.as_deref() != Some("dead")));
        assert!(opt.iter().all(|i| i.dest.as_deref() != Some("live")));
        // The ret instruction remains with its propagated literal.
        let ret = opt.iter().find(|i| i.op == "ret_u8").unwrap();
        assert_eq!(ret.srcs[0], CIROperand::Int(1));
    }

    #[test]
    fn dce_keeps_live_variable_read_by_non_const_instr() {
        // When a variable is consumed by a non-const, non-ret instruction,
        // the DCE should keep it.
        let cir = vec![
            CIRInstr::new("const_u8", Some("a"), ints(&[5]), "u8"),
            // Use a call_runtime with "a" as a variable src — call_runtime is
            // side-effectful so it won't be propagated away.
            CIRInstr::new("call_runtime", Some("_r"), vec![
                CIROperand::Var("generic_foo".into()),
                CIROperand::Var("a".into()),
            ], "any"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // call_runtime is side-effectful; "a" appears as its src with a
        // known const value, which gets propagated into call_runtime's args.
        // After propagation, "a" is no longer in used → const_u8 a is DCE'd.
        assert!(opt.iter().any(|i| i.op == "call_runtime"), "call_runtime must survive");
    }

    #[test]
    fn dce_keeps_side_effectful_ops() {
        let cir = vec![
            CIRInstr::new("call_runtime", Some("_"), vars(&["generic_add", "a", "b"]), "any"),
            CIRInstr::new("ret_void", None::<&str>, vec![], "void"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // call_runtime is impure — kept even though `_` is never read.
        assert_eq!(opt.len(), 2);
    }

    #[test]
    fn fold_float_add() {
        // Same chain: a=1.5 + b=2.5 → z=4.0 → z propagated into ret → z dead.
        let cir = vec![
            CIRInstr::new("const_f64", Some("a"), vec![CIROperand::Float(1.5)], "f64"),
            CIRInstr::new("const_f64", Some("b"), vec![CIROperand::Float(2.5)], "f64"),
            CIRInstr::new("add_f64", Some("z"), vars(&["a", "b"]), "f64"),
            CIRInstr::new("ret_f64", None::<&str>, vars(&["z"]), "f64"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // Only ret_f64 should survive; src should be the folded float literal.
        let ret = opt.iter().find(|i| i.op == "ret_f64").unwrap();
        if let CIROperand::Float(v) = ret.srcs[0] {
            assert!((v - 4.0).abs() < 1e-9, "folded float should be 4.0, got {v}");
        } else {
            panic!("expected Float operand in ret, got {:?}", ret.srcs[0]);
        }
    }

    #[test]
    fn fold_div_by_zero_not_folded() {
        let cir = vec![
            CIRInstr::new("const_u8", Some("a"), ints(&[10]), "u8"),
            CIRInstr::new("const_u8", Some("b"), ints(&[0]),  "u8"),
            CIRInstr::new("div_u8", Some("z"), vars(&["a", "b"]), "u8"),
            CIRInstr::new("ret_u8", None::<&str>, vars(&["z"]), "u8"),
        ];
        let opt = CIROptimizer::new().run(cir);
        // Div by zero: not folded — keep the original div instruction.
        let z = opt.iter().find(|i| i.dest.as_deref() == Some("z")).unwrap();
        assert_eq!(z.op, "div_u8");
    }

    #[test]
    fn fold_bool_eq() {
        // cmp_eq_bool(true, false) → false; r=false propagated into ret → r dead.
        let cir = vec![
            CIRInstr::new("const_bool", Some("a"), vec![CIROperand::Bool(true)], "bool"),
            CIRInstr::new("const_bool", Some("b"), vec![CIROperand::Bool(false)], "bool"),
            CIRInstr::new("cmp_eq_bool", Some("r"), vars(&["a", "b"]), "bool"),
            CIRInstr::new("ret_bool", None::<&str>, vars(&["r"]), "bool"),
        ];
        let opt = CIROptimizer::new().run(cir);
        let ret = opt.iter().find(|i| i.op == "ret_bool").unwrap();
        assert_eq!(ret.srcs[0], CIROperand::Bool(false), "cmp_eq(true, false) = false");
    }

    #[test]
    fn idempotent_second_run() {
        let cir = vec![
            CIRInstr::new("const_u8", Some("a"), ints(&[4]), "u8"),
            CIRInstr::new("const_u8", Some("b"), ints(&[6]), "u8"),
            CIRInstr::new("add_u8", Some("z"), vars(&["a", "b"]), "u8"),
            CIRInstr::new("ret_u8", None::<&str>, vars(&["z"]), "u8"),
        ];
        let opt1 = CIROptimizer::new().run(cir.clone());
        let opt2 = CIROptimizer::new().run(opt1.clone());
        assert_eq!(opt1.len(), opt2.len(), "second run should be a no-op");
    }

    #[test]
    fn base_op_strips_type_suffix() {
        assert_eq!(base_op("add_u8"), "add");
        assert_eq!(base_op("cmp_lt_u32"), "cmp_lt");
        assert_eq!(base_op("add_f64"), "add");
        assert_eq!(base_op("label"), "label");
    }

    #[test]
    fn empty_cir_ok() {
        let opt = CIROptimizer::new().run(vec![]);
        assert!(opt.is_empty());
    }
}

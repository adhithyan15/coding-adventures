//! # dynamic_arith — LANG-FULL E6d-2: dynamic integer arithmetic over `any`.
//!
//! ## What this pass does
//!
//! A dynamic (lisp) frontend emits arithmetic and comparison as
//! `call_builtin "+"/"-"/"*"/…` whose operands are **boxed** dynamic values
//! (`ref<any>` — the result of `car`, a parameter, another dynamic op) rather
//! than machine integers. The typed backends have no opcode for "add two boxed
//! values"; the `numeric.rs` pass only rewrites `+`→`add` when the operands are
//! already a concrete numeric type, and rejects the `any` case outright.
//!
//! This pass bridges that gap **structurally**, exactly like `cons` (heap.rs):
//! it expands each dynamic arithmetic `call_builtin` into
//!
//! ```text
//!   unbox a  → ia : i64      ;; only if `a` is a boxed ref<any>
//!   unbox b  → ib : i64      ;; only if `b` is a boxed ref<any>
//!   add  ia ib → s : i64     ;; the typed op every backend already lowers
//!   box  s   → dest : ref<any>   ;; re-box the machine result as a lisp value
//! ```
//!
//! Every op it emits — `unbox` / `add` / `box` — is one the code-gen backends
//! already run (the same ops `lower_heap_builtins` + the `dyn_repr` passes use
//! for `cons`/`car`), so **all five code-gen backends light up from this one
//! change**. A raw (already-unboxed) operand — an integer literal `const … :
//! i64` — is used directly, no spurious unbox.
//!
//! ## Value width
//!
//! The unboxed operands and the typed op use the **i64** machine width
//! uniformly (the spec's model): a raw literal atom is already `i64`, so a
//! mixed `(+ boxed 1)` needs no width juggling. The structural backends box
//! small integers as `i31ref` / `Integer` / boxed-int32 (a 32-bit payload), so
//! their `box`/`unbox` widen/narrow at the i64↔i32 boundary — a backend detail
//! kept out of this language-agnostic pass.
//!
//! ## Where it runs
//!
//! After `lower_heap_builtins` (so `car`'s result is a concrete `ref<any>`
//! `field_load`, and boxed operands are identifiable) and **before** the
//! `dyn_repr` boxing pass (which then treats the `box`ed result as any other
//! lisp value and unboxes it at the program's return boundary).
//!
//! ## Integer contract (layer 2)
//!
//! Layer 2 lowers the **integer** dynamic contract: operands are treated as
//! machine integers. A non-integer boxed operand (e.g. a cons) unboxes to a
//! garbage integer / traps in the backend's `unbox`, mirroring the E4/E5 bounds
//! traps — runtime mixed int/float dispatch is a later slice.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::IIRModule;
use std::collections::HashMap;

/// The reference type hint for a boxed "any lisp value".
const REF_ANY: &str = "ref<any>";
/// The machine integer width the unboxed operands and the typed op use.
const INT: &str = "i64";

/// Dynamic arithmetic / comparison builtins → the typed opcode they lower to.
/// Mirrors `numeric.rs`'s `NUMERIC_TABLE` for the binary operators, plus the
/// Scheme extended-division names (`quotient`/`remainder`/`modulo`) a lisp
/// frontend emits. Every listed op is a binary op the backends already lower.
const ARITH: &[(&str, &str)] = &[
    ("+", "add"),
    ("-", "sub"),
    ("*", "mul"),
    ("/", "div"),
    ("quotient", "div"),
    ("%", "mod"),
    ("remainder", "mod"),
    ("modulo", "mod"),
    ("=", "cmp_eq"),
    ("<", "cmp_lt"),
    ("<=", "cmp_le"),
    (">", "cmp_gt"),
    (">=", "cmp_ge"),
];

/// The typed opcode for a dynamic arithmetic builtin name, if it is one.
fn typed_op(name: &str) -> Option<&'static str> {
    ARITH.iter().find(|(n, _)| *n == name).map(|(_, op)| *op)
}

/// Whether a type hint denotes a *boxed* dynamic value that must be unboxed
/// before a typed op can consume it. `ref<any>` (a `car` result / dynamic op
/// result) and the still-abstract `any` both qualify; a concrete machine type
/// (`i64`, `i32`, …) does not.
fn is_boxed(hint: &str) -> bool {
    hint == REF_ANY || hint == "any"
}

/// Whether a typed op produces a boolean rather than an integer.
fn is_comparison(op: &str) -> bool {
    op.starts_with("cmp_")
}

/// Map each SSA destination (and parameter) to the type hint it was produced
/// with, so an operand's boxed-ness can be decided structurally.
fn producer_types(fn_: &IIRFunction) -> HashMap<String, String> {
    let mut m: HashMap<String, String> = HashMap::new();
    for (name, ty) in &fn_.params {
        m.insert(name.clone(), ty.clone());
    }
    for instr in &fn_.instructions {
        if let Some(dest) = &instr.dest {
            m.insert(dest.clone(), instr.type_hint.clone());
        }
    }
    m
}

/// Lower all dynamic arithmetic `call_builtin`s in `fn_` to `unbox`/typed-op/
/// `box` sequences. Rebuilds the instruction list (each op expands to up to
/// four instructions, so we cannot mutate in place — same technique as
/// `lower_heap_function`).
pub fn lower_dynamic_arith_function(fn_: &mut IIRFunction) {
    let types = producer_types(fn_);
    let old = std::mem::take(&mut fn_.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() * 2);
    // A monotonic suffix so the temporaries this pass introduces never collide.
    let mut counter = 0usize;

    for instr in old {
        // Only a binary arithmetic/comparison `call_builtin` is rewritten.
        let name = match (instr.op.as_str(), instr.srcs.first()) {
            ("call_builtin", Some(Operand::Var(n))) => n.clone(),
            _ => {
                out.push(instr);
                continue;
            }
        };
        let op = match typed_op(&name) {
            Some(op) => op,
            None => {
                out.push(instr);
                continue;
            }
        };
        // Exactly two argument operands (srcs[1], srcs[2]); anything else is
        // malformed — leave it for the validator to reject with full context.
        let (a, b) = match (instr.srcs.get(1), instr.srcs.get(2), instr.srcs.get(3)) {
            (Some(Operand::Var(a)), Some(Operand::Var(b)), None) => (a.clone(), b.clone()),
            _ => {
                out.push(instr);
                continue;
            }
        };
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => {
                out.push(instr);
                continue;
            }
        };

        // Unbox each boxed operand to a machine integer; pass a raw operand
        // (an unboxed literal / already-typed value) straight through.
        let mut unbox_operand = |v: &str, out: &mut Vec<IIRInstr>| -> String {
            let boxed = types.get(v).map(|t| is_boxed(t)).unwrap_or(false);
            if !boxed {
                return v.to_string();
            }
            counter += 1;
            let u = format!("{v}.unbox{counter}");
            out.push(IIRInstr::new(
                "unbox",
                Some(u.clone()),
                vec![Operand::Var(v.to_string())],
                INT,
            ));
            u
        };
        let ia = unbox_operand(&a, &mut out);
        let ib = unbox_operand(&b, &mut out);

        // The typed op writes a fresh machine-typed temporary; the original
        // dest name is kept for the `box` so downstream readers are unchanged.
        counter += 1;
        let s = format!("{dest}.raw{counter}");
        let op_ty = if is_comparison(op) { "bool" } else { INT };
        out.push(IIRInstr::new(
            op,
            Some(s.clone()),
            vec![Operand::Var(ia), Operand::Var(ib)],
            op_ty,
        ));

        // Re-box the machine result as a lisp value (`ref<any>`), preserving the
        // original destination name. The `dyn_repr` pass then treats it like
        // any other lisp value (unboxing it at the return boundary).
        out.push(IIRInstr::new("box", Some(dest), vec![Operand::Var(s)], REF_ANY));
    }

    fn_.instructions = out;
}

/// Module-level entry point: lower dynamic arithmetic in every function.
pub fn lower_dynamic_arith(module: &mut IIRModule) {
    for fn_ in &mut module.functions {
        lower_dynamic_arith_function(fn_);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn arith_fn(name: &str, a: (&str, &str), b: (&str, &str)) -> IIRFunction {
        // Two producing instructions establish the operand types, then a binary
        // `call_builtin`. `field_load` gives a `ref<any>` (boxed); `const` gives i64.
        let mk = |dest: &str, ty: &str| {
            if ty == "ref<any>" {
                IIRInstr::new("field_load", Some(dest.into()),
                    vec![Operand::Var("p".into()), Operand::Int(0)], ty)
            } else {
                IIRInstr::new("const", Some(dest.into()), vec![Operand::Int(1)], ty)
            }
        };
        IIRFunction::new(
            "main",
            vec![("p".into(), "ref<LispyPair>".into())],
            "any",
            vec![
                mk(a.0, a.1),
                mk(b.0, b.1),
                IIRInstr::new("call_builtin", Some("r".into()),
                    vec![Operand::Var(name.into()), Operand::Var(a.0.into()), Operand::Var(b.0.into())],
                    "any"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
            ],
        )
    }

    fn ops(f: &IIRFunction) -> Vec<String> {
        f.instructions.iter().map(|i| i.op.clone()).collect()
    }

    #[test]
    fn boxed_operand_is_unboxed_then_op_then_boxed() {
        // (+ boxed raw) → unbox boxed ; add ; box
        let mut f = arith_fn("+", ("x", "ref<any>"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f);
        assert_eq!(ops(&f), vec!["field_load", "const", "unbox", "add", "box", "ret"]);
        // The `add` consumes the unboxed operand and the raw one directly.
        let add = f.instructions.iter().find(|i| i.op == "add").unwrap();
        assert_eq!(add.type_hint, "i64");
        assert_eq!(add.srcs, vec![Operand::Var("x.unbox1".into()), Operand::Var("y".into())]);
        // The result is re-boxed under the original dest name.
        let boxed = f.instructions.iter().find(|i| i.op == "box").unwrap();
        assert_eq!(boxed.dest.as_deref(), Some("r"));
        assert_eq!(boxed.type_hint, "ref<any>");
    }

    #[test]
    fn both_boxed_operands_are_unboxed() {
        let mut f = arith_fn("*", ("x", "ref<any>"), ("y", "ref<any>"));
        lower_dynamic_arith_function(&mut f);
        assert_eq!(ops(&f), vec!["field_load", "field_load", "unbox", "unbox", "mul", "box", "ret"]);
    }

    #[test]
    fn comparison_lowers_to_cmp_and_boxes_a_bool() {
        let mut f = arith_fn("<", ("x", "ref<any>"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f);
        assert_eq!(ops(&f), vec!["field_load", "const", "unbox", "cmp_lt", "box", "ret"]);
        let cmp = f.instructions.iter().find(|i| i.op == "cmp_lt").unwrap();
        assert_eq!(cmp.type_hint, "bool");
    }

    #[test]
    fn raw_operands_are_not_unboxed() {
        // Both operands already machine ints — no unbox, still a typed op + box.
        let mut f = arith_fn("-", ("x", "i64"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f);
        assert_eq!(ops(&f), vec!["const", "const", "sub", "box", "ret"]);
    }

    #[test]
    fn non_arith_builtin_is_left_untouched() {
        let mut f = arith_fn("cons", ("x", "i64"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f);
        assert!(f.instructions.iter().any(|i| i.op == "call_builtin"));
        assert!(!f.instructions.iter().any(|i| i.op == "box"));
    }
}

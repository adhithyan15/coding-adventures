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
//!
//! ## The `any`-hint ambiguity (language-gated, like `dyn_repr.rs`)
//!
//! `ref<any>` is unambiguous: it is always a genuine boxed dynamic value
//! (a `car`/`cdr` result, a re-boxed dynamic-arithmetic result), in every
//! language. Bare `any` is not: McCarthy Lisp's parameters and lambda results
//! are genuinely tagged `LispyValue`s typed `any`, but Twig/Nib also stamp
//! `any` on every statically-unresolved parameter — which is passed as a raw,
//! unboxed machine `i64`, not a tagged word. `is_boxed` therefore only treats
//! bare `any` as boxed when the module's source language actually uses the
//! tagged-word value model (`dyn_repr.rs::is_lisp_language`) — the identical
//! gate `dyn_repr.rs` uses for the same ambiguity. Getting this wrong silently
//! corrupts every Twig comparison/arithmetic op with a parameter operand (the
//! unbox right-shifts the raw value by 3 before the typed op runs).

use crate::dyn_repr::is_lisp_language;
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
/// before a typed op can consume it.
///
/// `ref<any>` is **always** boxed — it is a heap-typed dynamic value (a
/// `car`/`cdr` result, a re-boxed dynamic-arithmetic result), never a
/// placeholder, regardless of source language.
///
/// Bare `any` is ambiguous and — exactly like `dyn_repr.rs`'s identically-named
/// seed rule (`lower_dyn_repr_function`'s `hint == REF_ANY || (is_lisp &&
/// hint == ANY_HINT)`) — must be gated on the module's source language: inside
/// a genuinely dynamic (lisp) module `any` means a real tagged `LispyValue`
/// (a lambda result, a lisp parameter); in Twig/Nib it is only a *pre-
/// resolution placeholder* on an ordinary machine value (every untyped Twig
/// function parameter is declared `any` but passed as a raw, unboxed `i64`).
/// Treating it as boxed unconditionally — the previous behaviour here —
/// corrupted every Twig comparison/arithmetic op with a parameter operand:
/// `(< n 2)` right-shifted `n` by 3 before comparing, so e.g. `n = 10` read
/// back as `1` and silently miscompared. `dyn_repr.rs` and this pass must
/// agree on what `any` means, so this reuses its `is_lisp_language` rather
/// than re-deriving it.
fn is_boxed(hint: &str, is_lisp: bool) -> bool {
    hint == REF_ANY || (is_lisp && hint == "any")
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
///
/// `is_lisp` gates whether a bare `any`-typed operand counts as boxed — see
/// `is_boxed`'s doc comment. Callers with a whole `IIRModule` should go
/// through `lower_dynamic_arith`, which derives this from `module.language`.
pub fn lower_dynamic_arith_function(fn_: &mut IIRFunction, is_lisp: bool) {
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
            let boxed = types.get(v).map(|t| is_boxed(t, is_lisp)).unwrap_or(false);
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
    let is_lisp = is_lisp_language(&module.language);
    for fn_ in &mut module.functions {
        lower_dynamic_arith_function(fn_, is_lisp);
    }
}

/// The runtime-call names the tagged-i64 world uses for box / unbox.
const BOX_INT_BUILTIN: &str = "dyn_box_int";
const UNBOX_INT_BUILTIN: &str = "dyn_unbox_int";

/// **E6d-2b — the tagged-i64 (native / LLVM) representation of `box` / `unbox`.**
///
/// `lower_dynamic_arith` emits the *generic* `box` / `unbox` IIR ops — the same
/// ops `cons`/`car` use — because the **structural** backends (WASM `i31ref`,
/// JVM `Integer`, CLR boxed-int32) lower them directly. The **tagged-i64** world
/// (native aarch64/x86_64 + LLVM) has no such opcode: a tagged word is
/// `n << 3`, produced/consumed by the runtime helpers `__dyn_box_int` /
/// `__dyn_unbox_int`. This pass rewrites each residual
///
/// ```text
///   unbox v   → u : i64        ⇒   call_builtin "dyn_unbox_int", v  → u : i64
///   box   s   → d : ref<any>   ⇒   call_builtin "dyn_box_int",   s  → d : ref<any>
/// ```
///
/// so `iir-to-llvm` (via its `DYN_BUILTINS` table) and the native backends (via
/// `V1_BUILTINS`) emit a `call/bl __dyn_box_int` / `__dyn_unbox_int`.
///
/// It runs **only on the native / LLVM pipeline** — never before the structural
/// pass, which keeps the generic ops. A `box` of a compile-time integer constant
/// is already boxed inline (`n << 3`) by `lower_dyn_repr` and never reaches here;
/// this handles the *dynamic* case (a machine-typed arithmetic result).
pub fn lower_box_unbox_to_runtime_calls(module: &mut IIRModule) {
    for fn_ in &mut module.functions {
        for instr in &mut fn_.instructions {
            let builtin = match instr.op.as_str() {
                "box" => BOX_INT_BUILTIN,
                "unbox" => UNBOX_INT_BUILTIN,
                _ => continue,
            };
            // `box`/`unbox` are unary: srcs == [value]. Rewrite to
            // `call_builtin "<builtin>", value`, preserving dest + type_hint.
            let value = match instr.srcs.first() {
                Some(v) => v.clone(),
                None => continue,
            };
            instr.op = "call_builtin".to_string();
            instr.srcs = vec![Operand::Var(builtin.to_string()), value];
        }
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
        lower_dynamic_arith_function(&mut f, true);
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
        lower_dynamic_arith_function(&mut f, true);
        assert_eq!(ops(&f), vec!["field_load", "field_load", "unbox", "unbox", "mul", "box", "ret"]);
    }

    #[test]
    fn comparison_lowers_to_cmp_and_boxes_a_bool() {
        let mut f = arith_fn("<", ("x", "ref<any>"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f, true);
        assert_eq!(ops(&f), vec!["field_load", "const", "unbox", "cmp_lt", "box", "ret"]);
        let cmp = f.instructions.iter().find(|i| i.op == "cmp_lt").unwrap();
        assert_eq!(cmp.type_hint, "bool");
    }

    #[test]
    fn raw_operands_are_not_unboxed() {
        // Both operands already machine ints — no unbox, still a typed op + box.
        let mut f = arith_fn("-", ("x", "i64"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f, true);
        assert_eq!(ops(&f), vec!["const", "const", "sub", "box", "ret"]);
    }

    #[test]
    fn non_arith_builtin_is_left_untouched() {
        let mut f = arith_fn("cons", ("x", "i64"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f, true);
        assert!(f.instructions.iter().any(|i| i.op == "call_builtin"));
        assert!(!f.instructions.iter().any(|i| i.op == "box"));
    }

    /// A function taking a bare-`any`-typed parameter `n` (never a
    /// `field_load`/`ref<any>` value — exactly how every untyped Twig
    /// function parameter is declared) compared against an `i64` literal.
    /// Mirrors Twig `(define (classify n) (if (< n 2) 111 222))`.
    fn any_param_cmp_fn(op: &str) -> IIRFunction {
        IIRFunction::new(
            "classify",
            vec![("n".into(), "any".into())],
            "any",
            vec![
                IIRInstr::new("const", Some("two".into()), vec![Operand::Int(2)], "i64"),
                IIRInstr::new(
                    "call_builtin",
                    Some("r".into()),
                    vec![Operand::Var(op.into()), Operand::Var("n".into()), Operand::Var("two".into())],
                    "any",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
            ],
        )
    }

    /// The regression this fix closes: a Twig-language module's bare-`any`
    /// parameter must NOT be treated as a boxed dynamic value — it is a raw,
    /// unboxed machine `i64` passed straight through the calling convention.
    /// Before the fix, `lower_dynamic_arith` inserted a spurious `unbox` here,
    /// corrupting every comparison against a function parameter (the bug this
    /// whole investigation traced: `(< n 2)` with `n = 10` misread as `1<2`).
    #[test]
    fn twig_any_param_operand_is_not_unboxed() {
        let mut module = IIRModule::new("m", "twig");
        module.functions.push(any_param_cmp_fn("<"));
        lower_dynamic_arith(&mut module);
        let f = &module.functions[0];
        assert_eq!(
            ops(f),
            vec!["const", "cmp_lt", "box", "ret"],
            "a Twig bare-any parameter must flow into the typed op unboxed"
        );
        let cmp = f.instructions.iter().find(|i| i.op == "cmp_lt").unwrap();
        assert_eq!(
            cmp.srcs,
            vec![Operand::Var("n".into()), Operand::Var("two".into())],
            "cmp_lt must consume the raw parameter, not an unboxed copy"
        );
    }

    /// The case the fix must NOT break: a McCarthy-Lisp-language module's
    /// bare-`any` parameter genuinely IS a tagged `LispyValue` (McCarthy has
    /// no machine arithmetic at all) and must still be unboxed before a typed
    /// comparison/arithmetic op can consume it.
    #[test]
    fn mccarthy_any_param_operand_is_still_unboxed() {
        let mut module = IIRModule::new("m", "mccarthy-lisp");
        module.functions.push(any_param_cmp_fn("<"));
        lower_dynamic_arith(&mut module);
        let f = &module.functions[0];
        assert_eq!(
            ops(f),
            vec!["const", "unbox", "cmp_lt", "box", "ret"],
            "a McCarthy bare-any parameter is a tagged value and must still be unboxed"
        );
    }

    /// E6d-2b: `lower_box_unbox_to_runtime_calls` turns the generic `box`/`unbox`
    /// ops (which `lower_dynamic_arith` emits) into `dyn_box_int`/`dyn_unbox_int`
    /// runtime `call_builtin`s for the tagged-i64 backends, preserving dest,
    /// operand, and type hint.
    #[test]
    fn box_unbox_ops_become_runtime_calls() {
        // (+ boxed raw): field_load ; const ; unbox ; add ; box ; ret
        let mut f = arith_fn("+", ("x", "ref<any>"), ("y", "i64"));
        lower_dynamic_arith_function(&mut f, true);
        let mut module = IIRModule::new("m", "mccarthy-lisp");
        module.functions.push(f);
        lower_box_unbox_to_runtime_calls(&mut module);
        let f = &module.functions[0];
        // No generic box/unbox ops remain.
        assert!(!f.instructions.iter().any(|i| i.op == "box" || i.op == "unbox"));
        // The unbox became `call_builtin "dyn_unbox_int", <boxed operand>` : i64.
        let unbox = f
            .instructions
            .iter()
            .find(|i| i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("dyn_unbox_int".into())))
            .expect("unbox → dyn_unbox_int call");
        assert_eq!(unbox.type_hint, "i64");
        assert_eq!(unbox.srcs.len(), 2, "dyn_unbox_int is unary (builtin name + 1 operand)");
        // The box became `call_builtin "dyn_box_int", <raw result>` : ref<any>,
        // keeping the original destination register.
        let boxed = f
            .instructions
            .iter()
            .find(|i| i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("dyn_box_int".into())))
            .expect("box → dyn_box_int call");
        assert_eq!(boxed.type_hint, "ref<any>");
        assert_eq!(boxed.dest.as_deref(), Some("r"));
    }
}

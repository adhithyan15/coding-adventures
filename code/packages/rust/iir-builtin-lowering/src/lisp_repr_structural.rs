//! # lisp_repr_structural — lisp-value representation for the *managed* backends (LANG77 / L3b-3a-3c).
//!
//! ## The problem this solves
//!
//! `lower_heap_builtins` turns `cons`/`car`/`cdr` into the **structural** heap
//! form — `alloc` / `field_store` / `field_load` over a `ref<LispyPair>` — which
//! the managed backends (wasm/jvm/clr/beam) materialise as a real GC object (on
//! wasm, a `$LispyPair` struct whose two fields are `anyref`). But the *atoms*
//! stored into those fields are still **raw machine integers** (`const Int : i64`),
//! and the program's result is still typed `any`. A managed backend can't accept
//! either: you cannot store an `i64` into an `anyref` field, and `any` is not a
//! concrete type.
//!
//! This pass is the managed-backend twin of [`crate::lower_lisp_repr`] (which
//! does the same job for the *native* NaN-box runtime, boxing with `n << 3`).
//! Here the value model is **uniform-anyref**: every lisp value is a WasmGC
//! `anyref`; a small integer atom is boxed as an `i31ref` (`box` → `ref.i31`),
//! and unboxed back to a machine integer (`unbox` → `i31.get_s`) at the one
//! boundary where a lisp value re-enters the machine world — the program's
//! return value.
//!
//! ## The rule (use-site directed, gate-free)
//!
//! Exactly like the native pass, the decision of *what to box* is structural,
//! never a per-language switch:
//!
//! - An integer atom **stored into a cons field** (`field_store`'s value
//!   operand) is **boxed**: we insert `box %b = %atom : ref<any>` and rewrite
//!   the store to use `%b`. The atom's `const` is narrowed to `i32` first,
//!   because `ref.i31` boxes a 31-bit payload (a value outside the `i31` range
//!   would need a heap bignum — the same limitation the native path has at
//!   `±2⁶⁰`).
//! - A value that is **already a reference** (the result of `alloc` /
//!   `field_load` / a nested `box`, or a `ref<…>`-typed const) is left alone —
//!   it is already an `anyref`.
//! - In the **entry function**, a `ret %r` whose `%r` is a reference is unboxed:
//!   `unbox %u = %r : i32 ; ret %u`, and the function's return type becomes
//!   `i32` (the machine exit code). Non-entry functions return lisp values to
//!   their callers and stay `ref<any>`.
//!
//! A function that touches no heap op is left entirely to
//! `concretize_scalar_any_for_wasm` (which retypes its `any` → `i64`); the two
//! passes partition the module — this one owns the heap functions, that one owns
//! the pure-scalar functions — so every value ends up concretely typed.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::IIRModule;
use std::collections::{HashMap, HashSet};

/// The reference type hint for a cons cell (and the nil sentinel).
const REF_PAIR: &str = "ref<LispyPair>";
/// The reference type hint for "any lisp value" — a cons field / `car` result.
const REF_ANY: &str = "ref<any>";

/// The inclusive bounds of an `i31ref` payload (a 31-bit *signed* integer).
/// An atom outside this range cannot be boxed as an `i31ref`; it would need a
/// heap bignum object (not yet implemented — the same limitation the native
/// runtime has at `±2⁶⁰`). Such atoms are left unboxed; the backend then
/// rejects the type mismatch loudly rather than silently truncating.
const I31_MAX: i64 = (1 << 30) - 1;
const I31_MIN: i64 = -(1 << 30);

/// Apply the structural representation pass to every heap-using function.
///
/// Run by `lang-aot::compile_source_to_wasm` **after** `lower_heap_builtins`
/// (so cons/car/cdr are already `alloc`/`field_*`) and alongside
/// `concretize_scalar_any_for_wasm` (which handles the pure-scalar functions
/// this pass skips). Safe to run on any module: a function with no heap op is
/// left untouched.
pub fn lower_lisp_repr_structural(module: &mut IIRModule) {
    let entry = module.entry_point.clone();
    for func in &mut module.functions {
        if !function_uses_heap(func) {
            continue; // pure scalar — concretize_scalar_any_for_wasm owns it.
        }
        let is_entry = entry.as_deref() == Some(func.name.as_str());
        lower_structural_function(func, is_entry);
    }
}

/// Does this function touch the lisp heap / reference model? Mirrors the
/// `uses_heap` check in `concretize_scalar_any_for_wasm` so the two passes
/// partition the module's functions cleanly.
fn function_uses_heap(func: &IIRFunction) -> bool {
    func.instructions.iter().any(|i| {
        matches!(i.op.as_str(), "alloc" | "field_load" | "field_store" | "is_null")
            || i.type_hint.starts_with("ref<")
    })
}

/// The registers that already hold a **reference** (`anyref`) — and so must NOT
/// be boxed again: the results of `alloc`, `field_load`, `box`, and any
/// `ref<…>`-typed const (e.g. the nil sentinel).
fn reference_registers(func: &IIRFunction) -> HashSet<String> {
    let mut refs = HashSet::new();
    for instr in &func.instructions {
        let is_ref_producer = matches!(instr.op.as_str(), "alloc" | "field_load" | "box")
            || (instr.op == "const" && instr.type_hint.starts_with("ref<"));
        if is_ref_producer {
            if let Some(dest) = &instr.dest {
                refs.insert(dest.clone());
            }
        }
    }
    refs
}

fn lower_structural_function(func: &mut IIRFunction, is_entry: bool) {
    let ref_regs = reference_registers(func);

    // ── 1. Which atoms must be boxed? The value operand of every `field_store`
    //       that is not already a reference. ──
    let mut needs_box: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        if instr.op == "field_store" {
            if let Some(Operand::Var(val)) = instr.srcs.get(2) {
                if !ref_regs.contains(val) {
                    needs_box.insert(val.clone());
                }
            }
        }
    }

    // ── 2. Rebuild the body, narrowing boxable atom `const`s to i32 and
    //       inserting a `box` before the store that first consumes each. ──
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(func.instructions.len() + needs_box.len());
    let mut boxed: HashMap<String, String> = HashMap::new(); // atom reg → boxed reg
    for instr in std::mem::take(&mut func.instructions) {
        // Narrow an atom `const Int(n) : i64` that we are about to box to `i32`,
        // so `ref.i31` (which takes an i32) is well-typed. Out-of-range atoms are
        // left as-is (and will be rejected downstream, never silently truncated).
        if instr.op == "const" {
            if let (Some(dest), Some(Operand::Int(n))) = (&instr.dest, instr.srcs.first()) {
                if needs_box.contains(dest) && (I31_MIN..=I31_MAX).contains(n) {
                    let mut c = instr.clone();
                    c.type_hint = "i32".to_string();
                    new_instrs.push(c);
                    continue;
                }
            }
        }

        if instr.op == "field_store" {
            if let Some(Operand::Var(val)) = instr.srcs.get(2).cloned() {
                if needs_box.contains(&val) {
                    // Insert (once per atom) `box %b = %val : ref<any>`.
                    let boxed_reg = boxed.entry(val.clone()).or_insert_with(|| {
                        let b = format!("{val}.box");
                        new_instrs.push(IIRInstr::new(
                            "box",
                            Some(b.clone()),
                            vec![Operand::Var(val.clone())],
                            REF_ANY,
                        ));
                        b
                    });
                    let mut store = instr.clone();
                    store.srcs[2] = Operand::Var(boxed_reg.clone());
                    new_instrs.push(store);
                    continue;
                }
            }
        }

        new_instrs.push(instr);
    }
    func.instructions = new_instrs;

    // ── 3. The machine boundary: unbox the entry function's reference result. ──
    set_return_representation(func, is_entry, &ref_regs);

    // ── 4. Defensive sweep: no `any`/`polymorphic` hint may survive (the
    //       managed backends reject them). References are `ref<…>`, never
    //       `any`, so coercing a stray `any` to `i64` is safe. ──
    for instr in &mut func.instructions {
        if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
            instr.type_hint = "i64".to_string();
        }
    }
}

/// Rewrite the function's `ret` and `return_type` so the result is a concrete
/// machine type. In the entry function a returned **reference** is unboxed to
/// `i32` (the process exit code); a returned scalar keeps its width. A non-entry
/// function that returns a reference keeps it as `ref<any>` for its caller.
fn set_return_representation(func: &mut IIRFunction, is_entry: bool, ref_regs: &HashSet<String>) {
    // Find the (single) returned register, if any.
    let ret_pos = func.instructions.iter().position(|i| i.op == "ret");
    let Some(ret_pos) = ret_pos else { return };
    let ret_reg = match func.instructions[ret_pos].srcs.first() {
        Some(Operand::Var(r)) => r.clone(),
        _ => {
            // `ret` of an immediate / nothing — just concretise the return type.
            if func.return_type == "any" || func.return_type == "polymorphic" {
                func.return_type = "i64".to_string();
            }
            return;
        }
    };
    let returns_ref = ref_regs.contains(&ret_reg);

    if is_entry && returns_ref {
        // Unbox: `%u = unbox %ret_reg : i32 ; ret %u`.
        let unboxed = format!("{ret_reg}.unbox");
        func.instructions.insert(
            ret_pos,
            IIRInstr::new("unbox", Some(unboxed.clone()), vec![Operand::Var(ret_reg)], "i32"),
        );
        let ret = &mut func.instructions[ret_pos + 1];
        ret.srcs = vec![Operand::Var(unboxed)];
        ret.type_hint = "i32".to_string();
        func.return_type = "i32".to_string();
    } else if returns_ref {
        // Non-entry: hand the lisp value back to the caller as a reference.
        func.instructions[ret_pos].type_hint = REF_ANY.to_string();
        if func.return_type == "any" || func.return_type == "polymorphic" || func.return_type == REF_PAIR {
            func.return_type = REF_ANY.to_string();
        }
    } else if func.return_type == "any" || func.return_type == "polymorphic" {
        // A scalar result in a heap function (e.g. a predicate's i32) — i64.
        func.return_type = "i64".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::IIRModule;

    /// Build the post-`lower_heap_builtins` IIR for `(CAR (CONS 7 9))`:
    /// const 7:i64, const 9:i64, alloc, field_store×2, field_load, ret.
    fn cons_car_module() -> IIRModule {
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(7)], "i64"),
            IIRInstr::new("const", Some("v1".into()), vec![Operand::Int(9)], "i64"),
            {
                let mut a = IIRInstr::new("alloc", Some("v2".into()), vec![], REF_PAIR);
                a.may_alloc = true;
                a
            },
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("v2".into()), Operand::Int(0), Operand::Var("v0".into())],
                "void",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("v2".into()), Operand::Int(1), Operand::Var("v1".into())],
                "void",
            ),
            IIRInstr::new(
                "field_load",
                Some("v3".into()),
                vec![Operand::Var("v2".into()), Operand::Int(0)],
                REF_ANY,
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v3".into())], "any"),
        ];
        let f = IIRFunction::new("main", vec![], "any", instrs);
        let mut m = IIRModule::new("cons", "mccarthy-lisp");
        m.entry_point = Some("main".to_string());
        m.functions = vec![f];
        m
    }

    fn ops(f: &IIRFunction) -> Vec<&str> {
        f.instructions.iter().map(|i| i.op.as_str()).collect()
    }

    #[test]
    fn boxes_atoms_and_unboxes_the_result() {
        let mut m = cons_car_module();
        lower_lisp_repr_structural(&mut m);
        let f = &m.functions[0];

        // Two `box`es (for atoms 7 and 9) and one `unbox` (the result) appear.
        assert_eq!(ops(f).iter().filter(|o| **o == "box").count(), 2);
        assert_eq!(ops(f).iter().filter(|o| **o == "unbox").count(), 1);

        // The atom consts are narrowed to i32 (so ref.i31 is well-typed).
        for i in &f.instructions {
            if i.op == "const" {
                assert_eq!(i.type_hint, "i32", "boxable atom narrowed to i32");
            }
        }

        // The entry function now returns a concrete i32 (the unboxed atom).
        assert_eq!(f.return_type, "i32");

        // Each field_store now stores a boxed (ref) value, not the raw atom.
        for i in &f.instructions {
            if i.op == "field_store" {
                match &i.srcs[2] {
                    Operand::Var(v) => assert!(v.ends_with(".box"), "store uses boxed value"),
                    other => panic!("unexpected store value {other:?}"),
                }
            }
        }

        // No `any`/`polymorphic` hint survives.
        assert!(f.instructions.iter().all(|i| i.type_hint != "any" && i.type_hint != "polymorphic"));
    }

    #[test]
    fn unbox_immediately_precedes_ret() {
        let mut m = cons_car_module();
        lower_lisp_repr_structural(&mut m);
        let f = &m.functions[0];
        let pos = f.instructions.iter().position(|i| i.op == "ret").unwrap();
        assert_eq!(f.instructions[pos - 1].op, "unbox", "unbox must feed ret");
        // ret reads the unbox result.
        match &f.instructions[pos].srcs[0] {
            Operand::Var(v) => assert!(v.ends_with(".unbox")),
            other => panic!("unexpected ret operand {other:?}"),
        }
    }

    #[test]
    fn pure_scalar_function_is_untouched() {
        // A function with no heap op must be left entirely alone (owned by
        // concretize_scalar_any_for_wasm instead).
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "any"),
        ];
        let f = IIRFunction::new("main", vec![], "any", instrs);
        let mut m = IIRModule::new("scalar", "mccarthy-lisp");
        m.entry_point = Some("main".to_string());
        m.functions = vec![f];
        let before = m.functions[0].instructions.len();
        lower_lisp_repr_structural(&mut m);
        // No box/unbox inserted; return type untouched (concretize handles it).
        assert_eq!(m.functions[0].instructions.len(), before);
        assert!(m.functions[0].instructions.iter().all(|i| i.op != "box" && i.op != "unbox"));
    }

    #[test]
    fn out_of_range_atom_is_not_narrowed_or_boxed_as_i31() {
        // An atom outside the i31 range must NOT be silently narrowed; it stays
        // i64 (and will be rejected downstream rather than truncated).
        let big = (1i64 << 40) + 1;
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(big)], "i64"),
            {
                let mut a = IIRInstr::new("alloc", Some("v2".into()), vec![], REF_PAIR);
                a.may_alloc = true;
                a
            },
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("v2".into()), Operand::Int(0), Operand::Var("v0".into())],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v2".into())], "any"),
        ];
        let f = IIRFunction::new("main", vec![], "any", instrs);
        let mut m = IIRModule::new("big", "mccarthy-lisp");
        m.entry_point = Some("main".to_string());
        m.functions = vec![f];
        lower_lisp_repr_structural(&mut m);
        let f = &m.functions[0];
        // The big const is still i64 (not narrowed). A box is still inserted
        // (the store needs a ref), but the const width is preserved so the
        // backend can detect the unboxable atom rather than silently lose bits.
        let c = f.instructions.iter().find(|i| i.op == "const").unwrap();
        assert_eq!(c.type_hint, "i64", "out-of-range atom keeps its width");
    }
}

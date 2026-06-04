//! # lisp_repr — type-directed lisp-value representation for native AOT (LANG77 / L3b-2c).
//!
//! ## The problem this solves
//!
//! `lispy-runtime`'s value model is **NaN-box tagged**: an integer is
//! `(n << 3)` with the low 3 bits `000`, a heap cons cell is a pointer with
//! the low 3 bits `111`, nil is the whole word `001`, and so on. The tag is
//! what lets `pair?`/`ATOM`/`EQ` tell an integer from a pair.
//!
//! L3b-2b routed `cons`/`car`/`cdr` to the C runtime but left integer
//! *payloads* as **raw machine words**. That is fine for pure data movement
//! (car/cdr just copy bits), but it is a landmine for the predicates landing
//! in L3b-2c: a raw integer `7` is `0b…111` in its low 3 bits — exactly the
//! **heap tag** — so `pair?(7)` would answer "yes, a pair" and dereference
//! `7` as a pointer. Integers destined for lisp positions must be **boxed**
//! (`n << 3`) so their tag is `000`.
//!
//! ## Why not just box every integer?
//!
//! Because the native backend is **shared**. A Twig/Nib program does machine
//! arithmetic on raw `i64`s (`30 + 12`), and a runtime helper like
//! `print_i64` expects a raw word. Boxing those would corrupt them. So the
//! decision of *what to box* must be **type-directed**, not a per-language
//! switch (there is deliberately no `if language == "mccarthy-lisp"` here).
//!
//! ## The rule (use-site directed, gate-free)
//!
//! A value is a **tagged `LispyValue`** exactly when it participates in the
//! lisp value model. We detect that structurally:
//!
//! - A `const Int(n) : i64` is **boxed** (`n << 3`) iff its register is used
//!   as an **argument to a lisp builtin** (`lispy_cons`/`lispy_car`/
//!   `lispy_cdr`, and the predicates added in later slices). A McCarthy
//!   program — which has no machine arithmetic at all — feeds every integer
//!   into `cons`, so all its integers box. A Twig arithmetic program feeds
//!   integers into `add`/`print_i64`, never a `lispy_*` call, so none box.
//! - A `const Int(0) : ref<LispyPair>` (the nil sentinel emitted by the
//!   frontend) becomes the **nil tag** `0b001`. Only lisp frontends ever
//!   emit that type hint, so this is unambiguous.
//! - A register **holds a boxed `LispyValue`** if it is the result of a lisp
//!   builtin (`lispy_cons`/`car`/`cdr`) or a boxed constant.
//!
//! ## The machine boundary: unbox at program exit
//!
//! A boxed value must be **unboxed** wherever it re-enters the machine world.
//! In McCarthy 1.0 (no arithmetic) the only such boundary is the **program's
//! result**: the entry function returns a `LispyValue`, but the process exit
//! code is a raw integer. So in the **entry function only**, a `ret %x` whose
//! `%x` is a boxed `LispyValue` becomes `%u = lispy_unbox_int(%x); ret %u`.
//! Non-entry functions return `LispyValue`s to their callers and are left
//! tagged. (`box(n)` then `unbox` at the same boundary is the identity, so a
//! program like `42` — whose constant never reaches a `lispy_*` call, hence
//! is never boxed, hence is never unboxed — still exits `42`.)
//!
//! ## Scope of this slice (L3b-2c-1)
//!
//! `cons`/`car`/`cdr` + boxing + nil-tag + unbox-at-exit. The predicates
//! (`pair?`/`not`/`equal?` = `ATOM`/`EQ`) and `COND` truthiness on tagged
//! booleans land in L3b-2c-2 — they extend [`LISP_BUILTINS`] and add the
//! boolean-result handling; the representation laid down here is what they
//! build on. Symbols (string-literal emission for `make_symbol`) are L3b-2c-3.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::IIRModule;
use std::collections::HashSet;

/// The frontend type hint for a reference to a lisp pair (and the nil
/// sentinel). Mirrors `mccarthy-lisp-iir-compiler`'s `REF_PAIR`.
const REF_PAIR: &str = "ref<LispyPair>";

/// The nil singleton's whole-word value (`lispy-runtime` `TAG_NIL = 0b001`).
const TAG_NIL: i64 = 1;

/// Boxable integer range — `lispy-runtime`'s immediate-int range (±2⁶⁰).
/// Outside this, `n << 3` would lose the sign bit; such values need a future
/// bignum heap object, so they are left unboxed (a pre-existing limitation
/// shared with the VM/JIT runtime).
const INT_MAX_BOXABLE: i64 = (1 << 60) - 1;
const INT_MIN_BOXABLE: i64 = -(1 << 60);

/// The native lisp-value builtins whose arguments are tagged `LispyValue`s.
/// L3b-2c-1 covers the cons data path; later slices extend this with
/// `lispy_pair_p`/`lispy_not`/`lispy_equal`/`lispy_make_symbol`.
const LISP_BUILTINS: &[&str] = &["lispy_cons", "lispy_car", "lispy_cdr"];

/// The unbox helper (`__twig_lispy_unbox_int`, arithmetic `>> 3`).
const UNBOX_BUILTIN: &str = "lispy_unbox_int";

/// If `instr` is `call_builtin "<lisp builtin>"`, return that name.
fn lisp_builtin_name(instr: &IIRInstr) -> Option<&str> {
    if instr.op != "call_builtin" {
        return None;
    }
    match instr.srcs.first() {
        Some(Operand::Var(name)) if LISP_BUILTINS.contains(&name.as_str()) => Some(name.as_str()),
        _ => None,
    }
}

/// Apply the representation pass to every function in `module`.
///
/// Runs in `twig-aot::prepare_module_for_aot` **after**
/// `lower_heap_builtins_runtime` (so cons/car/cdr are already `lispy_*`
/// calls). Safe to run on any module: a program with no `lispy_*` calls (every
/// Twig/Nib/Brainfuck program) has nothing to box and is left unchanged.
pub fn lower_lisp_repr(module: &mut IIRModule) {
    let entry = module.entry_point.clone();
    for func in &mut module.functions {
        let is_entry = entry.as_deref() == Some(func.name.as_str());
        lower_lisp_repr_function(func, is_entry);
    }
}

fn lower_lisp_repr_function(func: &mut IIRFunction, is_entry: bool) {
    // ── 1. Which registers feed a lisp builtin? Their constants get boxed. ──
    let mut lisp_arg_regs: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        if lisp_builtin_name(instr).is_some() {
            // srcs[0] is the builtin name; srcs[1..] are the value arguments.
            for src in instr.srcs.iter().skip(1) {
                if let Operand::Var(v) = src {
                    lisp_arg_regs.insert(v.clone());
                }
            }
        }
    }

    // ── 2. Box the qualifying integer constants; tag the nil sentinel. ──
    //
    // `boxed_regs` tracks every register now holding a tagged `LispyValue`,
    // seeded with the constants we box here and extended in step 3 with the
    // results of lisp builtins.
    let mut boxed_regs: HashSet<String> = HashSet::new();
    for instr in &mut func.instructions {
        if instr.op != "const" {
            continue;
        }
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        let n = match instr.srcs.first() {
            Some(Operand::Int(n)) => *n,
            _ => continue,
        };

        if instr.type_hint == REF_PAIR && n == 0 {
            // The nil sentinel: 0 → TAG_NIL (0b001).
            instr.srcs[0] = Operand::Int(TAG_NIL);
            boxed_regs.insert(dest);
        } else if lisp_arg_regs.contains(&dest)
            && (INT_MIN_BOXABLE..=INT_MAX_BOXABLE).contains(&n)
        {
            // A lisp-typed integer atom: box it (n << 3, tag 0b000).
            instr.srcs[0] = Operand::Int(n << 3);
            boxed_regs.insert(dest);
        }
    }

    // ── 3. Results of lisp builtins are tagged LispyValues too. ──
    for instr in &func.instructions {
        if lisp_builtin_name(instr).is_some() {
            if let Some(dest) = &instr.dest {
                boxed_regs.insert(dest.clone());
            }
        }
    }

    // ── 4. Unbox at the program-exit boundary (entry function only). ──
    if is_entry {
        insert_unbox_before_lisp_rets(func, &boxed_regs);
    }
}

/// In the entry function, rewrite each `ret %x` whose `%x` holds a boxed
/// `LispyValue` into `%u = lispy_unbox_int(%x); ret %u`, so the process exit
/// code is the raw integer value rather than the tagged word.
fn insert_unbox_before_lisp_rets(func: &mut IIRFunction, boxed_regs: &HashSet<String>) {
    // Nothing returns a boxed value → no work (avoids the Vec rebuild).
    let needs_unbox = func.instructions.iter().any(|i| {
        i.op == "ret"
            && matches!(i.srcs.first(), Some(Operand::Var(v)) if boxed_regs.contains(v))
    });
    if !needs_unbox {
        return;
    }

    let old = std::mem::take(&mut func.instructions);
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    let mut unbox_counter = 0usize;

    for instr in old {
        let ret_boxed_reg = if instr.op == "ret" {
            match instr.srcs.first() {
                Some(Operand::Var(v)) if boxed_regs.contains(v) => Some(v.clone()),
                _ => None,
            }
        } else {
            None
        };

        match ret_boxed_reg {
            Some(boxed) => {
                // %u = call_builtin "lispy_unbox_int", %boxed  : i64
                let u_reg = format!("__unbox_{unbox_counter}");
                unbox_counter += 1;
                new_instrs.push(IIRInstr::new(
                    "call_builtin",
                    Some(u_reg.clone()),
                    vec![Operand::Var(UNBOX_BUILTIN.to_string()), Operand::Var(boxed)],
                    "i64",
                ));
                // ret %u
                new_instrs.push(IIRInstr::new("ret", None, vec![Operand::Var(u_reg)], "i64"));
            }
            None => new_instrs.push(instr),
        }
    }

    func.instructions = new_instrs;
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRModule};

    fn module(instrs: Vec<IIRInstr>) -> IIRModule {
        let f = IIRFunction::new("main", vec![], "any", instrs);
        IIRModule {
            name: "test".into(),
            functions: vec![f],
            entry_point: Some("main".into()),
            language: "mccarthy-lisp".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    fn konst(dest: &str, n: i64, ty: &str) -> IIRInstr {
        IIRInstr::new("const", Some(dest.into()), vec![Operand::Int(n)], ty)
    }

    fn call_builtin(dest: Option<&str>, name: &str, args: &[&str], ty: &str) -> IIRInstr {
        let mut srcs = vec![Operand::Var(name.into())];
        srcs.extend(args.iter().map(|a| Operand::Var((*a).into())));
        IIRInstr::new("call_builtin", dest.map(Into::into), srcs, ty)
    }

    fn ret(reg: &str) -> IIRInstr {
        IIRInstr::new("ret", None, vec![Operand::Var(reg.into())], "any")
    }

    fn find_const(m: &IIRModule, dest: &str) -> i64 {
        for i in &m.functions[0].instructions {
            if i.op == "const" && i.dest.as_deref() == Some(dest) {
                if let Some(Operand::Int(n)) = i.srcs.first() {
                    return *n;
                }
            }
        }
        panic!("no const {dest}");
    }

    /// `(CAR (CONS 7 9))`: 7 and 9 feed lispy_cons → boxed; the car result is
    /// returned by the entry fn → unboxed.
    #[test]
    fn cons_car_boxes_ints_and_unboxes_result() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "lispy_cons", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("r"), "lispy_car", &["cell"], "any"),
            ret("r"),
        ]);
        lower_lisp_repr(&mut m);
        // 7 → 56, 9 → 72 (boxed: n << 3).
        assert_eq!(find_const(&m, "v0"), 7 << 3, "7 must box to 56");
        assert_eq!(find_const(&m, "v1"), 9 << 3, "9 must box to 72");
        // ret r → unbox r, ret %u.
        let instrs = &m.functions[0].instructions;
        let last = instrs.last().unwrap();
        assert_eq!(last.op, "ret");
        let unbox = &instrs[instrs.len() - 2];
        assert_eq!(unbox.op, "call_builtin");
        assert_eq!(unbox.srcs[0], Operand::Var("lispy_unbox_int".into()));
        assert_eq!(unbox.srcs[1], Operand::Var("r".into()));
        // ret now refers to the unbox result, not the boxed car.
        assert_ne!(last.srcs[0], Operand::Var("r".into()));
    }

    /// A bare integer `42`: the constant never reaches a `lispy_*` call, so it
    /// is not boxed, and the ret is not unboxed — exit 42 unchanged.
    #[test]
    fn scalar_int_is_left_raw() {
        let mut m = module(vec![konst("v0", 42, "i64"), ret("v0")]);
        lower_lisp_repr(&mut m);
        assert_eq!(find_const(&m, "v0"), 42, "scalar int must not be boxed");
        let last = m.functions[0].instructions.last().unwrap();
        assert_eq!(last.op, "ret");
        assert_eq!(last.srcs[0], Operand::Var("v0".into()), "ret must not be unboxed");
        assert_eq!(m.functions[0].instructions.len(), 2, "no unbox inserted");
    }

    /// A Twig-style arithmetic program: integers feed `add` (a machine op),
    /// never a `lispy_*` call, so nothing is boxed or unboxed.
    #[test]
    fn machine_arithmetic_is_untouched() {
        let mut m = module(vec![
            konst("a", 30, "i64"),
            konst("b", 12, "i64"),
            IIRInstr::new(
                "add",
                Some("s".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            ret("s"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(find_const(&m, "a"), 30);
        assert_eq!(find_const(&m, "b"), 12);
        assert_eq!(m.functions[0].instructions.len(), 4, "no unbox: result is machine-typed");
    }

    /// The nil sentinel (`Int(0) : ref<LispyPair>`) becomes TAG_NIL (1).
    #[test]
    fn nil_sentinel_becomes_tag_nil() {
        let mut m = module(vec![
            konst("n", 0, "ref<LispyPair>"),
            konst("v", 5, "i64"),
            call_builtin(Some("cell"), "lispy_cons", &["v", "n"], "ref<LispyPair>"),
            ret("cell"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(find_const(&m, "n"), TAG_NIL, "nil sentinel → 1");
        assert_eq!(find_const(&m, "v"), 5 << 3, "5 boxed → 40");
    }

    /// Only the entry function unboxes — a callee returning a LispyValue stays
    /// tagged for its caller.
    #[test]
    fn non_entry_function_is_not_unboxed() {
        let callee = IIRFunction::new(
            "helper",
            vec![],
            "any",
            vec![
                konst("v0", 7, "i64"),
                call_builtin(Some("cell"), "lispy_cons", &["v0", "v0"], "ref<LispyPair>"),
                ret("cell"),
            ],
        );
        let mut m = module(vec![konst("z", 0, "i64"), ret("z")]);
        m.functions.push(callee);
        lower_lisp_repr(&mut m);
        // helper's ret must NOT be rewritten to an unbox (it returns a pair).
        let helper = m.functions.iter().find(|f| f.name == "helper").unwrap();
        let last = helper.instructions.last().unwrap();
        assert_eq!(last.op, "ret");
        assert_eq!(last.srcs[0], Operand::Var("cell".into()));
    }

    /// End-to-end of the two native passes as `twig-aot` runs them: the
    /// frontend emits `call_builtin "cons"/"car"`; `lower_heap_builtins_runtime`
    /// renames them to `lispy_*`; then `lower_lisp_repr` boxes the atoms and
    /// unboxes the result. This is the exact `(CAR (CONS 7 9))` pipeline.
    #[test]
    fn composes_with_runtime_rename() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "cons", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("r"), "car", &["cell"], "any"),
            ret("r"),
        ]);
        // Pass 1: rename cons/car → lispy_cons/lispy_car.
        crate::heap::lower_heap_builtins_runtime(&mut m);
        // Pass 2: box atoms + unbox result.
        lower_lisp_repr(&mut m);

        assert_eq!(find_const(&m, "v0"), 7 << 3);
        assert_eq!(find_const(&m, "v1"), 9 << 3);
        let instrs = &m.functions[0].instructions;
        // cons/car are now lispy_*.
        assert_eq!(instrs[2].srcs[0], Operand::Var("lispy_cons".into()));
        assert_eq!(instrs[3].srcs[0], Operand::Var("lispy_car".into()));
        // The tail is: unbox(r) ; ret %unbox.
        let last = instrs.last().unwrap();
        assert_eq!(last.op, "ret");
        let unbox = &instrs[instrs.len() - 2];
        assert_eq!(unbox.srcs[0], Operand::Var("lispy_unbox_int".into()));
    }

    /// Out-of-range integers (beyond ±2⁶⁰) are left unboxed rather than
    /// silently truncated by the shift.
    #[test]
    fn out_of_range_int_is_not_boxed() {
        let huge = (1_i64 << 61) | 1;
        let mut m = module(vec![
            konst("v0", huge, "i64"),
            call_builtin(Some("cell"), "lispy_cons", &["v0", "v0"], "ref<LispyPair>"),
            ret("cell"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(find_const(&m, "v0"), huge, "out-of-range int must be left raw");
    }
}

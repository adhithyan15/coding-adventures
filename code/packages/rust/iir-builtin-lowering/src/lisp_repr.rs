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

/// The native lisp-value builtins whose arguments are tagged `LispyValue`s,
/// so an integer constant flowing into one must be boxed. L3b-2c-1 added the
/// cons data path; L3b-2c-2 adds the `ATOM`/`EQ` predicates (their integer
/// atoms — e.g. the `5` in `(ATOM 5)` or `(EQ 5 5)` — must box so their tag
/// is `000`, not the heap tag a raw int's low bits collide with).
const LISP_BUILTINS: &[&str] = &[
    "lispy_cons",
    "lispy_car",
    "lispy_cdr",
    "lispy_pair_p",
    "lispy_not",
    "lispy_equal",
];

/// The unbox helper (`__twig_lispy_unbox_int`, arithmetic `>> 3`).
const UNBOX_BUILTIN: &str = "lispy_unbox_int";

/// The truthiness helper (`__twig_lispy_truthy`): a tagged `LispyValue` → a
/// raw machine `0`/`1` (false iff `#f` or nil), so the backend's
/// `jmp_if_false` — which tests a raw word against zero — branches correctly
/// on a `COND` predicate that produced a tagged boolean.
const TRUTHY_BUILTIN: &str = "lispy_truthy";

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

/// Rename `call_builtin "not"` → `lispy_not` when its argument is the result
/// of a `lispy_*` builtin — the `ATOM` = `not(pair?)` shape. `not` is also a
/// numeric builtin (Twig's machine boolean-not), so this *type-directed* check
/// is what keeps the two apart: a Twig `not` (whose argument is a raw `cmp`
/// result, not a `lispy_*` value) is left untouched for the numeric lowering.
fn rename_lisp_not(func: &mut IIRFunction) {
    // Dests of the `lispy_*` builtins that produce tagged values (cons/car/cdr
    // and the predicates renamed by `lower_heap_builtins_runtime`).
    let mut lispy_results: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        if instr.op == "call_builtin" {
            if let Some(Operand::Var(name)) = instr.srcs.first() {
                if name.starts_with("lispy_") {
                    if let Some(dest) = &instr.dest {
                        lispy_results.insert(dest.clone());
                    }
                }
            }
        }
    }
    for instr in &mut func.instructions {
        if instr.op != "call_builtin" {
            continue;
        }
        let is_not = matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "not");
        let arg_is_lispy =
            matches!(instr.srcs.get(1), Some(Operand::Var(a)) if lispy_results.contains(a));
        if is_not && arg_is_lispy {
            instr.srcs[0] = Operand::Var("lispy_not".to_string());
        }
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
    // ── 0. Type-directed `not` → `lispy_not`. ──
    //
    // `not` is ambiguous: a *numeric* builtin (Twig's machine boolean-not) and
    // the second half of McCarthy's `ATOM` (= `not(pair?)`, a lisp not). The
    // unconditional rename pass can't tell them apart, so it leaves `not`
    // alone. Here we have enough context: rename `not` → `lispy_not` only when
    // its argument is the result of a `lispy_*` builtin (e.g. `lispy_pair_p`),
    // which is exactly the `ATOM` shape. Twig's `not` (arg is a raw `cmp`
    // result, not a `lispy_*` value) is left for the numeric lowering.
    rename_lisp_not(func);

    // ── 1. Registers that feed a lisp builtin (their integer atoms box). ──
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

    // ── 2. Classify which registers hold a tagged `LispyValue` (no mutation). ──
    //
    // Seeds:
    //   • a lisp-builtin result (`lispy_cons`/`car`/`cdr`/`pair_p`/`not`/`equal`),
    //   • the nil sentinel const (`Int(0) : ref<LispyPair>`),
    //   • an integer const that feeds a lisp builtin (a lisp atom).
    //
    // Then a **bidirectional** fixpoint over `mov` edges. `COND` funnels every
    // clause's value into one register with `mov result, <value>`, mixing e.g.
    // nil (tagged) with an integer-literal clause result. Both endpoints of a
    // `mov` must therefore agree: marking the literal tagged here makes step 3
    // box it, so the funnel register is *uniformly* tagged and the exit-unbox
    // is correct regardless of which clause ran. (For non-lisp modules nothing
    // is seeded, so nothing spreads — Twig/Nib are untouched.)
    let mut boxed_regs: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        if lisp_builtin_name(instr).is_some() {
            if let Some(dest) = &instr.dest {
                boxed_regs.insert(dest.clone());
            }
        } else if instr.op == "const" {
            if let (Some(dest), Some(Operand::Int(n))) = (&instr.dest, instr.srcs.first()) {
                let is_nil = instr.type_hint == REF_PAIR && *n == 0;
                if is_nil || lisp_arg_regs.contains(dest) {
                    boxed_regs.insert(dest.clone());
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for instr in &func.instructions {
            if instr.op != "mov" {
                continue;
            }
            if let (Some(dest), Some(Operand::Var(src))) = (&instr.dest, instr.srcs.first()) {
                if boxed_regs.contains(dest) != boxed_regs.contains(src) {
                    boxed_regs.insert(dest.clone());
                    boxed_regs.insert(src.clone());
                    changed = true;
                }
            }
        }
    }

    // ── 3. Box the integer/nil constant of every tagged register. ──
    for instr in &mut func.instructions {
        if instr.op != "const" {
            continue;
        }
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        if !boxed_regs.contains(&dest) {
            continue;
        }
        let n = match instr.srcs.first() {
            Some(Operand::Int(n)) => *n,
            _ => continue,
        };
        if instr.type_hint == REF_PAIR && n == 0 {
            // The nil sentinel: 0 → TAG_NIL (0b001).
            instr.srcs[0] = Operand::Int(TAG_NIL);
        } else if (INT_MIN_BOXABLE..=INT_MAX_BOXABLE).contains(&n) {
            // A lisp-typed integer atom: box it (n << 3, tag 0b000).
            instr.srcs[0] = Operand::Int(n << 3);
        }
        // Out-of-range: left raw (the documented ±2⁶⁰ limitation).
    }

    // ── 4. Normalise tagged conditions for `jmp_if_false` (all functions). ──
    //
    // A `COND` predicate evaluates to a tagged LispyValue (e.g. `#f` = 3 from
    // `ATOM`). The backend's `jmp_if_false` tests a *raw* word against zero, so
    // a bare `#f` (3 ≠ 0) would read as true. We wrap such a condition in
    // `lispy_truthy` (→ raw 0/1). Type-directed: only conditions that hold a
    // tagged value are wrapped, so a Twig `if` (whose condition is a raw `cmp`
    // result, not in `boxed_regs`) is left untouched. (A bare integer-literal
    // predicate that is not `mov`-connected to a tagged value stays raw and
    // unwrapped — correct for any non-zero literal; a literal `0` predicate is
    // a known minor corner, since raw 0 reads as false though lisp `0` is
    // truthy.)
    wrap_tagged_conditions(func, &boxed_regs);

    // ── 5. Unbox at the program-exit boundary (entry function only). ──
    if is_entry {
        insert_unbox_before_lisp_rets(func, &boxed_regs);
    }
}

/// Rewrite each `jmp_if_false %cond, label` whose `%cond` holds a tagged
/// `LispyValue` into `%t = lispy_truthy(%cond); jmp_if_false %t, label`, so the
/// branch tests lisp truthiness (false iff `#f`/nil) rather than the raw
/// tagged word. A condition not in `boxed_regs` (a raw machine bool from a
/// `cmp`, or an unboxed integer — non-zero, hence already truthy) is left
/// alone.
fn wrap_tagged_conditions(func: &mut IIRFunction, boxed_regs: &HashSet<String>) {
    let needs_wrap = func.instructions.iter().any(|i| {
        i.op == "jmp_if_false"
            && matches!(i.srcs.first(), Some(Operand::Var(v)) if boxed_regs.contains(v))
    });
    if !needs_wrap {
        return;
    }

    let old = std::mem::take(&mut func.instructions);
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    let mut truthy_counter = 0usize;

    for instr in old {
        // A `jmp_if_false` whose condition (srcs[0]) is a tagged LispyValue.
        let tagged_cond = if instr.op == "jmp_if_false" {
            match instr.srcs.first() {
                Some(Operand::Var(v)) if boxed_regs.contains(v) => Some(v.clone()),
                _ => None,
            }
        } else {
            None
        };

        match tagged_cond {
            Some(cond) => {
                let t_reg = format!("__truthy_{truthy_counter}");
                truthy_counter += 1;
                // %t = call_builtin "lispy_truthy", %cond  : i64
                new_instrs.push(IIRInstr::new(
                    "call_builtin",
                    Some(t_reg.clone()),
                    vec![Operand::Var(TRUTHY_BUILTIN.to_string()), Operand::Var(cond)],
                    "i64",
                ));
                // jmp_if_false %t, <label>  (preserve the original target operand)
                let mut jif = instr;
                jif.srcs[0] = Operand::Var(t_reg);
                new_instrs.push(jif);
            }
            None => new_instrs.push(instr),
        }
    }

    func.instructions = new_instrs;
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

    fn mov(dest: &str, src: &str) -> IIRInstr {
        IIRInstr::new("mov", Some(dest.into()), vec![Operand::Var(src.into())], "any")
    }

    fn jmp_if_false(cond: &str, label: &str) -> IIRInstr {
        IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond.into()), Operand::Var(label.into())],
            "void",
        )
    }

    /// Count instructions matching `op` with a given builtin name (or any).
    fn count_builtin(m: &IIRModule, name: &str) -> usize {
        m.functions[0]
            .instructions
            .iter()
            .filter(|i| {
                i.op == "call_builtin"
                    && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == name)
            })
            .count()
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

    // ── L3b-2c-2: ATOM/EQ predicates + COND truthiness ──────────────────

    /// `(ATOM 5)` lowers (after the rename) to `not(pair?(5))`. The `5` feeds
    /// `lispy_pair_p`, so it must box.
    #[test]
    fn predicate_arg_int_is_boxed() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "lispy_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "lispy_not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(find_const(&m, "v0"), 5 << 3, "ATOM's int atom must box");
    }

    /// A `jmp_if_false` whose condition is a tagged value (a predicate result)
    /// gets a `lispy_truthy` normaliser inserted before it.
    #[test]
    fn tagged_cond_is_wrapped_with_truthy() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "lispy_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "lispy_not", &["p"], "bool"),
            jmp_if_false("a", "L_next"),
            IIRInstr::new("label", None, vec![Operand::Var("L_next".into())], "void"),
            ret("a"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(count_builtin(&m, "lispy_truthy"), 1, "tagged cond must be wrapped");
        // The jmp_if_false now tests the truthy result, not the raw tagged bool.
        let jif = m.functions[0].instructions.iter().find(|i| i.op == "jmp_if_false").unwrap();
        assert!(
            matches!(&jif.srcs[0], Operand::Var(v) if v.starts_with("__truthy_")),
            "jmp_if_false must test the truthy result, got {:?}", jif.srcs[0],
        );
    }

    /// A raw machine condition (e.g. a Twig `cmp` result, not in boxed_regs)
    /// is left alone — no `lispy_truthy` wrap.
    #[test]
    fn raw_cond_is_not_wrapped() {
        let mut m = module(vec![
            konst("a", 1, "i64"),
            konst("b", 2, "i64"),
            IIRInstr::new(
                "eq",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            jmp_if_false("c", "L_next"),
            IIRInstr::new("label", None, vec![Operand::Var("L_next".into())], "void"),
            ret("c"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(count_builtin(&m, "lispy_truthy"), 0, "raw cmp cond must NOT be wrapped");
    }

    /// A COND result funnelled by `mov` from a lisp value (a `car` result)
    /// must be recognised as tagged and unboxed at the entry ret.
    #[test]
    fn mov_propagates_taggedness_for_unbox() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "lispy_cons", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("c"), "lispy_car", &["cell"], "any"),
            mov("result", "c"),
            ret("result"),
        ]);
        lower_lisp_repr(&mut m);
        // The car result flows through `mov result, c`; `result` is therefore a
        // tagged value and the entry ret must unbox it.
        assert_eq!(count_builtin(&m, "lispy_unbox_int"), 1, "mov'd lisp result must unbox");
        let last = m.functions[0].instructions.last().unwrap();
        assert_eq!(last.op, "ret");
        assert!(matches!(&last.srcs[0], Operand::Var(v) if v.starts_with("__unbox_")));
    }

    /// The COND mixing case: a clause's integer literal and the nil
    /// fallthrough both funnel into one `result` via `mov`. Bidirectional
    /// propagation must box the literal so `result` is uniformly tagged and
    /// unboxes correctly (else a run that yields `7` would unbox a raw 7 → 0).
    #[test]
    fn cond_mixed_literal_and_nil_both_box() {
        let mut m = module(vec![
            konst("seven", 7, "i64"),
            mov("result", "seven"),
            konst("nilv", 0, "ref<LispyPair>"),
            mov("result", "nilv"),
            ret("result"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(find_const(&m, "seven"), 7 << 3, "clause literal must box (mov-tied to nil)");
        assert_eq!(find_const(&m, "nilv"), TAG_NIL, "nil tagged");
        assert_eq!(count_builtin(&m, "lispy_unbox_int"), 1, "result must unbox at ret");
    }

    /// ATOM shape `not(pair?(x))`: the generic `not` whose arg is a `lispy_*`
    /// result is renamed to `lispy_not`.
    #[test]
    fn not_after_lispy_result_becomes_lispy_not() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "lispy_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(count_builtin(&m, "lispy_not"), 1, "ATOM's not must become lispy_not");
        assert_eq!(count_builtin(&m, "not"), 0);
    }

    /// Twig shape `not(<raw cmp result>)`: the `not` (also a numeric builtin)
    /// must be left untouched so the numeric lowering makes it a machine not.
    #[test]
    fn not_on_raw_value_is_left_for_numeric() {
        let mut m = module(vec![
            konst("a", 1, "i64"),
            konst("b", 2, "i64"),
            IIRInstr::new(
                "eq",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            call_builtin(Some("r"), "not", &["c"], "bool"),
            ret("r"),
        ]);
        lower_lisp_repr(&mut m);
        assert_eq!(count_builtin(&m, "not"), 1, "Twig's machine not must be left alone");
        assert_eq!(count_builtin(&m, "lispy_not"), 0);
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

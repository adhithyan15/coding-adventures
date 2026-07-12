//! # dyn_repr — type-directed lisp-value representation for native AOT (LANG77 / L3b-2c).
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
//!   as an **argument to a lisp builtin** (`dyn_cons`/`dyn_car`/
//!   `dyn_cdr`, and the predicates added in later slices). A McCarthy
//!   program — which has no machine arithmetic at all — feeds every integer
//!   into `cons`, so all its integers box. A Twig arithmetic program feeds
//!   integers into `add`/`print_i64`, never a `dyn_*` call, so none box.
//! - A `const Int(0) : ref<LispyPair>` (the nil sentinel emitted by the
//!   frontend) becomes the **nil tag** `0b001`. Only lisp frontends ever
//!   emit that type hint, so this is unambiguous.
//! - A register **holds a boxed `LispyValue`** if it is the result of a lisp
//!   builtin (`dyn_cons`/`car`/`cdr`) or a boxed constant.
//!
//! ## The machine boundary: unbox at program exit
//!
//! A boxed value must be **unboxed** wherever it re-enters the machine world.
//! In McCarthy 1.0 (no arithmetic) the only such boundary is the **program's
//! result**: the entry function returns a `LispyValue`, but the process exit
//! code is a raw integer. So in the **entry function only**, a `ret %x` whose
//! `%x` is a boxed `LispyValue` becomes `%u = dyn_unbox_int(%x); ret %u`.
//! Non-entry functions return `LispyValue`s to their callers and are left
//! tagged. (`box(n)` then `unbox` at the same boundary is the identity, so a
//! program like `42` — whose constant never reaches a `dyn_*` call, hence
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
/// The two type hints that denote a **boxed `DynValue`** — the language-neutral
/// tagged word.  `ref<any>` is a heap-typed dynamic value (a `dyn_car`/`dyn_cdr`
/// result, a re-boxed arithmetic result); `any` is the still-abstract dynamic
/// value (a user-lambda result, a dynamic parameter).  A register produced with
/// either hint holds a `DynValue` *by what it is*, independent of which
/// primitive produced it — the producer-agnostic classification of DVAL01 §3.3.
const REF_ANY: &str = "ref<any>";
const ANY_HINT: &str = "any";
/// Whether a type hint denotes a boxed `DynValue` (see [`REF_ANY`]).
fn is_dynvalue_hint(hint: &str) -> bool {
    hint == REF_ANY || hint == ANY_HINT
}

/// The type hint for a symbol literal. After `intern_symbols` runs, such a
/// const already holds the finished tagged immediate `(id<<32)|TAG_SYMBOL` —
/// it is a `LispyValue` (so it joins `boxed_regs`) but must NOT be boxed again
/// (shifting it would corrupt the id/tag).
const SYMBOL_HINT: &str = "symbol";

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
    "dyn_cons",
    "dyn_car",
    "dyn_cdr",
    "dyn_pair_p",
    "dyn_not",
    "dyn_equal",
];

/// The unbox helper (`__dyn_unbox_int`, arithmetic `>> 3`).
const UNBOX_BUILTIN: &str = "dyn_unbox_int";

/// The truthiness helper (`__dyn_truthy`): a tagged `LispyValue` → a
/// raw machine `0`/`1` (false iff `#f` or nil), so the backend's
/// `jmp_if_false` — which tests a raw word against zero — branches correctly
/// on a `COND` predicate that produced a tagged boolean.
const TRUTHY_BUILTIN: &str = "dyn_truthy";

/// The universal exit-coercion helper (`__dyn_to_exit_code`): a tagged
/// `LispyValue` of *statically unknown* runtime tag → a raw `i64` exit code,
/// dispatching on the tag at RUN time (int → `>> 3`; `#t`/`#f`/nil → `1`/`0`;
/// symbol/pair → the tagged word verbatim). Used for a **lambda** result (F7):
/// the program-exit boundary sees a `call` whose return type is `any`, so the
/// right coercion can't be picked at compile time the way `unbox_int` (int) /
/// `truthy` (bool) / verbatim (symbol) are — this single runtime switch handles
/// every case.
const EXIT_CODE_BUILTIN: &str = "dyn_to_exit_code";

/// True when a module's source language uses the tagged-word lisp value model
/// (today: McCarthy 1960 Lisp). Gates the lambda-aware `call` handling so a Twig
/// module — which shares this pass and also types untyped params `any` — is left
/// completely untouched.
fn is_lisp_language(language: &str) -> bool {
    language == "mccarthy-lisp"
}

/// True when `instr` is a `call` to a **lisp function** (a `LAMBDA`/`LABEL`,
/// identified by `lisp_funcs`). Its parameters and result use the tagged-word
/// value model, so an integer atom argument must be boxed and the result is a
/// `LispyValue` of unknown runtime tag. A *non-lisp* `call` (e.g. a Twig `fib`
/// recursion) is excluded, so this pass stays a no-op for non-lisp modules.
fn is_lisp_function_call(instr: &IIRInstr, lisp_funcs: &HashSet<String>) -> bool {
    instr.op == "call"
        && matches!(instr.srcs.first(), Some(Operand::Var(c)) if lisp_funcs.contains(c))
}

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

/// Rename `call_builtin "not"` → `dyn_not` when its argument is the result
/// of a `dyn_*` builtin — the `ATOM` = `not(pair?)` shape. `not` is also a
/// numeric builtin (Twig's machine boolean-not), so this *type-directed* check
/// is what keeps the two apart: a Twig `not` (whose argument is a raw `cmp`
/// result, not a `dyn_*` value) is left untouched for the numeric lowering.
fn rename_lisp_not(func: &mut IIRFunction) {
    // Dests of the `dyn_*` builtins that produce tagged values (cons/car/cdr
    // and the predicates renamed by `lower_heap_builtins_runtime`).
    let mut dyn_results: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        if instr.op == "call_builtin" {
            if let Some(Operand::Var(name)) = instr.srcs.first() {
                if name.starts_with("dyn_") {
                    if let Some(dest) = &instr.dest {
                        dyn_results.insert(dest.clone());
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
            matches!(instr.srcs.get(1), Some(Operand::Var(a)) if dyn_results.contains(a));
        if is_not && arg_is_lispy {
            instr.srcs[0] = Operand::Var("dyn_not".to_string());
        }
    }
}

/// Apply the representation pass to every function in `module`.
///
/// Runs in `twig-aot::prepare_module_for_aot` **after**
/// `lower_heap_builtins_runtime` (so cons/car/cdr are already `dyn_*`
/// calls). Safe to run on any module: a program with no `dyn_*` calls (every
/// Twig/Nib/Brainfuck program) has nothing to box and is left unchanged.
pub fn lower_dyn_repr(module: &mut IIRModule) {
    let entry = module.entry_point.clone();
    // The set of **lisp functions** (uniform value model at their boundary):
    // those that use the heap / lisp predicates or take a lisp (`any`) param,
    // closed under *calling*. This is exactly how the managed structural pass
    // partitions a module — reusing it keeps the tagged-word and managed passes
    // in agreement on what a "lisp call" is. Only a `call` whose callee is a
    // lisp function boxes its atom arguments / coerces its result.
    //
    // CRUCIAL: gate on the source language. Twig also types untyped params
    // `any` and shares this pass (twig-aot runs it on every module), so the
    // `any`-param heuristic alone would mis-flag a Twig `(define (fib n) …)` as
    // lisp and corrupt it. The empty set for a non-lisp module makes every new
    // `call`-handling branch inert — the pass stays a faithful no-op for Twig.
    let is_lisp = is_lisp_language(&module.language);
    let lisp_funcs = if is_lisp {
        crate::dyn_repr_structural::lisp_functions(module)
    } else {
        HashSet::new()
    };
    for func in &mut module.functions {
        let is_entry = entry.as_deref() == Some(func.name.as_str());
        lower_dyn_repr_function(func, is_entry, &lisp_funcs, is_lisp);
    }
}

fn lower_dyn_repr_function(
    func: &mut IIRFunction,
    is_entry: bool,
    lisp_funcs: &HashSet<String>,
    is_lisp: bool,
) {
    // ── 0. Type-directed `not` → `dyn_not`. ──
    //
    // `not` is ambiguous: a *numeric* builtin (Twig's machine boolean-not) and
    // the second half of McCarthy's `ATOM` (= `not(pair?)`, a lisp not). The
    // unconditional rename pass can't tell them apart, so it leaves `not`
    // alone. Here we have enough context: rename `not` → `dyn_not` only when
    // its argument is the result of a `dyn_*` builtin (e.g. `dyn_pair_p`),
    // which is exactly the `ATOM` shape. Twig's `not` (arg is a raw `cmp`
    // result, not a `dyn_*` value) is left for the numeric lowering.
    rename_lisp_not(func);

    // ── 1. Registers that feed a lisp call (their integer atoms box). ──
    //
    // Two callers put a value into a lisp context, so an integer *atom* passed
    // to either must be boxed (`n << 3`) to become a tagged `LispyValue`:
    //   • a lisp **builtin** (`call_builtin "dyn_*"`) — `cons`/`car`/`equal`/…;
    //   • a user **lambda / function** (`call` returning the polymorphic `any`),
    //     whose parameters are themselves lisp values (F7). Without this, an int
    //     argument arrives raw — e.g. `5` has tag bits `0b101` (= `#t`) and `7`
    //     has `0b111` (= a heap pair), so the body misreads it.
    // In both, `srcs[0]` is the callee/name and `srcs[1..]` are the arguments.
    let mut lisp_arg_regs: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        let is_lisp_call =
            lisp_builtin_name(instr).is_some() || is_lisp_function_call(instr, lisp_funcs);
        if is_lisp_call {
            for src in instr.srcs.iter().skip(1) {
                if let Operand::Var(v) = src {
                    lisp_arg_regs.insert(v.clone());
                }
            }
        }
    }

    // ── 2. Classify which registers hold a tagged `DynValue` (no mutation). ──
    //
    // Seeds (DVAL01 §3.3 — **producer-agnostic**):
    //   • **any op whose result type is a `DynValue`** (`any` / `ref<any>`) — a
    //     `dyn_car`/`dyn_cdr` result, a re-boxed `dyn_box_int` arithmetic result,
    //     a user lambda `call` returning the polymorphic `any`, … The register
    //     holds a tagged word because of *what it is*, not because its producer's
    //     name is on a hard-coded lisp allow-list. This is the generalisation
    //     that lets a boxed *arithmetic* result be exit-unboxed like any other
    //     `DynValue` (the concrete failure the dynamic-arithmetic slice hit).
    //   • a lisp-builtin result whose hint is *not* itself `ref<any>` — namely
    //     `dyn_cons` / nil, typed `ref<LispyPair>` — still seeded by name.
    //   • the nil sentinel const (`Int(0) : ref<LispyPair>`),
    //   • an integer const that feeds a lisp builtin (a lisp atom).
    //
    // The `DynValue`-hint seed is gated on `is_lisp`: Twig/Nib use `any` as a
    // *pre-resolution placeholder* on ordinary machine values (see the language
    // gate in `lower_dyn_repr`), so seeding on the hint outside a genuinely
    // dynamic module would mis-box them. Within a lisp module `any`/`ref<any>`
    // means exactly "tagged dynamic value".
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
        if let Some(dest) = &instr.dest {
            // Producer-agnostic: a register produced with a `DynValue` hint is a
            // tagged value, whatever primitive produced it.
            if is_lisp && is_dynvalue_hint(&instr.type_hint) {
                boxed_regs.insert(dest.clone());
                continue;
            }
        }
        if lisp_builtin_name(instr).is_some() {
            if let Some(dest) = &instr.dest {
                boxed_regs.insert(dest.clone());
            }
        } else if instr.op == "const" {
            if let (Some(dest), Some(Operand::Int(n))) = (&instr.dest, instr.srcs.first()) {
                let is_nil = instr.type_hint == REF_PAIR && *n == 0;
                let is_symbol = instr.type_hint == SYMBOL_HINT;
                if is_nil || is_symbol || lisp_arg_regs.contains(dest) {
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
        if instr.type_hint == SYMBOL_HINT {
            // Already a finished tagged symbol immediate from `intern_symbols`
            // — leave it; boxing (`<< 3`) would corrupt the id and tag.
        } else if instr.type_hint == REF_PAIR && n == 0 {
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
    // `dyn_truthy` (→ raw 0/1). Type-directed: only conditions that hold a
    // tagged value are wrapped, so a Twig `if` (whose condition is a raw `cmp`
    // result, not in `boxed_regs`) is left untouched. (A bare integer-literal
    // predicate that is not `mov`-connected to a tagged value stays raw and
    // unwrapped — correct for any non-zero literal; a literal `0` predicate is
    // a known minor corner, since raw 0 reads as false though lisp `0` is
    // truthy.)
    wrap_tagged_conditions(func, &boxed_regs);

    // ── 5. Unbox at the program-exit boundary (entry function only). ──
    if is_entry {
        insert_unbox_before_lisp_rets(func, &boxed_regs, lisp_funcs);
    }
}

/// Rewrite each `jmp_if_false %cond, label` whose `%cond` holds a tagged
/// `LispyValue` into `%t = dyn_truthy(%cond); jmp_if_false %t, label`, so the
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
                // %t = call_builtin "dyn_truthy", %cond  : i64
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
/// `LispyValue` into `%u = dyn_unbox_int(%x); ret %u`, so the process exit
/// code is the raw integer value rather than the tagged word.
fn insert_unbox_before_lisp_rets(
    func: &mut IIRFunction,
    boxed_regs: &HashSet<String>,
    lisp_funcs: &HashSet<String>,
) {
    // McCarthy lambda-result handling (F7): a user function / lambda result is a
    // `call` whose return type is the polymorphic `any` (or a lisp `ref<Lispy…>`).
    // Its runtime tag (int / bool / symbol / pair) is unknown at compile time, so
    // none of the static coercions (`unbox_int` / `truthy` / verbatim) is right on
    // its own. Returning such a value runs it through `dyn_to_exit_code`, a
    // runtime tag switch. (Lisp *builtin* results — `car`/`cons`/… — are already
    // in `boxed_regs` and keep their existing handling; only plain `call`s join
    // this set.)
    let call_result_regs: HashSet<String> = func
        .instructions
        .iter()
        .filter(|i| is_lisp_function_call(i, lisp_funcs))
        .filter_map(|i| i.dest.clone())
        .collect();

    // Nothing returns a coercible lisp value → no work (avoids the Vec rebuild).
    let needs_unbox = func.instructions.iter().any(|i| {
        i.op == "ret"
            && matches!(i.srcs.first(), Some(Operand::Var(v))
                if boxed_regs.contains(v) || call_result_regs.contains(v))
    });
    if !needs_unbox {
        return;
    }

    // McCarthy boolean-result handling (the L3b-2c-2 gap, closed in W12b-2): a
    // value produced by a predicate (`pair?`/`equal?`/`not`) is a tagged boolean
    // (`LISPY_TRUE = 5` / `LISPY_FALSE = 3`), NOT a tagged integer. Unboxing it
    // (`>> 3`) would give `0` for *true* — wrong. So at the program-exit boundary
    // a boolean result is coerced with `dyn_truthy` (→ raw `0`/`1`) instead of
    // `dyn_unbox_int`. A register is "boolean" if the instruction that produced
    // it carries the `bool` type hint (the predicates do).
    let bool_regs: HashSet<String> = func
        .instructions
        .iter()
        .filter(|i| i.type_hint == "bool")
        .filter_map(|i| i.dest.clone())
        .collect();

    // McCarthy symbol-result handling (F6): a SYMBOL is already a finished tagged
    // immediate from `intern_symbols` (`(id << shift) | TAG_SYMBOL`), NOT a boxed
    // integer — `dyn_unbox_int` (`>> 3`) would corrupt the id+tag. The program
    // result IS the tagged symbol word, so such a `ret` is returned verbatim (just
    // retyped to `i64`, the tagged-word width).
    let symbol_regs: HashSet<String> = func
        .instructions
        .iter()
        .filter(|i| i.type_hint == SYMBOL_HINT)
        .filter_map(|i| i.dest.clone())
        .collect();

    let old = std::mem::take(&mut func.instructions);
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    let mut unbox_counter = 0usize;

    for instr in old {
        // The register a `ret` returns, if it holds a coercible lisp value.
        let ret_reg = if instr.op == "ret" {
            match instr.srcs.first() {
                Some(Operand::Var(v)) if boxed_regs.contains(v) || call_result_regs.contains(v) => {
                    Some(v.clone())
                }
                _ => None,
            }
        } else {
            None
        };

        match ret_reg {
            Some(reg) if symbol_regs.contains(&reg) => {
                // A symbol result is returned as its tagged word (no coercion).
                new_instrs.push(IIRInstr::new("ret", None, vec![Operand::Var(reg)], "i64"));
            }
            Some(reg) => {
                // Pick the exit coercion by what we statically know about `reg`:
                //   • a lambda / user-`call` result (tag unknown) → `dyn_to_exit_code`
                //     (a RUNTIME tag switch — int/bool/symbol/pair all handled);
                //   • a boolean (predicate) result → `dyn_truthy` (→ 0/1);
                //   • otherwise a boxed integer → `dyn_unbox_int` (→ `>> 3`).
                // All three yield a raw `i64`.
                let coerce = if call_result_regs.contains(&reg) {
                    EXIT_CODE_BUILTIN
                } else if bool_regs.contains(&reg) {
                    TRUTHY_BUILTIN
                } else {
                    UNBOX_BUILTIN
                };
                // %u = call_builtin "<coerce>", %reg  : i64
                let u_reg = format!("__unbox_{unbox_counter}");
                unbox_counter += 1;
                new_instrs.push(IIRInstr::new(
                    "call_builtin",
                    Some(u_reg.clone()),
                    vec![Operand::Var(coerce.to_string()), Operand::Var(reg)],
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

    /// `(CAR (CONS 7 9))`: 7 and 9 feed dyn_cons → boxed; the car result is
    /// returned by the entry fn → unboxed.
    #[test]
    fn cons_car_boxes_ints_and_unboxes_result() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "dyn_cons", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("r"), "dyn_car", &["cell"], "any"),
            ret("r"),
        ]);
        lower_dyn_repr(&mut m);
        // 7 → 56, 9 → 72 (boxed: n << 3).
        assert_eq!(find_const(&m, "v0"), 7 << 3, "7 must box to 56");
        assert_eq!(find_const(&m, "v1"), 9 << 3, "9 must box to 72");
        // ret r → unbox r, ret %u.
        let instrs = &m.functions[0].instructions;
        let last = instrs.last().unwrap();
        assert_eq!(last.op, "ret");
        let unbox = &instrs[instrs.len() - 2];
        assert_eq!(unbox.op, "call_builtin");
        assert_eq!(unbox.srcs[0], Operand::Var("dyn_unbox_int".into()));
        assert_eq!(unbox.srcs[1], Operand::Var("r".into()));
        // ret now refers to the unbox result, not the boxed car.
        assert_ne!(last.srcs[0], Operand::Var("r".into()));
    }

    /// DVAL01-3 (§3.3): a boxed **arithmetic** result — a `dyn_box_int` result
    /// typed `ref<any>`, which is deliberately **not** on the `LISP_BUILTINS`
    /// allow-list — is recognised as a `DynValue` by its *result type* and
    /// exit-unboxed, exactly like a `dyn_car` result. Before the producer-
    /// agnostic seed this register was invisible to the classifier, so the
    /// program returned a tagged word (`n << 3`) instead of the machine exit
    /// code — the exact failure the dynamic-arithmetic slice (E6d-2b) hit.
    #[test]
    fn boxed_non_cons_dynvalue_is_exit_unboxed() {
        let mut m = module(vec![
            // A raw machine int, boxed by a primitive that is *not* cons/car/…
            konst("x", 5, "i64"),
            call_builtin(Some("r"), "dyn_box_int", &["x"], "ref<any>"),
            ret("r"),
        ]);
        lower_dyn_repr(&mut m);
        // `x` feeds box_int as a raw machine word — it must NOT be re-boxed.
        assert_eq!(find_const(&m, "x"), 5, "box_int's operand stays a raw i64");
        // The exit boundary unboxes the tagged result to a machine word.
        assert_eq!(
            count_builtin(&m, "dyn_unbox_int"),
            1,
            "the box_int result must be exit-unboxed (producer-agnostic classification)",
        );
        let last = m.functions[0].instructions.last().unwrap();
        assert_eq!(last.op, "ret");
        assert_ne!(
            last.srcs[0],
            Operand::Var("r".into()),
            "ret must consume the unboxed value, not the tagged r",
        );
    }

    /// The producer-agnostic seed stays gated on the source language: a **Twig**
    /// module (which uses `any` as a pre-resolution placeholder on ordinary
    /// machine values) is a no-op — no boxing, no unbox inserted.
    #[test]
    fn twig_any_hint_is_not_boxed() {
        let mut m = module(vec![konst("v", 42, "any"), ret("v")]);
        m.language = "twig".into();
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "v"), 42, "Twig `any` value must not be boxed");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "no unbox in a Twig module");
        assert_eq!(m.functions[0].instructions.len(), 2, "Twig module left unchanged");
    }

    /// A bare integer `42`: the constant never reaches a `dyn_*` call, so it
    /// is not boxed, and the ret is not unboxed — exit 42 unchanged.
    #[test]
    fn scalar_int_is_left_raw() {
        let mut m = module(vec![konst("v0", 42, "i64"), ret("v0")]);
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "v0"), 42, "scalar int must not be boxed");
        let last = m.functions[0].instructions.last().unwrap();
        assert_eq!(last.op, "ret");
        assert_eq!(last.srcs[0], Operand::Var("v0".into()), "ret must not be unboxed");
        assert_eq!(m.functions[0].instructions.len(), 2, "no unbox inserted");
    }

    /// A Twig-style arithmetic program: integers feed `add` (a machine op),
    /// never a `dyn_*` call, so nothing is boxed or unboxed.
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
        lower_dyn_repr(&mut m);
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
            call_builtin(Some("cell"), "dyn_cons", &["v", "n"], "ref<LispyPair>"),
            ret("cell"),
        ]);
        lower_dyn_repr(&mut m);
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
                call_builtin(Some("cell"), "dyn_cons", &["v0", "v0"], "ref<LispyPair>"),
                ret("cell"),
            ],
        );
        let mut m = module(vec![konst("z", 0, "i64"), ret("z")]);
        m.functions.push(callee);
        lower_dyn_repr(&mut m);
        // helper's ret must NOT be rewritten to an unbox (it returns a pair).
        let helper = m.functions.iter().find(|f| f.name == "helper").unwrap();
        let last = helper.instructions.last().unwrap();
        assert_eq!(last.op, "ret");
        assert_eq!(last.srcs[0], Operand::Var("cell".into()));
    }

    /// End-to-end of the two native passes as `twig-aot` runs them: the
    /// frontend emits `call_builtin "cons"/"car"`; `lower_heap_builtins_runtime`
    /// renames them to `dyn_*`; then `lower_dyn_repr` boxes the atoms and
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
        // Pass 1: rename cons/car → dyn_cons/dyn_car.
        crate::heap::lower_heap_builtins_runtime(&mut m);
        // Pass 2: box atoms + unbox result.
        lower_dyn_repr(&mut m);

        assert_eq!(find_const(&m, "v0"), 7 << 3);
        assert_eq!(find_const(&m, "v1"), 9 << 3);
        let instrs = &m.functions[0].instructions;
        // cons/car are now dyn_*.
        assert_eq!(instrs[2].srcs[0], Operand::Var("dyn_cons".into()));
        assert_eq!(instrs[3].srcs[0], Operand::Var("dyn_car".into()));
        // The tail is: unbox(r) ; ret %unbox.
        let last = instrs.last().unwrap();
        assert_eq!(last.op, "ret");
        let unbox = &instrs[instrs.len() - 2];
        assert_eq!(unbox.srcs[0], Operand::Var("dyn_unbox_int".into()));
    }

    // ── L3b-2c-2: ATOM/EQ predicates + COND truthiness ──────────────────

    /// `(ATOM 5)` lowers (after the rename) to `not(pair?(5))`. The `5` feeds
    /// `dyn_pair_p`, so it must box.
    #[test]
    fn predicate_arg_int_is_boxed() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "dyn_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "dyn_not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "v0"), 5 << 3, "ATOM's int atom must box");
    }

    /// A `jmp_if_false` whose condition is a tagged value (a predicate result)
    /// gets a `dyn_truthy` normaliser inserted before it.
    #[test]
    fn tagged_cond_is_wrapped_with_truthy() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "dyn_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "dyn_not", &["p"], "bool"),
            jmp_if_false("a", "L_next"),
            IIRInstr::new("label", None, vec![Operand::Var("L_next".into())], "void"),
            ret("a"),
        ]);
        lower_dyn_repr(&mut m);
        // TWO `dyn_truthy` calls now: (1) the `COND` clause-test wrap, and
        // (2) the bool-typed `ret a` result coercion (McCarthy W12b-2 — a
        // boolean program result becomes raw 0/1 via `truthy`, not `unbox_int`).
        assert_eq!(count_builtin(&m, "dyn_truthy"), 2, "cond test + bool result both wrapped");
        // The boolean result is coerced with `dyn_truthy`, NOT `dyn_unbox_int`.
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "bool result must not be unboxed");
        // The jmp_if_false now tests the truthy result, not the raw tagged bool.
        let jif = m.functions[0].instructions.iter().find(|i| i.op == "jmp_if_false").unwrap();
        assert!(
            matches!(&jif.srcs[0], Operand::Var(v) if v.starts_with("__truthy_")),
            "jmp_if_false must test the truthy result, got {:?}", jif.srcs[0],
        );
    }

    /// McCarthy W12b-2 — the program-exit coercion is **type-directed**:
    /// an INTEGER result is unboxed (`>> 3`), a BOOLEAN result (a predicate) is
    /// run through `dyn_truthy` (→ raw 0/1). Unboxing `LISPY_TRUE` (=5) would
    /// give `5 >> 3 = 0` — *wrong* for true. So a bool result must never be unboxed.
    #[test]
    fn integer_result_unboxed_boolean_result_truthied() {
        // (CAR (CONS 7 9)) — an integer result → unbox, NOT truthy.
        let mut int_m = module(vec![
            konst("a", 56, "i64"),
            konst("b", 72, "i64"),
            call_builtin(Some("p"), "dyn_cons", &["a", "b"], "ref<LispyPair>"),
            call_builtin(Some("h"), "dyn_car", &["p"], "i64"),
            ret("h"),
        ]);
        lower_dyn_repr(&mut int_m);
        assert_eq!(count_builtin(&int_m, "dyn_unbox_int"), 1, "int result is unboxed");
        assert_eq!(count_builtin(&int_m, "dyn_truthy"), 0, "int result is not truthied");

        // (ATOM 7) = (not (pair? 7)) — a boolean result → truthy, NOT unbox.
        let mut bool_m = module(vec![
            konst("v0", 56, "i64"),
            call_builtin(Some("p"), "dyn_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "dyn_not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr(&mut bool_m);
        assert_eq!(count_builtin(&bool_m, "dyn_truthy"), 1, "bool result is truthied");
        assert_eq!(count_builtin(&bool_m, "dyn_unbox_int"), 0, "bool result is NOT unboxed");
    }

    /// McCarthy W13 (F6): a SYMBOL program result is returned verbatim (its tagged
    /// immediate) — NOT `unbox_int`'d (`>> 3` would corrupt the id+tag) and NOT
    /// `truthy`'d. `(QUOTE A)` lowers to `const … : symbol` then a bare `ret`.
    #[test]
    fn symbol_result_returned_verbatim() {
        let mut m = module(vec![
            // `(QUOTE A)` after intern: a finished tagged symbol immediate.
            IIRInstr::new("const", Some("s".into()), vec![Operand::Var("A".into())], SYMBOL_HINT),
            ret("s"),
        ]);
        lower_dyn_repr(&mut m);
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "a symbol result is NOT unboxed");
        assert_eq!(count_builtin(&m, "dyn_truthy"), 0, "a symbol result is NOT truthied");
        // The final instruction is a bare `ret` of the symbol register.
        let last = m.functions[0].instructions.last().unwrap();
        assert_eq!(last.op, "ret", "ends with a bare ret of the tagged symbol word");
    }

    /// Build a module with `main` (entry, `functions[0]`) plus extra functions.
    fn multi_fn_module(main: Vec<IIRInstr>, extra: Vec<IIRFunction>, language: &str) -> IIRModule {
        let mut functions = vec![IIRFunction::new("main", vec![], "any", main)];
        functions.extend(extra);
        IIRModule {
            name: "test".into(),
            functions,
            entry_point: Some("main".into()),
            language: language.into(),
            exports: vec![],
            imports: vec![],
        }
    }

    /// `main { a = 5; r = <callee>(a); ret r }` plus an identity `<callee>(X){ ret X }`.
    fn apply_identity_module(callee: &str, language: &str) -> IIRModule {
        let id = IIRFunction::new(callee, vec![("X".into(), "any".into())], "any", vec![ret("X")]);
        let main = vec![
            konst("a", 5, "i64"),
            IIRInstr::new(
                "call",
                Some("r".into()),
                vec![Operand::Var(callee.into()), Operand::Var("a".into())],
                "any",
            ),
            ret("r"),
        ];
        multi_fn_module(main, vec![id], language)
    }

    /// McCarthy W13b (F7): a `call` to a lisp function boxes its integer atom
    /// argument (`5 << 3 = 40`) and coerces its polymorphic result at the program
    /// exit with the runtime tag switch `dyn_to_exit_code`.
    #[test]
    fn lambda_call_boxes_int_arg_and_coerces_result() {
        let mut m = apply_identity_module("lambda_0", "mccarthy-lisp");
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "a"), 40, "int atom arg must box before the lambda");
        assert_eq!(count_builtin(&m, "dyn_to_exit_code"), 1, "lambda result → to_exit_code");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "polymorphic result is NOT unboxed");
    }

    /// The same IIR shape in a **Twig** module is left completely untouched: an
    /// untyped Twig param is also `any`, but the source language is not lisp, so
    /// the int arg stays raw and the result is returned verbatim. (Guards the
    /// regression where `(define (fib n) …)` was mis-boxed.)
    #[test]
    fn non_lisp_call_is_left_untouched() {
        let mut m = apply_identity_module("fib", "twig");
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "a"), 5, "a Twig int arg stays a raw machine word");
        assert_eq!(count_builtin(&m, "dyn_to_exit_code"), 0, "a Twig call is never coerced");
        assert_eq!(count_builtin(&m, "dyn_box_int"), 0, "a Twig arg is never boxed");
    }

    /// A raw machine condition (e.g. a Twig `cmp` result, not in boxed_regs)
    /// is left alone — no `dyn_truthy` wrap.
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
        lower_dyn_repr(&mut m);
        assert_eq!(count_builtin(&m, "dyn_truthy"), 0, "raw cmp cond must NOT be wrapped");
    }

    /// A COND result funnelled by `mov` from a lisp value (a `car` result)
    /// must be recognised as tagged and unboxed at the entry ret.
    #[test]
    fn mov_propagates_taggedness_for_unbox() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "dyn_cons", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("c"), "dyn_car", &["cell"], "any"),
            mov("result", "c"),
            ret("result"),
        ]);
        lower_dyn_repr(&mut m);
        // The car result flows through `mov result, c`; `result` is therefore a
        // tagged value and the entry ret must unbox it.
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 1, "mov'd lisp result must unbox");
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
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "seven"), 7 << 3, "clause literal must box (mov-tied to nil)");
        assert_eq!(find_const(&m, "nilv"), TAG_NIL, "nil tagged");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 1, "result must unbox at ret");
    }

    /// ATOM shape `not(pair?(x))`: the generic `not` whose arg is a `dyn_*`
    /// result is renamed to `dyn_not`.
    #[test]
    fn not_after_lispy_result_becomes_lispy_not() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "dyn_pair_p", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr(&mut m);
        assert_eq!(count_builtin(&m, "dyn_not"), 1, "ATOM's not must become dyn_not");
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
        lower_dyn_repr(&mut m);
        assert_eq!(count_builtin(&m, "not"), 1, "Twig's machine not must be left alone");
        assert_eq!(count_builtin(&m, "dyn_not"), 0);
    }

    /// A symbol immediate (`const Int(id<<32|2):symbol`, from `intern_symbols`)
    /// is treated as a tagged value but must NOT be boxed — shifting it would
    /// corrupt the id/tag. Here `'A` feeds `dyn_equal`, so it would be a box
    /// candidate but for the symbol guard.
    #[test]
    fn symbol_immediate_is_tagged_but_not_boxed() {
        let sym_bits = (1_i64 << 32) | 0b010; // id 1, symbol tag
        let mut m = module(vec![
            IIRInstr::new("const", Some("s".into()), vec![Operand::Int(sym_bits)], "symbol"),
            call_builtin(Some("e"), "dyn_equal", &["s", "s"], "bool"),
            ret("e"),
        ]);
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "s"), sym_bits, "symbol immediate must be left unboxed");
    }

    /// Out-of-range integers (beyond ±2⁶⁰) are left unboxed rather than
    /// silently truncated by the shift.
    #[test]
    fn out_of_range_int_is_not_boxed() {
        let huge = (1_i64 << 61) | 1;
        let mut m = module(vec![
            konst("v0", huge, "i64"),
            call_builtin(Some("cell"), "dyn_cons", &["v0", "v0"], "ref<LispyPair>"),
            ret("cell"),
        ]);
        lower_dyn_repr(&mut m);
        assert_eq!(find_const(&m, "v0"), huge, "out-of-range int must be left raw");
    }
}

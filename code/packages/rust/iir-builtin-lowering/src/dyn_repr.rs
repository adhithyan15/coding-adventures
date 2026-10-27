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
//!   as an **argument to a lisp builtin** (`dyn_pair_new`/`dyn_pair_first`/
//!   `dyn_pair_rest`, and the predicates added in later slices). A McCarthy
//!   program — which has no machine arithmetic at all — feeds every integer
//!   into `cons`, so all its integers box. A Twig arithmetic program feeds
//!   integers into `add`/`print_i64`, never a `dyn_*` call, so none box.
//! - A `const Int(0) : ref<LispyPair>` (the nil sentinel emitted by the
//!   frontend) becomes the **nil tag** `0b001`. Only lisp frontends ever
//!   emit that type hint, so this is unambiguous.
//! - A register **holds a boxed `LispyValue`** if it is the result of a lisp
//!   builtin (`dyn_pair_new`/`dyn_pair_first`/`dyn_pair_rest`) or a boxed constant.
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
use std::collections::{HashMap, HashSet};

use crate::lisp_semantics;

/// The frontend type hint for a reference to a lisp pair (and the nil
/// sentinel). Mirrors `mccarthy-lisp-iir-compiler`'s `REF_PAIR`.
const REF_PAIR: &str = "ref<LispyPair>";
/// The two type hints that denote a **boxed `DynValue`** — the language-neutral
/// tagged word.  `ref<any>` is a heap-typed dynamic value (a `dyn_pair_first`/`dyn_pair_rest`
/// result, a re-boxed arithmetic result); `any` is the still-abstract dynamic
/// value (a user-lambda result, a dynamic parameter).  A register produced with
/// either hint holds a `DynValue` *by what it is*, independent of which
/// primitive produced it — the producer-agnostic classification of DVAL01 §3.3.
const REF_ANY: &str = "ref<any>";

/// The type hint for a symbol literal. After `intern_symbols` runs, such a
/// const already holds the finished tagged immediate `(id<<32)|TAG_SYMBOL` —
/// it is a `LispyValue` (so it joins `boxed_regs`) but must NOT be boxed again
/// (shifting it would corrupt the id/tag).
const SYMBOL_HINT: &str = "symbol";

/// The frontend's still-unresolved dynamic hint (see [`raw_only_params`]).
const ANY_HINT: &str = "any";

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
    "dyn_pair_new",
    "dyn_pair_first",
    "dyn_pair_rest",
    "dyn_is_pair",
    // `null?` — its result is a tagged #t/#f (like `pair?`), so it must join
    // `boxed_regs`: this both wraps the cons-walk helpers' `jmp_if_false` with
    // Lisp truthiness (→ raw 0/1) AND lets a top-level `(null? …)` exit-code through
    // the tagged switch instead of being misread as the nil word.
    "dyn_is_nil",
];

/// The compiler-emitted Lisp helpers (LANG83 PR7f-2, [`crate::lisp_semantics`])
/// that take tagged values and return one, so they classify exactly like the
/// builtins above: an integer atom argument boxes, and the result is a tagged
/// boolean. They replace the runtime calls `dyn_not` / `dyn_equal`.
const LISP_VALUE_HELPERS: &[&str] = &[lisp_semantics::NOT, lisp_semantics::EQUAL];

/// Builtins whose **argument** is a tagged value but whose **result** is a raw
/// machine integer, so an integer atom flowing in must box (its tag is then
/// `000`) while the result must not join `boxed_regs`. LANG83 PR7f's
/// `dyn_tag` is the one: `dyn_tag 5` asks for the tag of the integer 5 (0),
/// not of the raw word 5 (`0b101`, the `#t` tag), and it answers a plain
/// integer.
const LISP_ARG_ONLY_BUILTINS: &[&str] = &["dyn_tag"];

/// `true` if `instr` is a `call_builtin` of a [`LISP_ARG_ONLY_BUILTINS`] name.
fn is_lisp_arg_only_builtin(instr: &IIRInstr) -> bool {
    instr.op == "call_builtin"
        && matches!(instr.srcs.first(), Some(Operand::Var(n)) if LISP_ARG_ONLY_BUILTINS.contains(&n.as_str()))
}

/// The unbox helper (`__dyn_unbox_int`, arithmetic `>> 3`).
const UNBOX_BUILTIN: &str = "dyn_unbox_int";

/// The truthiness helper ([`lisp_semantics::TRUTHY`], `__lisp_truthy`): a
/// tagged value → a raw machine `0`/`1` (false iff `#f` or nil), so the
/// backend's `jmp_if_false` — which tests a raw word against zero — branches
/// correctly on a `COND` predicate that produced a tagged boolean. Lisp's
/// rule, so a module-local function the lowering emits, not a runtime call
/// (it was `dyn_truthy` → `__dyn_truthy` before LANG83 PR7f-2).
const TRUTHY_HELPER: &str = lisp_semantics::TRUTHY;

/// The universal exit-coercion helper ([`lisp_semantics::EXIT_CODE`],
/// `__lisp_exit_code`): a tagged value of *statically unknown* runtime tag → a
/// raw `i64` exit code, dispatching on the tag at RUN time (int → `>> 3`;
/// `#t`/`#f`/nil → `1`/`0`; symbol/pair → the tagged word verbatim). Used for a
/// **lambda** result (F7): the program-exit boundary sees a `call` whose
/// return type is `any`, so the right coercion can't be picked at compile time
/// the way `unbox_int` (int) / `truthy` (bool) / verbatim (symbol) are — this
/// single run-time switch handles every case. (It was `dyn_to_exit_code`
/// before LANG83 PR7f-2.)
const EXIT_CODE_HELPER: &str = lisp_semantics::EXIT_CODE;

// `is_lisp_language` is gone. It existed only to disambiguate bare `any`, which
// McCarthy used for "a tagged LispyValue" and Twig/Nib used for "statically
// unresolved, passed raw". The frontends now say which they mean — `ref<any>` is
// the tagged dynamic value — so the passes read the IIR instead of asking who
// wrote it. An IR that needs to know its source language is not language-agnostic.

/// True when `instr` is a `call` to a **lisp function** (a `LAMBDA`/`LABEL`,
/// identified by `lisp_funcs`). Its parameters and result use the tagged-word
/// value model, so an integer atom argument must be boxed and the result is a
/// `LispyValue` of unknown runtime tag. A *non-lisp* `call` (e.g. a Twig `fib`
/// recursion) is excluded, so this pass stays a no-op for non-lisp modules.
fn is_lisp_function_call(instr: &IIRInstr, lisp_funcs: &HashSet<String>) -> bool {
    instr.op == "call"
        && matches!(instr.srcs.first(), Some(Operand::Var(c)) if lisp_funcs.contains(c))
}

/// If `instr` is `call_builtin "<lisp builtin>"` or a `call` of one of the
/// [`LISP_VALUE_HELPERS`], return that name.
fn lisp_builtin_name(instr: &IIRInstr) -> Option<&str> {
    let names: &[&str] = match instr.op.as_str() {
        "call_builtin" => LISP_BUILTINS,
        "call" => LISP_VALUE_HELPERS,
        _ => return None,
    };
    match instr.srcs.first() {
        Some(Operand::Var(name)) if names.contains(&name.as_str()) => Some(name.as_str()),
        _ => None,
    }
}

/// `call_builtin "equal?" a b` → `call __lisp_equal a b` (LANG83 PR7f-2).
///
/// `equal?` has no machine meaning, so unlike `not` the rewrite is
/// unconditional. Its result is a tagged boolean, so it is typed `ref<any>`
/// (as `lower_heap_builtins_runtime` already retyped it).
fn rename_lisp_equal(func: &mut IIRFunction) {
    for instr in &mut func.instructions {
        let is_equal = instr.op == "call_builtin"
            && matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "equal?");
        if is_equal {
            instr.op = "call".to_string();
            instr.srcs[0] = Operand::Var(lisp_semantics::EQUAL.to_string());
            instr.type_hint = REF_ANY.to_string();
        }
    }
}

/// Turn the rewritten `call_builtin "not"` (see [`rename_lisp_not`]) into a
/// `call` of the helper.
fn as_helper_call(instr: &mut IIRInstr, helper: &str) {
    instr.op = "call".to_string();
    instr.srcs[0] = Operand::Var(helper.to_string());
}

/// Rewrite `call_builtin "not"` → `call __lisp_not` when its argument is the
/// result of a `dyn_*` builtin or a Lisp value helper — the `ATOM` =
/// `not(pair?)` shape. `not` is also a
/// numeric builtin (Twig's machine boolean-not), so this *type-directed* check
/// is what keeps the two apart: a Twig `not` (whose argument is a raw `cmp`
/// result, not a `dyn_*` value) is left untouched for the numeric lowering.
fn rename_lisp_not(func: &mut IIRFunction) {
    // Dests of the `dyn_*` builtins that produce tagged values (cons/car/cdr
    // and the predicates renamed by `lower_heap_builtins_runtime`), and of the
    // `__lisp_equal` / `__lisp_not` helper calls.
    let mut dyn_results: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        let tagged = match (instr.op.as_str(), instr.srcs.first()) {
            ("call_builtin", Some(Operand::Var(name))) => name.starts_with("dyn_"),
            ("call", Some(Operand::Var(name))) => LISP_VALUE_HELPERS.contains(&name.as_str()),
            _ => false,
        };
        if tagged {
            if let Some(dest) = &instr.dest {
                dyn_results.insert(dest.clone());
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
            as_helper_call(instr, lisp_semantics::NOT);
        }
    }
}

/// Apply the representation pass to every function in `module`.
///
/// Runs in `twig-aot::prepare_module_for_aot` **after**
/// `lower_heap_builtins_runtime` (so cons/car/cdr are already `dyn_*`
/// calls). Safe to run on any module: a program with no `dyn_*` calls (every
/// Twig/Nib/Brainfuck program) has nothing to box and is left unchanged.
///
/// # Errors
///
/// [`lisp_semantics::ReservedNameError`], before anything is rewritten, when
/// the module defines or calls a `__lisp_*` name the compiler did not write
/// ([`lisp_semantics::check_reserved_names`]). The pass recognises its own
/// rule markers and helpers by those names, so a program that could spell
/// one could stand in for `equal?` or have its call expanded as `not`.
pub fn lower_dyn_repr(module: &mut IIRModule) -> Result<(), lisp_semantics::ReservedNameError> {
    lisp_semantics::check_reserved_names(module)?;
    lower_dyn_repr_marked(module);
    // LANG83 PR7f-2: the Lisp rules the pass marked with `call`s become
    // compiler-emitted IIR. Truthiness and `not` expand inline (straight-line,
    // so a function that uses them stays JIT-compilable); `equal?` and the
    // exit-code switch become functions of this module, appended last (so no
    // step of the pass rewrites their bodies) and only when something calls
    // them.
    for func in &mut module.functions {
        lisp_semantics::expand_inline_rules(func);
    }
    lisp_semantics::add_used_helpers(module)
}

/// [`lower_dyn_repr`] up to, not including, writing out the Lisp rules: a use
/// of truthiness, `not`, `equal?` or the exit coercion is still a `call` of
/// the rule's name ([`lisp_semantics`]), which the unit tests count.
fn lower_dyn_repr_marked(module: &mut IIRModule) {
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
    // No language gate. `lisp_functions` seeds from `ref<any>`/`symbol` params
    // and heap ops — evidence carried by the IIR itself — and closes under
    // calling. A module whose frontend emits none of those yields an empty set,
    // so this pass is a faithful no-op for it without having to be told which
    // frontend wrote it.
    // Which callees take a **tagged** argument list? That is decided by their
    // declared parameter types, not by what their bodies allocate.
    //
    // `lisp_functions` seeds from `function_uses_heap(f) || has_lisp_param(f)`,
    // and the heap clause is wrong for this purpose: a Twig union constructor
    // `(union Opt (Some (v : int)) …)` allocates a cons cell internally — so it
    // "uses the heap" — while its parameter is a bare `any` it receives raw.
    // Treating it as tagged made `call Some(42)` box the 42, and the `match`
    // that extracted it returned the tagged word: the program exited
    // `(42 << 3) & 0xFF = 80` instead of 42. (The old language gate hid this by
    // forcing the set empty for every non-lisp module, which is exactly the
    // coupling being removed — so the underlying rule has to be right now.)
    //
    // The Lisp helpers (LANG83 PR7f-2) are already in their final, tagged-word
    // form, so they are set aside while the module is classified and rewritten
    // (re-running the pass, or a module that arrives with them, must not box
    // their constants or coerce their results), then put back. Only the
    // compiler's own helper qualifies — the same body instruction for
    // instruction, not merely the same name — so a program's function can
    // never be waved through as one (and `lower_dyn_repr` has already refused
    // any other `__lisp_*` definition).
    let (helpers, rest): (Vec<IIRFunction>, Vec<IIRFunction>) = std::mem::take(&mut module.functions)
        .into_iter()
        .partition(lisp_semantics::is_compiler_helper);
    module.functions = rest;
    let lisp_funcs = crate::dyn_repr_structural::tagged_boundary_functions(module);
    // Every fact about call boundaries is computed on the module as
    // `lower_dynamic_arith` left it, before any function is rewritten below.
    let boundary = CallBoundary::analyse(module, lisp_funcs);
    for func in &mut module.functions {
        let is_entry = entry.as_deref() == Some(func.name.as_str());
        lower_dyn_repr_function(func, is_entry, &boundary);
    }
    module.functions.extend(helpers);
}

fn lower_dyn_repr_function(func: &mut IIRFunction, is_entry: bool, boundary: &CallBoundary) {
    let lisp_funcs = &boundary.lisp_funcs;
    // ── 0. `equal?` → `__lisp_equal`, and type-directed `not` → `__lisp_not`. ──
    //
    // `not` is ambiguous: a *numeric* builtin (Twig's machine boolean-not) and
    // the second half of McCarthy's `ATOM` (= `not(pair?)`, a lisp not). The
    // unconditional rename pass can't tell them apart, so it leaves `not`
    // alone. Here we have enough context: rename `not` → `__lisp_not` only when
    // its argument is the result of a `dyn_*` builtin (e.g. `dyn_is_pair`),
    // which is exactly the `ATOM` shape. Twig's `not` (arg is a raw `cmp`
    // result, not a `dyn_*` value) is left for the numeric lowering.
    rename_lisp_equal(func);
    rename_lisp_not(func);

    // ── 1–2. Which registers hold a tagged word (see `tagged_registers`). ──
    let boxed_regs = tagged_registers(func, boundary);

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

    // ── 3b. Unbox a provably tagged argument bound for a raw-only parameter. ──
    unbox_tagged_args_for_raw_only_params(func, boundary);

    // ── 4. Normalise tagged conditions for `jmp_if_false` (all functions). ──
    //
    // A `COND` predicate evaluates to a tagged LispyValue (e.g. `#f` = 3 from
    // `ATOM`). The backend's `jmp_if_false` tests a *raw* word against zero, so
    // a bare `#f` (3 ≠ 0) would read as true. We wrap such a condition in
    // Lisp truthiness (`__lisp_truthy`, expanded inline → raw 0/1). Type-directed: only conditions that hold a
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
    } else if lisp_funcs.contains(&func.name) {
        // ── 5b. Box a tagged function's RAW scalar result. ──
        //
        // A function on the tagged boundary promises its callers a tagged word.
        // `((LAMBDA () 5))` returns the raw `5`, and the caller's exit coercion
        // then reads it as a tagged value: `5` is `0b101`, the whole-word `#t`
        // tag, so the program exited 1 instead of 5.
        //
        // The managed pass has had this since the start (`dyn_repr_structural`'s
        // non-entry box branch); the tagged pass never grew the counterpart. It
        // stayed hidden because the entry's coercion was `unbox_int` — `5 >> 3 =
        // 0`, also wrong, just wrong differently — until the nullary lambda
        // reached the correct runtime tag switch and exposed it.
        let tagged_params = arriving_tagged_params(func, boundary);
        box_raw_scalar_return(func, &boxed_regs, &tagged_params);
    }
}

/// Steps 1–2 of [`lower_dyn_repr_function`], without mutation: the registers
/// that hold (or, after step 3 boxes their constant, will hold) a tagged word.
/// A pure function of the function body and the module's call-boundary facts,
/// so the module-level analyses predict *exactly* what the rewrite does
/// (`CallBoundary::predicted`) instead of approximating it.
fn tagged_registers(func: &IIRFunction, boundary: &CallBoundary) -> HashSet<String> {
    let lisp_funcs = &boundary.lisp_funcs;
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
    //
    // Except a **raw-only** parameter of a user function (see
    // [`raw_only_params`]): one whose value is only ever read by machine
    // arithmetic. Its body reads it raw — `lower_dynamic_arith` treats a bare
    // `any` as a machine integer — so boxing the literal there is what made
    // `(define (h y) (+ y 1)) (h 1)` compute `8 + 1 = 9`.
    let mut lisp_arg_regs: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        let is_builtin = lisp_builtin_name(instr).is_some() || is_lisp_arg_only_builtin(instr);
        if !is_builtin && !is_lisp_function_call(instr, lisp_funcs) {
            continue;
        }
        for (k, src) in instr.srcs.iter().enumerate().skip(1) {
            if !is_builtin && boundary.is_raw_only_arg(instr, k) {
                continue;
            }
            if let Operand::Var(v) = src {
                lisp_arg_regs.insert(v.clone());
            }
        }
    }

    // ── 2. Classify which registers hold a tagged `DynValue` (no mutation). ──
    //
    // Seeds (DVAL01 §3.3 — **producer-agnostic**):
    //   • **any op whose result type is a `DynValue`** (`any` / `ref<any>`) — a
    //     `dyn_pair_first`/`dyn_pair_rest` result, a re-boxed `dyn_box_int` arithmetic result,
    //     a user lambda `call` returning the polymorphic `any`, … The register
    //     holds a tagged word because of *what it is*, not because its producer's
    //     name is on a hard-coded lisp allow-list. This is the generalisation
    //     that lets a boxed *arithmetic* result be exit-unboxed like any other
    //     `DynValue` (the concrete failure the dynamic-arithmetic slice hit).
    //   • a lisp-builtin result whose hint is *not* itself `ref<any>` — namely
    //     `dyn_pair_new` / nil, typed `ref<LispyPair>` — still seeded by name.
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
            //
            //   • `ref<any>` is **always** a genuine tagged heap value — a
            //     `dyn_pair_first`/`dyn_pair_rest` result, a re-boxed `dyn_box_int` arithmetic
            //     result (E6d-2b). It is *never* a placeholder, so seed it
            //     unconditionally — this is what lets a **Twig** dynamic-arith
            //     result (`(+ (car …) 1)`) be exit-unboxed on the tagged-i64
            //     backends, not just McCarthy Lisp.
            //   • bare `any` is a genuine DynValue *inside a lisp module* but a
            //     pre-resolution placeholder on ordinary machine values in
            //     Twig/Nib (see the language gate in `lower_dyn_repr`), so its
            //     seed stays gated on `is_lisp`.
            let hint = instr.type_hint.as_str();
            if hint == REF_ANY {
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

    boxed_regs
}

/// Box the returned register of a tagged-boundary function when it still holds a
/// raw machine scalar, so the value crossing the boundary is a `LispyValue`.
fn box_raw_scalar_return(
    func: &mut IIRFunction,
    boxed_regs: &HashSet<String>,
    tagged_params: &HashSet<String>,
) {
    let Some(ret_pos) = func.instructions.iter().position(|i| i.op == "ret") else {
        return;
    };
    let Some(Operand::Var(ret_reg)) = func.instructions[ret_pos].srcs.first().cloned() else {
        return;
    };
    // Already tagged — a `boxed_regs` value, or a copy (`let`, an `if` join)
    // of one or of a parameter that arrives tagged — is returned unchanged.
    // `(define (g x c) (k 0) (if c x x))` with every caller passing a cell
    // used to box the copy of `x` and forge `p << 3`.
    if is_tagged_copy(func, &ret_reg, tagged_params, boxed_regs, &mut HashSet::new()) {
        return;
    }
    // A tagged PARAMETER is already a `LispyValue`; `boxed_regs` is built from
    // instructions and so never contains one. `((LAMBDA (X) X) 5)` returns its
    // parameter untouched, and boxing it a second time made the entry's single
    // unbox leave `5 << 3 = 40`.
    //
    // A bare-`any` parameter every call site provably passes tagged
    // ([`tagged_at_every_site`]) is the same: `(define (g x) (first …) x)
    // (g 5)` receives the boxed `40` and returned `40 << 3`. (Both are in
    // `tagged_params` only while nothing reassigns them.)
    let param_is_tagged = tagged_params.contains(&ret_reg) && !is_reassigned(func, &ret_reg);
    if param_is_tagged {
        return;
    }
    let boxed = format!("{ret_reg}.retbox");
    func.instructions.insert(
        ret_pos,
        IIRInstr::new("box", Some(boxed.clone()), vec![Operand::Var(ret_reg)], REF_ANY),
    );
    let ret = &mut func.instructions[ret_pos + 1];
    ret.srcs = vec![Operand::Var(boxed)];
    ret.type_hint = REF_ANY.to_string();
}

/// Rewrite each `jmp_if_false %cond, label` whose `%cond` holds a tagged
/// `LispyValue` into `%t = truthy(%cond); jmp_if_false %t, label`, so the
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

    // E6d-6: a `jmp_if_false` guard that is a *boxed machine bool* (a boxed
    // comparison result — `= tag …` in a `match`) must branch on its raw truth
    // value, NOT go through Lisp truthiness. A boxed `#f` is `dyn_box_int(0)` = a
    // tagged integer 0, and truthiness reads an integer atom as true (only
    // `#f`/nil are false in McCarthy), so it would wrongly take the arm — the
    // native twin of the managed `dyn_repr_structural` bug. We map each such guard
    // to its raw pre-box `bool` register and repoint the jump to it. (`box` is
    // still `box` here — the `dyn_box_int` rename runs after this pass.)
    let boxed_bool: HashMap<String, String> = func
        .instructions
        .iter()
        .filter(|i| i.op == "jmp_if_false")
        .filter_map(|i| match i.srcs.first() {
            Some(Operand::Var(c)) => Some(c.clone()),
            _ => None,
        })
        .filter_map(|c| {
            let boxed = func.instructions.iter().find(|i| i.dest.as_deref() == Some(&c))?;
            if boxed.op != "box" {
                return None;
            }
            match boxed.srcs.first() {
                Some(Operand::Var(src))
                    if func
                        .instructions
                        .iter()
                        .find(|i| i.dest.as_deref() == Some(src))
                        .map(|i| i.type_hint == "bool")
                        .unwrap_or(false) =>
                {
                    Some((c, src.clone()))
                }
                _ => None,
            }
        })
        .collect();

    let old = std::mem::take(&mut func.instructions);
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    let mut truthy_counter = 0usize;

    for mut instr in old {
        // A boxed-bool guard: repoint to the raw `bool`, emit the jump unwrapped.
        if instr.op == "jmp_if_false" {
            if let Some(Operand::Var(c)) = instr.srcs.first().cloned() {
                if let Some(raw) = boxed_bool.get(&c) {
                    instr.srcs[0] = Operand::Var(raw.clone());
                    new_instrs.push(instr);
                    continue;
                }
            }
        }
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
                // %t = call __lisp_truthy, %cond  : i64
                new_instrs.push(IIRInstr::new(
                    "call",
                    Some(t_reg.clone()),
                    vec![Operand::Var(TRUTHY_HELPER.to_string()), Operand::Var(cond)],
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
    // its own. Returning such a value runs it through `__lisp_exit_code`, a
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
    // a boolean result is coerced with Lisp truthiness (→ raw `0`/`1`) instead of
    // `dyn_unbox_int`. A register is "boolean" if the instruction that produced
    // it carries the `bool` type hint (the predicates do).
    // A "boolean" reg is one typed `bool`, OR one produced by a tagged-bool
    // **predicate** builtin (`null?`/`pair?`/`not`/`equal?`). The predicates return
    // a tagged `#t`(5)/`#f`(3), and the frontend often types their result `any`
    // rather than `bool` — so keying on the type hint alone misses a top-level
    // `(null? …)` and it wrongly falls to the `unbox_int` path (`5 >> 3 = 0`, i.e.
    // *true* exits as 0). Recognising them by name routes them to truthiness.
    const BOOL_PREDICATES: &[&str] = &["dyn_is_nil", "dyn_is_pair"];
    let bool_regs: HashSet<String> = func
        .instructions
        .iter()
        .filter(|i| {
            i.type_hint == "bool"
                || matches!(i.srcs.first(), Some(Operand::Var(v))
                    if (i.op == "call_builtin" && BOOL_PREDICATES.contains(&v.as_str()))
                        || (i.op == "call" && LISP_VALUE_HELPERS.contains(&v.as_str())))
        })
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
                //   • a lambda / user-`call` result (tag unknown) → `__lisp_exit_code`
                //     (a RUNTIME tag switch — int/bool/symbol/pair all handled);
                //   • a boolean (predicate) result → `__lisp_truthy` (→ 0/1);
                //   • otherwise a boxed integer → `dyn_unbox_int` (→ `>> 3`).
                // All three yield a raw `i64`.
                // The first two are Lisp's rules, so module-local helper
                // calls (`call`); unboxing is a representation primitive
                // (`call_builtin`).
                let (op, coerce) = if call_result_regs.contains(&reg) {
                    ("call", EXIT_CODE_HELPER)
                } else if bool_regs.contains(&reg) {
                    ("call", TRUTHY_HELPER)
                } else {
                    ("call_builtin", UNBOX_BUILTIN)
                };
                // %u = <op> "<coerce>", %reg  : i64
                let u_reg = format!("__unbox_{unbox_counter}");
                unbox_counter += 1;
                new_instrs.push(IIRInstr::new(
                    op,
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

// ---------------------------------------------------------------------------
// Call boundaries: which representation does each value cross in?
// ---------------------------------------------------------------------------
//
// A Twig parameter the frontend could not type is stamped bare `any`. That one
// hint covers two physically different words: a heap pointer (`p | 0b111`),
// which is the *same* word in the raw and the tagged model, and an integer,
// which is `n` raw but `n << 3` tagged. Nothing in the IR says which one a
// given `any` holds, so converting one blindly is unsound in both directions —
// boxing a pointer (`p << 3`) or unboxing one (`p >> 3`) makes the runtime's
// `*(v & ~7)` read wild memory, and leaving a raw integer where a tagged word
// is expected is no better (a raw `15` has the pair tag `0b111`).
//
// So this pass converts only on a **proof**, and every proof is computed by
// the SAME functions the rewrite later uses, against the SAME predicted
// register classification (`CallBoundary::predicted`, from
// [`tagged_registers`]). Two rules keep the proofs honest:
//
// * a parameter register is only "the argument" while nothing in the body
//   writes it — Twig's `let` reuses the source name as the register, so
//   `(let ((x 15)) x)` inside `(define (g x) …)` overwrites `x`
//   ([`is_reassigned`]);
// * an integer constant the rewrite boxes (step 3) is tagged, not raw — so
//   "will be boxed" never disagrees with what is boxed.
//
// What it proves:
// * **raw-only** parameters ([`raw_only_params`]) are read by nothing but
//   machine arithmetic. They can never be dereferenced, so the caller leaves
//   an integer literal raw for them (step 1) and unboxes an argument it can
//   prove is a tagged integer (step 3b).
// * a boundary function returns a parameter unchanged instead of boxing it
//   again only when every call site provably passes it tagged
//   ([`tagged_at_every_site`]).
//
// Everything else keeps its previous lowering, which is the honest statement
// of what remains: a value known only as `any` (a forward global, an `any`
// call result) still crosses as-is, and `box_raw_scalar_return` still boxes an
// unproven returned value as before (unsound for a pointer, as it was).
//
// What this pass deliberately does NOT do: box a computed raw value on its way
// to a tagged position (a `cons` operand, an `equal?` operand, the argument of
// `pair?`/`null?`, a parameter only those read). Every such value can end up
// in, or be compared with, a cons cell, and cells have no single
// representation on this pipeline yet — main boxes an integer *literal* stored
// into a cell but stores a computed value raw, and the readers are split the
// same way. Converting one side made a correct-by-coincidence program wrong:
// `(pair? 7)` became correctly false, which exposed a raw cell read that main
// had been reading "right" by accident — and any wrong answer can be guarded
// into a dead-branch dereference (`(if (= W(e) e) A (car 15))`). That work
// waits for one cell representation (#18451, #18480).

/// Ops that consume a raw machine word in every operand position and never
/// dereference it.
const MACHINE_OPS: &[&str] = &[
    "add", "sub", "mul", "div", "mod", "neg", "and", "or", "xor", "shl", "shr",
];

fn is_machine_op(op: &str) -> bool {
    MACHINE_OPS.contains(&op) || op.starts_with("cmp_")
}

/// A concrete machine-scalar type hint: an integer or a boolean.
fn is_machine_scalar_hint(hint: &str) -> bool {
    matches!(hint, "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64")
}

/// Is a parameter declared `ty` tagged by its declaration alone?
fn is_tagged_param_type(ty: &str) -> bool {
    ty == SYMBOL_HINT || ty.starts_with("ref<")
}

/// Does any instruction in `func` write register `reg`? A parameter that is
/// written — `(let ((x 15)) x)` reuses the parameter's name — no longer holds
/// the caller's argument at every read, so no fact about the argument may be
/// carried to it.
fn is_reassigned(func: &IIRFunction, reg: &str) -> bool {
    func.instructions.iter().any(|i| i.dest.as_deref() == Some(reg))
}

/// The index of parameter `reg` when it is a parameter nothing reassigns.
fn unwritten_param(func: &IIRFunction, reg: &str) -> Option<usize> {
    let i = func.params.iter().position(|(name, _)| name == reg)?;
    (!is_reassigned(func, reg)).then_some(i)
}

/// Where a value provably comes from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Source {
    /// A tagged word: a `box` result, a boxed constant, or a heap read.
    Tagged,
    /// The function's own (never reassigned) parameter at this index.
    Param(usize),
}

/// Everything `lower_dyn_repr_function` needs to know about the module's call
/// boundaries. Built in [`CallBoundary::analyse`].
struct CallBoundary {
    lisp_funcs: HashSet<String>,
    /// Per function, per parameter: raw-only? (See [`raw_only_params`].)
    raw_only: HashMap<String, Vec<bool>>,
    /// Per function: the `boxed_regs` step 2 will compute for it.
    predicted: HashMap<String, HashSet<String>>,
    /// Per function: the integer constants step 3 will box (tagged words).
    predicted_consts: HashMap<String, HashSet<String>>,
    /// Per function whose every return is provable: where the result comes from.
    summaries: HashMap<String, HashSet<Source>>,
    /// Off-boundary functions whose every return is a provably raw integer.
    raw_fns: HashSet<String>,
    /// Per boundary function, per parameter: does every call site provably
    /// pass a tagged word? (See [`tagged_at_every_site`].)
    tagged_at_every_site: HashMap<String, Vec<bool>>,
}

impl CallBoundary {
    fn analyse(module: &IIRModule, lisp_funcs: HashSet<String>) -> Self {
        let raw_only = raw_only_params(module, &lisp_funcs);
        let mut b = CallBoundary {
            raw_only,
            predicted: HashMap::new(),
            predicted_consts: HashMap::new(),
            summaries: HashMap::new(),
            raw_fns: HashSet::new(),
            tagged_at_every_site: HashMap::new(),
            lisp_funcs,
        };
        // Predict step 2 exactly: run it on a copy after step 0's rename.
        let predicted = module
            .functions
            .iter()
            .map(|f| {
                let mut copy = f.clone();
                rename_lisp_not(&mut copy);
                (f.name.clone(), tagged_registers(&copy, &b))
            })
            .collect();
        b.predicted = predicted;
        b.predicted_consts = module
            .functions
            .iter()
            .map(|f| (f.name.clone(), b.compute_boxed_consts(f)))
            .collect();
        // Summaries, raw-returning functions and tagged-at-every-site flags
        // feed each other; each only ever grows as facts are proven, so iterate
        // to the joint fixpoint (bounded, as a guard).
        for _ in 0..64 {
            let raw_fns = raw_returning_functions(module, &b);
            let flags = tagged_at_every_site(module, &b);
            let summaries = tagged_return_summaries(module, &b);
            let stable =
                raw_fns == b.raw_fns && flags == b.tagged_at_every_site && summaries == b.summaries;
            b.raw_fns = raw_fns;
            b.tagged_at_every_site = flags;
            b.summaries = summaries;
            if stable {
                break;
            }
        }
        b
    }

    /// The integer constants step 3 boxes in `func` (they hold tagged words),
    /// as predicted before any rewrite.
    fn boxed_consts(&self, func: &IIRFunction) -> HashSet<String> {
        self.predicted_consts.get(&func.name).cloned().unwrap_or_default()
    }

    fn compute_boxed_consts(&self, func: &IIRFunction) -> HashSet<String> {
        let Some(regs) = self.predicted.get(&func.name) else { return HashSet::new() };
        func.instructions
            .iter()
            .filter(|i| {
                i.op == "const"
                    && i.type_hint != SYMBOL_HINT
                    && !i.type_hint.starts_with("ref<")
                    && matches!(i.srcs.first(), Some(Operand::Int(n))
                        if (INT_MIN_BOXABLE..=INT_MAX_BOXABLE).contains(n))
            })
            .filter_map(|i| i.dest.clone())
            .filter(|d| regs.contains(d))
            .collect()
    }

    fn flag(map: &HashMap<String, Vec<bool>>, instr: &IIRInstr, k: usize) -> bool {
        if instr.op != "call" || k == 0 {
            return false;
        }
        let Some(Operand::Var(callee)) = instr.srcs.first() else { return false };
        map.get(callee).and_then(|f| f.get(k - 1)).copied().unwrap_or(false)
    }

    /// Is argument `k` (an index into `instr.srcs`) of this `call` bound for a
    /// raw-only parameter?
    fn is_raw_only_arg(&self, instr: &IIRInstr, k: usize) -> bool {
        Self::flag(&self.raw_only, instr, k)
    }

}

/// Is argument `k` (an index into `instr.srcs`) of this `call` handed straight
/// back as the call's result by a callee that does nothing else with it?
fn is_passthrough_arg(
    instr: &IIRInstr,
    k: usize,
    passthrough: &HashMap<String, Vec<bool>>,
) -> bool {
    CallBoundary::flag(passthrough, instr, k)
}

/// The registers that are copies of `root`, transitively, plus `root`: `mov`
/// destinations, and the result of a call that passes it straight back
/// (`(id y)` — see [`passthrough_params`]).
fn value_aliases(
    func: &IIRFunction,
    root: &str,
    passthrough: &HashMap<String, Vec<bool>>,
) -> HashSet<String> {
    let mut aliases: HashSet<String> = HashSet::from([root.to_string()]);
    loop {
        let before = aliases.len();
        for instr in &func.instructions {
            let Some(dest) = &instr.dest else { continue };
            let copies = instr.srcs.iter().enumerate().any(|(k, src)| {
                matches!(src, Operand::Var(v) if aliases.contains(v))
                    && ((instr.op == "mov" && k == 0) || is_passthrough_arg(instr, k, passthrough))
            });
            if copies {
                aliases.insert(dest.clone());
            }
        }
        if aliases.len() == before {
            return aliases;
        }
    }
}

/// The parameters each **off-boundary** function only returns: the parameter
/// is never reassigned, and every read of it, or of a `mov` copy of it, is a
/// `mov` or a `ret`. Such a callee (`(define (id x) x)`, the `a`/`b` of
/// `(define (pick c a b) (if c a b))`) neither converts nor dereferences the
/// value. Boundary functions are excluded: `box_raw_scalar_return` may box
/// what they return.
fn passthrough_params(
    module: &IIRModule,
    lisp_funcs: &HashSet<String>,
) -> HashMap<String, Vec<bool>> {
    let none = HashMap::new();
    module
        .functions
        .iter()
        .filter(|f| !lisp_funcs.contains(&f.name))
        .map(|func| {
            let flags = func
                .params
                .iter()
                .map(|(name, _)| {
                    if is_reassigned(func, name) {
                        return false;
                    }
                    let names = value_aliases(func, name, &none);
                    func.instructions.iter().all(|instr| {
                        instr.op == "mov"
                            || instr.op == "ret"
                            || !instr
                                .srcs
                                .iter()
                                .any(|s| matches!(s, Operand::Var(v) if names.contains(v)))
                    })
                })
                .collect();
            (func.name.clone(), flags)
        })
        .collect()
}

/// Candidate flags for a parameter-use analysis: bare-`any` parameters that
/// nothing in the body reassigns.
fn any_param_candidates(func: &IIRFunction) -> Vec<bool> {
    func.params
        .iter()
        .map(|(name, ty)| ty == ANY_HINT && !is_reassigned(func, name))
        .collect()
}

/// The **raw-only** parameters of every function.
///
/// A bare-`any`, never reassigned parameter is raw-only when every read of
/// it, or of a copy of it, is (a) an operand of a [machine op](MACHINE_OPS),
/// (b) an argument to a call whose callee parameter at that position is itself
/// raw-only, or (c) an argument a [passthrough](passthrough_params) callee only
/// hands back — whose result then counts as a copy. A greatest fixpoint:
/// anything else (a heap builtin, a `ret`, a branch condition, a `box`, a
/// store, another call) strikes it out.
///
/// Off-boundary functions are candidates too: their callers never box a
/// literal for them (step 1 only boxes for boundary callees), but a boxed
/// computed value still reaches them — the recursive `(loop (- n 1))` passes
/// `box(n - 1)`, which compared with `0` forever until the stack overflowed —
/// so step 3b unboxes for them as well.
fn raw_only_params(
    module: &IIRModule,
    lisp_funcs: &HashSet<String>,
) -> HashMap<String, Vec<bool>> {
    let passthrough = passthrough_params(module, lisp_funcs);
    let mut raw_only: HashMap<String, Vec<bool>> = HashMap::new();
    let mut aliases: HashMap<(String, usize), HashSet<String>> = HashMap::new();
    for func in &module.functions {
        let flags = any_param_candidates(func);
        for (i, (name, _)) in func.params.iter().enumerate() {
            if flags[i] {
                aliases.insert((func.name.clone(), i), value_aliases(func, name, &passthrough));
            }
        }
        raw_only.insert(func.name.clone(), flags);
    }

    loop {
        let mut struck: Vec<(String, usize)> = Vec::new();
        for func in &module.functions {
            let flags = &raw_only[&func.name];
            for (i, &candidate) in flags.iter().enumerate() {
                if !candidate {
                    continue;
                }
                let names = &aliases[&(func.name.clone(), i)];
                let ok = func.instructions.iter().all(|instr| {
                    instr.srcs.iter().enumerate().all(|(k, src)| {
                        let Operand::Var(v) = src else { return true };
                        if !names.contains(v) || (k == 0 && instr.op == "call") {
                            return true;
                        }
                        instr.op == "mov"
                            || is_machine_op(&instr.op)
                            || is_passthrough_arg(instr, k, &passthrough)
                            || CallBoundary::flag(&raw_only, instr, k)
                    })
                });
                if !ok {
                    struck.push((func.name.clone(), i));
                }
            }
        }
        if struck.is_empty() {
            return raw_only;
        }
        for (name, i) in struck {
            raw_only.get_mut(&name).expect("candidate's function")[i] = false;
        }
    }
}

/// Does `reg` of `func` provably hold a **raw machine integer** (or boolean)
/// at every point, *after* this pass's rewrite?
///
/// True only when every producer is one of: an integer constant step 3 does
/// not box; an op typed with a machine-scalar hint (a typed `add`/`cmp_*`,
/// `str_len`) or a non-`dyn_*` builtin with such a hint; a call to a
/// raw-returning function, or to a passthrough callee whose handed-back
/// argument is itself provably raw and not converted on the way in; a `mov` of
/// one of these; or a never-reassigned parameter *declared* with a
/// machine-scalar type. Never an `unbox` result, never anything known only as
/// bare `any` — that may be a pointer.
fn holds_provably_raw(
    func: &IIRFunction,
    reg: &str,
    b: &CallBoundary,
    boxed_consts: &HashSet<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(reg.to_string()) {
        return true; // a `mov` cycle: decided by its other producers.
    }
    if let Some((_, ty)) = func.params.iter().find(|(name, _)| name == reg) {
        return !is_reassigned(func, reg) && is_machine_scalar_hint(ty);
    }
    let mut producers = func
        .instructions
        .iter()
        .filter(|i| i.dest.as_deref() == Some(reg))
        .peekable();
    if producers.peek().is_none() {
        return false;
    }
    producers.all(|instr| match instr.op.as_str() {
        "const" => {
            matches!(instr.srcs.first(), Some(Operand::Int(n))
                if (INT_MIN_BOXABLE..=INT_MAX_BOXABLE).contains(n))
                && !instr.type_hint.starts_with("ref<")
                && instr.type_hint != SYMBOL_HINT
                && !boxed_consts.contains(reg)
        }
        "mov" => match instr.srcs.first() {
            Some(Operand::Var(src)) => holds_provably_raw(func, src, b, boxed_consts, visited),
            _ => false,
        },
        "call" => {
            let Some(Operand::Var(callee)) = instr.srcs.first() else { return false };
            if b.raw_fns.contains(callee) {
                return true;
            }
            if b.lisp_funcs.contains(callee) {
                return false;
            }
            let Some(sources) = b.summaries.get(callee) else { return false };
            !sources.is_empty()
                && sources.iter().all(|s| match s {
                    Source::Tagged => false,
                    Source::Param(j) => matches!(instr.srcs.get(j + 1), Some(Operand::Var(a))
                        if holds_provably_raw(func, a, b, boxed_consts, visited)),
                })
        }
        "call_builtin" => {
            is_machine_scalar_hint(&instr.type_hint)
                && !matches!(instr.srcs.first(), Some(Operand::Var(n)) if n.starts_with("dyn_"))
        }
        // Any other producer typed as a machine scalar — a typed `add`, `cmp_*`,
        // `str_len`, `load_byte` — except `unbox`, whose raw result is meant for
        // a raw consumer and must not be re-boxed behind its back.
        op => op != "unbox" && is_machine_scalar_hint(&instr.type_hint),
    })
}

/// The off-boundary, non-entry-relevant functions whose every `ret` is
/// [provably raw](holds_provably_raw). Boundary functions are excluded:
/// `box_raw_scalar_return` boxes their raw results.
fn raw_returning_functions(module: &IIRModule, b: &CallBoundary) -> HashSet<String> {
    module
        .functions
        .iter()
        .filter(|f| !b.lisp_funcs.contains(&f.name))
        .filter(|func| {
            let consts = b.boxed_consts(func);
            let mut rets = func.instructions.iter().filter(|i| i.op == "ret").peekable();
            rets.peek().is_some()
                && rets.all(|r| match r.srcs.first() {
                    Some(Operand::Var(v)) => {
                        holds_provably_raw(func, v, b, &consts, &mut HashSet::new())
                    }
                    _ => false,
                })
        })
        .map(|f| f.name.clone())
        .collect()
}

/// Trace `reg` in `func` to the [`Source`]s it provably comes from, or `None`
/// when any producer is not provable.
///
/// A cell read (`dyn_pair_first`/`dyn_pair_rest`/`field_load`) is never a
/// source: a cell may hold a boxed literal or a raw computed word.
/// An earlier draft counted cons-cell reads as tagged and unboxed them for a
/// raw-only parameter, which misread every raw-stored value — and any such
/// wrong answer can be guarded into a dead-branch dereference.
fn trace_sources(
    func: &IIRFunction,
    reg: &str,
    b: &CallBoundary,
    boxed_consts: &HashSet<String>,
    visited: &mut HashSet<String>,
) -> Option<HashSet<Source>> {
    if !visited.insert(reg.to_string()) {
        return Some(HashSet::new()); // a `mov` cycle: decided by its other producers.
    }
    if func.params.iter().any(|(name, _)| name == reg) {
        return unwritten_param(func, reg).map(|i| HashSet::from([Source::Param(i)]));
    }
    let mut out: HashSet<Source> = HashSet::new();
    let mut any_producer = false;
    for instr in func.instructions.iter().filter(|i| i.dest.as_deref() == Some(reg)) {
        any_producer = true;
        match instr.op.as_str() {
            "box" => {
                out.insert(Source::Tagged);
            }
            "const" if boxed_consts.contains(reg) => {
                out.insert(Source::Tagged);
            }
            "mov" => {
                let Some(Operand::Var(src)) = instr.srcs.first() else { return None };
                out.extend(trace_sources(func, src, b, boxed_consts, visited)?);
            }
            "call" => {
                let Some(Operand::Var(callee)) = instr.srcs.first() else { return None };
                for source in b.summaries.get(callee)? {
                    match source {
                        Source::Tagged => {
                            out.insert(Source::Tagged);
                        }
                        // The callee hands back its own argument. A raw-only
                        // position may have been unboxed on the way in (3b):
                        // unprovable.
                        Source::Param(j) => {
                            let k = j + 1;
                            if b.is_raw_only_arg(instr, k) {
                                return None;
                            }
                            let Some(Operand::Var(arg)) = instr.srcs.get(k) else { return None };
                            out.extend(trace_sources(func, arg, b, boxed_consts, visited)?);
                        }
                    }
                }
            }
            _ => return None,
        }
    }
    any_producer.then_some(out)
}

/// Is `reg` of a tagged-boundary function only ever a copy (`mov`) of
/// parameters in `tagged_params`, of registers in `boxed_regs`, or both?
/// `box_raw_scalar_return` returns such a value unchanged.
fn is_tagged_copy(
    func: &IIRFunction,
    reg: &str,
    tagged_params: &HashSet<String>,
    boxed_regs: &HashSet<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(reg.to_string()) {
        return true;
    }
    if boxed_regs.contains(reg) {
        return true;
    }
    if func.params.iter().any(|(name, _)| name == reg) {
        return tagged_params.contains(reg) && !is_reassigned(func, reg);
    }
    let mut producers = func
        .instructions
        .iter()
        .filter(|i| i.dest.as_deref() == Some(reg))
        .peekable();
    producers.peek().is_some()
        && producers.all(|i| {
            i.op == "mov"
                && matches!(i.srcs.first(), Some(Operand::Var(src))
                    if is_tagged_copy(func, src, tagged_params, boxed_regs, visited))
        })
}

/// The parameters of `func` that are tagged when they arrive: declared
/// `ref<…>`/`symbol`, or flagged by [`tagged_at_every_site`] (never
/// reassigned).
fn arriving_tagged_params(func: &IIRFunction, b: &CallBoundary) -> HashSet<String> {
    let flags = b.tagged_at_every_site.get(&func.name);
    func.params
        .iter()
        .enumerate()
        .filter(|(i, (name, ty))| {
            !is_reassigned(func, name)
                && (is_tagged_param_type(ty)
                    || flags.and_then(|f| f.get(*i)).copied().unwrap_or(false))
        })
        .map(|(_, (name, _))| name.clone())
        .collect()
}

/// For every function whose every `ret` is provable, where its result comes
/// from. A tagged-boundary function is summarised as it is AFTER
/// `box_raw_scalar_return` (which touches only its first `ret`): a value that
/// pass returns unchanged is traced; a provably raw one it boxes is
/// [`Source::Tagged`]; anything it would box without proof is unprovable.
fn tagged_return_summaries(
    module: &IIRModule,
    b: &CallBoundary,
) -> HashMap<String, HashSet<Source>> {
    let mut summaries: HashMap<String, HashSet<Source>> = HashMap::new();
    for func in &module.functions {
        let consts = b.boxed_consts(func);
        let rets: Vec<&IIRInstr> = func.instructions.iter().filter(|i| i.op == "ret").collect();
        let on_boundary = b.lisp_funcs.contains(&func.name);
        if rets.is_empty() || (on_boundary && rets.len() != 1) {
            continue;
        }
        let mut sources: HashSet<Source> = HashSet::new();
        let provable = rets.iter().all(|r| {
            let Some(Operand::Var(v)) = r.srcs.first() else { return false };
            let traced = if on_boundary {
                let boxed_regs = b.predicted.get(&func.name).cloned().unwrap_or_default();
                let tagged = arriving_tagged_params(func, b);
                if is_tagged_copy(func, v, &tagged, &boxed_regs, &mut HashSet::new()) {
                    trace_sources(func, v, b, &consts, &mut HashSet::new())
                } else if holds_provably_raw(func, v, b, &consts, &mut HashSet::new()) {
                    Some(HashSet::from([Source::Tagged]))
                } else {
                    None
                }
            } else {
                trace_sources(func, v, b, &consts, &mut HashSet::new())
            };
            match traced {
                Some(s) if !s.is_empty() => {
                    sources.extend(s);
                    true
                }
                _ => false,
            }
        });
        if provable {
            summaries.insert(func.name.clone(), sources);
        }
    }
    summaries
}

/// The bare-`any` parameters of tagged-boundary functions that **every** call
/// site provably passes as a tagged word, so the callee may return them
/// unchanged instead of boxing them a second time.
///
/// "Tagged after lowering" is decided with the functions the rewrite itself
/// uses: a constant step 3 boxes, a `box`/lisp-builtin result, or the caller's own parameter
/// flagged the same way. Nothing reached through a `mov` join and nothing
/// reassigned qualifies. A function with no direct call site (a closure body)
/// gets no flags. A greatest fixpoint, for forwarding cycles.
fn tagged_at_every_site(module: &IIRModule, b: &CallBoundary) -> HashMap<String, Vec<bool>> {
    let mut flags: HashMap<String, Vec<bool>> = HashMap::new();
    for func in &module.functions {
        if !b.lisp_funcs.contains(&func.name) {
            continue;
        }
        let has_site = module.functions.iter().any(|g| {
            g.instructions.iter().any(|i| {
                i.op == "call" && matches!(i.srcs.first(), Some(Operand::Var(c)) if *c == func.name)
            })
        });
        let raw_only = b.raw_only.get(&func.name);
        let f: Vec<bool> = any_param_candidates(func)
            .into_iter()
            .enumerate()
            .map(|(i, c)| c && has_site && !raw_only.and_then(|r| r.get(i)).copied().unwrap_or(false))
            .collect();
        flags.insert(func.name.clone(), f);
    }
    loop {
        let mut struck: Vec<(String, usize)> = Vec::new();
        for caller in &module.functions {
            let consts = b.boxed_consts(caller);
            for instr in &caller.instructions {
                if instr.op != "call" {
                    continue;
                }
                let Some(Operand::Var(callee)) = instr.srcs.first() else { continue };
                let Some(callee_flags) = flags.get(callee) else { continue };
                for (i, &tagged) in callee_flags.iter().enumerate() {
                    if !tagged {
                        continue;
                    }
                    let k = i + 1;
                    let ok = match instr.srcs.get(k) {
                        Some(Operand::Var(v)) => {
                            consts.contains(v) || directly_tagged(caller, v, &flags)
                        }
                        _ => false,
                    };
                    if !ok {
                        struck.push((callee.clone(), i));
                    }
                }
            }
        }
        if struck.is_empty() {
            return flags;
        }
        for (name, i) in struck {
            flags.get_mut(&name).expect("flagged function")[i] = false;
        }
    }
}

/// Is `reg` a tagged word by its own producer — every producer a `box` or a
/// lisp builtin that makes a fresh tagged word — or the caller's never-reassigned parameter that is
/// tagged on arrival? (No `mov` joins: a join may mix in an unproven value.)
fn directly_tagged(func: &IIRFunction, reg: &str, flags: &HashMap<String, Vec<bool>>) -> bool {
    if let Some(i) = func.params.iter().position(|(name, _)| name == reg) {
        return !is_reassigned(func, reg)
            && (is_tagged_param_type(&func.params[i].1)
                || flags.get(&func.name).and_then(|f| f.get(i)).copied().unwrap_or(false));
    }
    let mut producers = func
        .instructions
        .iter()
        .filter(|i| i.dest.as_deref() == Some(reg))
        .peekable();
    // Fresh tagged words only: `box`, a new cell, a predicate's `#t`/`#f`. Not
    // a heap READ (`dyn_pair_first`/`dyn_pair_rest`): a cell may hold a value
    // stored raw.
    producers.peek().is_some()
        && producers.all(|i| {
            i.op == "box"
                || matches!(lisp_builtin_name(i), Some(n) if n != "dyn_pair_first" && n != "dyn_pair_rest")
        })
}

/// Step 3b: before a call, unbox each argument bound for a raw-only parameter
/// that is provably a tagged integer — `(h (h 1))` passes `h`'s boxed result
/// straight back into `h`'s raw-only `y`, which read it as `2 << 3`.
///
/// "Provably" means [`trace_sources`] finds only [`Source::Tagged`]: a
/// caller-side parameter or anything unproven is left alone. Unboxing is safe
/// even if the tagged word were not an integer, because a raw-only parameter
/// is only ever fed to arithmetic, never dereferenced.
fn unbox_tagged_args_for_raw_only_params(func: &mut IIRFunction, b: &CallBoundary) {
    let snapshot = func.clone();
    let consts = b.boxed_consts(&snapshot);
    let is_tagged = |v: &str| {
        trace_sources(&snapshot, v, b, &consts, &mut HashSet::new())
            .is_some_and(|s| !s.is_empty() && s.iter().all(|x| *x == Source::Tagged))
    };
    let targets: Vec<(usize, usize)> = snapshot
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, instr)| instr.op == "call")
        .flat_map(|(at, instr)| {
            (1..instr.srcs.len())
                .filter(|&k| b.is_raw_only_arg(instr, k))
                .filter(|&k| matches!(&instr.srcs[k], Operand::Var(v) if is_tagged(v)))
                .map(move |k| (at, k))
                .collect::<Vec<_>>()
        })
        .collect();
    if targets.is_empty() {
        return;
    }
    let old = std::mem::take(&mut func.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() + targets.len());
    for (at, mut instr) in old.into_iter().enumerate() {
        for &(_, k) in targets.iter().filter(|(t, _)| *t == at) {
            let Operand::Var(v) = instr.srcs[k].clone() else { continue };
            let raw = format!("{v}.argraw{at}_{k}");
            out.push(IIRInstr::new("unbox", Some(raw.clone()), vec![Operand::Var(v)], "i64"));
            instr.srcs[k] = Operand::Var(raw);
        }
        out.push(instr);
    }
    func.instructions = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lisp_semantics::{EQUAL, EXIT_CODE, NOT, TRUTHY};
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
            release: None,
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
    /// `call_builtin name` or (for a Lisp rule, LANG83 PR7f-2) `call name`
    /// instructions in `main`.
    fn count_builtin(m: &IIRModule, name: &str) -> usize {
        m.functions[0]
            .instructions
            .iter()
            .filter(|i| {
                (i.op == "call_builtin" || i.op == "call")
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

    /// `(CAR (CONS 7 9))`: 7 and 9 feed dyn_pair_new → boxed; the car result is
    /// returned by the entry fn → unboxed.
    #[test]
    fn cons_car_boxes_ints_and_unboxes_result() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "dyn_pair_new", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("r"), "dyn_pair_first", &["cell"], "any"),
            ret("r"),
        ]);
        lower_dyn_repr_marked(&mut m);
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
    /// exit-unboxed, exactly like a `dyn_pair_first` result. Before the producer-
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
        lower_dyn_repr_marked(&mut m);
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

    /// A top-level `null?` (a tagged-`#t`/`#f` predicate result typed `any`) must be
    /// exit-coerced with `truthiness (`__lisp_truthy`)`, NOT `dyn_unbox_int` — otherwise a true result
    /// (`#t = 5`) is unboxed to `5 >> 3 = 0` and the program wrongly exits 0.
    /// Regression for `(null? (list))` exiting 0 instead of 1 on native / LLVM.
    #[test]
    fn top_level_null_p_predicate_is_truthy_coerced_not_unboxed() {
        let mut m = module(vec![
            konst("nil", 0, "ref<LispyPair>"),
            // the frontend types the predicate result `any`, not `bool`
            call_builtin(Some("b"), "dyn_is_nil", &["nil"], "any"),
            ret("b"),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(
            count_builtin(&m, TRUTHY), 1,
            "a predicate result at the exit boundary is truthy-coerced",
        );
        assert_eq!(
            count_builtin(&m, "dyn_unbox_int"), 0,
            "it must NOT be unboxed (that would map #t=5 to 0)",
        );
    }

    /// The producer-agnostic seed stays gated on the source language: a **Twig**
    /// module (which uses `any` as a pre-resolution placeholder on ordinary
    /// machine values) is a no-op — no boxing, no unbox inserted.
    #[test]
    fn twig_any_hint_is_not_boxed() {
        let mut m = module(vec![konst("v", 42, "any"), ret("v")]);
        m.language = "twig".into();
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "v"), 42, "Twig `any` value must not be boxed");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "no unbox in a Twig module");
        assert_eq!(m.functions[0].instructions.len(), 2, "Twig module left unchanged");
    }

    /// A bare integer `42`: the constant never reaches a `dyn_*` call, so it
    /// is not boxed, and the ret is not unboxed — exit 42 unchanged.
    #[test]
    fn scalar_int_is_left_raw() {
        let mut m = module(vec![konst("v0", 42, "i64"), ret("v0")]);
        lower_dyn_repr_marked(&mut m);
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
        lower_dyn_repr_marked(&mut m);
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
            call_builtin(Some("cell"), "dyn_pair_new", &["v", "n"], "ref<LispyPair>"),
            ret("cell"),
        ]);
        lower_dyn_repr_marked(&mut m);
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
                call_builtin(Some("cell"), "dyn_pair_new", &["v0", "v0"], "ref<LispyPair>"),
                ret("cell"),
            ],
        );
        let mut m = module(vec![konst("z", 0, "i64"), ret("z")]);
        m.functions.push(callee);
        lower_dyn_repr_marked(&mut m);
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
        // Pass 1: rename cons/car → dyn_pair_new/dyn_pair_first.
        crate::heap::lower_heap_builtins_runtime(&mut m);
        // Pass 2: box atoms + unbox result.
        lower_dyn_repr_marked(&mut m);

        assert_eq!(find_const(&m, "v0"), 7 << 3);
        assert_eq!(find_const(&m, "v1"), 9 << 3);
        let instrs = &m.functions[0].instructions;
        // cons/car are now dyn_*.
        assert_eq!(instrs[2].srcs[0], Operand::Var("dyn_pair_new".into()));
        assert_eq!(instrs[3].srcs[0], Operand::Var("dyn_pair_first".into()));
        // The tail is: unbox(r) ; ret %unbox.
        let last = instrs.last().unwrap();
        assert_eq!(last.op, "ret");
        let unbox = &instrs[instrs.len() - 2];
        assert_eq!(unbox.srcs[0], Operand::Var("dyn_unbox_int".into()));
    }

    // ── L3b-2c-2: ATOM/EQ predicates + COND truthiness ──────────────────

    /// `(ATOM 5)` lowers (after the rename) to `not(pair?(5))`. The `5` feeds
    /// `dyn_is_pair`, so it must box.
    #[test]
    fn predicate_arg_int_is_boxed() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "dyn_is_pair", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "v0"), 5 << 3, "ATOM's int atom must box");
    }

    /// A `jmp_if_false` whose condition is a tagged value (a predicate result)
    /// gets a `truthiness (`__lisp_truthy`)` normaliser inserted before it.
    #[test]
    fn tagged_cond_is_wrapped_with_truthy() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "dyn_is_pair", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            jmp_if_false("a", "L_next"),
            IIRInstr::new("label", None, vec![Operand::Var("L_next".into())], "void"),
            ret("a"),
        ]);
        lower_dyn_repr_marked(&mut m);
        // TWO `truthiness (`__lisp_truthy`)` calls now: (1) the `COND` clause-test wrap, and
        // (2) the bool-typed `ret a` result coercion (McCarthy W12b-2 — a
        // boolean program result becomes raw 0/1 via `truthy`, not `unbox_int`).
        assert_eq!(count_builtin(&m, TRUTHY), 2, "cond test + bool result both wrapped");
        // The boolean result is coerced with `truthiness (`__lisp_truthy`)`, NOT `dyn_unbox_int`.
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
    /// run through `truthiness (`__lisp_truthy`)` (→ raw 0/1). Unboxing `LISPY_TRUE` (=5) would
    /// give `5 >> 3 = 0` — *wrong* for true. So a bool result must never be unboxed.
    #[test]
    fn integer_result_unboxed_boolean_result_truthied() {
        // (CAR (CONS 7 9)) — an integer result → unbox, NOT truthy.
        let mut int_m = module(vec![
            konst("a", 56, "i64"),
            konst("b", 72, "i64"),
            call_builtin(Some("p"), "dyn_pair_new", &["a", "b"], "ref<LispyPair>"),
            call_builtin(Some("h"), "dyn_pair_first", &["p"], "i64"),
            ret("h"),
        ]);
        lower_dyn_repr_marked(&mut int_m);
        assert_eq!(count_builtin(&int_m, "dyn_unbox_int"), 1, "int result is unboxed");
        assert_eq!(count_builtin(&int_m, TRUTHY), 0, "int result is not truthied");

        // (ATOM 7) = (not (pair? 7)) — a boolean result → truthy, NOT unbox.
        let mut bool_m = module(vec![
            konst("v0", 56, "i64"),
            call_builtin(Some("p"), "dyn_is_pair", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr_marked(&mut bool_m);
        assert_eq!(count_builtin(&bool_m, TRUTHY), 1, "bool result is truthied");
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
        lower_dyn_repr_marked(&mut m);
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "a symbol result is NOT unboxed");
        assert_eq!(count_builtin(&m, TRUTHY), 0, "a symbol result is NOT truthied");
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
            release: None,
        }
    }

    /// `main { a = 5; r = <callee>(a); ret r }` plus an identity `<callee>(X){ ret X }`.
    /// An `identity(X)` called with the integer 5, where the callee's boundary
    /// type is the caller's choice. That type IS the contract now: `ref<any>`
    /// declares a tagged value, a raw type declares a machine word, and the pass
    /// reads it from the signature rather than from the module's language.
    fn apply_identity_module_typed(callee: &str, language: &str, boundary: &str) -> IIRModule {
        let id = IIRFunction::new(
            callee,
            vec![("X".into(), boundary.into())],
            boundary,
            vec![ret("X")],
        );
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

    /// The tagged-boundary case (a McCarthy `LAMBDA`).
    fn apply_identity_module(callee: &str, language: &str) -> IIRModule {
        apply_identity_module_typed(callee, language, "ref<any>")
    }

    /// McCarthy W13b (F7): a `call` to a lisp function boxes its integer atom
    /// argument (`5 << 3 = 40`) and coerces its polymorphic result at the program
    /// exit with the runtime tag switch ``__lisp_exit_code``.
    #[test]
    fn lambda_call_boxes_int_arg_and_coerces_result() {
        let mut m = apply_identity_module("lambda_0", "mccarthy-lisp");
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "a"), 40, "int atom arg must box before the lambda");
        assert_eq!(count_builtin(&m, EXIT_CODE), 1, "lambda result → to_exit_code");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "polymorphic result is NOT unboxed");
    }

    /// The same IIR shape in a **Twig** module is left completely untouched: an
    /// untyped Twig param is also `any`, but the source language is not lisp, so
    /// the int arg stays raw and the result is returned verbatim. (Guards the
    /// regression where `(define (fib n) …)` was mis-boxed.)
    #[test]
    fn non_lisp_call_is_left_untouched() {
        // A Twig `(define (fib n) …)`: the parameter is statically unresolved,
        // stamped bare `any`, and passed as a raw machine word.
        let mut m = apply_identity_module_typed("fib", "twig", "any");
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "a"), 5, "a Twig int arg stays a raw machine word");
        assert_eq!(count_builtin(&m, EXIT_CODE), 0, "a Twig call is never coerced");
        assert_eq!(count_builtin(&m, "dyn_box_int"), 0, "a Twig arg is never boxed");
    }

    /// A raw machine condition (e.g. a Twig `cmp` result, not in boxed_regs)
    /// is left alone — no `truthiness (`__lisp_truthy`)` wrap.
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
        lower_dyn_repr_marked(&mut m);
        assert_eq!(count_builtin(&m, TRUTHY), 0, "raw cmp cond must NOT be wrapped");
    }

    /// A COND result funnelled by `mov` from a lisp value (a `car` result)
    /// must be recognised as tagged and unboxed at the entry ret.
    #[test]
    fn mov_propagates_taggedness_for_unbox() {
        let mut m = module(vec![
            konst("v0", 7, "i64"),
            konst("v1", 9, "i64"),
            call_builtin(Some("cell"), "dyn_pair_new", &["v0", "v1"], "ref<LispyPair>"),
            call_builtin(Some("c"), "dyn_pair_first", &["cell"], "any"),
            mov("result", "c"),
            ret("result"),
        ]);
        lower_dyn_repr_marked(&mut m);
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
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "seven"), 7 << 3, "clause literal must box (mov-tied to nil)");
        assert_eq!(find_const(&m, "nilv"), TAG_NIL, "nil tagged");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 1, "result must unbox at ret");
    }

    /// ATOM shape `not(pair?(x))`: the generic `not` whose arg is a `dyn_*`
    /// result is renamed to `__lisp_not`.
    #[test]
    fn not_after_lispy_result_becomes_lispy_not() {
        let mut m = module(vec![
            konst("v0", 5, "i64"),
            call_builtin(Some("p"), "dyn_is_pair", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(count_builtin(&m, NOT), 1, "ATOM's not must become __lisp_not");
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
        lower_dyn_repr_marked(&mut m);
        assert_eq!(count_builtin(&m, "not"), 1, "Twig's machine not must be left alone");
        assert_eq!(count_builtin(&m, NOT), 0);
    }

    /// A symbol immediate (`const Int(id<<32|2):symbol`, from `intern_symbols`)
    /// is treated as a tagged value but must NOT be boxed — shifting it would
    /// corrupt the id/tag. Here `'A` feeds `__lisp_equal`, so it would be a box
    /// candidate but for the symbol guard.
    #[test]
    fn symbol_immediate_is_tagged_but_not_boxed() {
        let sym_bits = (1_i64 << 32) | 0b010; // id 1, symbol tag
        let mut m = module(vec![
            IIRInstr::new("const", Some("s".into()), vec![Operand::Int(sym_bits)], "symbol"),
            call_builtin(Some("e"), "equal?", &["s", "s"], "bool"),
            ret("e"),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "s"), sym_bits, "symbol immediate must be left unboxed");
    }

    /// Out-of-range integers (beyond ±2⁶⁰) are left unboxed rather than
    /// silently truncated by the shift.
    #[test]
    fn out_of_range_int_is_not_boxed() {
        let huge = (1_i64 << 61) | 1;
        let mut m = module(vec![
            konst("v0", huge, "i64"),
            call_builtin(Some("cell"), "dyn_pair_new", &["v0", "v0"], "ref<LispyPair>"),
            ret("cell"),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "v0"), huge, "out-of-range int must be left raw");
    }

    // ── Call boundaries: raw-only parameters (iir-builtin-lowering 0.43.0) ──

    fn var(v: &str) -> Operand {
        Operand::Var(v.into())
    }

    fn op(op: &str, dest: &str, srcs: &[&str], ty: &str) -> IIRInstr {
        IIRInstr::new(op, Some(dest.into()), srcs.iter().map(|s| var(s)).collect(), ty)
    }

    fn call(dest: &str, callee: &str, args: &[&str], ty: &str) -> IIRInstr {
        let mut srcs = vec![var(callee)];
        srcs.extend(args.iter().map(|a| var(a)));
        IIRInstr::new("call", Some(dest.into()), srcs, ty)
    }

    fn func_of<'a>(m: &'a IIRModule, name: &str) -> &'a IIRFunction {
        m.functions.iter().find(|f| f.name == name).expect("function present")
    }

    fn const_of(f: &IIRFunction, dest: &str) -> i64 {
        match f.instructions.iter().find(|i| i.dest.as_deref() == Some(dest)) {
            Some(IIRInstr { srcs, .. }) => match srcs.first() {
                Some(Operand::Int(n)) => *n,
                other => panic!("{dest} is not an int const: {other:?}"),
            },
            None => panic!("no {dest}"),
        }
    }

    fn twig(functions: Vec<IIRFunction>) -> IIRModule {
        IIRModule {
            name: "test".into(),
            functions,
            entry_point: Some("main".into()),
            language: "twig".into(),
            exports: vec![],
            imports: vec![],
            release: None,
        }
    }

    /// `(define (h y) (+ y 1))` as `lower_dynamic_arith` leaves it: `y` read raw
    /// by `add`, the sum re-boxed, so `h` returns `ref<any>` (tagged boundary).
    fn inc() -> IIRFunction {
        IIRFunction::new(
            "h",
            vec![("y".into(), "any".into())],
            "ref<any>",
            vec![
                konst("one", 1, "i64"),
                op("add", "s.raw", &["y", "one"], "i64"),
                op("box", "s", &["s.raw"], "ref<any>"),
                IIRInstr::new("ret", None, vec![var("s")], "any"),
            ],
        )
    }

    fn main_fn(instrs: Vec<IIRInstr>) -> IIRFunction {
        IIRFunction::new("main", vec![], "any", instrs)
    }

    /// Regression: `(h 1)` exited 9 on native/LLVM — the literal was boxed for
    /// a parameter the body only reads as a machine integer.
    #[test]
    fn a_literal_for_a_raw_only_parameter_stays_raw() {
        let mut m = twig(vec![
            inc(),
            main_fn(vec![konst("a", 1, "i64"), call("r", "h", &["a"], "ref<any>"), ret("r")]),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(const_of(func_of(&m, "main"), "a"), 1);
        assert!(!func_of(&m, "h").instructions.iter().any(|i| i.op == "unbox"));
    }

    /// The guard: a parameter that feeds `cons` must still receive a tagged word.
    #[test]
    fn a_literal_for_a_heap_parameter_is_still_boxed() {
        let f = IIRFunction::new(
            "f",
            vec![("x".into(), "any".into())],
            "ref<any>",
            vec![
                konst("nil", 0, REF_PAIR),
                call_builtin(Some("c"), "dyn_pair_new", &["x", "nil"], "ref<any>"),
                ret("c"),
            ],
        );
        let mut m = twig(vec![
            f,
            main_fn(vec![konst("a", 5, "i64"), call("r", "f", &["a"], "ref<any>"), ret("r")]),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(const_of(func_of(&m, "main"), "a"), 40);
    }

    /// A parameter used both arithmetically and as a heap value is NOT raw-only:
    /// its contract is left exactly as before (the literal is boxed).
    #[test]
    fn a_mixed_use_parameter_keeps_the_tagged_contract() {
        let f = IIRFunction::new(
            "f",
            vec![("x".into(), "any".into())],
            "ref<any>",
            vec![
                konst("one", 1, "i64"),
                op("add", "unused", &["x", "one"], "i64"),
                konst("nil", 0, REF_PAIR),
                call_builtin(Some("c"), "dyn_pair_new", &["x", "nil"], "ref<any>"),
                ret("c"),
            ],
        );
        let mut m = twig(vec![
            f,
            main_fn(vec![konst("a", 5, "i64"), call("r", "f", &["a"], "ref<any>"), ret("r")]),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(const_of(func_of(&m, "main"), "a"), 40);
    }

    /// `mov` copies (`let`), forwarding to another raw-only parameter (`k`), and
    /// an identity call (`(id y)`) all keep a parameter raw-only.
    #[test]
    fn copies_and_forwarding_keep_a_parameter_raw_only() {
        let k = IIRFunction::new(
            "k",
            vec![("y".into(), "any".into())],
            "ref<any>",
            vec![op("mov", "z", &["y"], "any"), call("r", "h", &["z"], "ref<any>"), ret("r")],
        );
        let id = IIRFunction::new("id", vec![("x".into(), "any".into())], "any", vec![ret("x")]);
        let j = IIRFunction::new(
            "j",
            vec![("y".into(), "any".into())],
            "ref<any>",
            vec![
                call("c", "id", &["y"], "any"),
                konst("one", 1, "i64"),
                op("add", "s.raw", &["c", "one"], "i64"),
                op("box", "s", &["s.raw"], "ref<any>"),
                ret("s"),
            ],
        );
        let module = twig(vec![inc(), k, id, j]);
        let lisp = crate::dyn_repr_structural::tagged_boundary_functions(&module);
        let raw_only = raw_only_params(&module, &lisp);
        assert_eq!(raw_only["h"], vec![true]);
        assert_eq!(raw_only["k"], vec![true], "forwarded to a raw-only parameter");
        assert_eq!(raw_only["j"], vec![true], "an identity call's result is a copy");
        assert_eq!(raw_only["id"], vec![false], "returning a value is not arithmetic");
    }

    /// Escapes disqualify: a closure capture (`box`), a branch condition, or an
    /// argument to an off-boundary function that might dereference it.
    #[test]
    fn an_escaping_parameter_is_not_raw_only() {
        let capture = IIRFunction::new(
            "cap",
            vec![("x".into(), "any".into())],
            "ref<any>",
            vec![op("box", "b", &["x"], "ref<any>"), ret("b")],
        );
        let deref = IIRFunction::new(
            "n",
            vec![("p".into(), "any".into())],
            "i64",
            vec![call_builtin(Some("t"), "dyn_is_nil", &["p"], "ref<any>"), ret("t")],
        );
        let passes = IIRFunction::new(
            "q",
            vec![("y".into(), "any".into())],
            "ref<any>",
            vec![call("t", "n", &["y"], "i64"), ret("t")],
        );
        let module = twig(vec![capture, deref, passes]);
        let lisp = crate::dyn_repr_structural::tagged_boundary_functions(&module);
        let raw_only = raw_only_params(&module, &lisp);
        assert_eq!(raw_only["cap"], vec![false]);
        assert_eq!(raw_only["n"], vec![false]);
        assert_eq!(raw_only["q"], vec![false]);
    }

    /// `(h (h 1))` and `(h (id (h 1)))`: an argument PROVABLY tagged (a boxing
    /// callee's result, directly or through an identity call) is unboxed for a
    /// raw-only parameter (was 73 on native/LLVM).
    #[test]
    fn a_provably_tagged_argument_is_unboxed_for_a_raw_only_parameter() {
        let id = IIRFunction::new("id", vec![("x".into(), "any".into())], "any", vec![ret("x")]);
        let mut m = twig(vec![
            inc(),
            id,
            main_fn(vec![
                konst("a", 1, "i64"),
                call("inner", "h", &["a"], "ref<any>"),
                call("same", "id", &["inner"], "any"),
                call("r", "h", &["same"], "ref<any>"),
                ret("r"),
            ]),
        ]);
        lower_dyn_repr_marked(&mut m);
        let main = func_of(&m, "main");
        // The last call of `h` (the exit coercion after it is a `call` too).
        let outer = main.instructions.iter().rev().find(|i| i.srcs.first() == Some(&var("h"))).unwrap();
        let Operand::Var(arg) = &outer.srcs[1] else { panic!("var arg") };
        let producer = main
            .instructions
            .iter()
            .find(|i| i.dest.as_deref() == Some(arg.as_str()))
            .unwrap();
        assert_eq!(producer.op, "unbox");
        assert_eq!(producer.srcs, vec![var("same")]);
    }

    /// Unprovable values are never converted: a raw computed value, a global
    /// read, a caller parameter, and — the memory-safety case — a pointer
    /// through an identity function bound for a non-raw-only parameter.
    #[test]
    fn an_unprovable_argument_is_passed_as_is() {
        let id = IIRFunction::new("id", vec![("x".into(), "any".into())], "any", vec![ret("x")]);
        let first = IIRFunction::new(
            "first",
            vec![("p".into(), "any".into())],
            "ref<any>",
            vec![call_builtin(Some("v"), "dyn_pair_first", &["p"], "ref<any>"), ret("v")],
        );
        let mut m = twig(vec![
            inc(),
            id,
            first,
            main_fn(vec![
                konst("a", 1, "i64"),
                konst("b", 2, "i64"),
                op("add", "sum", &["a", "b"], "i64"),
                call("r1", "h", &["sum"], "ref<any>"),
                IIRInstr::new("global_load", Some("g".into()), vec![Operand::Str("g".into())], "any"),
                call("r2", "h", &["g"], "ref<any>"),
                konst("nil", 0, REF_PAIR),
                call_builtin(Some("cell"), "dyn_pair_new", &["a", "nil"], "ref<any>"),
                call("p", "id", &["cell"], "any"),
                call("r3", "first", &["p"], "ref<any>"),
                ret("r3"),
            ]),
        ]);
        lower_dyn_repr_marked(&mut m);
        let main = func_of(&m, "main");
        assert!(
            !main.instructions.iter().any(|i| i.op == "unbox" || i.op == "box"),
            "nothing is converted: {:?}",
            main.instructions
        );
    }

    /// An off-boundary function's raw-only parameter receives `box(n - 1)` from
    /// its own recursive call; it is unboxed (was a stack overflow).
    #[test]
    fn a_boxed_recursive_argument_is_unboxed_off_the_boundary() {
        let lp = IIRFunction::new(
            "loop",
            vec![("n".into(), "any".into())],
            "any",
            vec![
                konst("one", 1, "i64"),
                op("sub", "d.raw", &["n", "one"], "i64"),
                op("box", "d", &["d.raw"], "ref<any>"),
                call("r", "loop", &["d"], "any"),
                ret("r"),
            ],
        );
        let mut m = twig(vec![lp, main_fn(vec![ret("unused")])]);
        lower_dyn_repr_marked(&mut m);
        let lp = func_of(&m, "loop");
        let c = lp.instructions.iter().find(|i| i.op == "call").unwrap();
        let Operand::Var(arg) = &c.srcs[1] else { panic!("var arg") };
        assert!(arg.contains(".argraw"), "the recursive argument is unboxed: {arg}");
    }

    /// A McCarthy-style `ref<any>` parameter is tagged by declaration: never a
    /// raw-only candidate, so its literal is boxed exactly as before.
    #[test]
    fn a_ref_any_parameter_is_never_raw_only() {
        let lam = IIRFunction::new(
            "lam",
            vec![("x".into(), "ref<any>".into())],
            "ref<any>",
            vec![op("unbox", "u", &["x"], "i64"), op("add", "s", &["u", "u"], "i64"), ret("s")],
        );
        let mut m = twig(vec![
            lam,
            main_fn(vec![konst("a", 5, "i64"), call("r", "lam", &["a"], "ref<any>"), ret("r")]),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(const_of(func_of(&m, "main"), "a"), 40);
    }

    // ── Cells and tagged positions keep main's representation; tagged-at-every-site ──

    /// Nothing computed is boxed on its way to a tagged position: a raw
    /// integer stored into `cons` stays raw (#18427 needs a uniform cell
    /// representation, #18451, #18480), and so does the same value passed to `pair?`.
    /// Boxing it for `pair?` alone made `(pair? 7)` correctly false — and so
    /// exposed a raw cell read that main had been reading right by accident.
    #[test]
    fn a_computed_raw_integer_is_never_boxed_for_a_tagged_position() {
        let mut m = twig(vec![main_fn(vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HELLO".into())], "str"),
            op("str_len", "n", &["s"], "i64"),
            konst("nil", 0, REF_PAIR),
            call_builtin(Some("c"), "dyn_pair_new", &["n", "nil"], "ref<any>"),
            call_builtin(Some("p"), "dyn_is_pair", &["n"], "ref<any>"),
            ret("p"),
        ])]);
        lower_dyn_repr_marked(&mut m);
        let main = func_of(&m, "main");
        assert!(!main.instructions.iter().any(|i| i.op == "box"), "{:?}", main.instructions);
        for name in ["dyn_pair_new", "dyn_is_pair"] {
            let call = main.instructions.iter().find(|i| i.srcs.first() == Some(&var(name))).unwrap();
            assert_eq!(call.srcs[1], var("n"), "{name} gets the raw word, as on main");
        }
    }

    /// Never boxed for a tagged position: an `unbox` result (deliberately raw)
    /// and a bare-`any` parameter (it may be a pointer).
    #[test]
    fn an_unbox_result_or_an_any_parameter_is_never_boxed() {
        let f = IIRFunction::new(
            "f",
            vec![("x".into(), "any".into()), ("t".into(), "ref<any>".into())],
            "ref<any>",
            vec![
                op("unbox", "u", &["t"], "i64"),
                konst("nil", 0, REF_PAIR),
                call_builtin(Some("c1"), "dyn_pair_new", &["u", "nil"], "ref<any>"),
                call_builtin(Some("c2"), "dyn_pair_new", &["x", "c1"], "ref<any>"),
                ret("c2"),
            ],
        );
        let mut m = twig(vec![f, main_fn(vec![ret("unused")])]);
        lower_dyn_repr_marked(&mut m);
        assert!(!func_of(&m, "f").instructions.iter().any(|i| i.op == "box"));
    }

    /// A record constructor is off the tagged boundary and its fields are raw
    /// words: a literal argument stays raw, and a provably raw value stored by
    /// `field_store` is not boxed (its accessor's callers read it raw).
    #[test]
    fn a_record_constructor_keeps_raw_fields() {
        let point = IIRFunction::new(
            "Point",
            vec![("x".into(), "any".into())],
            "any",
            vec![
                IIRInstr::new("alloc", Some("cell".into()), vec![], REF_PAIR),
                IIRInstr::new(
                    "field_store",
                    None,
                    vec![var("cell"), Operand::Int(0), var("x")],
                    "void",
                ),
                konst("seven", 7, "i64"),
                IIRInstr::new(
                    "field_store",
                    None,
                    vec![var("cell"), Operand::Int(1), var("seven")],
                    "void",
                ),
                ret("cell"),
            ],
        );
        let mut m = twig(vec![
            point,
            main_fn(vec![konst("a", 42, "i64"), call("p", "Point", &["a"], "any"), ret("p")]),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(const_of(func_of(&m, "main"), "a"), 42);
        assert!(!func_of(&m, "Point").instructions.iter().any(|i| i.op == "box"));
    }

    /// A parameter written by a `let` that reuses its name no longer holds the
    /// argument: `(define (g x) (k 0) (let ((x 15)) x))` must still box the
    /// raw 15 on return (it was returned raw and `pair?` dereferenced it).
    #[test]
    fn a_reassigned_parameter_is_never_trusted() {
        let k = IIRFunction::new(
            "k",
            vec![("y".into(), "ref<any>".into())],
            "ref<any>",
            vec![ret("y")],
        );
        let g = IIRFunction::new(
            "g",
            vec![("x".into(), "any".into())],
            "any",
            vec![
                konst("nil", 0, REF_PAIR),
                call("ignored", "k", &["nil"], "ref<any>"),
                konst("x", 15, "i64"),
                ret("x"),
            ],
        );
        let mut m = twig(vec![
            k,
            g,
            main_fn(vec![konst("a", 3, "i64"), call("r", "g", &["a"], "any"), ret("r")]),
        ]);
        let module = m.clone();
        let lisp = crate::dyn_repr_structural::tagged_boundary_functions(&module);
        let b = CallBoundary::analyse(&module, lisp);
        assert_eq!(b.tagged_at_every_site["g"], vec![false], "x is written by the body");
        assert!(!b.summaries.get("g").is_some_and(|s| s.contains(&Source::Param(0))));
        lower_dyn_repr_marked(&mut m);
        let g = func_of(&m, "g");
        let r = g.instructions.iter().find(|i| i.op == "ret").unwrap();
        let Operand::Var(v) = &r.srcs[0] else { panic!("var") };
        assert!(v.ends_with(".retbox"), "the reassigned value is boxed on return: {v}");
    }

    /// A provably raw value passed to a parameter the callee only tests with
    /// `pair?` is passed as on main — raw — even when a `mov` join also put it
    /// in `boxed_regs`; and the parameter is then not "tagged at every site",
    /// so the callee keeps main's return handling.
    #[test]
    fn a_raw_value_for_a_type_predicate_parameter_is_passed_as_on_main() {
        let k = IIRFunction::new(
            "k",
            vec![("y".into(), "ref<any>".into())],
            "ref<any>",
            vec![ret("y")],
        );
        let g = IIRFunction::new(
            "g",
            vec![("x".into(), "any".into())],
            "ref<any>",
            vec![
                konst("nil", 0, REF_PAIR),
                call("ignored", "k", &["nil"], "ref<any>"),
                call_builtin(Some("c"), "dyn_is_pair", &["x"], "ref<any>"),
                ret("c"),
            ],
        );
        let mut m = twig(vec![
            k,
            g,
            main_fn(vec![
                konst("a", 7, "i64"),
                konst("b", 8, "i64"),
                op("add", "v", &["a", "b"], "i64"),
                konst("nil", 0, REF_PAIR),
                konst("one", 1, "i64"),
                call_builtin(Some("c"), "dyn_pair_new", &["one", "nil"], "ref<any>"),
                call_builtin(Some("hd"), "dyn_pair_first", &["c"], "ref<any>"),
                op("mov", "r", &["v"], "any"),
                op("mov", "r", &["hd"], "any"),
                call("w", "g", &["v"], "any"),
                ret("w"),
            ]),
        ]);
        lower_dyn_repr_marked(&mut m);
        let main = func_of(&m, "main");
        assert!(!main.instructions.iter().any(|i| i.op == "box"), "{:?}", main.instructions);
        let c = main.instructions.iter().find(|i| i.srcs.first() == Some(&var("g"))).unwrap();
        assert_eq!(c.srcs[1], var("v"), "v reaches g raw, as on main");
    }

    /// Round-4 finding: a provably raw argument for a MIXED-use parameter (read
    /// by `cmp_eq` and returned) is not boxed — the body compares it raw, and a
    /// boxed 15 took the dead `(car l)` branch and dereferenced an integer.
    #[test]
    fn a_raw_value_for_a_mixed_use_parameter_is_not_boxed() {
        let k = IIRFunction::new("k", vec![("y".into(), "ref<any>".into())], "ref<any>", vec![ret("y")]);
        let f = IIRFunction::new(
            "f",
            vec![("x".into(), "any".into()), ("l".into(), "any".into())],
            "any",
            vec![
                konst("nil", 0, REF_PAIR),
                call("ignored", "k", &["nil"], "ref<any>"),
                konst("fifteen", 15, "i64"),
                op("cmp_eq", "t", &["x", "fifteen"], "bool"),
                call_builtin(Some("hd"), "dyn_pair_first", &["l"], "ref<any>"),
                ret("x"),
            ],
        );
        let mut m = twig(vec![
            k,
            f,
            main_fn(vec![
                konst("a", 7, "i64"),
                konst("b", 8, "i64"),
                op("add", "v", &["a", "b"], "i64"),
                konst("l", 15, "i64"),
                call("r", "f", &["v", "l"], "any"),
                ret("r"),
            ]),
        ]);
        let module = m.clone();
        let lisp = crate::dyn_repr_structural::tagged_boundary_functions(&module);
        assert!(lisp.contains("f"));
        lower_dyn_repr_marked(&mut m);
        let main = func_of(&m, "main");
        let c = main.instructions.iter().find(|i| i.srcs.first() == Some(&var("f"))).unwrap();
        assert_eq!(c.srcs[1], var("v"), "x is compared raw, so its argument is not boxed");
    }

    /// Finding-4 shape: a copy (`let`, `if` join) of a parameter every site
    /// passes tagged is returned unchanged, not boxed into a forged pointer.
    #[test]
    fn a_copy_of_a_tagged_parameter_is_returned_unchanged() {
        let k = IIRFunction::new(
            "k",
            vec![("y".into(), "ref<any>".into())],
            "ref<any>",
            vec![ret("y")],
        );
        let g = IIRFunction::new(
            "g",
            vec![("x".into(), "any".into())],
            "any",
            vec![
                konst("nil", 0, REF_PAIR),
                call("ignored", "k", &["nil"], "ref<any>"),
                op("mov", "y", &["x"], "any"),
                ret("y"),
            ],
        );
        let mut m = twig(vec![
            k,
            g,
            main_fn(vec![
                konst("seven", 7, "i64"),
                konst("nil", 0, REF_PAIR),
                call_builtin(Some("c"), "dyn_pair_new", &["seven", "nil"], "ref<any>"),
                call("r", "g", &["c"], "any"),
                ret("r"),
            ]),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert!(!func_of(&m, "g").instructions.iter().any(|i| i.op == "box"));
    }

    /// `(define (g x) (first (cons 1 nil)) x) (g 5)`: every site passes `x`
    /// tagged (the literal is boxed), so `g` returns it unchanged — and only
    /// then is `g` summarised as handing back its argument.
    #[test]
    fn a_parameter_tagged_at_every_site_is_returned_unchanged() {
        let first = IIRFunction::new(
            "first",
            vec![("p".into(), "ref<any>".into())],
            "ref<any>",
            vec![call_builtin(Some("v"), "dyn_pair_first", &["p"], "ref<any>"), ret("v")],
        );
        let g = IIRFunction::new(
            "g",
            vec![("x".into(), "any".into())],
            "any",
            vec![
                konst("nil", 0, REF_PAIR),
                call("ignored", "first", &["nil"], "ref<any>"),
                ret("x"),
            ],
        );
        let mut m = twig(vec![
            first,
            g,
            main_fn(vec![konst("a", 5, "i64"), call("r", "g", &["a"], "any"), ret("r")]),
        ]);
        lower_dyn_repr_marked(&mut m);
        let g = func_of(&m, "g");
        assert!(!g.instructions.iter().any(|i| i.op == "box"), "no second box: {:?}", g.instructions);
        assert_eq!(const_of(func_of(&m, "main"), "a"), 40);
    }

    /// The counterpart: a site passing an unprovable value (a global read)
    /// keeps the old return-boxing, and no `Param` summary is given.
    #[test]
    fn a_parameter_with_an_unprovable_site_is_still_boxed_on_return() {
        let first = IIRFunction::new(
            "first",
            vec![("p".into(), "ref<any>".into())],
            "ref<any>",
            vec![call_builtin(Some("v"), "dyn_pair_first", &["p"], "ref<any>"), ret("v")],
        );
        let g = IIRFunction::new(
            "g",
            vec![("x".into(), "any".into())],
            "any",
            vec![
                konst("nil", 0, REF_PAIR),
                call("ignored", "first", &["nil"], "ref<any>"),
                ret("x"),
            ],
        );
        let module = twig(vec![
            first,
            g,
            main_fn(vec![
                IIRInstr::new("global_load", Some("gv".into()), vec![Operand::Str("gv".into())], "any"),
                call("r", "g", &["gv"], "any"),
                ret("r"),
            ]),
        ]);
        let mut m = module.clone();
        lower_dyn_repr_marked(&mut m);
        assert!(func_of(&m, "g").instructions.iter().any(|i| i.op == "box"));
    }

    /// LANG83 PR7f-2: the full pass writes the Lisp rules out. `(ATOM 7)` as a
    /// program result needs `not` and truthiness, both expanded inline, so
    /// `main` keeps no `call` and the module gains no function; `(EQ 'A 'A)`
    /// calls the module's own `__lisp_equal`; a lambda result goes through the
    /// module's `__lisp_exit_code`. No runtime call to a language rule is left.
    #[test]
    fn the_full_pass_writes_the_lisp_rules_as_module_code() {
        let mut atom = module(vec![
            konst("v0", 7, "i64"),
            call_builtin(Some("p"), "dyn_is_pair", &["v0"], "bool"),
            call_builtin(Some("a"), "not", &["p"], "bool"),
            ret("a"),
        ]);
        lower_dyn_repr(&mut atom).expect("lower_dyn_repr");
        assert_eq!(atom.functions.len(), 1, "inline rules add no function");
        assert!(atom.functions[0].instructions.iter().all(|i| i.op != "call"), "main stays call-free");
        // Truthiness compares the whole word (not `dyn_tag`): two `cmp_ne`s
        // for `not`, two for the exit truthiness.
        assert_eq!(count_builtin(&atom, "dyn_tag"), 0, "truthiness reads the whole word, not the tag");
        let word_tests = atom.functions[0].instructions.iter().filter(|i| i.op == "cmp_ne").count();
        assert_eq!(word_tests, 4, "x ≠ #f and x ≠ nil, for not and for the exit truthiness");

        let sym_bits = (1_i64 << 32) | 0b010;
        let mut eq = module(vec![
            IIRInstr::new("const", Some("s".into()), vec![Operand::Int(sym_bits)], "symbol"),
            call_builtin(Some("e"), "equal?", &["s", "s"], "any"),
            ret("e"),
        ]);
        lower_dyn_repr(&mut eq).expect("lower_dyn_repr");
        assert_eq!(count_builtin(&eq, EQUAL), 1);
        let names: Vec<&str> = eq.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["main", EQUAL]);

        let mut lambda = apply_identity_module("lambda_0", "mccarthy-lisp");
        lower_dyn_repr(&mut lambda).expect("lower_dyn_repr");
        assert_eq!(count_builtin(&lambda, EXIT_CODE), 1);
        assert!(lambda.functions.iter().any(|f| f.name == EXIT_CODE));

        for m in [&atom, &eq, &lambda] {
            for f in &m.functions {
                for i in &f.instructions {
                    let name = match i.srcs.first() {
                        Some(Operand::Var(n)) if i.op == "call_builtin" || i.op == "call" => n.as_str(),
                        _ => continue,
                    };
                    assert!(
                        !matches!(name, "dyn_truthy" | "dyn_not" | "dyn_equal" | "dyn_to_exit_code" | TRUTHY | NOT),
                        "{}: {name} survived",
                        f.name
                    );
                }
            }
        }
    }

    /// Running the pass on a module that already holds the helpers (twice, or
    /// on hand-built IIR) leaves the helpers' bodies alone: their constants
    /// are words, not atoms to box.
    #[test]
    fn helpers_already_in_the_module_are_not_rewritten() {
        let mut m = apply_identity_module("lambda_0", "mccarthy-lisp");
        lower_dyn_repr(&mut m).expect("lower_dyn_repr");
        let before = m.functions.iter().find(|f| f.name == EXIT_CODE).unwrap().clone();
        lower_dyn_repr(&mut m).expect("lower_dyn_repr");
        let after = m.functions.iter().find(|f| f.name == EXIT_CODE).unwrap();
        assert_eq!(format!("{:?}", before.instructions), format!("{:?}", after.instructions));
        assert_eq!(m.functions.iter().filter(|f| f.name == EXIT_CODE).count(), 1);
    }

    /// A program function that carries a helper's name — `(define
    /// (__lisp_equal a b) 42)` — is refused before anything is rewritten,
    /// instead of standing in for `equal?` (security review of PR7f-2).
    #[test]
    fn a_program_function_named_like_a_lisp_helper_is_refused() {
        let user_equal = IIRFunction::new(
            EQUAL,
            vec![("a".into(), "any".into()), ("b".into(), "any".into())],
            "any",
            vec![konst("k", 42, "i64"), ret("k")],
        );
        let main = vec![
            konst("x", 1, "i64"),
            konst("y", 2, "i64"),
            call_builtin(Some("e"), "equal?", &["x", "y"], "any"),
            ret("e"),
        ];
        let mut m = multi_fn_module(main, vec![user_equal], "twig");
        let before = format!("{:?}", m.functions.iter().map(|f| &f.instructions).collect::<Vec<_>>());
        let err = lower_dyn_repr(&mut m).expect_err("a user __lisp_equal must be refused");
        assert_eq!(err.name, EQUAL);
        assert_eq!(err.caller, None);
        assert!(err.to_string().contains("reserved"), "{err}");
        let after = format!("{:?}", m.functions.iter().map(|f| &f.instructions).collect::<Vec<_>>());
        assert_eq!(before, after, "a refused module is not rewritten");

        // The same for a definition of an inline rule's marker name.
        for name in [NOT, TRUTHY] {
            let f = IIRFunction::new(name, vec![("x".into(), "any".into())], "any", vec![ret("x")]);
            let mut m = multi_fn_module(vec![konst("z", 0, "i64"), ret("z")], vec![f], "twig");
            let err = lower_dyn_repr(&mut m).expect_err(name);
            assert_eq!((err.name.as_str(), err.caller), (name, None));
        }

        // Any other name with the prefix collides with nothing the pass
        // recognises, and this pass also runs on Nib's and ALGOL's modules:
        // it lowers. (Twig source still cannot spell it — its frontend
        // reserves the whole prefix.)
        let f = IIRFunction::new("__lisp_anything", vec![("x".into(), "any".into())], "any", vec![ret("x")]);
        let mut m = multi_fn_module(vec![konst("z", 0, "i64"), ret("z")], vec![f], "nib");
        lower_dyn_repr(&mut m).expect("an unreserved __lisp_ name lowers");
    }

    /// A program `call` of a reserved name the compiler did not write —
    /// `(__lisp_not 5)` — is refused rather than expanded as Lisp's `not`.
    #[test]
    fn a_program_call_of_a_lisp_rule_name_is_refused() {
        for name in [NOT, TRUTHY, EQUAL, EXIT_CODE] {
            let main = vec![
                konst("x", 5, "i64"),
                IIRInstr::new("call", Some("r".into()), vec![Operand::Var(name.into()), Operand::Var("x".into())], "any"),
                ret("r"),
            ];
            let mut m = module(main);
            let err = lower_dyn_repr(&mut m).expect_err(name);
            assert_eq!(err.name, name);
            assert_eq!(err.caller.as_deref(), Some("main"));
        }
    }

    /// The compiler's own helpers are recognised by their body, not their
    /// name: a lowered module lowers again, but a doctored copy of a helper
    /// (one instruction changed) is refused.
    #[test]
    fn only_the_compilers_own_helper_body_passes_the_reserved_name_check() {
        let mut m = apply_identity_module("lambda_0", "mccarthy-lisp");
        lower_dyn_repr(&mut m).expect("first lowering");
        assert!(lisp_semantics::check_reserved_names(&m).is_ok());
        let helper = m.functions.iter_mut().find(|f| f.name == EXIT_CODE).unwrap();
        let k = helper.instructions.iter().position(|i| i.op == "const").unwrap();
        helper.instructions[k].srcs[0] = Operand::Int(99);
        let err = lower_dyn_repr(&mut m).expect_err("a doctored helper must be refused");
        assert_eq!((err.name.as_str(), err.caller), (EXIT_CODE, None));
    }

    /// LANG83 PR7f: `dyn_tag`'s argument is a tagged value (an integer atom
    /// flowing in boxes, so its tag reads 0, not the `#t` tag the raw `5`
    /// would show), but its result is a plain integer: it is not boxed, and a
    /// `ret` of it in the entry function gets no exit coercion.
    #[test]
    fn dyn_tag_boxes_its_argument_but_not_its_result() {
        let mut m = module(vec![
            konst("v", 5, "i64"),
            call_builtin(Some("t"), "dyn_tag", &["v"], "i64"),
            ret("t"),
        ]);
        lower_dyn_repr_marked(&mut m);
        assert_eq!(find_const(&m, "v"), 5 << 3, "the atom must be boxed");
        assert_eq!(count_builtin(&m, "dyn_unbox_int"), 0, "the tag is already an integer");
        assert_eq!(count_builtin(&m, TRUTHY), 0);
        assert_eq!(count_builtin(&m, EXIT_CODE), 0);
    }
}

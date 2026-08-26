//! # dyn_repr_structural — lisp-value representation for the *managed* backends (LANG77 / L3b-3a-3c).
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
//! This pass is the managed-backend twin of [`crate::lower_dyn_repr`] (which
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

/// Lisp builtins that survive `lower_heap_builtins` as `call_builtin`s (the
/// cons data path — cons/car/cdr/null? — is already structural by now). Their
/// presence marks a function as using the lisp value model, so **this** pass —
/// not the scalar concretizer — owns it. Mirrors the `LISP_BUILTINS` list in
/// `lang-aot::concretize_scalar_any_for_wasm` so the two passes partition the
/// module's functions with no overlap or gap.
const LISP_BUILTINS: &[&str] = &["pair?", "not", "equal?", "make_symbol", "make_nil", "null?"];

/// The subset of [`LISP_BUILTINS`] whose **value** arguments are lisp values, so
/// an integer atom flowing into one must be boxed as an `i31ref` (exactly as an
/// atom stored into a cons field is). `not` is absent — it takes a machine
/// boolean (the `i32` result of a predicate), not a lisp value.
const LISP_VALUE_ARG_BUILTINS: &[&str] = &["pair?", "equal?"];

/// If `instr` is a `call_builtin` whose name is a lisp builtin, return that name.
fn lisp_builtin_name(instr: &IIRInstr) -> Option<&str> {
    if instr.op != "call_builtin" {
        return None;
    }
    match instr.srcs.first() {
        Some(Operand::Var(name)) if LISP_BUILTINS.contains(&name.as_str()) => Some(name.as_str()),
        _ => None,
    }
}

/// Whether a type hint denotes a value that lowers to a wasm `i32` (as opposed
/// to `i64`). Predicates produce `"bool"`, which the wasm backend maps to `i32`.
fn is_i32_width(hint: &str) -> bool {
    matches!(hint, "bool" | "i8" | "i16" | "i32" | "u8" | "u16" | "u32")
}

/// The type hint of the instruction that defines `reg`, if any.
fn producer_hint<'a>(func: &'a IIRFunction, reg: &str) -> Option<&'a str> {
    func.instructions
        .iter()
        .find(|i| i.dest.as_deref() == Some(reg))
        .map(|i| i.type_hint.as_str())
}

/// The `jmp_if_false` conditions that hold a **lisp value** rather than a
/// machine boolean — and so need a lisp-truthiness test (`COND` whose clause
/// guard is an atom, `nil`, a cons, or a variable, not a `pair?`/`EQ` result).
///
/// `jmp_if_false` branches when its condition is *false*, testing a raw `i32`
/// against zero. A predicate result (`pair?`/`not`/`equal?`, hint `"bool"`) is
/// exactly such an `i32`, so it is tested directly. But a lisp value is not: a
/// lisp integer atom is **always true** (even `0` — only `nil` is false in
/// McCarthy), and `nil` is a reference, not an `i32`. We detect a lisp-value
/// condition structurally (its producer's hint is not `"bool"`) and the rebuild
/// wraps it: `t = not(is_null(cond))` — i.e. `t` is 1 unless `cond` is `nil`.
fn lisp_conditions(func: &IIRFunction) -> HashSet<String> {
    let mut set = HashSet::new();
    for instr in &func.instructions {
        if instr.op == "jmp_if_false" {
            if let Some(Operand::Var(c)) = instr.srcs.first() {
                if producer_hint(func, c) != Some("bool") {
                    set.insert(c.clone());
                }
            }
        }
    }
    set
}

/// If `reg` is produced by `box %reg = %src` where `%src`'s producer hint is
/// `"bool"`, return `%src` — the raw machine bool behind a boxed comparison /
/// predicate result.
///
/// Such a condition must branch on its **truth value**, not on nil-ness. The
/// dynamic-arithmetic pass boxes every comparison result (`=`/`<`/… → a boxed
/// `bool`), so a `jmp_if_false` on it sees a `ref<any>` and would be treated as a
/// lisp value by [`lisp_conditions`]. But a boxed `#f` is `ref.i31(0)` — a
/// **non-null** reference — so the McCarthy nil-truthiness wrap
/// (`not(is_null(..))`) would wrongly read it as true, mis-dispatching e.g. every
/// `match` arm (E6d-6). Testing the raw pre-box `bool` directly is correct.
fn boxed_bool_source(func: &IIRFunction, reg: &str) -> Option<String> {
    let boxed = func.instructions.iter().find(|i| i.dest.as_deref() == Some(reg))?;
    if boxed.op != "box" {
        return None;
    }
    match boxed.srcs.first() {
        Some(Operand::Var(src)) if producer_hint(func, src) == Some("bool") => Some(src.clone()),
        _ => None,
    }
}

/// The `jmp_if_false` conditions that are a **boxed machine bool** (a boxed
/// comparison result), mapped to the raw pre-box `bool` register to test instead.
/// See [`boxed_bool_source`].
fn boxed_bool_conditions(func: &IIRFunction) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for instr in &func.instructions {
        if instr.op == "jmp_if_false" {
            if let Some(Operand::Var(c)) = instr.srcs.first() {
                if let Some(src) = boxed_bool_source(func, c) {
                    map.insert(c.clone(), src);
                }
            }
        }
    }
    map
}

/// The `srcs` indices of `instr` that hold a **lisp value** (and so whose
/// integer atoms must be boxed): `field_store`'s value operand, and the value
/// arguments of a `pair?`/`equal?` call. Other positions (the builtin name, a
/// field index, a machine-boolean `not` arg) are excluded.
fn lisp_value_src_indices(instr: &IIRInstr) -> Vec<usize> {
    match instr.op.as_str() {
        "field_store" => vec![2],
        "call_builtin" => match instr.srcs.first() {
            Some(Operand::Var(name)) if LISP_VALUE_ARG_BUILTINS.contains(&name.as_str()) => {
                (1..instr.srcs.len()).collect()
            }
            _ => vec![],
        },
        _ => vec![],
    }
}

/// Whether a function takes **lisp-value parameters** — a `LAMBDA`/`LABEL`
/// lifted to a function. The frontend types a lisp parameter `"any"` (or
/// `"symbol"`); a Twig function's params are concrete (`i32`/`i64`). Such a
/// function participates in the uniform-anyref **function boundary** even if its
/// body touches no heap op (e.g. `(LAMBDA (X) X)`).
fn has_lisp_param(func: &IIRFunction) -> bool {
    func.params.iter().any(|(_, t)| t == "symbol" || t.starts_with("ref<"))
}

/// Compute the set of **lisp functions** in a module — those that use the
/// uniform-anyref value model at their boundary. Seeded by functions that use
/// the heap/predicates or take lisp params, then closed under *calling*: a
/// function that `call`s a lisp function is itself lisp (its call site must box
/// the args and treat the result as a reference). For a McCarthy module every
/// function ends up lisp; a Twig module's pure-scalar functions never enter.
pub(crate) fn lisp_functions(module: &IIRModule) -> HashSet<String> {
    seeded_lisp_functions(module, true)
}

/// The functions whose **calling boundary** is tagged — i.e. whose callers must
/// box their arguments.
///
/// Seeded from declared parameter types ONLY. A body that allocates tells you
/// the function uses the heap; it tells you nothing about the ABI its callers
/// must satisfy. A Twig union constructor allocates a cons cell internally while
/// taking its argument raw, and boxing at its call sites corrupts the value.
pub(crate) fn tagged_boundary_functions(module: &IIRModule) -> HashSet<String> {
    seeded_lisp_functions(module, false)
}

fn seeded_lisp_functions(module: &IIRModule, include_heap_bodies: bool) -> HashSet<String> {
    let mut lisp: HashSet<String> = module
        .functions
        .iter()
        // A boundary has two sides. Seeding only from parameters misses a
        // **nullary** tagged function — `((LAMBDA () (ATOM 7)))` has no params
        // to inspect, so its caller's `call` was not recognised as tagged and
        // the entry coerced the result with the static `dyn_unbox_int` (`>> 3`)
        // instead of the runtime tag switch `dyn_to_exit_code`. `#t` is the
        // whole word `0b101`, so `5 >> 3 = 0`: the program reported FALSE for a
        // true predicate, with no diagnostic. The declared return type is the
        // other half of the signature and says so.
        //
        // This does not reinstate the Twig union-constructor bug: Twig's
        // `Some`/`None` declare `-> any` (raw), McCarthy's lambdas declare
        // `-> ref<any>`, so the two stay separated by type, which is the point.
        .filter(|f| {
            (include_heap_bodies && function_uses_heap(f))
                || has_lisp_param(f)
                || f.return_type == "symbol"
                || f.return_type.starts_with("ref<")
        })
        .map(|f| f.name.clone())
        .collect();
    // Fixpoint: a caller of a lisp function is lisp.
    loop {
        let mut changed = false;
        for f in &module.functions {
            if lisp.contains(&f.name) {
                continue;
            }
            let calls_lisp = f.instructions.iter().any(|i| {
                i.op == "call"
                    && matches!(i.srcs.first(), Some(Operand::Var(callee)) if lisp.contains(callee))
            });
            if calls_lisp {
                lisp.insert(f.name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    lisp
}

/// The argument `srcs` indices of a `call` to a **lisp function** (so each atom
/// argument is boxed as an `i31ref` before crossing the function boundary). The
/// callee name is `srcs[0]`; the arguments are `srcs[1..]`.
fn call_lisp_arg_indices(instr: &IIRInstr, lisp_funcs: &HashSet<String>) -> Vec<usize> {
    if instr.op == "call" {
        if let Some(Operand::Var(callee)) = instr.srcs.first() {
            if lisp_funcs.contains(callee) {
                return (1..instr.srcs.len()).collect();
            }
        }
    }
    Vec::new()
}

/// Apply the structural representation pass to every lisp function.
///
/// Run by `lang-aot::compile_source_to_wasm` **after** `lower_heap_builtins`
/// (so cons/car/cdr are already `alloc`/`field_*`) and alongside
/// `concretize_scalar_any_for_wasm` (which handles the pure-scalar functions
/// this pass skips). Safe to run on any module: a non-lisp function is left
/// untouched. Lisp functions share a **uniform-anyref boundary** — every
/// parameter, call argument, call result, and (non-entry) return is an
/// `anyref` — so a `LAMBDA`/`LABEL` can be called and can recurse.
pub fn lower_dyn_repr_structural(module: &mut IIRModule) {
    let entry = module.entry_point.clone();
    let lisp_funcs = lisp_functions(module);
    for func in &mut module.functions {
        if !lisp_funcs.contains(&func.name) {
            continue; // pure scalar — concretize_scalar_any_for_wasm owns it.
        }
        let is_entry = entry.as_deref() == Some(func.name.as_str());
        lower_structural_function(func, is_entry, &lisp_funcs);
    }
}

/// Does this function touch the lisp heap / reference model? Mirrors the
/// `uses_heap` check in `concretize_scalar_any_for_wasm` so the two passes
/// partition the module's functions cleanly.
fn function_uses_heap(func: &IIRFunction) -> bool {
    func.instructions.iter().any(|i| {
        matches!(i.op.as_str(), "alloc" | "field_load" | "field_store" | "is_null")
            || i.type_hint.starts_with("ref<")
            || lisp_builtin_name(i).is_some()
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

/// Unbox a tagged parameter before the body does machine arithmetic on it.
///
/// The boundary above declares every lisp parameter `ref<any>`, so callers box.
/// A body that then reads the parameter directly in an `add`/`cmp_*`/bitwise op
/// is operating on the TAG, not the value:
///
/// ```text
/// fn __lambda_0(x: ref<any>)
///     add _r2.raw1 = x + 1   i64      <- x is boxed; this adds 1 to (41 << 3)
/// ```
///
/// `((lambda (x) (+ x 1)) 41)` returned 329 instead of 42 on every managed
/// backend, which is why the JVM and CIL pipelines refused `alloc_closure`
/// outright rather than lower it — the refusal was hiding a wrong answer.
///
/// The repair is deliberately **one-sided and local**: insert
/// `unbox %x.raw = %x : i64` at entry and rewrite only the machine-op operands
/// to `%x.raw`. The signature does not change, so every caller keeps boxing
/// exactly as before and no call site is touched.
///
/// That one-sidedness is the safety property. This pass can insert a `box`
/// (step 1 below boxes any non-reference flowing into a lisp-value position)
/// but has no way to insert an `unbox` at a call site — so retyping the
/// PARAMETER to a raw `i64` instead, which was the first attempt here, is only
/// sound if every caller happens to hold a raw word, and nothing checks that. A
/// forwarding wrapper `f(y) = g(y)`, or `(g (car p))`, hands `g` a reference
/// with no conversion anywhere. Fixing the body instead cannot have that
/// failure mode.
///
/// `MACHINE_OPS` is a positive list, so an operation missing from it simply
/// keeps reading the tagged parameter — the pre-existing behaviour for that
/// shape, not a new miscompile. Reference uses (`field_load` bases, cons
/// values, `pair?` arguments) are never rewritten: they want the reference.
fn unbox_machine_uses_of_tagged_params(func: &mut IIRFunction) {
    /// Operations that consume a raw machine word in every operand position.
    const MACHINE_OPS: &[&str] = &[
        "add", "sub", "mul", "div", "mod", "neg", "and", "or", "xor", "shl", "shr",
    ];
    let is_machine_op =
        |op: &str| MACHINE_OPS.contains(&op) || op.starts_with("cmp_");

    let tagged: Vec<String> = func
        .params
        .iter()
        .filter(|(_, ty)| ty.starts_with("ref<"))
        .map(|(name, _)| name.clone())
        .collect();

    for param in tagged {
        let raw = format!("{param}.rawparam");
        let mut used_raw = false;
        for instr in &mut func.instructions {
            if !is_machine_op(&instr.op) {
                continue;
            }
            for src in &mut instr.srcs {
                if matches!(src, Operand::Var(v) if *v == param) {
                    *src = Operand::Var(raw.clone());
                    used_raw = true;
                }
            }
        }
        // `lower_closures_to_heap` runs earlier, while the parameter is still
        // bare `any`, and boxes a capture it believes is a raw word. Once the
        // boundary above declares the parameter `ref<any>` that `box` is a
        // SECOND box over an already-tagged value, and the JVM rejects the
        // capturing closure with `Register 0 contains wrong type`. Boxing a
        // tagged value is never right, so collapse it to a copy.
        for instr in &mut func.instructions {
            if instr.op == "box" && matches!(instr.srcs.first(), Some(Operand::Var(v)) if *v == param)
            {
                instr.op = "mov".to_string();
                instr.type_hint = REF_ANY.to_string();
            }
        }

        if used_raw {
            func.instructions.insert(
                0,
                IIRInstr::new("unbox", Some(raw), vec![Operand::Var(param)], "i64"),
            );
        }
    }
}

fn lower_structural_function(
    func: &mut IIRFunction,
    is_entry: bool,
    lisp_funcs: &HashSet<String>,
) {
    // ── 0. The function boundary is uniform-anyref. Every lisp parameter is an
    //       `anyref` (a reference), and so is every result of a `call` to a lisp
    //       function. Retype the params and seed the reference set with both. ──
    for (_, ty) in func.params.iter_mut() {
        if ty == "any" || ty == "symbol" {
            *ty = REF_ANY.to_string();
        }
    }
    unbox_machine_uses_of_tagged_params(func);
    let mut ref_regs = reference_registers(func);
    for (name, ty) in &func.params {
        if ty.starts_with("ref<") {
            ref_regs.insert(name.clone());
        }
    }
    for instr in &mut func.instructions {
        let is_lisp_call = instr.op == "call"
            && matches!(instr.srcs.first(), Some(Operand::Var(c)) if lisp_funcs.contains(c));
        if is_lisp_call || !call_lisp_arg_indices(instr, lisp_funcs).is_empty() {
            if let Some(dest) = &instr.dest {
                ref_regs.insert(dest.clone()); // a lisp call returns an anyref.
            }
        }
        // A lisp `call` returns `ref<any>` (the callee's uniform-anyref result).
        // The frontend hints the call `i64`; retype it so a **strict** backend
        // (JVM/CLR/BEAM) stores the result as a reference, not a machine int.
        // (The loose wasm model tolerated the stale hint; the JVM does not.)
        if is_lisp_call {
            instr.type_hint = REF_ANY.to_string();
        }
    }

    // ── Reference funnels. A `COND` lowers each clause to `mov %funnel, value`
    //    into one result register. If *any* clause yields a reference — a cons,
    //    `nil`, or a lisp `call` result (e.g. a recursive `LABEL`) — the funnel
    //    must be a reference in *every* clause, else a strict backend can't give
    //    the register one type. Propagate `ref`-ness through `mov` chains to a
    //    fixpoint; the rebuild then **boxes** each atom-valued `mov` into the
    //    funnel (instead of boxing the whole funnel once at `ret`, which is wrong
    //    when a clause already put a reference there). ──
    loop {
        let mut changed = false;
        for instr in &func.instructions {
            if instr.op != "mov" {
                continue;
            }
            if let (Some(dest), Some(Operand::Var(src))) = (&instr.dest, instr.srcs.first()) {
                if ref_regs.contains(src) && !ref_regs.contains(dest) {
                    ref_regs.insert(dest.clone());
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let lisp_conds = lisp_conditions(func);
    // A `jmp_if_false` whose guard is a *boxed machine bool* (a boxed comparison
    // result) must branch on the raw bool, not on nil-ness (E6d-6). Precomputed
    // here because the rebuild below drains `func.instructions`.
    let boxed_bool_conds = boxed_bool_conditions(func);

    // The lisp-value source indices of an instruction: the structural positions
    // (`field_store` value, `pair?`/`equal?` args) plus the arguments of a `call`
    // to a lisp function (boxed before crossing the boundary).
    let value_idxs_of = |instr: &IIRInstr| -> Vec<usize> {
        let mut idxs = lisp_value_src_indices(instr);
        idxs.extend(call_lisp_arg_indices(instr, lisp_funcs));
        idxs
    };

    // ── 1. Which atoms must be boxed? Any non-reference value that flows into a
    //       lisp-value position — a `field_store`'s value operand, a
    //       `pair?`/`equal?` argument, a lisp `call` argument, or a lisp-value
    //       `COND` guard (boxed so its truthiness can be tested with `is_null`). ──
    let mut needs_box: HashSet<String> = HashSet::new();
    for instr in &func.instructions {
        for idx in value_idxs_of(instr) {
            if let Some(Operand::Var(val)) = instr.srcs.get(idx) {
                if !ref_regs.contains(val) {
                    needs_box.insert(val.clone());
                }
            }
        }
    }
    // A lisp-value condition that is an integer **atom** must be boxed before its
    // `is_null` truthiness test. A condition that is already a reference (`nil` /
    // a cons / a variable) needs no boxing.
    for cond in &lisp_conds {
        if !ref_regs.contains(cond)
            && matches!(producer_hint(func, cond), Some("i64") | Some("i32"))
        {
            needs_box.insert(cond.clone());
        }
    }
    // An **atom** moved into a reference funnel is boxed *into* the funnel (the
    // `mov` becomes a `box`). Mark its source so any feeding `const` narrows to i32.
    for instr in &func.instructions {
        if instr.op == "mov" {
            if let (Some(dest), Some(Operand::Var(src))) = (&instr.dest, instr.srcs.first()) {
                if ref_regs.contains(dest) && !ref_regs.contains(src) {
                    needs_box.insert(src.clone());
                }
            }
        }
    }

    // ── 2. Rebuild the body, narrowing boxable atom `const`s to i32, boxing the
    //       lisp-value positions, and wrapping lisp-value `COND` guards with a
    //       truthiness test. ──
    let mut new_instrs: Vec<IIRInstr> = Vec::with_capacity(func.instructions.len() + needs_box.len());
    let mut boxed: HashMap<String, String> = HashMap::new(); // atom reg → boxed reg
    let mut truthy_counter = 0usize;
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

        // A `jmp_if_false` whose guard is a lisp value: test lisp truthiness.
        // Replace `jmp_if_false %cond, L` with
        //   %n = is_null(%cond_boxed)        ;; 1 iff cond is nil
        //   %t = not(%n)                     ;; 1 iff cond is truthy (non-nil)
        //   jmp_if_false %t, L               ;; branch iff cond is nil/false
        // so a lisp integer atom (even `0`) is true and only `nil` is false.
        if instr.op == "jmp_if_false" {
            if let Some(Operand::Var(cond)) = instr.srcs.first().cloned() {
                // E6d-6: a boxed machine bool (a boxed `=`/`<`/… result) branches
                // on its raw truth value, NOT nil-ness — repoint the guard to the
                // pre-box `bool` and emit the jump unwrapped. (A boxed `#f` is a
                // non-null `i31ref`, so the nil-truthiness wrap below would read it
                // as true and mis-dispatch, e.g. every `match` arm.)
                if let Some(raw) = boxed_bool_conds.get(&cond) {
                    let mut j = instr.clone();
                    j.srcs[0] = Operand::Var(raw.clone());
                    new_instrs.push(j);
                    continue;
                }
                // Only wrap a guard we can prove is a reference (so `is_null` is
                // well-typed) or a boxable integer atom (which we box first). A
                // lisp-value guard that is neither — e.g. a function parameter,
                // not yet emitted by any McCarthy→wasm path — is left untouched
                // rather than emitting `is_null` on a non-reference.
                if lisp_conds.contains(&cond)
                    && (ref_regs.contains(&cond) || needs_box.contains(&cond))
                {
                    let cond_ref = if needs_box.contains(&cond) {
                        boxed
                            .entry(cond.clone())
                            .or_insert_with(|| {
                                let b = format!("{cond}.box");
                                new_instrs.push(IIRInstr::new(
                                    "box",
                                    Some(b.clone()),
                                    vec![Operand::Var(cond.clone())],
                                    REF_ANY,
                                ));
                                b
                            })
                            .clone()
                    } else {
                        cond.clone()
                    };
                    let n = format!("__isnil_{truthy_counter}");
                    let t = format!("__truthy_{truthy_counter}");
                    truthy_counter += 1;
                    new_instrs.push(IIRInstr::new(
                        "is_null",
                        Some(n.clone()),
                        vec![Operand::Var(cond_ref)],
                        "bool",
                    ));
                    new_instrs.push(IIRInstr::new(
                        "call_builtin",
                        Some(t.clone()),
                        vec![Operand::Var("not".to_string()), Operand::Var(n)],
                        "bool",
                    ));
                    let mut j = instr.clone();
                    j.srcs[0] = Operand::Var(t);
                    new_instrs.push(j);
                    continue;
                }
            }
        }

        // A `mov` into a **reference funnel** (a `COND` result that some clause
        // fills with a reference): make every clause's value a reference. A
        // reference source becomes a `ref<any>` move; an atom source is boxed
        // *into* the funnel (`box %funnel = %atom`), so the funnel needs no
        // boxing at `ret`.
        if instr.op == "mov" {
            if let Some(dest) = instr.dest.clone() {
                if ref_regs.contains(&dest) {
                    if let Some(Operand::Var(src)) = instr.srcs.first().cloned() {
                        if ref_regs.contains(&src) {
                            let mut m = instr.clone();
                            m.type_hint = REF_ANY.to_string();
                            new_instrs.push(m);
                        } else {
                            new_instrs.push(IIRInstr::new(
                                "box",
                                Some(dest),
                                vec![Operand::Var(src)],
                                REF_ANY,
                            ));
                        }
                        continue;
                    }
                }
            }
        }

        let value_idxs = value_idxs_of(&instr);
        if !value_idxs.is_empty() {
            let mut rewritten = instr.clone();
            for idx in value_idxs {
                if let Some(Operand::Var(val)) = instr.srcs.get(idx).cloned() {
                    if needs_box.contains(&val) {
                        // Box each atom once (`box %b = %val : ref<any>`) and reuse.
                        let boxed_reg = boxed
                            .entry(val.clone())
                            .or_insert_with(|| {
                                let b = format!("{val}.box");
                                new_instrs.push(IIRInstr::new(
                                    "box",
                                    Some(b.clone()),
                                    vec![Operand::Var(val.clone())],
                                    REF_ANY,
                                ));
                                b
                            })
                            .clone();
                        rewritten.srcs[idx] = Operand::Var(boxed_reg);
                    }
                }
            }
            new_instrs.push(rewritten);
            continue;
        }

        new_instrs.push(instr);
    }
    func.instructions = new_instrs;

    // ── 3. The machine boundary: unbox the entry function's reference result;
    //       a non-entry lisp function returns an `anyref` for its caller. ──
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
/// machine type. In the entry function a returned **reference** maps `nil` to
/// process exit code zero and unboxes a non-null atom to `i32`; a returned
/// scalar keeps its width. A non-entry function that returns a reference keeps
/// it as `ref<any>` for its caller.
/// A dynamic type hint that a concrete machine type may replace.
fn is_narrowable_dynamic_hint(t: &str) -> bool {
    t == "any" || t == "polymorphic" || t == REF_ANY
}

fn set_return_representation(func: &mut IIRFunction, is_entry: bool, ref_regs: &HashSet<String>) {
    // Find the (single) returned register, if any.
    let ret_pos = func.instructions.iter().position(|i| i.op == "ret");
    let Some(ret_pos) = ret_pos else { return };
    let ret_reg = match func.instructions[ret_pos].srcs.first() {
        Some(Operand::Var(r)) => r.clone(),
        _ => {
            // `ret` of an immediate / nothing — just concretise the return type.
            if is_narrowable_dynamic_hint(&func.return_type) {
                func.return_type = "i64".to_string();
            }
            return;
        }
    };
    let returns_ref = ref_regs.contains(&ret_reg);

    if is_entry && returns_ref {
        // A nullable Lisp result needs an explicit machine-boundary mapping:
        // nil is process exit code zero, while a non-null atom may be unboxed.
        // Keeping the null check here preserves the spec-required trap for a
        // direct `i31.get_s(ref.null i31)` everywhere else.
        let unboxed = format!("{ret_reg}.unbox");
        let is_nil = format!("{ret_reg}.is_nil");
        let non_nil = format!("{ret_reg}.non_nil");
        let done = format!("{ret_reg}.exit_done");
        let boundary = vec![
            IIRInstr::new(
                "is_null",
                Some(is_nil.clone()),
                vec![Operand::Var(ret_reg.clone())],
                "bool",
            ),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var(is_nil), Operand::Var(non_nil.clone())],
                "void",
            ),
            IIRInstr::new("const", Some(unboxed.clone()), vec![Operand::Int(0)], "i32"),
            IIRInstr::new("jmp", None, vec![Operand::Var(done.clone())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var(non_nil)], "void"),
            IIRInstr::new(
                "unbox",
                Some(unboxed.clone()),
                vec![Operand::Var(ret_reg)],
                "i32",
            ),
            IIRInstr::new("label", None, vec![Operand::Var(done)], "void"),
        ];
        func.instructions.splice(ret_pos..ret_pos, boundary);
        let ret = &mut func.instructions[ret_pos + 7];
        ret.srcs = vec![Operand::Var(unboxed)];
        ret.type_hint = "i32".to_string();
        func.return_type = "i32".to_string();
    } else if returns_ref {
        // Non-entry: hand the lisp value back to the caller as a reference.
        func.instructions[ret_pos].type_hint = REF_ANY.to_string();
        if func.return_type == "any" || func.return_type == "polymorphic" || func.return_type == REF_PAIR {
            func.return_type = REF_ANY.to_string();
        }
    } else if !is_entry {
        // A non-entry lisp function (a `LAMBDA`/`LABEL`) whose result is a
        // **scalar** — a predicate boolean or an integer/symbol atom. The
        // function boundary is uniform-anyref, so box it and return `ref<any>`.
        // `box` (`ref.i31`) takes an i32, so narrow a boxable integer-atom const
        // first; a predicate boolean is already i32.
        if let Some(p) = func
            .instructions
            .iter_mut()
            .find(|i| i.dest.as_deref() == Some(ret_reg.as_str()))
        {
            if p.op == "const" {
                if let Some(Operand::Int(n)) = p.srcs.first() {
                    if (I31_MIN..=I31_MAX).contains(n) {
                        p.type_hint = "i32".to_string();
                    }
                }
            }
        }
        let boxed_ret = format!("{ret_reg}.retbox");
        func.instructions.insert(
            ret_pos,
            IIRInstr::new("box", Some(boxed_ret.clone()), vec![Operand::Var(ret_reg)], REF_ANY),
        );
        let ret = &mut func.instructions[ret_pos + 1];
        ret.srcs = vec![Operand::Var(boxed_ret)];
        ret.type_hint = REF_ANY.to_string();
        func.return_type = REF_ANY.to_string();
    } else if is_narrowable_dynamic_hint(&func.return_type)
        || is_narrowable_dynamic_hint(&func.instructions[ret_pos].type_hint)
    {
        // The **entry** function returning a non-reference scalar (e.g. a bare
        // `(EQ 5 5)` program). Concretise the return type to the value's width: a
        // predicate result (hint `"bool"`) is i32, everything else i64. Setting
        // the `ret` hint keeps the defensive `any` sweep from re-widening a bool.
        let width = func
            .instructions
            .iter()
            .find(|i| i.dest.as_deref() == Some(ret_reg.as_str()))
            .map(|i| i.type_hint.as_str())
            .map(|h| if is_i32_width(h) { "i32" } else { "i64" })
            .unwrap_or("i64");
        func.return_type = width.to_string();
        func.instructions[ret_pos].type_hint = width.to_string();
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

    /// `(ATOM 5)` after `lower_heap_builtins`: const 5, pair?(5), not(_), ret.
    fn atom_module() -> IIRModule {
        let instrs = vec![
            IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(5)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("v1".into()),
                vec![Operand::Var("pair?".into()), Operand::Var("v0".into())],
                "bool",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("v2".into()),
                vec![Operand::Var("not".into()), Operand::Var("v1".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v2".into())], "any"),
        ];
        let f = IIRFunction::new("main", vec![], "any", instrs);
        let mut m = IIRModule::new("atom", "mccarthy-lisp");
        m.entry_point = Some("main".to_string());
        m.functions = vec![f];
        m
    }

    #[test]
    fn predicate_atom_arg_is_boxed_and_bool_result_is_i32() {
        let mut m = atom_module();
        lower_dyn_repr_structural(&mut m);
        let f = &m.functions[0];

        // The atom feeding `pair?` is boxed (and narrowed to i32); `not`'s arg
        // (a machine boolean) is NOT boxed.
        assert_eq!(ops(f).iter().filter(|o| **o == "box").count(), 1, "exactly the pair? atom boxes");
        let pair = f.instructions.iter().find(|i| {
            matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "pair?")
        }).unwrap();
        match &pair.srcs[1] {
            Operand::Var(v) => assert!(v.ends_with(".box"), "pair? arg is the boxed atom"),
            other => panic!("unexpected pair? arg {other:?}"),
        }
        // The const atom is narrowed to i32 for `ref.i31`.
        let c = f.instructions.iter().find(|i| i.op == "const").unwrap();
        assert_eq!(c.type_hint, "i32");

        // A predicate result (bool) returns as i32 — NOT unboxed, NOT widened to
        // i64 — so the wasm function's result type matches the value.
        assert_eq!(f.return_type, "i32");
        assert!(ops(f).iter().all(|o| *o != "unbox"), "a boolean result is not unboxed");
        assert!(f.instructions.iter().all(|i| i.type_hint != "any"));
    }

    #[test]
    fn boxes_atoms_and_unboxes_the_result() {
        let mut m = cons_car_module();
        lower_dyn_repr_structural(&mut m);
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
    fn lisp_value_cond_gets_a_truthiness_test_but_a_bool_cond_does_not() {
        // Two COND-style guards: one a lisp atom (`jmp_if_false v_atom`), one a
        // machine boolean (`jmp_if_false v_bool` where v_bool is a `pair?`
        // result). Only the lisp-value guard is wrapped with `is_null` + `not`.
        let instrs = vec![
            // lisp-value guard: a bare atom 5.
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i64"),
            IIRInstr::new("jmp_if_false", None, vec![Operand::Var("a".into()), Operand::Var("L1".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("L1".into())], "void"),
            // machine-boolean guard: a pair? result.
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(6)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("p".into()),
                vec![Operand::Var("pair?".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("jmp_if_false", None, vec![Operand::Var("p".into()), Operand::Var("L2".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("L2".into())], "void"),
            IIRInstr::new("const", Some("r".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let f = IIRFunction::new("main", vec![], "any", instrs);
        let mut m = IIRModule::new("cond", "mccarthy-lisp");
        m.entry_point = Some("main".to_string());
        m.functions = vec![f];
        lower_dyn_repr_structural(&mut m);
        let f = &m.functions[0];

        // The atom guard `a` is boxed and tested via is_null + not.
        assert_eq!(ops(f).iter().filter(|o| **o == "is_null").count(), 1, "exactly the lisp guard tests is_null");

        // The two jmp_if_false survive; the first now tests a `__truthy_*` reg
        // (not the raw atom), the second still tests the raw `pair?` bool.
        let jifs: Vec<&IIRInstr> = f.instructions.iter().filter(|i| i.op == "jmp_if_false").collect();
        assert_eq!(jifs.len(), 2);
        match (&jifs[0].srcs[0], &jifs[1].srcs[0]) {
            (Operand::Var(c0), Operand::Var(c1)) => {
                assert!(c0.starts_with("__truthy_"), "lisp guard wrapped: {c0}");
                assert_eq!(c1, "p", "machine-bool guard tested directly");
            }
            _ => panic!("unexpected jmp_if_false operands"),
        }
    }

    #[test]
    fn nullable_entry_result_checks_for_nil_before_unboxing() {
        let mut m = cons_car_module();
        lower_dyn_repr_structural(&mut m);
        let f = &m.functions[0];
        let pos = f.instructions.iter().position(|i| i.op == "ret").unwrap();
        let boundary = &f.instructions[pos - 7..pos];
        assert_eq!(
            boundary.iter().map(|i| i.op.as_str()).collect::<Vec<_>>(),
            vec!["is_null", "jmp_if_false", "const", "jmp", "label", "unbox", "label"]
        );
        assert_eq!(boundary[2].srcs, vec![Operand::Int(0)], "nil maps to exit code zero");
        // Both paths assign the same exit register read by ret.
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
        lower_dyn_repr_structural(&mut m);
        // No box/unbox inserted; return type untouched (concretize handles it).
        assert_eq!(m.functions[0].instructions.len(), before);
        assert!(m.functions[0].instructions.iter().all(|i| i.op != "box" && i.op != "unbox"));
    }

    #[test]
    fn lambda_boundary_is_uniform_anyref() {
        // `((LAMBDA (X) X) 5)`: a lambda `lam(X)` returning X, called from main.
        // The boundary must be uniform anyref: X's param type → ref<any>, the
        // call arg (5) boxed, the lambda return ref<any>, and main unboxes.
        let lam = IIRFunction::new(
            "lam",
            // What the McCarthy frontend now emits directly: a tagged value is
            // `ref<any>`, said in the IIR rather than inferred from the module's
            // language.
            vec![("X".to_string(), "ref<any>".to_string())],
            "ref<any>",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("X".into())], "ref<any>")],
        );
        let main = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![
                IIRInstr::new("const", Some("v0".into()), vec![Operand::Int(5)], "i64"),
                IIRInstr::new(
                    "call",
                    Some("v1".into()),
                    vec![Operand::Var("lam".into()), Operand::Var("v0".into())],
                    "any",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("v1".into())], "any"),
            ],
        );
        let mut m = IIRModule::new("lam", "mccarthy-lisp");
        m.entry_point = Some("main".to_string());
        m.functions = vec![lam, main];
        lower_dyn_repr_structural(&mut m);

        let lam = &m.functions[0];
        let main = &m.functions[1];

        // The lambda's parameter is now an anyref reference.
        assert_eq!(lam.params[0].1, REF_ANY, "lisp param retyped to ref<any>");
        // The lambda returns an anyref to its caller.
        assert_eq!(lam.return_type, REF_ANY, "lambda returns ref<any>");

        // main boxes the call argument (5) before the call.
        assert_eq!(
            main.instructions.iter().filter(|i| i.op == "box").count(),
            1,
            "the call argument is boxed"
        );
        let call = main.instructions.iter().find(|i| i.op == "call").unwrap();
        match &call.srcs[1] {
            Operand::Var(a) => assert!(a.ends_with(".box"), "call passes the boxed arg"),
            other => panic!("unexpected call arg {other:?}"),
        }
        // main unboxes the call result at the entry/return boundary → i32.
        assert_eq!(main.instructions.iter().filter(|i| i.op == "unbox").count(), 1);
        assert_eq!(main.return_type, "i32");
    }

    #[test]
    fn reference_funnel_boxes_atom_branches() {
        // A `COND`-style funnel: one clause yields a reference (a cons), another
        // an atom. The funnel must become a uniform reference — the atom `mov` is
        // boxed *into* it, the ref `mov` is retyped, and `ret` returns `ref<any>`
        // WITHOUT a final whole-funnel box (which would mis-box the cons clause).
        // This is the case a recursive `LABEL` hits (the recursive call result is
        // the reference clause); it must work on a strict backend like the JVM.
        let f = IIRFunction::new(
            "label_0",
            vec![("L".to_string(), "any".to_string())],
            "any",
            vec![
                // clause A: funnel <- a cons (reference)
                IIRInstr::new("alloc", Some("p".into()), vec![], REF_PAIR),
                IIRInstr::new("mov", Some("fun".into()), vec![Operand::Var("p".into())], "i64"),
                // clause B: funnel <- an atom (needs boxing into the funnel)
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(99)], "i64"),
                IIRInstr::new("mov", Some("fun".into()), vec![Operand::Var("a".into())], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("fun".into())], "any"),
            ],
        );
        let mut m = IIRModule::new("rec", "mccarthy-lisp");
        m.entry_point = Some("main".to_string()); // label_0 is non-entry
        m.functions = vec![f];
        lower_dyn_repr_structural(&mut m);
        let f = &m.functions[0];

        // The atom branch `mov fun = a` became `box fun = a`.
        assert!(
            f.instructions
                .iter()
                .any(|i| i.op == "box" && i.dest.as_deref() == Some("fun")),
            "atom branch is boxed into the funnel"
        );
        // The reference branch stays a (ref<any>) move, never boxed.
        assert!(
            f.instructions.iter().any(|i| i.op == "mov"
                && i.dest.as_deref() == Some("fun")
                && i.type_hint == REF_ANY),
            "reference branch is a ref<any> move"
        );
        // The funnel is returned as a reference WITHOUT a trailing whole-funnel box.
        assert!(
            !f.instructions.iter().any(|i| i.dest.as_deref() == Some("fun.retbox")),
            "no whole-funnel retbox (each clause already produced a reference)"
        );
        assert_eq!(f.return_type, REF_ANY, "non-entry funnel function returns ref<any>");
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
        lower_dyn_repr_structural(&mut m);
        let f = &m.functions[0];
        // The big const is still i64 (not narrowed). A box is still inserted
        // (the store needs a ref), but the const width is preserved so the
        // backend can detect the unboxable atom rather than silently lose bits.
        let c = f.instructions.iter().find(|i| i.op == "const").unwrap();
        assert_eq!(c.type_hint, "i64", "out-of-range atom keeps its width");
    }

    #[test]
    fn boxed_bool_condition_branches_on_raw_bool_not_nil_truthiness() {
        // E6d-6: a `jmp_if_false` on a BOXED comparison result (the shape a Twig
        // `match` tag test produces after dynamic_arith: `cmp_eq → bool`, `box →
        // ref<any>`) must test the raw bool directly — NOT be wrapped with
        // `not(is_null(..))`. A boxed `#f` is a non-nil `i31ref`, so the
        // nil-truthiness wrap would read it as true and mis-dispatch every arm.
        let instrs = vec![
            // A heap op marks this a lisp function (owned by this pass).
            IIRInstr::new("field_load", Some("h".into()),
                vec![Operand::Var("x".into()), Operand::Int(0)], REF_ANY),
            IIRInstr::new("unbox", Some("xu".into()), vec![Operand::Var("x".into())], "i64"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("cmp_eq", Some("raw".into()),
                vec![Operand::Var("xu".into()), Operand::Var("z".into())], "bool"),
            IIRInstr::new("box", Some("cond".into()), vec![Operand::Var("raw".into())], REF_ANY),
            IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var("cond".into()), Operand::Var("L".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("h".into())], REF_ANY),
            IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("h".into())], REF_ANY),
        ];
        let f = IIRFunction::new(
            "main",
            vec![("x".to_string(), REF_ANY.to_string())],
            REF_ANY,
            instrs,
        );
        let mut m = IIRModule::new("m", "twig");
        m.entry_point = Some("main".into());
        m.functions = vec![f];
        lower_dyn_repr_structural(&mut m);
        let f = &m.functions[0];
        // The jmp_if_false now tests the RAW bool `raw`, not a wrapped truthy reg.
        let jif_pos = f
            .instructions
            .iter()
            .position(|i| i.op == "jmp_if_false")
            .unwrap();
        let jif = &f.instructions[jif_pos];
        assert_eq!(jif.srcs.first(), Some(&Operand::Var("raw".into())),
            "boxed-bool guard must branch on the raw pre-box bool");
        // No nil-truthiness wrapping was inserted before this condition. The
        // entry return boundary may independently test its nullable result.
        assert!(!f.instructions[..jif_pos].iter().any(|i| i.op == "is_null"),
            "no is_null truthiness wrap for a boxed-bool condition");
    }

    /// Helper: a heap-using function so this pass (not the scalar concretizer)
    /// owns the module.
    fn lambda_module(body: Vec<IIRInstr>) -> IIRModule {
        let lambda = IIRFunction::new("__lambda_0", vec![("x".into(), "any".into())], "any", body);
        let main = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![
                IIRInstr::new("alloc", Some("_c".into()), vec![], REF_PAIR),
                IIRInstr::new("ret", None, vec![Operand::Var("_c".into())], "any"),
            ],
        );
        let mut m = IIRModule::new("m", "twig");
        m.entry_point = Some("main".to_string());
        m.functions = vec![lambda, main];
        m
    }

    /// A tagged parameter must be UNBOXED before machine arithmetic reads it.
    ///
    /// The boundary declares every lisp parameter `ref<any>` and callers box for
    /// it, so a body that adds to the parameter directly is adding to the tag:
    /// `((lambda (x) (+ x 1)) 41)` returned `(41 << 3) + 1 = 329` instead of 42
    /// on every managed backend.
    #[test]
    fn a_tagged_parameter_is_unboxed_before_machine_arithmetic() {
        let mut m = lambda_module(vec![
            IIRInstr::new("const", Some("_n1".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "add",
                Some("_r2.raw1".into()),
                vec![Operand::Var("x".into()), Operand::Var("_n1".into())],
                "i64",
            ),
            // lower_dynamic_arith boxes the result upstream; that is also what
            // marks this function as using the heap value model.
            IIRInstr::new("box", Some("_r2".into()), vec![Operand::Var("_r2.raw1".into())], REF_ANY),
            IIRInstr::new("ret", None, vec![Operand::Var("_r2".into())], "any"),
        ]);
        lower_dyn_repr_structural(&mut m);
        let f = m.functions.iter().find(|f| f.name == "__lambda_0").unwrap();

        // The SIGNATURE is unchanged — callers keep boxing, no call site moves.
        assert_eq!(f.params[0].1, REF_ANY, "the parameter must stay tagged");

        // An unbox was inserted, and the add reads the unboxed register.
        let unbox = f
            .instructions
            .iter()
            .find(|i| i.op == "unbox" && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "x"))
            .expect("no unbox inserted for the tagged parameter");
        let raw = unbox.dest.clone().unwrap();
        let add = f.instructions.iter().find(|i| i.op == "add").unwrap();
        assert!(
            matches!(&add.srcs[0], Operand::Var(v) if *v == raw),
            "add still reads the tagged parameter: {:?}",
            add.srcs
        );
    }

    /// Boxing an already-tagged parameter is never right. `lower_closures_to_heap`
    /// runs while the parameter is bare `any` and boxes a capture it believes is
    /// raw; once the parameter is `ref<any>` that is a second box, and the JVM
    /// rejects the capturing closure with `Register 0 contains wrong type`.
    #[test]
    fn boxing_an_already_tagged_parameter_collapses_to_a_copy() {
        let mut m = lambda_module(vec![
            IIRInstr::new("box", Some("_cap".into()), vec![Operand::Var("x".into())], REF_ANY),
            IIRInstr::new("alloc", Some("_cell".into()), vec![], REF_PAIR),
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("_cell".into()), Operand::Int(0), Operand::Var("_cap".into())],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("_cell".into())], "any"),
        ]);
        lower_dyn_repr_structural(&mut m);
        let f = m.functions.iter().find(|f| f.name == "__lambda_0").unwrap();
        assert!(
            !f.instructions.iter().any(
                |i| i.op == "box" && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "x")
            ),
            "a tagged parameter is still being boxed a second time"
        );
    }

    /// The repair is one-sided: a parameter used only as a REFERENCE keeps the
    /// reference. Nothing is unboxed and no operand is rewritten, so record
    /// accessors and cons walkers are untouched.
    #[test]
    fn a_dereferenced_parameter_is_left_alone() {
        let mut m = lambda_module(vec![
            IIRInstr::new(
                "field_load",
                Some("_fv1".into()),
                vec![Operand::Var("x".into()), Operand::Int(0)],
                REF_ANY,
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("_fv1".into())], "any"),
        ]);
        lower_dyn_repr_structural(&mut m);
        let f = m.functions.iter().find(|f| f.name == "__lambda_0").unwrap();
        assert_eq!(f.params[0].1, REF_ANY);
        assert!(
            !f.instructions.iter().any(
                |i| i.op == "unbox" && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "x")
            ),
            "a reference-only parameter must not be unboxed"
        );
        let load = f.instructions.iter().find(|i| i.op == "field_load").unwrap();
        assert!(matches!(&load.srcs[0], Operand::Var(v) if v == "x"));
    }
}

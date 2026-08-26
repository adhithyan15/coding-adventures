//! # closure_heap — lower closures to the cons-cell heap substrate (E6d-7a).
//!
//! ## Why
//!
//! Only **JVM and CLR** run closures natively (`long[]`/`object[]` dispatch
//! arrays). **NativeAot, LLVM and WASM had no closure model** — the WASM backend
//! hard-rejects `alloc_closure`/`call_closure`, and the native/LLVM backends
//! `BackendRefused` them (the E6D7 design note wrongly assumed those two already
//! worked). This pass closes that gap **entirely at the IIR level**, using only
//! ops those backends already lower (`cons`/`car`/`cdr`, a dynamic `=`, `call`,
//! `jmp_if_false`) — so no backend needs new codegen (spec
//! `E6D7-wasm-closures.md`, option a, generalised from WASM to native + WASM).
//! (**LLVM is a follow-up**: the dispatcher's dynamic `=` index test hits a
//! pre-existing `lower_dynamic_arith` comparison-width bug on the LLVM column.)
//!
//! ## The representation — a closure is a cons chain
//!
//! A closure `alloc_closure(fn, cap0, cap1, …)` becomes the list
//!
//! ```text
//!   ( box(dispatch_index) . ( cap0 . ( cap1 . … . nil ) ) )
//! ```
//!
//! built with the same `cons` the E6d-1/3/5 heap path already lowers. The head
//! is a **boxed integer** — the closure's *dispatch index*, a dense module-local
//! id assigned to each distinct lambda body (alphabetically, so identical
//! modules produce identical output). The tail is the captured `DynValue`s.
//!
//! ## Calling — a synthesized dispatcher (the WASM twin of JVM `__callClosure`)
//!
//! `call_closure(handle, arg0, …)` becomes: box the args into a second cons
//! chain `(arg0 . (arg1 . … . nil))`, then `call __dyn_call_closure(handle,
//! args)`. The pass synthesizes that one function:
//!
//! ```text
//!   __dyn_call_closure(clo, args):
//!     idx_box = car(clo) ; caps = cdr(clo)
//!     if (= idx_box 0): r = body_0( <caps…>, <args…> ) ; ret r   ;; per-body
//!     if (= idx_box 1): r = body_1( <caps…>, <args…> ) ; ret r   ;; direct call
//!     …
//!     ret nil                                                    ;; unreachable
//! ```
//!
//! The index test is the **dynamic `=`** (the proven E6d-6 match/union tag test),
//! not a hand-rolled `unbox`+`cmp_eq`: the unboxed integer width differs across
//! backends (WASM i31/i32 vs native i64), and `lower_dynamic_arith` + E6d-6's
//! boxed-bool `jmp_if_false` already handle that uniformly.
//!
//! Each `body_K` is a **statically-known** function — a direct `call`, no
//! `call_indirect`/`funcref`/table. Its parameters are `captures ++ lambda
//! params` (the frontend's lambda-lifting order, `compiler.rs`), so the
//! dispatcher `car`/`cdr`-walks `caps` for the first `n_captures_K` and `args`
//! for the remaining `body.params.len() - n_captures_K`.
//!
//! ## Scope & gating
//!
//! Run on the **native + WASM pipelines** (JVM/CLR handle closures natively;
//! LLVM is a follow-up). A no-op for a module with no `alloc_closure`. Runs
//! *before* the heap lowering (which then lowers the `cons`/`car`/`cdr` this pass
//! emits — the structural `lower_heap_builtins` on WASM, the runtime
//! `lower_heap_builtins_runtime` on native).

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::IIRModule;
use std::collections::BTreeMap;

/// The synthesized dispatcher's function name.
const DISPATCHER: &str = "__dyn_call_closure";
/// The nil-sentinel `const` type hint — the empty-list terminator, exactly the
/// form `make_nil` / `desugar_list` emit.
const REF_PAIR: &str = "ref<LispyPair>";
/// The boxed-`DynValue` type hint (a tagged heap value; see `dyn_repr`).
const REF_ANY: &str = "ref<any>";

/// One entry in the closure dispatch table: a lambda body + how many of its
/// leading parameters are captures (the rest are call-time args).
struct Dispatch {
    fn_name: String,
    idx: i64,
    n_captures: usize,
    n_params: usize,
}

/// Is `instr` an `alloc_closure`? (srcs[0] = `Str(fn_name)`, srcs[1..] = captures.)
fn is_alloc_closure(instr: &IIRInstr) -> bool {
    instr.op == "alloc_closure"
}
/// Is `instr` a `call_closure`? (srcs[0] = handle, srcs[1..] = args.)
fn is_call_closure(instr: &IIRInstr) -> bool {
    instr.op == "call_closure"
}

/// Scan every function for `alloc_closure` and assign each distinct lambda body a
/// dense dispatch index (alphabetical → deterministic output). Mirrors the JVM
/// `collect_closure_dispatch`.
fn collect_dispatch(module: &IIRModule) -> Vec<Dispatch> {
    let mut names: BTreeMap<String, usize> = BTreeMap::new();
    for f in &module.functions {
        for instr in &f.instructions {
            if is_alloc_closure(instr) {
                if let Some(Operand::Str(name)) = instr.srcs.first() {
                    let n_caps = instr.srcs.len().saturating_sub(1);
                    names.entry(name.clone()).or_insert(n_caps);
                }
            }
        }
    }
    names
        .into_iter()
        .enumerate()
        .map(|(idx, (fn_name, n_captures))| {
            let n_params = module
                .get_function(&fn_name)
                .map(|f| f.params.len())
                .unwrap_or(n_captures);
            Dispatch { fn_name, idx: idx as i64, n_captures, n_params }
        })
        .collect()
}

/// Module-level entry: lower `alloc_closure`/`call_closure` to the cons-heap
/// form plus a synthesized dispatcher (native + WASM). A no-op if the module has
/// no closures.
pub fn lower_closures_to_heap(module: &mut IIRModule) {
    let dispatch = collect_dispatch(module);
    if dispatch.is_empty() {
        return; // no closures — nothing to do (every closure-free program).
    }
    let index_of: BTreeMap<String, i64> =
        dispatch.iter().map(|d| (d.fn_name.clone(), d.idx)).collect();

    // Does this language's lambda bodies take **boxed** parameters?
    //
    // The closure representation is a cons chain, and a cons cell can only hold
    // tagged `DynValue`s — so `dyn_repr` boxes every capture and argument on the
    // way in (a Twig literal `41` is consed as `box_int(41)`, the tagged word
    // `41 << 3 = 328`). What the *body* expects on the other side depends on the
    // source language's value model, and the two models disagree:
    //
    //   * A lisp language (McCarthy) types its parameters `any` and genuinely
    //     passes tagged words. Boxed in, boxed out — the chain round-trips.
    //   * Twig/Nib stamp a bare `any` on a statically-unresolved parameter and
    //     pass it as a **raw** machine `i64`, so `(+ x 1)` in a lambda body
    //     lowers to a plain `add x, 1`. `ref<any>` and bare `any` are two
    //     different types saying two different things, which is what lets this
    //     be decided from the signature alone.
    //
    // Handing a raw-model body the boxed word is silent corruption, not a crash:
    // `((lambda (x) (+ x 1)) 41)` computed `328 + 1 = 329`, re-boxed it, and the
    // program exited `329 & 0xFF = 73` instead of 42. It reproduced on every
    // tagged-word backend (native-AOT and LLVM) and on none of the structural
    // ones, because a backend whose `box` is the identity cannot tell the two
    // conventions apart — which is why the generic VM computed 42 and agreed
    // with nobody.
    //
    // So the dispatcher unboxes what it pulls out of the chain exactly when the
    // body expects raw. Boxing on the way in and unboxing on the way out are one
    // decision, and they belong to the same language gate.
    // Read the answer off the bodies themselves rather than off the module's
    // source language. A lambda body whose parameters are declared `ref<any>`
    // takes tagged values; one whose parameters are a raw machine type takes raw
    // ones. That is a property of the function's own signature, which is exactly
    // where a language-agnostic IR should carry it.
    let params_are_boxed = dispatch.iter().any(|d| {
        module
            .get_function(&d.fn_name)
            .is_some_and(|f| f.params.iter().any(|(_, t)| t == REF_ANY))
    });

    for f in &mut module.functions {
        rewrite_closure_ops(f, &index_of, params_are_boxed);
    }
    if !module.functions.iter().any(|f| f.name == DISPATCHER) {
        module.functions.push(build_dispatcher(&dispatch, params_are_boxed));
    }
}

/// Rewrite `alloc_closure`/`call_closure` in one function to cons-chain builds +
/// a `call __dyn_call_closure`.
fn rewrite_closure_ops(
    f: &mut IIRFunction,
    index_of: &BTreeMap<String, i64>,
    params_are_boxed: bool,
) {
    let types = producer_types(f);
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() * 2);
    // Monotonic suffix so the temporaries this pass introduces never collide.
    let mut counter = 0usize;

    for instr in old {
        if is_alloc_closure(&instr) {
            let dest = match &instr.dest {
                Some(d) => d.clone(),
                None => continue, // result unused → drop the (pure) alloc.
            };
            let fn_name = match instr.srcs.first() {
                Some(Operand::Str(n)) => n.clone(),
                _ => { out.push(instr); continue; }
            };
            let idx = match index_of.get(&fn_name) {
                Some(i) => *i,
                None => { out.push(instr); continue; }
            };
            let captures: Vec<Operand> = instr.srcs[1..].to_vec();
            emit_closure_alloc(&mut out, &dest, idx, &captures, &mut counter, params_are_boxed, &types);
            continue;
        }
        if is_call_closure(&instr) {
            let dest = match &instr.dest {
                Some(d) => d.clone(),
                None => continue,
            };
            let handle = match instr.srcs.first() {
                Some(op) => op.clone(),
                None => { out.push(instr); continue; }
            };
            let args: Vec<Operand> = instr.srcs[1..].to_vec();
            emit_closure_call(&mut out, &dest, handle, &args, &mut counter, params_are_boxed, &types);
            continue;
        }
        out.push(instr);
    }
    f.instructions = out;
}

/// Store one raw value into the closure/argument chain in tagged form.
///
/// A cons cell holds tagged `DynValue`s, so a raw-model value has to be boxed on
/// the way in — and it must be boxed *here*, explicitly, rather than left to
/// `dyn_repr` to notice. `dyn_repr` boxes what it can prove is raw, which for a
/// `const` literal it can (`41` is consed as the tagged word `328`) and for a
/// bare-`any` parameter it cannot — so a captured parameter went into the chain
/// **untagged** while a captured literal went in tagged. The chain then held two
/// different representations and no single extraction rule could be right for
/// both: unboxing the curried `(((lambda (x) (lambda (y) (+ x y))) 40) 2)`
/// recovered `40 >> 3 = 5` from the untagged capture and computed 7.
///
/// Owning both ends in this pass makes the representation a property of the
/// closure substrate rather than an emergent one.
///
/// ## The invariant this relies on, and why it is asserted
///
/// `box`/`unbox` are `<< 3`/`>> 3`, so the *value* round-trips for anything that
/// fits in 61 bits — including a heap address. But value round-tripping is not
/// the only property that matters: a cons cell is **traced by the collector**.
/// `__dyn_cons` registers its cell with a precise kind whose slots the collector
/// interprets as either a raw word or a tag-stripped pointer. A heap handle is
/// `addr | 0b111`; shifted left by 3 it becomes `addr*8 | 0b111000`, which
/// resolves to no live block under either interpretation. The collector would
/// stop tracing through the chain while the chain is the only thing holding the
/// referent — a use-after-free waiting for the first collection in the window.
///
/// So this uniform-shift representation is sound only while every raw-model
/// capture and argument is a **non-pointer**. That is true of every closure
/// program in the matrix today (they capture and pass integers), and it is the
/// interim contract until closure lowering grows a tag-directed extraction.
/// An invariant that holds by accident is one nobody notices breaking, so it is
/// asserted rather than described: the moment a raw-model closure captures a
/// string, a cons, or another closure, this fires — in every build, not just
/// debug — instead of silently producing a dangling reference at runtime. The
/// guard is only as good as its predicate, and `may_hold_a_pointer` documents
/// the one case it cannot see (a bare-`any` destination that happens to hold a
/// handle); read the two together.
fn store(
    out: &mut Vec<IIRInstr>,
    v: Operand,
    counter: &mut usize,
    box_it: bool,
    types: &BTreeMap<String, String>,
) -> Operand {
    if !box_it {
        return v;
    }
    // `assert!`, not `debug_assert!`: what this guards is a memory-safety property
    // of the code we *generate* (a dangling reference collected out from under a
    // live chain), and a release build of the compiler would emit it silently. One
    // map lookup per capture at compile time is not a price worth trading for that.
    assert!(
        !matches!(&v, Operand::Var(name) if types.get(name).is_some_and(|t| may_hold_a_pointer(t))),
        "closure_heap: a raw-model closure captured/passed the pointer-valued operand {v:?}. \
         Boxing it into the cons chain shifts its heap tag out of recognition, so the \
         collector can no longer find the referent through the chain that holds it. \
         Closure lowering needs tag-directed extraction before a raw-model closure can \
         carry a pointer."
    );
    let boxed = fresh(counter, "clobox");
    out.push(IIRInstr::new("box", Some(boxed.clone()), vec![v], REF_ANY));
    Operand::Var(boxed)
}

/// Could a value with this type hint be a heap pointer under the tagged-word
/// model? These are the hints a raw-model (Twig/Nib) frontend stamps on a
/// destination whose runtime value is a handle:
///
/// * `ref<…>` — an explicit reference (`ref<any>`, `ref<LispyPair>`).
/// * `closure` — what `alloc_closure` itself produces, so a closure capturing
///   another closure lands here.
/// * `str` — a Twig string is a heap handle on the native/LLVM backends.
///
/// **`any` is the residual hole, and it cannot be closed here.** A bare-`any`
/// destination in a raw-model module is usually a machine integer — that is the
/// whole premise of the `is_lisp_language` gate — but a higher-order Twig
/// function receiving a closure, list, or string also gets `any`. The two are
/// indistinguishable in the type hints, so asserting on `any` would reject every
/// closure program that works today. This assert therefore catches the cases the
/// frontend types precisely and leaves the ambiguous one to the tag-directed
/// extraction that supersedes this whole scheme.
fn may_hold_a_pointer(hint: &str) -> bool {
    hint.starts_with("ref<") || hint == "closure" || hint == "str"
}

/// Map each destination (and parameter) in `f` to the type hint it was produced
/// with, so `store` can tell a raw machine value from a reference. Same shape as
/// `dynamic_arith::producer_types`.
fn producer_types(f: &IIRFunction) -> BTreeMap<String, String> {
    let mut m: BTreeMap<String, String> = BTreeMap::new();
    for (name, ty) in &f.params {
        m.insert(name.clone(), ty.clone());
    }
    for instr in &f.instructions {
        if let Some(dest) = &instr.dest {
            m.insert(dest.clone(), instr.type_hint.clone());
        }
    }
    m
}

/// Emit `dest = ( box(idx) . ( cap0 . ( … . nil ) ) )`.
fn emit_closure_alloc(
    out: &mut Vec<IIRInstr>,
    dest: &str,
    idx: i64,
    captures: &[Operand],
    counter: &mut usize,
    params_are_boxed: bool,
    types: &BTreeMap<String, String>,
) {
    // Build the captures list bottom-up, seeded with nil.
    let mut chain = fresh(counter, "clonil");
    out.push(IIRInstr::new("const", Some(chain.clone()), vec![Operand::Int(0)], REF_PAIR));
    for cap in captures.iter().rev() {
        let next = fresh(counter, "clocons");
        let stored = store(out, cap.clone(), counter, !params_are_boxed, types);
        out.push(cons(&next, stored, Operand::Var(chain)));
        chain = next;
    }
    // Prepend the boxed dispatch index → the finished closure object.
    let idx_raw = fresh(counter, "cloidx");
    out.push(IIRInstr::new("const", Some(idx_raw.clone()), vec![Operand::Int(idx)], "i64"));
    let idx_box = fresh(counter, "cloidxb");
    out.push(IIRInstr::new("box", Some(idx_box.clone()), vec![Operand::Var(idx_raw)], REF_ANY));
    out.push(cons(dest, Operand::Var(idx_box), Operand::Var(chain)));
}

/// Emit `dest = __dyn_call_closure(handle, (arg0 . (arg1 . … . nil)))`.
fn emit_closure_call(
    out: &mut Vec<IIRInstr>,
    dest: &str,
    handle: Operand,
    args: &[Operand],
    counter: &mut usize,
    params_are_boxed: bool,
    types: &BTreeMap<String, String>,
) {
    let mut chain = fresh(counter, "argnil");
    out.push(IIRInstr::new("const", Some(chain.clone()), vec![Operand::Int(0)], REF_PAIR));
    for arg in args.iter().rev() {
        let next = fresh(counter, "argcons");
        let stored = store(out, arg.clone(), counter, !params_are_boxed, types);
        out.push(cons(&next, stored, Operand::Var(chain)));
        chain = next;
    }
    out.push(IIRInstr::new(
        "call",
        Some(dest.to_string()),
        vec![Operand::Var(DISPATCHER.to_string()), handle, Operand::Var(chain)],
        REF_ANY,
    ));
}

/// Build the synthesized `__dyn_call_closure(clo, args) -> ref<any>` dispatcher.
fn build_dispatcher(dispatch: &[Dispatch], params_are_boxed: bool) -> IIRFunction {
    let mut instrs: Vec<IIRInstr> = Vec::new();
    // Pull one value out of a cons chain and hand it to the body in the form the
    // body's calling convention expects: tagged for a lisp language, raw for
    // Twig/Nib (see the `params_are_boxed` note in `lower_closures_to_heap`).
    // Returns the variable holding the value to pass.
    let extract = |instrs: &mut Vec<IIRInstr>, dest: &str, from: &str| -> String {
        instrs.push(car(dest, from));
        if params_are_boxed {
            return dest.to_string();
        }
        let raw = format!("{dest}_raw");
        instrs.push(IIRInstr::new(
            "unbox",
            Some(raw.clone()),
            vec![Operand::Var(dest.to_string())],
            "i64",
        ));
        raw
    };
    // idx_box = car(clo)  (a BOXED integer) ; caps = cdr(clo)
    instrs.push(car("cd_idxbox", "clo"));
    instrs.push(cdr("cd_caps", "clo"));

    for d in dispatch {
        let k = d.idx;
        let next = format!("cd_next_{k}");
        // Compare the boxed dispatch index against the raw constant `k` with the
        // **dynamic** `=` — exactly the proven E6d-6 match/union tag test:
        // `lower_dynamic_arith` unboxes the boxed operand (`cd_idxbox`), uses the
        // raw `i64` const directly, and emits the correctly-typed `cmp_eq` per
        // backend (WASM i32/JVM/CLR/native i64), re-boxing the bool. E6d-6 made a
        // boxed-bool `jmp_if_false` branch on its raw bool on every backend — so
        // this is uniform where a hand-rolled `unbox`+`cmp_eq` was not (the
        // unboxed width differs: WASM i31/i32 vs native/LLVM i64).
        instrs.push(IIRInstr::new("const", Some(format!("cd_k{k}")), vec![Operand::Int(k)], "i64"));
        instrs.push(IIRInstr::new(
            "call_builtin",
            Some(format!("cd_eq{k}")),
            vec![Operand::Var("=".into()), Operand::Var("cd_idxbox".into()), Operand::Var(format!("cd_k{k}"))],
            "any",
        ));
        instrs.push(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(format!("cd_eq{k}")), Operand::Var(next.clone())],
            "void",
        ));
        // Walk `caps` for the first n_captures params, then `args` for the rest,
        // and call the body directly.
        let mut call_srcs: Vec<Operand> = vec![Operand::Var(d.fn_name.clone())];
        let mut cur = "cd_caps".to_string();
        for i in 0..d.n_captures {
            let capv = format!("cd_cap_{k}_{i}");
            let passed = extract(&mut instrs, &capv, &cur);
            call_srcs.push(Operand::Var(passed));
            if i + 1 < d.n_captures {
                let rest = format!("cd_caprest_{k}_{i}");
                instrs.push(cdr(&rest, &cur));
                cur = rest;
            }
        }
        let n_args = d.n_params.saturating_sub(d.n_captures);
        let mut acur = "args".to_string();
        for j in 0..n_args {
            let argv = format!("cd_arg_{k}_{j}");
            let passed = extract(&mut instrs, &argv, &acur);
            call_srcs.push(Operand::Var(passed));
            if j + 1 < n_args {
                let rest = format!("cd_argrest_{k}_{j}");
                instrs.push(cdr(&rest, &acur));
                acur = rest;
            }
        }
        let resv = format!("cd_res_{k}");
        // This `call`'s type hint is deliberately `ref<any>` regardless of
        // `params_are_boxed`/whether `d.fn_name` turns out to stay on the raw
        // or the tagged model — this pass runs on BOTH the native and the WASM
        // pipeline (see the module doc comment), and the two disagree on how
        // to react to a callee that turns out raw: WASM's
        // `lower_dyn_repr_structural` retypes a non-lisp callee's call result
        // itself (see that pass, which now also corrects this exact hint —
        // `closure_identity_returns_captured_value`), while the native
        // pipeline's tagged-word model does not need to (every value, raw or
        // boxed, is the same machine word there) and regressed when this hint
        // was changed here instead (`closures_run_on_native`/`_llvm` dropped
        // from 42 to 80 — some `box`/`unbox` pairing keyed off this hint
        // stopped matching). So the correction belongs in the WASM-specific
        // consumer of this IIR, not in this shared emitter.
        instrs.push(IIRInstr::new("call", Some(resv.clone()), call_srcs, REF_ANY));
        instrs.push(IIRInstr::new("ret", None, vec![Operand::Var(resv)], REF_ANY));
        instrs.push(IIRInstr::new("label", None, vec![Operand::Var(next)], "void"));
    }
    // Unreachable default (a closed world always matches): return nil.
    instrs.push(IIRInstr::new("const", Some("cd_nil".into()), vec![Operand::Int(0)], REF_PAIR));
    instrs.push(IIRInstr::new("ret", None, vec![Operand::Var("cd_nil".into())], REF_PAIR));

    IIRFunction::new(
        DISPATCHER,
        vec![("clo".into(), REF_ANY.into()), ("args".into(), REF_ANY.into())],
        REF_ANY,
        instrs,
    )
}

// --- small IIR builders -----------------------------------------------------

fn fresh(counter: &mut usize, tag: &str) -> String {
    *counter += 1;
    format!("__clo_{tag}{counter}")
}
/// `dest = cons(a, b)` via the shared heap `cons` builtin.
fn cons(dest: &str, a: Operand, b: Operand) -> IIRInstr {
    IIRInstr::new("call_builtin", Some(dest.to_string()), vec![Operand::Var("cons".into()), a, b], REF_ANY)
}
/// `dest = car(x)`.
fn car(dest: &str, x: &str) -> IIRInstr {
    IIRInstr::new("call_builtin", Some(dest.to_string()), vec![Operand::Var("car".into()), Operand::Var(x.to_string())], REF_ANY)
}
/// `dest = cdr(x)`.
fn cdr(dest: &str, x: &str) -> IIRInstr {
    IIRInstr::new("call_builtin", Some(dest.to_string()), vec![Operand::Var("cdr".into()), Operand::Var(x.to_string())], REF_ANY)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn module_with(funcs: Vec<IIRFunction>) -> IIRModule {
        let mut m = IIRModule::new("m", "twig");
        m.functions = funcs;
        m
    }

    /// A capture-free `((lambda (x) x) …)`: one body `__lambda_0(x)`.
    #[test]
    fn alloc_and_call_lower_to_cons_and_dispatch() {
        let body = IIRFunction::new("__lambda_0", vec![("x".into(), "any".into())], "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "any")]);
        let main = IIRFunction::new("main", vec![], "any", vec![
            IIRInstr::new("alloc_closure", Some("clo".into()), vec![Operand::Str("__lambda_0".into())], "closure"),
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(41)], "i64"),
            IIRInstr::new("call_closure", Some("r".into()), vec![Operand::Var("clo".into()), Operand::Var("a".into())], "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ]);
        let mut m = module_with(vec![body, main]);
        lower_closures_to_heap(&mut m);

        // No closure opcodes remain.
        assert!(!m.functions.iter().any(|f| f.instructions.iter().any(|i| i.op == "alloc_closure" || i.op == "call_closure")));
        // A dispatcher was injected.
        let disp = m.functions.iter().find(|f| f.name == DISPATCHER).expect("dispatcher injected");
        // It calls the one body directly and dispatches on a cmp_eq.
        assert!(disp.instructions.iter().any(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "=")));
        assert!(disp.instructions.iter().any(|i| i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "__lambda_0")));
        // main now builds the closure with `cons` and calls the dispatcher.
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == DISPATCHER)));
        assert!(main.instructions.iter().any(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "cons")));
    }

    /// A raw-model (Twig) module must box on the way into the cons chain and
    /// unbox on the way out, so the body still receives raw machine words.
    ///
    /// Regression: the chain held two representations at once — `dyn_repr` boxed
    /// a captured *literal* but could not prove a captured bare-`any` *parameter*
    /// was raw, so it went in untagged. Whatever single rule the dispatcher used
    /// was then wrong for one of them: `((lambda (x) (+ x 1)) 41)` exited 73
    /// (`(41 << 3) + 1`) and the curried form exited 80.
    #[test]
    fn raw_model_boxes_into_the_chain_and_unboxes_out_of_it() {
        let l0 = IIRFunction::new(
            "__lambda_0",
            vec![("x".into(), "any".into())],
            "any",
            vec![
                IIRInstr::new(
                    "alloc_closure",
                    Some("c".into()),
                    vec![Operand::Str("__lambda_1".into()), Operand::Var("x".into())],
                    "closure",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "any"),
            ],
        );
        let l1 = IIRFunction::new(
            "__lambda_1",
            vec![("x".into(), "any".into()), ("y".into(), "any".into())],
            "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("y".into())], "any")],
        );
        let mut m = module_with(vec![l0, l1]);
        lower_closures_to_heap(&mut m);

        // Producer side: the captured parameter is boxed before it is consed.
        let l0 = m.functions.iter().find(|f| f.name == "__lambda_0").unwrap();
        assert!(
            l0.instructions.iter().any(|i| i.op == "box"
                && matches!(i.srcs.first(), Some(Operand::Var(v)) if v == "x")),
            "the captured raw parameter must be boxed into the chain: {:?}",
            l0.instructions.iter().map(|i| i.op.clone()).collect::<Vec<_>>()
        );

        // Consumer side: every value the dispatcher pulls out is unboxed again.
        let disp = m.functions.iter().find(|f| f.name == DISPATCHER).unwrap();
        let cars = disp.instructions.iter().filter(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "car"))
            .count();
        let unboxes = disp.instructions.iter().filter(|i| i.op == "unbox").count();
        // One `car` reads the dispatch index (which the `=` test unboxes itself);
        // the rest read captures/args and must each be unboxed.
        assert_eq!(unboxes, cars - 1, "every extracted capture/arg is unboxed");
    }

    /// A raw-model closure that captures another **closure** must be rejected, not
    /// silently boxed: the capture is a heap handle (`addr | 0b111`), and shifting
    /// it left by 3 hides it from the collector while the chain is its only root.
    ///
    /// The first version of this guard tested `starts_with("ref<")`, which misses
    /// the two hints a Twig frontend actually stamps on pointer destinations —
    /// `closure` (from `alloc_closure` itself) and `str` — so the exact case its
    /// own doc comment promised to catch sailed through. Found in security review.
    #[test]
    #[should_panic(expected = "pointer-valued operand")]
    fn raw_model_closure_capturing_a_closure_is_rejected() {
        // `outer` allocates one closure, then captures THAT handle in a second —
        // the shape `let f = fn…; let g = fn(y) => f(y)` lowers to.
        let inner = IIRFunction::new(
            "__lambda_0",
            vec![("x".into(), "any".into())],
            "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "any")],
        );
        let outer = IIRFunction::new(
            "__lambda_1",
            vec![("f".into(), "any".into()), ("y".into(), "any".into())],
            "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("y".into())], "any")],
        );
        let main = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![
                // `f` is produced by `alloc_closure`, so its hint is `closure`.
                IIRInstr::new(
                    "alloc_closure",
                    Some("f".into()),
                    vec![Operand::Str("__lambda_0".into())],
                    "closure",
                ),
                // …and is then captured by a second closure.
                IIRInstr::new(
                    "alloc_closure",
                    Some("g".into()),
                    vec![Operand::Str("__lambda_1".into()), Operand::Var("f".into())],
                    "closure",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("g".into())], "any"),
            ],
        );
        let mut m = module_with(vec![inner, outer, main]);
        lower_closures_to_heap(&mut m);
    }

    /// The mirror image: a genuinely tagged (lisp) module passes values through
    /// untouched, because its bodies already take and return tagged words.
    #[test]
    fn lisp_model_passes_tagged_values_through_untouched() {
        let l0 = IIRFunction::new(
            "__lambda_0",
            // `ref<any>` — a body that genuinely takes a tagged value. The
            // module's language string is irrelevant to the decision now.
            vec![("x".into(), "ref<any>".into())],
            "ref<any>",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "ref<any>")],
        );
        let main = IIRFunction::new(
            "main",
            vec![],
            "any",
            vec![
                IIRInstr::new(
                    "alloc_closure",
                    Some("clo".into()),
                    vec![Operand::Str("__lambda_0".into())],
                    "closure",
                ),
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(41)], "i64"),
                IIRInstr::new(
                    "call_closure",
                    Some("r".into()),
                    vec![Operand::Var("clo".into()), Operand::Var("a".into())],
                    "any",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
            ],
        );
        let mut m = IIRModule::new("m", "mccarthy-lisp");
        m.functions = vec![l0, main];
        lower_closures_to_heap(&mut m);

        let disp = m.functions.iter().find(|f| f.name == DISPATCHER).unwrap();
        assert_eq!(
            disp.instructions.iter().filter(|i| i.op == "unbox").count(),
            0,
            "a lisp body takes tagged values — the dispatcher must not unbox"
        );
        let main = m.functions.iter().find(|f| f.name == "main").unwrap();
        assert_eq!(
            main.instructions.iter().filter(|i| i.op == "box").count(),
            1,
            "only the dispatch index is boxed; the argument is already tagged"
        );
    }

    /// Two bodies (a capturing closure) get distinct alphabetical indices and both
    /// appear in the dispatcher.
    #[test]
    fn two_bodies_get_distinct_dispatch_cases() {
        let l0 = IIRFunction::new("__lambda_0", vec![("x".into(), "any".into())], "any",
            vec![IIRInstr::new("alloc_closure", Some("c".into()),
                vec![Operand::Str("__lambda_1".into()), Operand::Var("x".into())], "closure"),
                 IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "any")]);
        let l1 = IIRFunction::new("__lambda_1", vec![("x".into(), "any".into()), ("y".into(), "any".into())], "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("y".into())], "any")]);
        let main = IIRFunction::new("main", vec![], "any", vec![
            IIRInstr::new("alloc_closure", Some("clo".into()), vec![Operand::Str("__lambda_0".into())], "closure"),
            IIRInstr::new("ret", None, vec![Operand::Var("clo".into())], "any"),
        ]);
        let mut m = module_with(vec![l1, l0, main]); // deliberately out of order
        lower_closures_to_heap(&mut m);
        let disp = m.functions.iter().find(|f| f.name == DISPATCHER).unwrap();
        // Two cmp_eq cases, calling __lambda_0 (idx 0, 0 caps + 1 arg) and
        // __lambda_1 (idx 1, 1 cap + 1 arg).
        assert_eq!(disp.instructions.iter().filter(|i| i.op == "call_builtin"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "=")).count(), 2);
        assert!(disp.instructions.iter().any(|i| i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "__lambda_0")));
        let call1 = disp.instructions.iter().find(|i| i.op == "call"
            && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "__lambda_1")).unwrap();
        // __lambda_1 is called with 1 capture + 1 arg = 2 operands (+ the fn name).
        assert_eq!(call1.srcs.len(), 3);
    }
}

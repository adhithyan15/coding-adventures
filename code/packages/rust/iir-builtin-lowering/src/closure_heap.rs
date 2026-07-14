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

/// Module-level entry: lower `alloc_closure`/`call_closure` to the cons-heap form
/// + a synthesized dispatcher, for the WASM backend. A no-op if the module has no
/// closures.
pub fn lower_closures_to_heap(module: &mut IIRModule) {
    let dispatch = collect_dispatch(module);
    if dispatch.is_empty() {
        return; // no closures — nothing to do (every closure-free program).
    }
    let index_of: BTreeMap<String, i64> =
        dispatch.iter().map(|d| (d.fn_name.clone(), d.idx)).collect();

    for f in &mut module.functions {
        rewrite_closure_ops(f, &index_of);
    }
    if !module.functions.iter().any(|f| f.name == DISPATCHER) {
        module.functions.push(build_dispatcher(&dispatch));
    }
}

/// Rewrite `alloc_closure`/`call_closure` in one function to cons-chain builds +
/// a `call __dyn_call_closure`.
fn rewrite_closure_ops(f: &mut IIRFunction, index_of: &BTreeMap<String, i64>) {
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
            emit_closure_alloc(&mut out, &dest, idx, &captures, &mut counter);
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
            emit_closure_call(&mut out, &dest, handle, &args, &mut counter);
            continue;
        }
        out.push(instr);
    }
    f.instructions = out;
}

/// Emit `dest = ( box(idx) . ( cap0 . ( … . nil ) ) )`.
fn emit_closure_alloc(
    out: &mut Vec<IIRInstr>,
    dest: &str,
    idx: i64,
    captures: &[Operand],
    counter: &mut usize,
) {
    // Build the captures list bottom-up, seeded with nil.
    let mut chain = fresh(counter, "clonil");
    out.push(IIRInstr::new("const", Some(chain.clone()), vec![Operand::Int(0)], REF_PAIR));
    for cap in captures.iter().rev() {
        let next = fresh(counter, "clocons");
        out.push(cons(&next, cap.clone(), Operand::Var(chain)));
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
) {
    let mut chain = fresh(counter, "argnil");
    out.push(IIRInstr::new("const", Some(chain.clone()), vec![Operand::Int(0)], REF_PAIR));
    for arg in args.iter().rev() {
        let next = fresh(counter, "argcons");
        out.push(cons(&next, arg.clone(), Operand::Var(chain)));
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
fn build_dispatcher(dispatch: &[Dispatch]) -> IIRFunction {
    let mut instrs: Vec<IIRInstr> = Vec::new();
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
            instrs.push(car(&capv, &cur));
            call_srcs.push(Operand::Var(capv));
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
            instrs.push(car(&argv, &acur));
            call_srcs.push(Operand::Var(argv));
            if j + 1 < n_args {
                let rest = format!("cd_argrest_{k}_{j}");
                instrs.push(cdr(&rest, &acur));
                acur = rest;
            }
        }
        let resv = format!("cd_res_{k}");
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

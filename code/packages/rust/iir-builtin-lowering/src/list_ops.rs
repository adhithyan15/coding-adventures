//! # list_ops — LANG-FULL E6d-3b: list *operations* over a cons chain.
//!
//! `E6d-3a` (`heap::desugar_list_in_function`) handled the `list` *constructor*
//! as a pure desugar to `cons`. The list *operations* — `length`, `list-ref`,
//! … — instead **walk** the cons chain, so they cannot be a straight-line
//! desugar; each one becomes a call to a small synthesized recursive helper
//! that is injected into the module (once) alongside the caller.
//!
//! ## The shape: one synthesized IIR helper per op
//!
//! `length` → a recursive `__dyn_list_length`:
//!
//! ```text
//!   fn __dyn_list_length(lst : ref<any>) -> ref<any> {
//!       is_nil = null?(lst)                 ;; → is_null   (heap.rs, E6d-1)
//!       if !is_nil goto recurse
//!       ret box 0                           ;; nil ⇒ boxed length 0
//!     recurse:
//!       rest = cdr(lst)                     ;; → field_load[1] / dyn_cdr
//!       sub  = __dyn_list_length(rest)      ;; recurse (boxed result)
//!       ret (+ 1 sub)                       ;; DYNAMIC add (E6d-2) → boxed
//!   }
//! ```
//!
//! `list-ref` → a recursive `__dyn_list_ref`:
//!
//! ```text
//!   fn __dyn_list_ref(lst : ref<any>, n : ref<any>) -> ref<any> {
//!       ni = unbox n                        ;; boxed index → machine i64
//!       if !(ni == 0) goto recurse          ;; TYPED cmp_eq → raw bool
//!       ret car(lst)                        ;; base: the n-th element
//!     recurse:
//!       rest = cdr(lst)                     ;; → field_load[1] / dyn_cdr
//!       ret __dyn_list_ref(rest, box(ni-1)) ;; TYPED sub, re-box for the call
//!   }
//! ```
//!
//! `append` → a recursive `__dyn_list_append` that *rebuilds* the first list:
//!
//! ```text
//!   fn __dyn_list_append(a : ref<any>, b : ref<any>) -> ref<any> {
//!       if !null?(a) goto recurse           ;; → is_null (heap.rs, E6d-1)
//!       ret b                               ;; append(nil, b) = b
//!     recurse:
//!       ret cons(car(a), __dyn_list_append(cdr(a), b))   ;; cons a new cell
//!   }
//! ```
//!
//! `append` needs no index, so no unbox/box: `a`/`b` and every value it touches
//! (`car(a)`, the recursive result) are already references. Its one new op is the
//! `cons` in the recursive arm — the same heap builtin (E6d-1) the head-of-heap
//! pass lowers for the injected helper too.
//!
//! `reverse` → a **tail-recursive accumulator** helper `__dyn_list_reverse`,
//! seeded at the call site with an empty (`nil`) accumulator:
//!
//! ```text
//!   reverse(a)          = reverse_acc(a, nil)          ;; nil seed at call site
//!   reverse_acc(a, acc) = if null?(a) then acc
//!                         else reverse_acc(cdr(a), cons(car(a), acc))
//! ```
//!
//! Consing each element of `a` onto the *front* of `acc` reverses the order. Like
//! `append` it needs no index (all references), and its only new op versus the
//! others is the call-site `nil` seed (`const 0 : ref<LispyPair>`, the sentinel
//! the `list` desugar emits) — the recursion reuses `null?`/`car`/`cdr`/`cons`.
//!
//! `assoc` → a recursive `__dyn_list_assoc` searching an **association list**
//! (a list of `(k . v)` cons pairs) for a key:
//!
//! ```text
//!   assoc(key, alist) = if null?(alist) then nil
//!                       else if key == car(car(alist)) then car(alist)
//!                       else assoc(key, cdr(alist))
//! ```
//!
//! The key comparison must yield a raw machine bool for `jmp_if_false`; since
//! `equal?` lowers unevenly across the managed/runtime paths, V1 `assoc` **unboxes
//! both keys to `i64` and compares with typed `cmp_eq`** (the `list-ref` index
//! technique), so V1 keys are **integers** — symbol keys arrive with E6d-4.
//!
//! ### The lisp boundary is uniform-anyref — the index is boxed too
//!
//! `length` *returns* a number, so both arms return a boxed `ref<any>` — a
//! function that returns a machine `i64` in one arm and a lisp `ref` in another
//! confuses `dyn_repr`'s lisp/typed partition (that mixed shape was the original
//! E6d-3b bug). Its increment is a **dynamic** `call_builtin "+"` (E6d-2
//! unboxes/adds/re-boxes) so nothing new must lower.
//!
//! `list-ref` also honours the boundary: `dyn_repr` boxes *every* argument to a
//! lisp function, so the index param must be `ref<any>` (a raw-`i64` param would
//! fault — `expected i64, got I32(2)` — when the caller hands it an `i31ref`).
//! The helper **unboxes the index once** to a machine `i64`, then the walk is
//! plain typed arithmetic: `(ni == 0)` → raw `cmp_eq : bool` (feeding
//! `jmp_if_false` directly, hint `"bool"`, not a lisp-truthiness test) and
//! `(ni - 1)` → raw `sub : i64`, re-boxed before the recursive `call`. Its
//! *return* is always a `car` result or the recursive call — both `ref<any>` —
//! so the return type is cleanly lisp, just like `length`.
//!
//! ## Where it runs
//!
//! `lower_list_ops` is invoked at the **head of both** `lower_heap_builtins` and
//! `lower_heap_builtins_runtime` (like `desugar_list_in_function`), *before* the
//! per-function heap lowering — so the helpers it injects have their
//! `null?`/`car`/`cdr` lowered by the same pass, on both the managed and
//! native/runtime paths, with no lang-aot pipeline change.
//!
//! Recursion depth = list length; V1 lists are short (the same shape as the
//! runtime's structural-recursion). Iterative rewriting is a later refinement.

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::IIRModule;

/// The synthesized recursive length helper's function name.
const LENGTH_HELPER: &str = "__dyn_list_length";
/// The synthesized recursive list-ref helper's function name.
const LISTREF_HELPER: &str = "__dyn_list_ref";
/// The synthesized recursive append helper's function name.
const APPEND_HELPER: &str = "__dyn_list_append";
/// The synthesized tail-recursive reverse (accumulator) helper's function name.
const REVERSE_HELPER: &str = "__dyn_list_reverse";
/// The synthesized recursive assoc (association-list search) helper's name.
const ASSOC_HELPER: &str = "__dyn_list_assoc";
/// The nil-sentinel `const` type hint (`const 0 : ref<LispyPair>`) — the exact
/// form `desugar_list_in_function`/`make_nil` emit for the empty list. `reverse`
/// seeds its accumulator with one.
const REF_PAIR: &str = "ref<LispyPair>";

/// Module-level entry: rewrite every list-*operation* `call_builtin` to a call to
/// its synthesized recursive helper, and inject each helper once if any call to
/// that op was present. Currently handles `length` and `list-ref`; further ops
/// (`append`, `reverse`, `assoc`) follow the same add-a-helper pattern.
pub fn lower_list_ops(module: &mut IIRModule) {
    lower_length(module);
    lower_list_ref(module);
    lower_append(module);
    lower_reverse(module);
    lower_assoc(module);
}

/// Rewrite `length` calls + inject `__dyn_list_length` once.
fn lower_length(module: &mut IIRModule) {
    let uses_length = module.functions.iter().any(|f| {
        f.instructions.iter().any(is_length_call)
    });
    if !uses_length {
        return;
    }
    for f in &mut module.functions {
        rewrite_length_calls(f);
    }
    // Inject the helper exactly once (idempotent: both heap entry points may
    // call this, but only one runs per pipeline, and this guards re-injection).
    if !module.functions.iter().any(|f| f.name == LENGTH_HELPER) {
        module.functions.push(build_length_helper());
    }
}

/// Rewrite `list-ref` calls + inject `__dyn_list_ref` once.
fn lower_list_ref(module: &mut IIRModule) {
    let uses_list_ref = module.functions.iter().any(|f| {
        f.instructions.iter().any(is_listref_call)
    });
    if !uses_list_ref {
        return;
    }
    for f in &mut module.functions {
        rewrite_listref_calls(f);
    }
    if !module.functions.iter().any(|f| f.name == LISTREF_HELPER) {
        module.functions.push(build_listref_helper());
    }
}

/// Rewrite `append` calls + inject `__dyn_list_append` once.
fn lower_append(module: &mut IIRModule) {
    let uses_append = module.functions.iter().any(|f| {
        f.instructions.iter().any(is_append_call)
    });
    if !uses_append {
        return;
    }
    for f in &mut module.functions {
        rewrite_append_calls(f);
    }
    if !module.functions.iter().any(|f| f.name == APPEND_HELPER) {
        module.functions.push(build_append_helper());
    }
}

/// Rewrite `reverse` calls + inject `__dyn_list_reverse` once.
fn lower_reverse(module: &mut IIRModule) {
    let uses_reverse = module.functions.iter().any(|f| {
        f.instructions.iter().any(is_reverse_call)
    });
    if !uses_reverse {
        return;
    }
    for f in &mut module.functions {
        rewrite_reverse_calls(f);
    }
    if !module.functions.iter().any(|f| f.name == REVERSE_HELPER) {
        module.functions.push(build_reverse_helper());
    }
}

/// Rewrite `assoc` calls + inject `__dyn_list_assoc` once.
fn lower_assoc(module: &mut IIRModule) {
    let uses_assoc = module.functions.iter().any(|f| {
        f.instructions.iter().any(is_assoc_call)
    });
    if !uses_assoc {
        return;
    }
    for f in &mut module.functions {
        rewrite_assoc_calls(f);
    }
    if !module.functions.iter().any(|f| f.name == ASSOC_HELPER) {
        module.functions.push(build_assoc_helper());
    }
}

/// Is `instr` a `call_builtin "length" …`?
fn is_length_call(instr: &IIRInstr) -> bool {
    instr.op == "call_builtin"
        && matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "length")
}

/// Replace each `call_builtin "length" lst -> dest` with
/// `call __dyn_list_length, lst -> {dest}.len_i64 : i64` + `box … -> dest : ref<any>`.
fn rewrite_length_calls(f: &mut IIRFunction) {
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    for instr in old {
        if !is_length_call(&instr) {
            out.push(instr);
            continue;
        }
        // No dest ⇒ result unused; drop the (pure) call.
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        // srcs = [Var("length"), lst].
        let lst = match instr.srcs.get(1) {
            Some(op) => op.clone(),
            None => { out.push(instr); continue; }
        };
        // The helper is a proper lisp function returning a boxed `ref<any>`
        // (a boxed integer count), so the call result goes straight into `dest`
        // — downstream (further dynamic ops / the program result) sees a lisp
        // value, exactly like a `car`/`length` result should.
        out.push(IIRInstr::new(
            "call",
            Some(dest),
            vec![Operand::Var(LENGTH_HELPER.to_string()), lst],
            "ref<any>",
        ));
    }
    f.instructions = out;
}

/// Build the recursive `__dyn_list_length(lst: ref<any>) -> ref<any>` helper.
///
/// It is a **proper lisp function**: both branches return a boxed `ref<any>`
/// value, so the shared `dyn_repr` boxing pass (which classifies a function that
/// calls lisp builtins as "lisp") handles it consistently — a mixed i64/ref
/// helper confused that partition. The `1 + length(cdr)` is a **dynamic** add
/// (`call_builtin "+"` over a raw `i64` `1` and the boxed recursive result),
/// lowered by the E6d-2 `dynamic_arith` pass (`unbox` the boxed operand, `add`,
/// `box` the result) — so nothing new must lower.
fn build_length_helper() -> IIRFunction {
    let recurse = "__dll_recurse".to_string();
    let instrs = vec![
        // is_nil = null?(lst)
        IIRInstr::new(
            "call_builtin", Some("__dll_is_nil".into()),
            vec![Operand::Var("null?".into()), Operand::Var("lst".into())], "bool",
        ),
        // if is_nil is FALSE → recurse; else fall through to the nil base case.
        IIRInstr::new(
            "jmp_if_false", None,
            vec![Operand::Var("__dll_is_nil".into()), Operand::Var(recurse.clone())], "void",
        ),
        // base: nil ⇒ boxed 0
        IIRInstr::new("const", Some("__dll_zero".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("box", Some("__dll_zbox".into()), vec![Operand::Var("__dll_zero".into())], "ref<any>"),
        IIRInstr::new("ret", None, vec![Operand::Var("__dll_zbox".into())], "ref<any>"),
        // recurse:
        IIRInstr::new("label", None, vec![Operand::Var(recurse)], "void"),
        IIRInstr::new(
            "call_builtin", Some("__dll_rest".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("lst".into())], "ref<any>",
        ),
        IIRInstr::new(
            "call", Some("__dll_sub".into()),
            vec![Operand::Var(LENGTH_HELPER.into()), Operand::Var("__dll_rest".into())], "ref<any>",
        ),
        IIRInstr::new("const", Some("__dll_one".into()), vec![Operand::Int(1)], "i64"),
        // 1 + length(cdr) — a DYNAMIC add: `1` (raw i64) + `__dll_sub` (boxed);
        // dynamic_arith unboxes the boxed operand, adds, and re-boxes the result.
        IIRInstr::new(
            "call_builtin", Some("__dll_res".into()),
            vec![Operand::Var("+".into()), Operand::Var("__dll_one".into()), Operand::Var("__dll_sub".into())],
            "ref<any>",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__dll_res".into())], "ref<any>"),
    ];
    IIRFunction::new(
        LENGTH_HELPER,
        vec![("lst".to_string(), "ref<any>".to_string())],
        "ref<any>",
        instrs,
    )
}

// ---------------------------------------------------------------------------
// list-ref
// ---------------------------------------------------------------------------

/// Is `instr` a `call_builtin "list-ref" …`?
fn is_listref_call(instr: &IIRInstr) -> bool {
    instr.op == "call_builtin"
        && matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "list-ref")
}

/// Replace each `call_builtin "list-ref" lst n -> dest` with
/// `call __dyn_list_ref, lst, n -> dest : ref<any>`.
///
/// `lst` (srcs[1]) is the cons chain and `n` (srcs[2]) the index; both are passed
/// straight through to the helper's two `ref<any>` params. The index reaches the
/// call site as `const n : i64`, but the `dyn_repr` pass boxes it (like every
/// lisp-call argument) before it crosses the boundary — the helper unboxes it
/// (see [`build_listref_helper`]). The helper returns the lisp element (a `car`
/// result), so its `ref<any>` return flows straight into `dest`.
fn rewrite_listref_calls(f: &mut IIRFunction) {
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    for instr in old {
        if !is_listref_call(&instr) {
            out.push(instr);
            continue;
        }
        // No dest ⇒ result unused; drop the (pure) call.
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        // srcs = [Var("list-ref"), lst, n]; a malformed arity is left for the
        // validator to reject with full context.
        let (lst, n) = match (instr.srcs.get(1), instr.srcs.get(2)) {
            (Some(lst), Some(n)) => (lst.clone(), n.clone()),
            _ => { out.push(instr); continue; }
        };
        out.push(IIRInstr::new(
            "call",
            Some(dest),
            vec![Operand::Var(LISTREF_HELPER.to_string()), lst, n],
            "ref<any>",
        ));
    }
    f.instructions = out;
}

/// Build the recursive `__dyn_list_ref(lst: ref<any>, n: ref<any>) -> ref<any>`
/// helper.
///
/// ### Both params are boxed lisp values — the index too
///
/// The managed / native lisp boundary is **uniform-anyref**: every argument to a
/// lisp function is boxed by the caller (`dyn_repr_structural` boxes each
/// `call`-arg atom; the native `lower_dyn_repr` does the same with a tagged
/// word). So the index `n` *cannot* be a raw `i64` param — the caller hands it
/// an `i31ref` (managed) / tagged word (native), and an `i64` param would fault
/// (`expected i64, got I32(2)`). We therefore take `n : ref<any>` and **unbox it
/// once** to a machine `i64` `__dlr_ni`.
///
/// With `__dlr_ni` in hand the index walk is plain typed machine arithmetic:
/// `cmp_eq __dlr_ni 0 : bool` feeds `jmp_if_false` (a raw machine bool, not a
/// lisp-truthiness test — hint `"bool"`, so `dyn_repr` leaves it alone), and
/// `sub __dlr_ni 1 : i64` decrements. The decremented index is re-boxed
/// (`box __dlr_nm1b`) before the recursive `call`, mirroring the explicit `box`
/// the `length` helper's base case uses — a proven-portable shape on all five
/// code-gen backends. The base case returns `car(lst)` (the n-th element) and
/// the recursive case the recursive call — both `ref<any>`, so the return is
/// cleanly a lisp value.
///
/// (Out-of-bounds `n ≥ length` walks off the nil tail into `car(nil)`; that is
/// undefined here, matching the V1 runtime's own unguarded `list-ref`.)
fn build_listref_helper() -> IIRFunction {
    let recurse = "__dlr_recurse".to_string();
    let instrs = vec![
        // Unbox the boxed index once → a machine i64 the typed ops below consume.
        IIRInstr::new("unbox", Some("__dlr_ni".into()), vec![Operand::Var("n".into())], "i64"),
        // is_zero = (ni == 0)  — a TYPED comparison producing a raw machine bool.
        IIRInstr::new("const", Some("__dlr_zero".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new(
            "cmp_eq", Some("__dlr_isz".into()),
            vec![Operand::Var("__dlr_ni".into()), Operand::Var("__dlr_zero".into())], "bool",
        ),
        // if is_zero is FALSE → recurse; else fall through to the base case.
        IIRInstr::new(
            "jmp_if_false", None,
            vec![Operand::Var("__dlr_isz".into()), Operand::Var(recurse.clone())], "void",
        ),
        // base: ni == 0 ⇒ car(lst) is the requested element.
        IIRInstr::new(
            "call_builtin", Some("__dlr_head".into()),
            vec![Operand::Var("car".into()), Operand::Var("lst".into())], "ref<any>",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__dlr_head".into())], "ref<any>"),
        // recurse: list-ref(cdr(lst), ni - 1)
        IIRInstr::new("label", None, vec![Operand::Var(recurse)], "void"),
        IIRInstr::new(
            "call_builtin", Some("__dlr_rest".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("lst".into())], "ref<any>",
        ),
        IIRInstr::new("const", Some("__dlr_one".into()), vec![Operand::Int(1)], "i64"),
        IIRInstr::new(
            "sub", Some("__dlr_nm1".into()),
            vec![Operand::Var("__dlr_ni".into()), Operand::Var("__dlr_one".into())], "i64",
        ),
        // Re-box the decremented index for the uniform-anyref recursive call.
        IIRInstr::new("box", Some("__dlr_nm1b".into()), vec![Operand::Var("__dlr_nm1".into())], "ref<any>"),
        IIRInstr::new(
            "call", Some("__dlr_res".into()),
            vec![
                Operand::Var(LISTREF_HELPER.into()),
                Operand::Var("__dlr_rest".into()),
                Operand::Var("__dlr_nm1b".into()),
            ],
            "ref<any>",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__dlr_res".into())], "ref<any>"),
    ];
    IIRFunction::new(
        LISTREF_HELPER,
        vec![
            ("lst".to_string(), "ref<any>".to_string()),
            ("n".to_string(), "ref<any>".to_string()),
        ],
        "ref<any>",
        instrs,
    )
}

// ---------------------------------------------------------------------------
// append
// ---------------------------------------------------------------------------

/// Is `instr` a `call_builtin "append" …`?
fn is_append_call(instr: &IIRInstr) -> bool {
    instr.op == "call_builtin"
        && matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "append")
}

/// Replace each `call_builtin "append" a b -> dest` with
/// `call __dyn_list_append, a, b -> dest : ref<any>`.
///
/// `a` (srcs[1]) and `b` (srcs[2]) are both cons chains (lisp `ref<any>`), passed
/// straight to the helper's two `ref<any>` params. The helper returns the rebuilt
/// list (a fresh `cons` chain), so its `ref<any>` return flows into `dest`.
fn rewrite_append_calls(f: &mut IIRFunction) {
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    for instr in old {
        if !is_append_call(&instr) {
            out.push(instr);
            continue;
        }
        // No dest ⇒ result unused; drop the (pure) call.
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        // srcs = [Var("append"), a, b]; a malformed arity is left for the
        // validator to reject with full context.
        let (a, b) = match (instr.srcs.get(1), instr.srcs.get(2)) {
            (Some(a), Some(b)) => (a.clone(), b.clone()),
            _ => { out.push(instr); continue; }
        };
        out.push(IIRInstr::new(
            "call",
            Some(dest),
            vec![Operand::Var(APPEND_HELPER.to_string()), a, b],
            "ref<any>",
        ));
    }
    f.instructions = out;
}

/// Build the recursive `__dyn_list_append(a: ref<any>, b: ref<any>) -> ref<any>`
/// helper.
///
/// `append` *rebuilds* the first list in front of the second:
///
/// ```text
///   append(a, b) = if null?(a) then b
///                  else cons(car(a), append(cdr(a), b))
/// ```
///
/// Unlike `list-ref` there is **no index** — both arguments are lisp `ref<any>`
/// lists, and every value the helper touches (`car(a)`, the recursive result, the
/// second list `b`) is already a reference, so no unbox/box is needed. The only
/// new op versus `length`/`list-ref` is a `cons` in the recursive arm — the same
/// heap builtin (E6d-1) that the head-of-heap-lowering pass rewrites to
/// `alloc`/`field_store` for the injected helper too, so nothing new must lower.
/// Both arms return `ref<any>` (the second list, or a fresh cons), so the return
/// is cleanly a lisp value.
fn build_append_helper() -> IIRFunction {
    let recurse = "__dla_recurse".to_string();
    let instrs = vec![
        // is_nil = null?(a)
        IIRInstr::new(
            "call_builtin", Some("__dla_is_nil".into()),
            vec![Operand::Var("null?".into()), Operand::Var("a".into())], "bool",
        ),
        // if a is NON-nil → recurse; else fall through: a empty ⇒ result is b.
        IIRInstr::new(
            "jmp_if_false", None,
            vec![Operand::Var("__dla_is_nil".into()), Operand::Var(recurse.clone())], "void",
        ),
        // base: null?(a) ⇒ append(nil, b) = b.
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "ref<any>"),
        // recurse: cons(car(a), append(cdr(a), b))
        IIRInstr::new("label", None, vec![Operand::Var(recurse)], "void"),
        IIRInstr::new(
            "call_builtin", Some("__dla_head".into()),
            vec![Operand::Var("car".into()), Operand::Var("a".into())], "ref<any>",
        ),
        IIRInstr::new(
            "call_builtin", Some("__dla_rest".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("a".into())], "ref<any>",
        ),
        IIRInstr::new(
            "call", Some("__dla_tail".into()),
            vec![
                Operand::Var(APPEND_HELPER.into()),
                Operand::Var("__dla_rest".into()),
                Operand::Var("b".into()),
            ],
            "ref<any>",
        ),
        IIRInstr::new(
            "call_builtin", Some("__dla_res".into()),
            vec![
                Operand::Var("cons".into()),
                Operand::Var("__dla_head".into()),
                Operand::Var("__dla_tail".into()),
            ],
            "ref<any>",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__dla_res".into())], "ref<any>"),
    ];
    IIRFunction::new(
        APPEND_HELPER,
        vec![
            ("a".to_string(), "ref<any>".to_string()),
            ("b".to_string(), "ref<any>".to_string()),
        ],
        "ref<any>",
        instrs,
    )
}

// ---------------------------------------------------------------------------
// reverse
// ---------------------------------------------------------------------------

/// Is `instr` a `call_builtin "reverse" …`?
fn is_reverse_call(instr: &IIRInstr) -> bool {
    instr.op == "call_builtin"
        && matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "reverse")
}

/// Replace each `call_builtin "reverse" a -> dest` with a **nil seed** + a call
/// to the tail-recursive accumulator helper:
///
/// ```text
///   {dest}.rev_nil = const 0 : ref<LispyPair>          ;; the empty accumulator
///   dest           = call __dyn_list_reverse, a, {dest}.rev_nil : ref<any>
/// ```
///
/// `reverse(a)` is `reverse_acc(a, nil)` — the accumulator starts empty and each
/// element is consed on front, so the list comes out reversed (see
/// [`build_reverse_helper`]). The nil `const` is the exact sentinel the `list`
/// desugar / `make_nil` emit; its name is derived from `dest` (unique per SSA
/// destination) so two `reverse` sites never collide.
fn rewrite_reverse_calls(f: &mut IIRFunction) {
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    for instr in old {
        if !is_reverse_call(&instr) {
            out.push(instr);
            continue;
        }
        // No dest ⇒ result unused; drop the (pure) call.
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        // srcs = [Var("reverse"), a]; a malformed arity is left for the validator.
        let a = match instr.srcs.get(1) {
            Some(a) => a.clone(),
            None => { out.push(instr); continue; }
        };
        // Seed the accumulator with a fresh nil, then call the acc helper.
        let nil_name = format!("{dest}.rev_nil");
        out.push(IIRInstr::new(
            "const", Some(nil_name.clone()), vec![Operand::Int(0)], REF_PAIR,
        ));
        out.push(IIRInstr::new(
            "call",
            Some(dest),
            vec![Operand::Var(REVERSE_HELPER.to_string()), a, Operand::Var(nil_name)],
            "ref<any>",
        ));
    }
    f.instructions = out;
}

/// Build the tail-recursive accumulator helper
/// `__dyn_list_reverse(a: ref<any>, acc: ref<any>) -> ref<any>`.
///
/// ```text
///   reverse_acc(a, acc) = if null?(a) then acc
///                         else reverse_acc(cdr(a), cons(car(a), acc))
/// ```
///
/// The accumulator `acc` starts as `nil` (seeded at the call site) and grows by
/// consing each element of `a` onto its **front** — which reverses the order.
/// Like `append` there is no index, so no unbox/box: `a`, `acc`, `car(a)`, and
/// the fresh `cons` are all lisp `ref<any>` references. The recursion is in tail
/// position (`ret` of the recursive call); the IIR backends do not yet do TCO, so
/// depth is bounded by the list length (the same structural-recursion budget the
/// other helpers use). Both arms return `ref<any>`.
fn build_reverse_helper() -> IIRFunction {
    let recurse = "__dlv_recurse".to_string();
    let instrs = vec![
        // is_nil = null?(a)
        IIRInstr::new(
            "call_builtin", Some("__dlv_is_nil".into()),
            vec![Operand::Var("null?".into()), Operand::Var("a".into())], "bool",
        ),
        // if a is NON-nil → recurse; else fall through: a empty ⇒ result is acc.
        IIRInstr::new(
            "jmp_if_false", None,
            vec![Operand::Var("__dlv_is_nil".into()), Operand::Var(recurse.clone())], "void",
        ),
        // base: null?(a) ⇒ the accumulator holds the fully-reversed list.
        IIRInstr::new("ret", None, vec![Operand::Var("acc".into())], "ref<any>"),
        // recurse: reverse_acc(cdr(a), cons(car(a), acc))
        IIRInstr::new("label", None, vec![Operand::Var(recurse)], "void"),
        IIRInstr::new(
            "call_builtin", Some("__dlv_head".into()),
            vec![Operand::Var("car".into()), Operand::Var("a".into())], "ref<any>",
        ),
        IIRInstr::new(
            "call_builtin", Some("__dlv_rest".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("a".into())], "ref<any>",
        ),
        // newacc = cons(car(a), acc) — cons the head onto the accumulator's front.
        IIRInstr::new(
            "call_builtin", Some("__dlv_newacc".into()),
            vec![
                Operand::Var("cons".into()),
                Operand::Var("__dlv_head".into()),
                Operand::Var("acc".into()),
            ],
            "ref<any>",
        ),
        IIRInstr::new(
            "call", Some("__dlv_res".into()),
            vec![
                Operand::Var(REVERSE_HELPER.into()),
                Operand::Var("__dlv_rest".into()),
                Operand::Var("__dlv_newacc".into()),
            ],
            "ref<any>",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__dlv_res".into())], "ref<any>"),
    ];
    IIRFunction::new(
        REVERSE_HELPER,
        vec![
            ("a".to_string(), "ref<any>".to_string()),
            ("acc".to_string(), "ref<any>".to_string()),
        ],
        "ref<any>",
        instrs,
    )
}

// ---------------------------------------------------------------------------
// assoc
// ---------------------------------------------------------------------------

/// Is `instr` a `call_builtin "assoc" …`?
fn is_assoc_call(instr: &IIRInstr) -> bool {
    instr.op == "call_builtin"
        && matches!(instr.srcs.first(), Some(Operand::Var(n)) if n == "assoc")
}

/// Replace each `call_builtin "assoc" key alist -> dest` with
/// `call __dyn_list_assoc, key, alist -> dest : ref<any>`.
///
/// `key` (srcs[1]) and `alist` (srcs[2] — an association list, a list of
/// `(k . v)` cons pairs) are both lisp `ref<any>`, passed to the helper's two
/// `ref<any>` params. The helper returns the matching `(k . v)` pair or `nil`.
fn rewrite_assoc_calls(f: &mut IIRFunction) {
    let old = std::mem::take(&mut f.instructions);
    let mut out: Vec<IIRInstr> = Vec::with_capacity(old.len() + 2);
    for instr in old {
        if !is_assoc_call(&instr) {
            out.push(instr);
            continue;
        }
        // No dest ⇒ result unused; drop the (pure) call.
        let dest = match &instr.dest {
            Some(d) => d.clone(),
            None => continue,
        };
        // srcs = [Var("assoc"), key, alist]; a malformed arity is left for the
        // validator to reject with full context.
        let (key, alist) = match (instr.srcs.get(1), instr.srcs.get(2)) {
            (Some(key), Some(alist)) => (key.clone(), alist.clone()),
            _ => { out.push(instr); continue; }
        };
        out.push(IIRInstr::new(
            "call",
            Some(dest),
            vec![Operand::Var(ASSOC_HELPER.to_string()), key, alist],
            "ref<any>",
        ));
    }
    f.instructions = out;
}

/// Build the recursive
/// `__dyn_list_assoc(key: ref<any>, alist: ref<any>) -> ref<any>` helper.
///
/// `assoc` searches an **association list** — a list of `(k . v)` cons pairs — and
/// returns the first pair whose key equals `key`, or `nil` if none match:
///
/// ```text
///   assoc(key, alist) = if null?(alist) then nil
///                       else let pair = car(alist) in
///                            if key == car(pair) then pair
///                            else assoc(key, cdr(alist))
/// ```
///
/// ### V1 scope: integer keys via `unbox` + typed `cmp_eq`
///
/// The key comparison must produce a **raw machine bool** to feed `jmp_if_false`.
/// The `equal?` builtin lowers differently across the managed / runtime paths
/// (`dyn_equal` on native) and its result is not a plain branch bool, so — exactly
/// as `list-ref` does for its index — this V1 helper **unboxes both keys to `i64`**
/// and compares with a typed `cmp_eq`. That restricts V1 `assoc` to **integer
/// keys** (every E6d-3b proof so far uses integer atoms); symbol keys arrive with
/// E6d-4 (interned symbols → `eq?` bit-equality). The alist cells, the returned
/// pair, and `nil` are all references, so no boxing is needed on the value path.
fn build_assoc_helper() -> IIRFunction {
    let search = "__das_search".to_string();
    let next = "__das_next".to_string();
    let instrs = vec![
        // is_nil = null?(alist)
        IIRInstr::new(
            "call_builtin", Some("__das_is_nil".into()),
            vec![Operand::Var("null?".into()), Operand::Var("alist".into())], "bool",
        ),
        // if alist is NON-nil → search; else fall through: exhausted ⇒ nil.
        IIRInstr::new(
            "jmp_if_false", None,
            vec![Operand::Var("__das_is_nil".into()), Operand::Var(search.clone())], "void",
        ),
        // base: alist empty ⇒ key not found ⇒ nil.
        IIRInstr::new("const", Some("__das_nil".into()), vec![Operand::Int(0)], REF_PAIR),
        IIRInstr::new("ret", None, vec![Operand::Var("__das_nil".into())], "ref<any>"),
        // search: compare `key` with the current pair's key.
        IIRInstr::new("label", None, vec![Operand::Var(search)], "void"),
        IIRInstr::new(
            "call_builtin", Some("__das_pair".into()),
            vec![Operand::Var("car".into()), Operand::Var("alist".into())], "ref<any>",
        ),
        IIRInstr::new(
            "call_builtin", Some("__das_pkey".into()),
            vec![Operand::Var("car".into()), Operand::Var("__das_pair".into())], "ref<any>",
        ),
        // Unbox both keys to machine i64 and compare (typed → raw bool).
        IIRInstr::new("unbox", Some("__das_ki".into()), vec![Operand::Var("key".into())], "i64"),
        IIRInstr::new("unbox", Some("__das_pki".into()), vec![Operand::Var("__das_pkey".into())], "i64"),
        IIRInstr::new(
            "cmp_eq", Some("__das_eq".into()),
            vec![Operand::Var("__das_ki".into()), Operand::Var("__das_pki".into())], "bool",
        ),
        // if keys DIFFER → next; else fall through: match ⇒ return this pair.
        IIRInstr::new(
            "jmp_if_false", None,
            vec![Operand::Var("__das_eq".into()), Operand::Var(next.clone())], "void",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__das_pair".into())], "ref<any>"),
        // next: recurse on the rest of the alist.
        IIRInstr::new("label", None, vec![Operand::Var(next)], "void"),
        IIRInstr::new(
            "call_builtin", Some("__das_rest".into()),
            vec![Operand::Var("cdr".into()), Operand::Var("alist".into())], "ref<any>",
        ),
        IIRInstr::new(
            "call", Some("__das_res".into()),
            vec![
                Operand::Var(ASSOC_HELPER.into()),
                Operand::Var("key".into()),
                Operand::Var("__das_rest".into()),
            ],
            "ref<any>",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("__das_res".into())], "ref<any>"),
    ];
    IIRFunction::new(
        ASSOC_HELPER,
        vec![
            ("key".to_string(), "ref<any>".to_string()),
            ("alist".to_string(), "ref<any>".to_string()),
        ],
        "ref<any>",
        instrs,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_with(instrs: Vec<IIRInstr>) -> IIRModule {
        IIRModule {
            name: "test".into(),
            functions: vec![IIRFunction::new("main", vec![], "any", instrs)],
            entry_point: Some("main".into()),
            language: "twig".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    fn length_call(dest: &str, lst: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin", Some(dest.into()),
            vec![Operand::Var("length".into()), Operand::Var(lst.into())], "any",
        )
    }

    #[test]
    fn no_length_is_noop() {
        let mut m = module_with(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.len(), 1, "no helper injected when length is unused");
    }

    #[test]
    fn length_call_becomes_helper_call() {
        let mut m = module_with(vec![length_call("r", "lst")]);
        lower_list_ops(&mut m);
        let body = &m.functions[0].instructions;
        // call __dyn_list_length, lst -> r : ref<any>  (helper returns a boxed value)
        let call = &body[0];
        assert_eq!(call.op, "call");
        assert_eq!(call.srcs[0], Operand::Var(LENGTH_HELPER.into()));
        assert_eq!(call.srcs[1], Operand::Var("lst".into()));
        assert_eq!(call.dest.as_deref(), Some("r"));
        assert_eq!(call.type_hint, "ref<any>");
        assert!(body.iter().all(|i| !is_length_call(i)), "no length call_builtin survives");
    }

    #[test]
    fn helper_is_injected_once() {
        // Two length calls (even across a helper re-scan) inject exactly one helper.
        let mut m = module_with(vec![length_call("a", "l1"), length_call("b", "l2")]);
        lower_list_ops(&mut m);
        assert_eq!(
            m.functions.iter().filter(|f| f.name == LENGTH_HELPER).count(), 1,
            "exactly one helper",
        );
        // Idempotent: a second pass does not add another.
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == LENGTH_HELPER).count(), 1);
    }

    #[test]
    fn helper_shape_is_recursive_cons_walk() {
        let mut m = module_with(vec![length_call("r", "lst")]);
        lower_list_ops(&mut m);
        let helper = m.functions.iter().find(|f| f.name == LENGTH_HELPER).unwrap();
        assert_eq!(helper.return_type, "ref<any>", "helper is a proper lisp function");
        assert_eq!(helper.params, vec![("lst".to_string(), "ref<any>".to_string())]);
        let ops: Vec<&str> = helper.instructions.iter().map(|i| i.op.as_str()).collect();
        // null? → jmp_if_false → const/box/ret (base) → label → cdr → recursive call → +/ret
        assert!(ops.contains(&"jmp_if_false"));
        assert!(ops.contains(&"label"));
        assert!(ops.contains(&"box"), "base case boxes the 0 count");
        // the increment is a dynamic `call_builtin "+"`, not a typed add.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("+".into()))));
        assert_eq!(ops.iter().filter(|o| **o == "ret").count(), 2, "base + recursive ret");
        // The recursive call targets the helper itself.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call" && i.srcs.first() == Some(&Operand::Var(LENGTH_HELPER.into()))));
    }

    // ── list-ref ────────────────────────────────────────────────────────────

    fn listref_call(dest: &str, lst: &str, n: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin", Some(dest.into()),
            vec![
                Operand::Var("list-ref".into()),
                Operand::Var(lst.into()),
                Operand::Var(n.into()),
            ],
            "any",
        )
    }

    #[test]
    fn no_listref_is_noop() {
        let mut m = module_with(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.len(), 1, "no helper injected when list-ref is unused");
    }

    #[test]
    fn listref_call_becomes_helper_call() {
        let mut m = module_with(vec![listref_call("r", "lst", "idx")]);
        lower_list_ops(&mut m);
        let body = &m.functions[0].instructions;
        // call __dyn_list_ref, lst, idx -> r : ref<any>
        let call = &body[0];
        assert_eq!(call.op, "call");
        assert_eq!(call.srcs[0], Operand::Var(LISTREF_HELPER.into()));
        assert_eq!(call.srcs[1], Operand::Var("lst".into()));
        assert_eq!(call.srcs[2], Operand::Var("idx".into()));
        assert_eq!(call.dest.as_deref(), Some("r"));
        assert_eq!(call.type_hint, "ref<any>");
        assert!(body.iter().all(|i| !is_listref_call(i)), "no list-ref call_builtin survives");
    }

    #[test]
    fn listref_helper_is_injected_once() {
        let mut m = module_with(vec![
            listref_call("a", "l1", "i1"),
            listref_call("b", "l2", "i2"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(
            m.functions.iter().filter(|f| f.name == LISTREF_HELPER).count(), 1,
            "exactly one list-ref helper",
        );
        // Idempotent: a second pass does not add another.
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == LISTREF_HELPER).count(), 1);
    }

    #[test]
    fn listref_helper_shape_walks_index_with_typed_ops() {
        let mut m = module_with(vec![listref_call("r", "lst", "idx")]);
        lower_list_ops(&mut m);
        let helper = m.functions.iter().find(|f| f.name == LISTREF_HELPER).unwrap();
        assert_eq!(helper.return_type, "ref<any>", "helper returns a lisp value");
        assert_eq!(
            helper.params,
            vec![
                ("lst".to_string(), "ref<any>".to_string()),
                ("n".to_string(), "ref<any>".to_string()),
            ],
            "both params are boxed lisp values (uniform-anyref boundary)",
        );
        let ops: Vec<&str> = helper.instructions.iter().map(|i| i.op.as_str()).collect();
        // The boxed index is unboxed once to a machine i64 before the typed walk,
        // and the decremented index is re-boxed for the recursive call.
        assert!(ops.contains(&"unbox"), "boxed index unboxed to i64");
        assert!(ops.contains(&"box"), "decremented index re-boxed for recursion");
        // The index test is a TYPED cmp_eq (raw bool), not a dynamic call_builtin.
        assert!(ops.contains(&"cmp_eq"), "typed index==0 test");
        assert!(ops.contains(&"jmp_if_false"));
        // The decrement is a TYPED sub, not a dynamic call_builtin "+"/"-".
        assert!(ops.contains(&"sub"), "typed index decrement");
        assert!(
            !helper.instructions.iter().any(|i|
                i.op == "call_builtin" && matches!(i.srcs.first(),
                    Some(Operand::Var(n)) if n == "+" || n == "-" || n == "=")),
            "index arithmetic is typed, never a dynamic-arith call_builtin",
        );
        // The base case reads the element via car; the walk steps via cdr.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("car".into()))));
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("cdr".into()))));
        assert_eq!(ops.iter().filter(|o| **o == "ret").count(), 2, "base + recursive ret");
        // The recursive call targets the helper itself.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call" && i.srcs.first() == Some(&Operand::Var(LISTREF_HELPER.into()))));
    }

    #[test]
    fn length_and_listref_coexist() {
        // A module using BOTH ops injects both helpers, exactly once each.
        let mut m = module_with(vec![
            length_call("a", "l1"),
            listref_call("b", "l2", "i2"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == LENGTH_HELPER).count(), 1);
        assert_eq!(m.functions.iter().filter(|f| f.name == LISTREF_HELPER).count(), 1);
    }

    // ── append ──────────────────────────────────────────────────────────────

    fn append_call(dest: &str, a: &str, b: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin", Some(dest.into()),
            vec![
                Operand::Var("append".into()),
                Operand::Var(a.into()),
                Operand::Var(b.into()),
            ],
            "any",
        )
    }

    #[test]
    fn no_append_is_noop() {
        let mut m = module_with(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.len(), 1, "no helper injected when append is unused");
    }

    #[test]
    fn append_call_becomes_helper_call() {
        let mut m = module_with(vec![append_call("r", "xs", "ys")]);
        lower_list_ops(&mut m);
        let body = &m.functions[0].instructions;
        // call __dyn_list_append, xs, ys -> r : ref<any>
        let call = &body[0];
        assert_eq!(call.op, "call");
        assert_eq!(call.srcs[0], Operand::Var(APPEND_HELPER.into()));
        assert_eq!(call.srcs[1], Operand::Var("xs".into()));
        assert_eq!(call.srcs[2], Operand::Var("ys".into()));
        assert_eq!(call.dest.as_deref(), Some("r"));
        assert_eq!(call.type_hint, "ref<any>");
        assert!(body.iter().all(|i| !is_append_call(i)), "no append call_builtin survives");
    }

    #[test]
    fn append_helper_is_injected_once() {
        let mut m = module_with(vec![
            append_call("a", "l1", "r1"),
            append_call("b", "l2", "r2"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(
            m.functions.iter().filter(|f| f.name == APPEND_HELPER).count(), 1,
            "exactly one append helper",
        );
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == APPEND_HELPER).count(), 1);
    }

    #[test]
    fn append_helper_shape_rebuilds_via_cons() {
        let mut m = module_with(vec![append_call("r", "xs", "ys")]);
        lower_list_ops(&mut m);
        let helper = m.functions.iter().find(|f| f.name == APPEND_HELPER).unwrap();
        assert_eq!(helper.return_type, "ref<any>", "helper returns a lisp value");
        assert_eq!(
            helper.params,
            vec![
                ("a".to_string(), "ref<any>".to_string()),
                ("b".to_string(), "ref<any>".to_string()),
            ],
            "both params are lisp lists",
        );
        let ops: Vec<&str> = helper.instructions.iter().map(|i| i.op.as_str()).collect();
        // Terminates on null?(a); rebuilds with cons(car(a), recurse(cdr(a), b)).
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("null?".into()))));
        assert!(ops.contains(&"jmp_if_false"));
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("car".into()))));
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("cdr".into()))));
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("cons".into()))),
            "recursive arm rebuilds via cons");
        // No index arithmetic — append has no index.
        assert!(!ops.contains(&"unbox") && !ops.contains(&"cmp_eq") && !ops.contains(&"sub"),
            "append does no index arithmetic");
        assert_eq!(ops.iter().filter(|o| **o == "ret").count(), 2, "base + recursive ret");
        // The recursive call targets the helper itself.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call" && i.srcs.first() == Some(&Operand::Var(APPEND_HELPER.into()))));
    }

    #[test]
    fn all_three_ops_coexist() {
        // A module using length + list-ref + append injects all three helpers.
        let mut m = module_with(vec![
            length_call("a", "l1"),
            listref_call("b", "l2", "i2"),
            append_call("c", "l3", "l4"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == LENGTH_HELPER).count(), 1);
        assert_eq!(m.functions.iter().filter(|f| f.name == LISTREF_HELPER).count(), 1);
        assert_eq!(m.functions.iter().filter(|f| f.name == APPEND_HELPER).count(), 1);
    }

    // ── reverse ─────────────────────────────────────────────────────────────

    fn reverse_call(dest: &str, a: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin", Some(dest.into()),
            vec![Operand::Var("reverse".into()), Operand::Var(a.into())],
            "any",
        )
    }

    #[test]
    fn no_reverse_is_noop() {
        let mut m = module_with(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.len(), 1, "no helper injected when reverse is unused");
    }

    #[test]
    fn reverse_call_seeds_nil_and_calls_acc_helper() {
        let mut m = module_with(vec![reverse_call("r", "xs")]);
        lower_list_ops(&mut m);
        let body = &m.functions[0].instructions;
        // const r.rev_nil = 0 : ref<LispyPair>   (the empty accumulator seed)
        let nil = &body[0];
        assert_eq!(nil.op, "const");
        assert_eq!(nil.type_hint, REF_PAIR);
        assert_eq!(nil.srcs.first(), Some(&Operand::Int(0)));
        let nil_name = nil.dest.clone().unwrap();
        // call __dyn_list_reverse, xs, r.rev_nil -> r : ref<any>
        let call = &body[1];
        assert_eq!(call.op, "call");
        assert_eq!(call.srcs[0], Operand::Var(REVERSE_HELPER.into()));
        assert_eq!(call.srcs[1], Operand::Var("xs".into()));
        assert_eq!(call.srcs[2], Operand::Var(nil_name));
        assert_eq!(call.dest.as_deref(), Some("r"));
        assert_eq!(call.type_hint, "ref<any>");
        assert!(body.iter().all(|i| !is_reverse_call(i)), "no reverse call_builtin survives");
    }

    #[test]
    fn reverse_helper_is_injected_once() {
        let mut m = module_with(vec![reverse_call("a", "l1"), reverse_call("b", "l2")]);
        lower_list_ops(&mut m);
        assert_eq!(
            m.functions.iter().filter(|f| f.name == REVERSE_HELPER).count(), 1,
            "exactly one reverse helper",
        );
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == REVERSE_HELPER).count(), 1);
    }

    #[test]
    fn reverse_helper_shape_is_tail_recursive_accumulator() {
        let mut m = module_with(vec![reverse_call("r", "xs")]);
        lower_list_ops(&mut m);
        let helper = m.functions.iter().find(|f| f.name == REVERSE_HELPER).unwrap();
        assert_eq!(helper.return_type, "ref<any>");
        assert_eq!(
            helper.params,
            vec![
                ("a".to_string(), "ref<any>".to_string()),
                ("acc".to_string(), "ref<any>".to_string()),
            ],
            "the helper carries an accumulator param",
        );
        // Terminates on null?(a), returning the accumulator; rebuilds via cons.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("null?".into()))));
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("cons".into()))),
            "each step conses the head onto the accumulator");
        // The base case returns `acc` (not `a`); the accumulator holds the result.
        let base_ret = helper.instructions.iter().find(|i|
            i.op == "ret" && i.srcs.first() == Some(&Operand::Var("acc".into())));
        assert!(base_ret.is_some(), "base case returns the accumulator");
        // No index arithmetic.
        let ops: Vec<&str> = helper.instructions.iter().map(|i| i.op.as_str()).collect();
        assert!(!ops.contains(&"unbox") && !ops.contains(&"cmp_eq") && !ops.contains(&"sub"));
        // The recursive call targets the helper itself, in tail position.
        assert!(helper.instructions.iter().any(|i|
            i.op == "call" && i.srcs.first() == Some(&Operand::Var(REVERSE_HELPER.into()))));
    }

    #[test]
    fn all_four_ops_coexist() {
        // A module using length + list-ref + append + reverse injects all four.
        let mut m = module_with(vec![
            length_call("a", "l1"),
            listref_call("b", "l2", "i2"),
            append_call("c", "l3", "l4"),
            reverse_call("d", "l5"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == LENGTH_HELPER).count(), 1);
        assert_eq!(m.functions.iter().filter(|f| f.name == LISTREF_HELPER).count(), 1);
        assert_eq!(m.functions.iter().filter(|f| f.name == APPEND_HELPER).count(), 1);
        assert_eq!(m.functions.iter().filter(|f| f.name == REVERSE_HELPER).count(), 1);
    }

    // ── assoc ───────────────────────────────────────────────────────────────

    fn assoc_call(dest: &str, key: &str, alist: &str) -> IIRInstr {
        IIRInstr::new(
            "call_builtin", Some(dest.into()),
            vec![
                Operand::Var("assoc".into()),
                Operand::Var(key.into()),
                Operand::Var(alist.into()),
            ],
            "any",
        )
    }

    #[test]
    fn no_assoc_is_noop() {
        let mut m = module_with(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
        ]);
        lower_list_ops(&mut m);
        assert_eq!(m.functions.len(), 1, "no helper injected when assoc is unused");
    }

    #[test]
    fn assoc_call_becomes_helper_call() {
        let mut m = module_with(vec![assoc_call("r", "k", "al")]);
        lower_list_ops(&mut m);
        let body = &m.functions[0].instructions;
        let call = &body[0];
        assert_eq!(call.op, "call");
        assert_eq!(call.srcs[0], Operand::Var(ASSOC_HELPER.into()));
        assert_eq!(call.srcs[1], Operand::Var("k".into()));
        assert_eq!(call.srcs[2], Operand::Var("al".into()));
        assert_eq!(call.dest.as_deref(), Some("r"));
        assert_eq!(call.type_hint, "ref<any>");
        assert!(body.iter().all(|i| !is_assoc_call(i)), "no assoc call_builtin survives");
    }

    #[test]
    fn assoc_helper_is_injected_once() {
        let mut m = module_with(vec![assoc_call("a", "k1", "al1"), assoc_call("b", "k2", "al2")]);
        lower_list_ops(&mut m);
        assert_eq!(
            m.functions.iter().filter(|f| f.name == ASSOC_HELPER).count(), 1,
            "exactly one assoc helper",
        );
        lower_list_ops(&mut m);
        assert_eq!(m.functions.iter().filter(|f| f.name == ASSOC_HELPER).count(), 1);
    }

    #[test]
    fn assoc_helper_shape_searches_pairs_by_key() {
        let mut m = module_with(vec![assoc_call("r", "k", "al")]);
        lower_list_ops(&mut m);
        let helper = m.functions.iter().find(|f| f.name == ASSOC_HELPER).unwrap();
        assert_eq!(helper.return_type, "ref<any>");
        assert_eq!(
            helper.params,
            vec![
                ("key".to_string(), "ref<any>".to_string()),
                ("alist".to_string(), "ref<any>".to_string()),
            ],
        );
        let ops: Vec<&str> = helper.instructions.iter().map(|i| i.op.as_str()).collect();
        // Terminates on null?(alist); the key test is a TYPED cmp_eq over unboxed
        // keys (two unboxes), gating two jmp_if_false branches (nil-check + key-check).
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("null?".into()))));
        assert_eq!(ops.iter().filter(|o| **o == "unbox").count(), 2, "both keys unboxed");
        assert!(ops.contains(&"cmp_eq"), "typed key equality");
        assert_eq!(ops.iter().filter(|o| **o == "jmp_if_false").count(), 2, "nil-check + key-check");
        // Two `car`s per step: car(alist) = the pair, car(pair) = its key.
        assert_eq!(
            helper.instructions.iter().filter(|i|
                i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("car".into()))).count(),
            2, "car(alist) then car(pair)",
        );
        assert!(helper.instructions.iter().any(|i|
            i.op == "call_builtin" && i.srcs.first() == Some(&Operand::Var("cdr".into()))));
        // Three rets: not-found (nil), match (pair), recurse.
        assert_eq!(ops.iter().filter(|o| **o == "ret").count(), 3);
        assert!(helper.instructions.iter().any(|i|
            i.op == "call" && i.srcs.first() == Some(&Operand::Var(ASSOC_HELPER.into()))));
    }

    #[test]
    fn all_five_ops_coexist() {
        // length + list-ref + append + reverse + assoc → all five helpers, once each.
        let mut m = module_with(vec![
            length_call("a", "l1"),
            listref_call("b", "l2", "i2"),
            append_call("c", "l3", "l4"),
            reverse_call("d", "l5"),
            assoc_call("e", "k", "al"),
        ]);
        lower_list_ops(&mut m);
        for helper in [LENGTH_HELPER, LISTREF_HELPER, APPEND_HELPER, REVERSE_HELPER, ASSOC_HELPER] {
            assert_eq!(
                m.functions.iter().filter(|f| f.name == helper).count(), 1,
                "exactly one {helper}",
            );
        }
    }
}

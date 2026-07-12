//! # list_ops — LANG-FULL E6d-3b: list *operations* over a cons chain.
//!
//! `E6d-3a` (`heap::desugar_list_in_function`) handled the `list` *constructor*
//! as a pure desugar to `cons`. The list *operations* — `length`, `list-ref`,
//! … — instead **walk** the cons chain, so they cannot be a straight-line
//! desugar; they need a small recursive helper.
//!
//! ## The trick: one synthesized IIR helper, pure typed body
//!
//! `length` becomes a call to a synthesized recursive function
//!
//! ```text
//!   fn __dyn_list_length(lst : ref<any>) -> i64 {
//!       is_nil = null?(lst)            ;; → is_null   (lowered by heap.rs)
//!       if !is_nil goto recurse
//!       ret 0                          ;; nil ⇒ length 0
//!     recurse:
//!       rest = cdr(lst)                ;; → field_load[1] / dyn_cdr
//!       sub  = __dyn_list_length(rest) ;; recurse
//!       ret 1 + sub                    ;; TYPED i64 add — no boxing
//!   }
//! ```
//!
//! The helper's body is **pure typed `i64`** apart from `null?`/`cdr` — the two
//! dynamic ops McCarthy Lisp already runs on every code-gen backend. There is no
//! dynamic arithmetic *inside* the helper (the `+` is a machine `add` over the
//! `i64` count and the `i64` recursive result), so nothing new must lower. The
//! only dynamic-value boundary is at the **call site**: `length` must return a
//! lisp value, so we `box` the `i64` count back to a `ref<any>`. Both the box and
//! the helper's `null?`/`cdr` are handled by the passes that already run.
//!
//! ## Where it runs
//!
//! `lower_list_ops` is invoked at the **head of both** `lower_heap_builtins` and
//! `lower_heap_builtins_runtime` (like `desugar_list_in_function`), *before* the
//! per-function heap lowering — so the helper it injects has its `null?`/`cdr`
//! lowered by the same pass, on both the managed and native/runtime paths, with
//! no lang-aot pipeline change.
//!
//! Recursion depth = list length; V1 lists are short (the same shape as the
//! runtime's structural-recursion). Iterative rewriting is a later refinement.

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::IIRModule;

/// The synthesized recursive length helper's function name.
const LENGTH_HELPER: &str = "__dyn_list_length";

/// Module-level entry: rewrite every `call_builtin "length"` to a call to the
/// synthesized `__dyn_list_length` helper (+ a `box` of the i64 result), and
/// inject the helper once if any `length` call was present.
pub fn lower_list_ops(module: &mut IIRModule) {
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
}

//! # `WolframBackend` — a decorator over the shared `SymbolicBackend`.
//!
//! W-5 adds Wolfram's list/functional/numeric built-ins (`Length`, `Map`, …)
//! *without* editing `symbolic-vm`'s shared `build_handler_table` — that table
//! has 50+ downstream dependents, and these heads are a Wolfram-lane concern.
//! Instead this backend **wraps** a [`SymbolicBackend`] and overrides exactly one
//! decision: which handler answers a given head.
//!
//! ```text
//!            ┌──────────────────────────────────────────┐
//!   VM ─────▶│ WolframBackend                           │
//!            │  handler_for(name):                      │
//!            │    ├─ in W-5 builtin table? ─► use it    │  Length, First, Last,
//!            │    └─ else ─────────────────────────────┐│  Part, Append, Range,
//!            │  lookup / bind / on_unresolved /        ││  Map, Apply, N
//!            │  on_unknown_head / rules / hold_heads ──┼┼─► delegate ───────┐
//!            └─────────────────────────────────────────┘│                   ▼
//!                                                        └──────▶ SymbolicBackend
//!                                                                 (Add, Sin, If,
//!                                                                  List, Assign, …)
//! ```
//!
//! Everything except `handler_for` is a straight delegation to the inner
//! backend, so the entire W-4 engine — arithmetic, the `Plus`→`Add` bridge,
//! comparisons, logic, the held `If`, user-defined functions, `/.` — is reused
//! unchanged. `hold_heads` in particular still comes from the inner backend, so
//! `If` stays held (only its taken branch is evaluated). None of the new W-5
//! heads are held: their arguments are eagerly evaluated before the handler runs,
//! which is what `Length[Append[{1}, 2]]` relies on.

use std::collections::{HashMap, HashSet};

use symbolic_vm::backend::{Backend, Handler, Rule};
use symbolic_vm::SymbolicBackend;

use symbolic_ir::{IRApply, IRNode};

use crate::builtins::{build_wolfram_builtins, ITERATION_HEADS, SCOPING_HEADS};

/// The Wolfram evaluation backend: a [`SymbolicBackend`] plus the W-5 built-in
/// handler table.
pub struct WolframBackend {
    /// The shared symbolic engine — owns the environment and every W-4 handler.
    inner: SymbolicBackend,
    /// The W-5 list/functional/numeric handlers, consulted *before* `inner`.
    builtins: HashMap<String, Handler>,
    /// The set of heads whose args must NOT be pre-evaluated — the union of the
    /// inner backend's held set (`If`, `Assign`, `Define`, …), the W-7 iteration
    /// heads (`Table`, `Do`, `Sum`, `Product`), which must hold their body +
    /// iterator spec so the local index can be bound per step, and the W-8
    /// local-scoping heads (`With`, `Module`, `Block`), which must hold their
    /// declaration list + body so the locals can be bound into the body.
    ///
    /// `hold_heads` returns `&HashSet`, so the union must be *materialised and
    /// owned* here — we cannot synthesise it per-call. It is computed once at
    /// construction from a snapshot of the inner held set.
    held: HashSet<String>,
}

impl WolframBackend {
    /// Create a Wolfram backend over a fresh [`SymbolicBackend`].
    pub fn new() -> Self {
        let inner = SymbolicBackend::new();
        // Snapshot the inner held set and add the W-7 iteration heads. The
        // inner held set is fixed at construction (the W-4 control heads), so a
        // one-time union is correct — no inner head is added after this point.
        let mut held: HashSet<String> = inner.hold_heads().iter().cloned().collect();
        for head in ITERATION_HEADS {
            held.insert(head.to_string());
        }
        // W-8 local-scoping heads must also be held so their decl list + body
        // arrive unevaluated and the locals can be bound into the body.
        for head in SCOPING_HEADS {
            held.insert(head.to_string());
        }
        Self {
            inner,
            builtins: build_wolfram_builtins(),
            held,
        }
    }
}

impl Default for WolframBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for WolframBackend {
    fn lookup(&self, name: &str) -> Option<IRNode> {
        self.inner.lookup(name)
    }

    fn bind(&mut self, name: &str, value: IRNode) {
        self.inner.bind(name, value);
    }

    fn on_unresolved(&self, name: &str) -> IRNode {
        self.inner.on_unresolved(name)
    }

    fn on_unknown_head(&self, expr: IRApply) -> IRNode {
        self.inner.on_unknown_head(expr)
    }

    fn rules(&self) -> &[Rule] {
        self.inner.rules()
    }

    /// Consult the W-5 built-in table first; fall back to the inner backend's
    /// handler table for every other head (`Add`, `Sin`, `If`, `List`, …).
    fn handler_for(&self, head_name: &str) -> Option<&Handler> {
        self.builtins
            .get(head_name)
            .or_else(|| self.inner.handler_for(head_name))
    }

    fn hold_heads(&self) -> &HashSet<String> {
        // The W-5 heads are all non-held (eager args). W-7 adds the iteration
        // heads (`Table`, `Do`, `Sum`, `Product`) and W-8 adds the scoping heads
        // (`With`, `Module`, `Block`) to the inner backend's held set (`If`,
        // `Assign`, `Define`, …); the union was materialised in `new()`.
        &self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, int, sym, LIST};
    use symbolic_vm::VM;

    #[test]
    fn delegates_arithmetic_to_the_inner_backend() {
        // Add is not a W-5 head — it must reach the inner SymbolicBackend.
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let out = vm.eval(apply(sym("Add"), vec![int(2), int(3)]));
        assert_eq!(out, int(5));
    }

    #[test]
    fn dispatches_a_w5_builtin() {
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let out = vm.eval(apply(sym("Length"), vec![apply(sym(LIST), vec![int(1), int(2)])]));
        assert_eq!(out, int(2));
    }

    #[test]
    fn bindings_round_trip_through_the_inner_backend() {
        let mut backend = WolframBackend::new();
        backend.bind("k", int(7));
        assert_eq!(backend.lookup("k"), Some(int(7)));
    }

    #[test]
    fn if_stays_held_via_the_inner_hold_set() {
        assert!(WolframBackend::new().hold_heads().contains("If"));
    }

    #[test]
    fn iteration_heads_are_held() {
        // W-7: Table/Do/Sum/Product must be held so the body + iterator spec
        // arrive unevaluated and the local index can be bound per step. The
        // inner held set (If, …) must survive the union.
        let backend = WolframBackend::new();
        let held = backend.hold_heads();
        for head in ["Table", "Do", "Sum", "Product"] {
            assert!(held.contains(head), "{head} should be held");
        }
        assert!(held.contains("If"), "inner held set must be preserved");
    }

    #[test]
    fn scoping_heads_are_held() {
        // W-8: With/Module/Block must be held so the decl list + body arrive
        // unevaluated and the locals can be bound into the body. The inner held
        // set (If, …) and the W-7 iteration heads must survive the union.
        let backend = WolframBackend::new();
        let held = backend.hold_heads();
        for head in ["With", "Module", "Block"] {
            assert!(held.contains(head), "{head} should be held");
        }
        assert!(held.contains("If"), "inner held set must be preserved");
        assert!(held.contains("Table"), "W-7 held set must be preserved");
    }

    #[test]
    fn with_evaluates_end_to_end_through_the_vm() {
        // With[{x = 3}, x^2] → 9: the decl RHS is evaluated, x is substituted
        // into the held body, and the squaring re-eval goes through the real VM.
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let out = vm.eval(apply(
            sym("With"),
            vec![
                apply(
                    sym(LIST),
                    vec![apply(sym("Assign"), vec![sym("x"), int(3)])],
                ),
                apply(sym("Pow"), vec![sym("x"), int(2)]),
            ],
        ));
        assert_eq!(out, int(9));
    }

    #[test]
    fn table_evaluates_end_to_end_through_the_vm() {
        // A full Table[i^2, {i, 3}] over the WolframBackend: held args, per-step
        // substitution, and the squaring re-eval all go through the real VM.
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let out = vm.eval(apply(
            sym("Table"),
            vec![
                apply(sym("Pow"), vec![sym("i"), int(2)]),
                apply(sym(LIST), vec![sym("i"), int(3)]),
            ],
        ));
        assert_eq!(out, apply(sym(LIST), vec![int(1), int(4), int(9)]));
    }
}

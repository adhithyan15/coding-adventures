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
use std::sync::Arc;

use symbolic_vm::backend::{Backend, Handler, Rule};
use symbolic_vm::vm::substitute;
use symbolic_vm::SymbolicBackend;

use symbolic_ir::{apply, IRApply, IRNode, ADD, AND, MUL, OR};

use crate::builtins::{build_wolfram_builtins, CONDITIONAL_HEADS, ITERATION_HEADS, SCOPING_HEADS};
use crate::lower::{FUNCTION_HEAD, SLOT_HEAD, SLOT_SEQUENCE_HEAD};

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
    /// iterator spec so the local index can be bound per step, the W-8
    /// local-scoping heads (`With`, `Module`, `Block`), which must hold their
    /// declaration list + body so the locals can be bound into the body, and the
    /// W-14 conditional heads (`Which`, `Switch`), which must hold their
    /// condition/value pairs so only the selected branch is ever evaluated.
    ///
    /// `hold_heads` returns `&HashSet`, so the union must be *materialised and
    /// owned* here — we cannot synthesise it per-call. It is computed once at
    /// construction from a snapshot of the inner held set.
    held: HashSet<String>,
    /// The rewrite rules consulted by the VM *before* head dispatch: the inner
    /// backend's rules followed by the W-11 pure-function application rule. Like
    /// `held`, `rules()` returns a `&[Rule]`, so the combined list must be
    /// materialised and owned here.
    rules: Vec<Rule>,
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
        // W-14 conditional heads (`Which`, `Switch`) must be held so every
        // condition/value and the `expr`/forms arrive unevaluated and ONLY the
        // selected branch is evaluated — a non-taken branch (which might error or
        // have a side effect) must never run. `If` is already in the inner held
        // set, so it is not added here.
        for head in CONDITIONAL_HEADS {
            held.insert(head.to_string());
        }
        // W-11: the inner backend's rules followed by the pure-function
        // application rule. The inner rules come first so existing rewrites keep
        // their priority; the Function rule only ever fires on a shape no inner
        // rule matches (an `Apply` whose head is itself a `Function[…]`).
        let mut rules: Vec<Rule> = inner.rules().to_vec();
        rules.push(pure_function_apply_rule());
        Self {
            inner,
            builtins: build_wolfram_builtins(),
            held,
            rules,
        }
    }
}

/// The W-11 pure-function application rewrite rule.
///
/// The VM consults `Backend::rules()` in `eval_apply` *before* head dispatch, on
/// the already-arg-evaluated `IRApply`, and **re-evaluates the transform's
/// result**. This rule's predicate matches an `Apply` whose head is a
/// `Function[…]` *and* which is actually **reducible** — a well-formed function
/// record with matching arity. The transform then substitutes the args for the
/// parameters / slots and returns the body for the VM to re-evaluate.
///
/// Gating reducibility in the **predicate** (not the transform) is critical for
/// termination: an arity-mismatched / malformed `Function[…][args]` does NOT fire
/// the rule, so the VM falls through to `on_unknown_head` and leaves it
/// unevaluated — rather than the transform returning the same shape, which the
/// VM would re-evaluate, re-match, and loop on forever (a self-DoS).
///
/// Because the rule fires inside `vm.eval`, it composes for free with every
/// higher-order builtin (`Map`, `Select`, `Nest`) that already does
/// `vm.eval(build_canonical_application(f, …))`.
fn pure_function_apply_rule() -> Rule {
    let predicate = Arc::new(|expr: &IRApply| is_reducible_function_apply(expr));
    let transform = Arc::new(|expr: IRApply| apply_pure_function(expr));
    (predicate, transform)
}

/// True if `expr` is a `Function[…][args]` that the rule can actually reduce:
/// the head is a `Function[…]` apply, and either
/// - a **named** `Function[List(params…), body]` whose param count equals the
///   number of args (all params plain symbols), or
/// - a **slot-based** `Function[body]` (one body arg — slot arity is open).
///
/// Anything else (a non-`Function` head, a malformed record, an arity mismatch)
/// returns `false`, so the application is left unevaluated instead of looping.
fn is_reducible_function_apply(expr: &IRApply) -> bool {
    let IRNode::Apply(func) = &expr.head else {
        return false;
    };
    let IRNode::Symbol(h) = &func.head else {
        return false;
    };
    if h != FUNCTION_HEAD {
        return false;
    }
    match func.args.as_slice() {
        // Named form: param count must match the call's arg count.
        [params_ir, _body] => {
            list_symbol_names(params_ir).is_some_and(|names| names.len() == expr.args.len())
        }
        // Slot-based form: always reducible (slots index into the args).
        [_body] => true,
        _ => false,
    }
}

/// Apply `Function[…][args]` by substitution, returning the body to re-evaluate.
///
/// Two function shapes are handled:
/// - **Named** `Function[List(p1, …, pn), body]`: bind `p1 → arg1, …` by plain
///   symbol name and `substitute` into the body. Arity must match; a mismatch
///   leaves the whole application unevaluated (returned unchanged).
/// - **Slot-based** `Function[body]`: substitute `Slot[k] → argk` and splice
///   `SlotSequence[k] → argk, argk+1, …` into the body, then return it.
///
/// A malformed `Function` record (wrong arity, non-symbol params) leaves the
/// application unevaluated rather than panicking — the same fail-soft convention
/// every W-5/W-7 builtin follows.
fn apply_pure_function(expr: IRApply) -> IRNode {
    let unevaluated = |expr: IRApply| IRNode::Apply(Box::new(expr));

    // The head is `Function[…]` (guaranteed by the predicate).
    let IRNode::Apply(func) = &expr.head else {
        return unevaluated(expr);
    };
    let args = &expr.args;

    match func.args.as_slice() {
        // Named form: Function[List(params…), body].
        [params_ir, body] => {
            let Some(param_names) = list_symbol_names(params_ir) else {
                return unevaluated(expr);
            };
            if param_names.len() != args.len() {
                return unevaluated(expr); // arity mismatch
            }
            let mapping: HashMap<String, IRNode> =
                param_names.into_iter().zip(args.iter().cloned()).collect();
            substitute(body.clone(), &mapping)
        }
        // Slot-based form: Function[body]. Substitute Slot[k]/SlotSequence[k].
        [body] => substitute_slots(body.clone(), args),
        // Any other arity is not a function we can apply.
        _ => unevaluated(expr),
    }
}

/// Extract the plain parameter symbol names from a `List(p1, …, pn)` node, or
/// `None` if it is not a `List` of bare symbols.
fn list_symbol_names(node: &IRNode) -> Option<Vec<String>> {
    let IRNode::Apply(app) = node else {
        return None;
    };
    let IRNode::Symbol(head) = &app.head else {
        return None;
    };
    if head != symbolic_ir::LIST {
        return None;
    }
    app.args
        .iter()
        .map(|p| match p {
            IRNode::Symbol(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Substitute slots in a slot-based pure-function body against `args`.
///
/// `Slot[k]` → `args[k-1]` (1-indexed; out-of-range slots are left as-is so the
/// form fails soft rather than panicking). `SlotSequence[k]` is **spliced**: when
/// it appears as an *argument* inside an enclosing application, it expands to
/// `args[k-1], args[k], …`, growing that application's argument list. A
/// `SlotSequence` in any other position (e.g. a bare body) substitutes to its
/// first element, the closest single-value reading.
fn substitute_slots(node: IRNode, args: &[IRNode]) -> IRNode {
    // A `Slot[k]` / `SlotSequence[k]` is itself an `Apply`, so check for it
    // BEFORE the generic Apply walk — otherwise we would descend into its
    // integer index and never substitute the slot. (A bare `SlotSequence` not in
    // argument position degrades to its first element, the closest single value;
    // the splice case is handled in the argument loop below.)
    if let Some(k) = slot_index(&node) {
        return args.get(k - 1).cloned().unwrap_or(node);
    }
    if let Some(k) = slot_sequence_index(&node) {
        return args.get(k - 1).cloned().unwrap_or(node);
    }
    match node {
        IRNode::Apply(app) => {
            let head = substitute_slots(app.head, args);
            // Splice SlotSequence args; substitute everything else element-wise.
            let mut new_args: Vec<IRNode> = Vec::with_capacity(app.args.len());
            for a in app.args {
                if let Some(k) = slot_sequence_index(&a) {
                    // ## (k) → args[k-1..] spliced into this argument list.
                    if k >= 1 {
                        new_args.extend(args.iter().skip(k - 1).cloned());
                    }
                } else {
                    new_args.push(substitute_slots(a, args));
                }
            }
            // If a `##` splice grew an *associative* IR head (`Add`/`Mul`/`And`/
            // `Or`) past two operands, left-fold it into a binary chain — the
            // SAME normalisation lowering's `build_application` does for an
            // explicit `Plus[1, 2, 3]`. Without this, `Plus[##]&[1, 2, 3]` would
            // splice to a raw n-ary `Add[1, 2, 3]` that the VM's binary `Add`
            // handler leaves as `1 + 2 + 3` instead of folding to `6`.
            fold_associative(head, new_args)
        }
        // Literals and bare symbols pass through (slots were already handled at
        // the top of this function, before the Apply walk).
        other => other,
    }
}

/// Rebuild `head[args]`, left-folding an associative IR head (`Add`/`Mul`/`And`/
/// `Or`) of 3+ operands into a binary chain so the VM's binary handlers fold it
/// (`Add[1, 2, 3]` → `Add[Add[1, 2], 3]`). Mirrors lowering's `build_application`
/// fold; used after a `##` splice may have grown an associative head's arg list.
fn fold_associative(head: IRNode, args: Vec<IRNode>) -> IRNode {
    if let IRNode::Symbol(name) = &head {
        if matches!(name.as_str(), ADD | MUL | AND | OR) && args.len() > 2 {
            let mut it = args.into_iter();
            let mut acc = it.next().unwrap();
            for next in it {
                acc = apply(head.clone(), vec![acc, next]);
            }
            return acc;
        }
    }
    IRNode::Apply(Box::new(IRApply { head, args }))
}

/// If `node` is `Slot[k]` with a positive integer `k`, return `k`.
fn slot_index(node: &IRNode) -> Option<usize> {
    single_int_apply(node, SLOT_HEAD)
}

/// If `node` is `SlotSequence[k]` with a positive integer `k`, return `k`.
fn slot_sequence_index(node: &IRNode) -> Option<usize> {
    single_int_apply(node, SLOT_SEQUENCE_HEAD)
}

/// Shared shape check: `head[k]` where `k` is a positive `Integer`.
fn single_int_apply(node: &IRNode, head_name: &str) -> Option<usize> {
    let IRNode::Apply(app) = node else {
        return None;
    };
    let IRNode::Symbol(h) = &app.head else {
        return None;
    };
    if h != head_name || app.args.len() != 1 {
        return None;
    }
    match &app.args[0] {
        IRNode::Integer(k) if *k >= 1 => Some(*k as usize),
        _ => None,
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

    /// The inner backend's rewrite rules plus the W-11 pure-function application
    /// rule (materialised once in `new()`).
    fn rules(&self) -> &[Rule] {
        &self.rules
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
    fn pure_function_apply_rule_is_registered() {
        // The W-11 rule must be appended after the inner backend's rules.
        let backend = WolframBackend::new();
        let inner_count = SymbolicBackend::new().rules().len();
        assert_eq!(
            backend.rules().len(),
            inner_count + 1,
            "the pure-function rule must be appended once"
        );
    }

    #[test]
    fn named_pure_function_applies_via_the_vm() {
        // Function[List[x], x^2] applied to 5 → 25, through the real VM (the rule
        // substitutes x → 5 and the squaring re-eval folds 5^2 = 25).
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let func = apply(
            sym(FUNCTION_HEAD),
            vec![
                apply(sym(LIST), vec![sym("x")]),
                apply(sym("Pow"), vec![sym("x"), int(2)]),
            ],
        );
        let out = vm.eval(IRNode::Apply(Box::new(IRApply {
            head: func,
            args: vec![int(5)],
        })));
        assert_eq!(out, int(25));
    }

    #[test]
    fn slot_pure_function_applies_via_the_vm() {
        // Function[Pow[Slot[1], 2]] applied to 6 → 36.
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let func = apply(
            sym(FUNCTION_HEAD),
            vec![apply(
                sym("Pow"),
                vec![apply(sym(SLOT_HEAD), vec![int(1)]), int(2)],
            )],
        );
        let out = vm.eval(IRNode::Apply(Box::new(IRApply {
            head: func,
            args: vec![int(6)],
        })));
        assert_eq!(out, int(36));
    }

    #[test]
    fn slot_sequence_splices_into_an_application() {
        // Function[Add[##]] applied to (1, 2, 3) → Add[1, 2, 3], left-folded and
        // summed to 6: the SlotSequence[1] splices all three args into the Add
        // arg list (the body is already lowered to the IR head `Add`, which is
        // what the rule sees at eval time).
        use symbolic_ir::ADD;
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let func = apply(
            sym(FUNCTION_HEAD),
            vec![apply(
                sym(ADD),
                vec![apply(sym(SLOT_SEQUENCE_HEAD), vec![int(1)])],
            )],
        );
        let out = vm.eval(IRNode::Apply(Box::new(IRApply {
            head: func,
            args: vec![int(1), int(2), int(3)],
        })));
        assert_eq!(out, int(6));
    }

    #[test]
    fn arity_mismatch_leaves_the_application_unevaluated() {
        // A two-param named function applied to one arg cannot reduce; the rule
        // returns the application unchanged (fail-soft, no panic).
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let func = apply(
            sym(FUNCTION_HEAD),
            vec![
                apply(sym(LIST), vec![sym("x"), sym("y")]),
                apply(sym("Pow"), vec![sym("x"), int(2)]),
            ],
        );
        let app = IRNode::Apply(Box::new(IRApply {
            head: func.clone(),
            args: vec![int(5)],
        }));
        // Unevaluated: the head stays a Function node and the arg list is [5].
        let out = vm.eval(app);
        let IRNode::Apply(outer) = &out else {
            panic!("expected the application to be returned unchanged");
        };
        assert_eq!(outer.args, vec![int(5)]);
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

    #[test]
    fn conditional_heads_are_held() {
        // W-14: Which/Switch must be held so their condition/value pairs arrive
        // unevaluated and only the selected branch is evaluated. The inner held
        // set (If) and the W-7/W-8 held sets must survive the union.
        let backend = WolframBackend::new();
        let held = backend.hold_heads();
        for head in ["Which", "Switch"] {
            assert!(held.contains(head), "{head} should be held");
        }
        assert!(held.contains("If"), "inner held set must be preserved");
        assert!(held.contains("Table"), "W-7 held set must be preserved");
        assert!(held.contains("With"), "W-8 held set must be preserved");
    }

    #[test]
    fn which_evaluates_end_to_end_through_the_vm() {
        // Which[2 > 1, "a"] → "a": held args, the comparison condition is
        // evaluated to True by the handler, and only the selected value returns.
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let out = vm.eval(apply(
            sym("Which"),
            vec![
                apply(sym(symbolic_ir::GREATER), vec![int(2), int(1)]),
                symbolic_ir::str_node("a"),
            ],
        ));
        assert_eq!(out, symbolic_ir::str_node("a"));
    }

    #[test]
    fn switch_does_not_evaluate_a_non_selected_branch() {
        // Switch[1, 1, 2, _, Pow[1,0,0]] → 2. If the held machinery were broken and
        // the default value were eagerly evaluated, the malformed Pow would surface;
        // instead the first form (1) matches and only its value (2) is evaluated.
        let mut vm = VM::new(Box::new(WolframBackend::new()));
        let out = vm.eval(apply(
            sym("Switch"),
            vec![
                int(1),
                int(1),
                int(2),
                apply(sym("Blank"), vec![]),
                apply(sym("Pow"), vec![int(1), int(0), int(0)]),
            ],
        ));
        assert_eq!(out, int(2));
    }
}

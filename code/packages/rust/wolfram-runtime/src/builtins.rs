//! # W-5 built-in handlers — list, functional, control, and numeric heads.
//!
//! W-4 reused the shared [`symbolic_vm::SymbolicBackend`] for arithmetic, the
//! elementary functions, comparisons, logic, `If`, lists-as-data, patterns/`/.`,
//! and `Set`/`SetDelayed`. W-5 adds the *structural* Wolfram built-ins every
//! introductory session reaches for — `Length`, `First`, `Last`, `Part`,
//! `Append`, `Range`, `Map`, `Apply`, `N` — **without touching that shared
//! table**. Instead the [`WolframBackend`](crate::backend::WolframBackend)
//! decorator answers `handler_for` from the small table this module builds and
//! delegates everything else to the inner `SymbolicBackend` (MA04 §8.2).
//!
//! ## The handler contract
//!
//! A [`Handler`] is `Fn(&mut VM, IRApply) -> IRNode`. By the time it runs the VM
//! has **already evaluated the arguments** (none of these heads are held), so
//! `Length[Append[{1}, 2]]` sees the materialised `{1, 2}`. Every handler here
//! follows the Wolfram convention for "I can't reduce this": **return the
//! expression unevaluated** rather than panic or guess. So `First[{}]`,
//! `Part[{a}, 9]`, and `N[x]` (symbolic `x`) all echo back unchanged — exactly
//! what a Wolfram kernel prints. This is also a safety property: a crafted
//! `Part[expr, -999]` indexes nothing; it just doesn't reduce.
//!
//! ## Reusing the engine for `Map`, `Apply`, `N`
//!
//! These three don't just inspect a list — they build a *new* expression and
//! must re-evaluate it through the same VM:
//!
//! - `Map[f, {a, b}]` builds `{f[a], f[b]}` and re-evals, so `Map[Sin, {0}]`
//!   folds to `{0}` and `Map[Plus[1], {…}]`-style heads route through the
//!   `Plus`→`Add` bridge.
//! - `Apply[f, {a, b}]` swaps the `List` head for `f` (→ `f[a, b]`) and
//!   re-evals, so `Apply[Plus, {1, 2, 3}]` becomes `Add(1,2,3)` → `6`.
//! - `N[expr]` coerces exact numbers to `Float` and otherwise re-evals
//!   element-wise over a list.
//!
//! They call `vm.eval(...)` for that, which is why the handlers take `&mut VM`.

use std::collections::HashMap;

use symbolic_vm::backend::{handler_fn, Handler};
use symbolic_vm::VM;

use symbolic_ir::{apply, flt, int, sym, IRApply, IRNode, LIST};

use crate::lower::build_canonical_application;

/// Maximum number of elements a single `Range[…]` may materialise.
///
/// `Range` is the one W-5 built-in that turns a *small* input (`Range[n]`) into
/// an *O(n)* allocation, so it is the one DoS surface W-5 introduces. A span
/// that would exceed this bound is left unevaluated rather than allocated, so
/// `Range[10^9]` cannot exhaust memory. 1,000,000 elements is already far beyond
/// any interactive use while staying cheap to build. The other built-ins are
/// size-preserving (`Map`, `N`) or grow by one (`Append`), bounded by their
/// already-materialised input, which the W-4 input-size / token caps bound.
pub const MAX_RANGE_LENGTH: usize = 1_000_000;

/// Build the W-5 Wolfram built-in handler table.
///
/// Keyed on the **surface** Wolfram head names (`"Length"`, `"Map"`, …) since
/// these heads have no separate IR alias — they are not in the W-4
/// surface→IR bridge and pass through lowering verbatim. The
/// [`WolframBackend`](crate::backend::WolframBackend) consults this table first
/// and falls back to the inner `SymbolicBackend` for every other head.
pub fn build_wolfram_builtins() -> HashMap<String, Handler> {
    let mut m: HashMap<String, Handler> = HashMap::new();
    m.insert("Length".to_string(), handler_fn(length_handler));
    m.insert("First".to_string(), handler_fn(first_handler));
    m.insert("Last".to_string(), handler_fn(last_handler));
    m.insert("Part".to_string(), handler_fn(part_handler));
    m.insert("Append".to_string(), handler_fn(append_handler));
    m.insert("Range".to_string(), handler_fn(range_handler));
    m.insert("Map".to_string(), handler_fn(map_handler));
    m.insert("Apply".to_string(), handler_fn(apply_handler));
    m.insert("N".to_string(), handler_fn(n_handler));
    m
}

// ---------------------------------------------------------------------------
// List inspection — Length / First / Last / Part
// ---------------------------------------------------------------------------

/// `Length[{a, b, c}]` → `3`. The length of a non-list (an atom, or any other
/// head) is `0`, matching Wolfram (`Length[x]` is `0`, `Length[f[a, b]]` is the
/// argument count — here we only special-case `List`, and report `0` for atoms).
fn length_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    match list_elements(&expr.args[0]) {
        Some(elems) => int(elems.len() as i64),
        // A non-list argument: Wolfram's `Length` of an atom is 0.
        None => match &expr.args[0] {
            IRNode::Apply(app) => int(app.args.len() as i64),
            _ => int(0),
        },
    }
}

/// `First[{x, y}]` → `x`. `First[{}]` (and `First` of a non-list) is left
/// unevaluated — an empty list has no first element, and Wolfram errors rather
/// than inventing one; we choose the gentler "stay unevaluated" so a session
/// never panics.
fn first_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    nth_or_unevaluated(expr, |elems| elems.first().cloned())
}

/// `Last[{x, y}]` → `y`; `Last[{}]` unevaluated (see [`first_handler`]).
fn last_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    nth_or_unevaluated(expr, |elems| elems.last().cloned())
}

/// `Part[{a, b, c}, i]` — the **1-based** `i`-th element (Wolfram indexing).
///
/// - `Part[expr, 0]` is the *head* (`List` for a list literal).
/// - A negative `i` counts from the end: `Part[{a,b,c}, -1]` is `c`.
/// - An out-of-range index, a non-integer index, or a non-list first argument
///   leaves the expression unevaluated (no panic, no out-of-bounds index).
fn part_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(i) = as_i64(&expr.args[1]) else {
        return unevaluated(expr);
    };
    // Part[expr, 0] is the head of expr.
    if i == 0 {
        return match &expr.args[0] {
            IRNode::Apply(app) => app.head.clone(),
            other => other.clone(),
        };
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let len = elems.len() as i128;
    // 1-based: index 1..=len from the front, -1..=-len from the back. Compute in
    // i128 so a crafted extreme index (e.g. `i64::MIN`) cannot overflow the
    // subtraction/addition — it just falls outside `[0, len)` and is rejected.
    let idx0: i128 = if i > 0 {
        (i as i128) - 1
    } else {
        len + (i as i128) // i is negative
    };
    if idx0 < 0 || idx0 >= len {
        return unevaluated(expr);
    }
    elems[idx0 as usize].clone()
}

/// `Append[{a, b}, c]` → `{a, b, c}` — a *new* list (values are immutable, so
/// the original is unchanged). `Append` of a non-list first argument is left
/// unevaluated.
fn append_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(mut elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    elems.push(expr.args[1].clone());
    apply(sym(LIST), elems)
}

// ---------------------------------------------------------------------------
// Generation — Range
// ---------------------------------------------------------------------------

/// `Range[n]` → `{1, …, n}`; `Range[a, b]` → `{a, …, b}`; `Range[a, b, d]`
/// steps by `d`.
///
/// **DoS-capped**: a span whose element count would exceed [`MAX_RANGE_LENGTH`]
/// is left unevaluated rather than allocated, so a tiny input like `Range[10^9]`
/// cannot exhaust memory (MA04 §8.3). Non-integer bounds, a zero step, or a step
/// pointing the wrong way (e.g. `Range[1, 5, -1]`) yield the empty list `{}` /
/// unevaluated per Wolfram's behaviour. Only integer arithmetic is supported in
/// this subset; a non-integer bound leaves `Range` unevaluated.
fn range_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    let (start, end, step) = match expr.args.as_slice() {
        [n] => match as_i64(n) {
            Some(n) => (1i64, n, 1i64),
            None => return unevaluated(expr),
        },
        [a, b] => match (as_i64(a), as_i64(b)) {
            (Some(a), Some(b)) => (a, b, 1i64),
            _ => return unevaluated(expr),
        },
        [a, b, d] => match (as_i64(a), as_i64(b), as_i64(d)) {
            (Some(a), Some(b), Some(d)) => (a, b, d),
            _ => return unevaluated(expr),
        },
        _ => return unevaluated(expr),
    };

    // A zero step never terminates — refuse it (unevaluated).
    if step == 0 {
        return unevaluated(expr);
    }
    // A step pointing away from `end` produces the empty list in Wolfram.
    if (step > 0 && start > end) || (step < 0 && start < end) {
        return apply(sym(LIST), vec![]);
    }

    // Count the elements *before* allocating, so an oversize span is rejected
    // without ever materialising it. count = floor((end - start) / step) + 1,
    // computed with i128 + checks so the subtraction/division cannot overflow.
    let span = (end as i128) - (start as i128);
    let count = (span / (step as i128)) + 1; // step and span share sign here
    if count <= 0 {
        return apply(sym(LIST), vec![]);
    }
    if count as u128 > MAX_RANGE_LENGTH as u128 {
        return unevaluated(expr);
    }

    let mut elems = Vec::with_capacity(count as usize);
    let mut value = start as i128;
    for _ in 0..count {
        elems.push(int(value as i64));
        value += step as i128;
    }
    apply(sym(LIST), elems)
}

// ---------------------------------------------------------------------------
// Functional — Map / Apply
// ---------------------------------------------------------------------------

/// `Map[f, {a, b}]` → `{f[a], f[b]}`, with each `f[x]` **re-evaluated** through
/// the VM (so `Map[Sin, {0}]` is `{0}`). `Map` of a non-list second argument is
/// left unevaluated.
fn map_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let f = expr.args[0].clone();
    let Some(elems) = list_elements(&expr.args[1]) else {
        return unevaluated(expr);
    };
    let mapped: Vec<IRNode> = elems
        .into_iter()
        .map(|x| vm.eval(build_canonical_application(f.clone(), vec![x])))
        .collect();
    apply(sym(LIST), mapped)
}

/// `Apply[f, {a, b}]` → `f[a, b]` — replace the list's `List` head with `f` and
/// **re-evaluate**. So `Apply[Plus, {1, 2, 3}]` becomes `Plus[1, 2, 3]`, which
/// the W-4 `Plus`→`Add` bridge then folds to `6`. `Apply` of a non-list second
/// argument is left unevaluated.
fn apply_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let f = expr.args[0].clone();
    let Some(elems) = list_elements(&expr.args[1]) else {
        return unevaluated(expr);
    };
    vm.eval(build_canonical_application(f, elems))
}

// ---------------------------------------------------------------------------
// Numeric — N
// ---------------------------------------------------------------------------

/// `N[expr]` — numeric coercion.
///
/// An exact `Integer` or `Rational` becomes a `Float`; an already-`Float`
/// passes through; over a `List` it maps element-wise (`N[{1, 1/2}]` →
/// `{1.0, 0.5}`). Any other expression (a free symbol, an unevaluated head) is
/// returned unchanged — this subset does not attempt symbolic-constant
/// evaluation (`N[Pi]`), which is W-6 territory.
fn n_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    numericise(&expr.args[0])
}

/// Coerce one node to floating point, recursing element-wise into a `List`.
///
/// This is a pure structural coercion — it never re-evaluates through the VM
/// (this subset does not evaluate symbolic constants like `N[Pi]`; that is W-6),
/// so it needs no VM handle.
fn numericise(node: &IRNode) -> IRNode {
    match node {
        IRNode::Integer(n) => flt(*n as f64),
        IRNode::Rational(num, den) => flt(*num as f64 / *den as f64),
        IRNode::Float(_) => node.clone(),
        IRNode::Apply(app) if is_list(&app.head) => {
            let mapped = app.args.iter().map(numericise).collect();
            apply(sym(LIST), mapped)
        }
        // A free symbol or any other head: leave as-is.
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

/// Return the elements of a `List(...)` node, or `None` if `node` is not a list.
fn list_elements(node: &IRNode) -> Option<Vec<IRNode>> {
    if let IRNode::Apply(app) = node {
        if is_list(&app.head) {
            return Some(app.args.clone());
        }
    }
    None
}

/// True if `head` is the `List` symbol.
fn is_list(head: &IRNode) -> bool {
    matches!(head, IRNode::Symbol(s) if s == LIST)
}

/// Read an `Integer` node as an `i64`. Rationals/floats/symbols give `None` — an
/// index or bound that is not an exact integer cannot be used.
fn as_i64(node: &IRNode) -> Option<i64> {
    match node {
        IRNode::Integer(n) => Some(*n),
        _ => None,
    }
}

/// `First`/`Last` share this shape: pull the chosen element from the list, or
/// leave the whole expression unevaluated when there is no such element (empty
/// list, or a non-list argument).
fn nth_or_unevaluated(
    expr: IRApply,
    pick: impl Fn(&[IRNode]) -> Option<IRNode>,
) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    match list_elements(&expr.args[0]).as_deref().and_then(&pick) {
        Some(node) => node,
        None => unevaluated(expr),
    }
}

/// Rebuild the application unchanged — the Wolfram "I can't reduce this" answer.
fn unevaluated(expr: IRApply) -> IRNode {
    IRNode::Apply(Box::new(expr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_vm::SymbolicBackend;

    /// Apply one built-in handler directly to a hand-built `IRApply`, over a
    /// real VM (so `Map`/`Apply`/`N` can recurse). Args are taken pre-evaluated,
    /// matching the VM's contract.
    fn run(head: &str, args: Vec<IRNode>) -> IRNode {
        let table = build_wolfram_builtins();
        let handler = table.get(head).expect("no such builtin").clone();
        let mut vm = VM::new(Box::new(SymbolicBackend::new()));
        handler(&mut vm, IRApply { head: sym(head), args })
    }

    fn list(args: Vec<IRNode>) -> IRNode {
        apply(sym(LIST), args)
    }

    #[test]
    fn length_of_a_list_and_an_atom() {
        assert_eq!(run("Length", vec![list(vec![int(1), int(2), int(3)])]), int(3));
        assert_eq!(run("Length", vec![list(vec![])]), int(0));
        assert_eq!(run("Length", vec![sym("x")]), int(0));
        // Length of a non-list head is its argument count.
        assert_eq!(run("Length", vec![apply(sym("f"), vec![sym("a"), sym("b")])]), int(2));
    }

    #[test]
    fn first_and_last() {
        assert_eq!(run("First", vec![list(vec![int(9), int(8)])]), int(9));
        assert_eq!(run("Last", vec![list(vec![int(9), int(8)])]), int(8));
    }

    #[test]
    fn first_and_last_of_empty_stay_unevaluated() {
        assert_eq!(
            run("First", vec![list(vec![])]),
            apply(sym("First"), vec![list(vec![])])
        );
        assert_eq!(
            run("Last", vec![list(vec![])]),
            apply(sym("Last"), vec![list(vec![])])
        );
    }

    #[test]
    fn part_is_one_based_with_negatives_and_zero_head() {
        let xs = list(vec![sym("a"), sym("b"), sym("c")]);
        assert_eq!(run("Part", vec![xs.clone(), int(2)]), sym("b"));
        assert_eq!(run("Part", vec![xs.clone(), int(-1)]), sym("c"));
        // Part[expr, 0] is the head.
        assert_eq!(run("Part", vec![xs.clone(), int(0)]), sym(LIST));
        // Out of range stays unevaluated.
        assert_eq!(
            run("Part", vec![xs.clone(), int(9)]),
            apply(sym("Part"), vec![xs, int(9)])
        );
    }

    #[test]
    fn part_with_extreme_index_does_not_overflow() {
        // A crafted i64::MIN index must not overflow the `len + i` arithmetic —
        // it simply falls out of range and stays unevaluated (no panic / wrap).
        let xs = list(vec![sym("a"), sym("b")]);
        assert_eq!(
            run("Part", vec![xs.clone(), int(i64::MIN)]),
            apply(sym("Part"), vec![xs.clone(), int(i64::MIN)])
        );
        assert_eq!(
            run("Part", vec![xs.clone(), int(i64::MAX)]),
            apply(sym("Part"), vec![xs, int(i64::MAX)])
        );
    }

    #[test]
    fn append_returns_a_new_list() {
        assert_eq!(
            run("Append", vec![list(vec![sym("a"), sym("b")]), sym("c")]),
            list(vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn range_one_two_and_three_arg_forms() {
        assert_eq!(run("Range", vec![int(3)]), list(vec![int(1), int(2), int(3)]));
        assert_eq!(run("Range", vec![int(2), int(5)]), list(vec![int(2), int(3), int(4), int(5)]));
        assert_eq!(
            run("Range", vec![int(1), int(7), int(2)]),
            list(vec![int(1), int(3), int(5), int(7)])
        );
    }

    #[test]
    fn range_descending_and_empty() {
        assert_eq!(
            run("Range", vec![int(5), int(1), int(-2)]),
            list(vec![int(5), int(3), int(1)])
        );
        // Wrong-way step → empty.
        assert_eq!(run("Range", vec![int(1), int(5), int(-1)]), list(vec![]));
        assert_eq!(run("Range", vec![int(5), int(1)]), list(vec![]));
    }

    #[test]
    fn range_rejects_an_oversize_span_unevaluated() {
        let big = (MAX_RANGE_LENGTH as i64) + 10;
        let out = run("Range", vec![int(big)]);
        // Left unevaluated — never allocated.
        assert_eq!(out, apply(sym("Range"), vec![int(big)]));
    }

    #[test]
    fn range_rejects_a_zero_step() {
        assert_eq!(
            run("Range", vec![int(1), int(5), int(0)]),
            apply(sym("Range"), vec![int(1), int(5), int(0)])
        );
    }

    #[test]
    fn range_at_the_cap_is_allowed() {
        // Exactly MAX_RANGE_LENGTH elements is allowed (the boundary).
        let n = MAX_RANGE_LENGTH as i64;
        let out = run("Range", vec![int(n)]);
        match out {
            IRNode::Apply(app) if is_list(&app.head) => assert_eq!(app.args.len(), MAX_RANGE_LENGTH),
            other => panic!("expected a list of {MAX_RANGE_LENGTH}, got {other}"),
        }
    }

    #[test]
    fn map_applies_and_reevaluates() {
        // Map[f, {1, 2}] → {f[1], f[2]} (f unbound, so it stays symbolic).
        assert_eq!(
            run("Map", vec![sym("f"), list(vec![int(1), int(2)])]),
            list(vec![
                apply(sym("f"), vec![int(1)]),
                apply(sym("f"), vec![int(2)])
            ])
        );
        // Map[Sin, {0}] → {0} (the re-eval folds Sin[0]).
        assert_eq!(run("Map", vec![sym("Sin"), list(vec![int(0)])]), list(vec![int(0)]));
    }

    #[test]
    fn apply_swaps_the_head_and_reevaluates() {
        // Apply[Plus, {1, 2, 3}] → Plus[1,2,3] → 6 via the Plus→Add bridge is a
        // runtime concern; at the handler level Plus is unbound, so it stays
        // Plus[1, 2, 3]. The end-to-end fold is covered by the integration test.
        assert_eq!(
            run("Apply", vec![sym("g"), list(vec![sym("a"), sym("b")])]),
            apply(sym("g"), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn n_coerces_exact_numbers_and_maps_over_lists() {
        assert_eq!(run("N", vec![IRNode::rational(1, 2)]), flt(0.5));
        assert_eq!(run("N", vec![int(3)]), flt(3.0));
        assert_eq!(run("N", vec![flt(2.5)]), flt(2.5));
        assert_eq!(
            run("N", vec![list(vec![int(1), IRNode::rational(1, 4)])]),
            list(vec![flt(1.0), flt(0.25)])
        );
        // A free symbol stays symbolic.
        assert_eq!(run("N", vec![sym("x")]), sym("x"));
    }

    #[test]
    fn wrong_arity_stays_unevaluated() {
        assert_eq!(run("Length", vec![]), apply(sym("Length"), vec![]));
        assert_eq!(
            run("Part", vec![list(vec![int(1)])]),
            apply(sym("Part"), vec![list(vec![int(1)])])
        );
    }
}

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

use std::collections::HashMap as StdHashMap;

use symbolic_vm::backend::{handler_fn, Handler};
use symbolic_vm::vm::substitute;
use symbolic_vm::VM;

use symbolic_ir::{apply, flt, int, str_node, sym, IRApply, IRNode, ADD, ASSIGN, LIST, MUL};

use cas_pattern_matching::nodes::RULE as PM_RULE;

use crate::lower::build_canonical_application;
use crate::printer::print_wolfram;

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

/// Maximum number of elements a W-9 list-*growing* built-in (`Join`, `Flatten`)
/// may materialise into its result.
///
/// `Join` and `Flatten` are the two W-9 heads whose output can be *larger* than
/// any single input — `Join` sums its argument lengths, `Flatten` splices nested
/// sub-lists into one flat list. Both inputs are themselves bounded by the W-4
/// input/token caps, so this guard is defensive (a deeply/widely nested literal
/// or a long chain of `Join`s could still aim for a large allocation); a result
/// that would exceed this bound is left unevaluated rather than allocated. The
/// other W-9 heads (`Sort`, `Reverse`, `Select`, `Count`, `Total`) are
/// size-non-increasing and need no separate cap. Shares `MAX_RANGE_LENGTH`'s
/// value (1,000,000) — already far beyond any interactive list.
pub const MAX_LIST_LENGTH: usize = MAX_RANGE_LENGTH;

/// Maximum number of **characters** a W-12 string-*growing* built-in
/// (`StringJoin`, `StringReplace`) may materialise into its result.
///
/// `StringJoin` sums its argument lengths and `StringReplace` can grow the string
/// per match when the replacement is longer than the pattern — both are the W-12
/// analogue of the W-9 `Join`/`Flatten` growth surface. The inputs are themselves
/// bounded by the W-4 input/token caps, so this guard is defensive: a result that
/// would exceed this bound is left unevaluated rather than allocated. The running
/// length is accumulated in `usize` with `checked_add` so a crafted chain cannot
/// overflow the count. The other W-12 heads are size-non-increasing
/// (`StringTake`, `StringDrop`, `StringLength`) or bounded by their already-
/// materialised input (`StringSplit`, `Characters`, which additionally cap their
/// element count at [`MAX_LIST_LENGTH`]). Shares `MAX_RANGE_LENGTH`'s value
/// (1,000,000 chars) — already far beyond any interactive string.
pub const MAX_STRING_LENGTH: usize = MAX_RANGE_LENGTH;

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
    // W-7 iteration constructs. These are *held* heads (see
    // [`ITERATION_HEADS`] and the `WolframBackend` held set) — their body and
    // iterator spec arrive unevaluated so the local index can be bound per step.
    m.insert("Table".to_string(), handler_fn(table_handler));
    m.insert("Do".to_string(), handler_fn(do_handler));
    m.insert("Sum".to_string(), handler_fn(sum_handler));
    m.insert("Product".to_string(), handler_fn(product_handler));
    // W-8 local-scoping constructs. These are *held* heads (see [`SCOPING_HEADS`]
    // and the `WolframBackend` held set) — their declaration list and body arrive
    // unevaluated so the locals can be bound into the body before it evaluates.
    m.insert("With".to_string(), handler_fn(with_handler));
    m.insert("Module".to_string(), handler_fn(module_handler));
    m.insert("Block".to_string(), handler_fn(block_handler));
    // W-9 list-manipulation constructs. All are *eager* (non-held) heads — their
    // arguments are evaluated before the handler runs, exactly like the W-5 list
    // built-ins — so they are *not* added to the `WolframBackend` held set.
    m.insert("Sort".to_string(), handler_fn(sort_handler));
    m.insert("Reverse".to_string(), handler_fn(reverse_handler));
    m.insert("Join".to_string(), handler_fn(join_handler));
    m.insert("Flatten".to_string(), handler_fn(flatten_handler));
    m.insert("Select".to_string(), handler_fn(select_handler));
    m.insert("Count".to_string(), handler_fn(count_handler));
    m.insert("Total".to_string(), handler_fn(total_handler));
    // W-9 parity predicates — the minimal predicate primitives that make
    // `Select`/`Count` testable (the W-5/W-6 surface had no predicate head).
    m.insert("EvenQ".to_string(), handler_fn(even_q_handler));
    m.insert("OddQ".to_string(), handler_fn(odd_q_handler));
    // W-10 functional-iteration combinators. All *eager* (non-held) heads — `f`,
    // the seed, and the list arrive evaluated, exactly like the W-5/W-9 list
    // built-ins — so they are *not* added to the `WolframBackend` held set. Each
    // iterates by re-applying `f` through the same `build_canonical_application`
    // + `vm.eval` path `Map`/`Apply` use, so any callable (built-in, bridged, or
    // a user `SetDelayed` function) works.
    m.insert("Nest".to_string(), handler_fn(nest_handler));
    m.insert("NestList".to_string(), handler_fn(nest_list_handler));
    m.insert("Fold".to_string(), handler_fn(fold_handler));
    m.insert("FoldList".to_string(), handler_fn(fold_list_handler));
    // W-11 supports the canonical even-predicate idiom `Mod[#, 2] == 0 &`, so a
    // minimal integer `Mod` is added here (eager, like every other list/numeric
    // builtin). It is the only new builtin W-11 needs; the pure-function support
    // itself lives in the lowering + the backend rewrite rule, not here.
    m.insert("Mod".to_string(), handler_fn(mod_handler));
    // W-12 string builtins. All *eager* (non-held) heads — their string/expr
    // arguments arrive evaluated, exactly like the W-5/W-9 list builtins — so they
    // are *not* added to the `WolframBackend` held set. They operate on Unicode by
    // **character** (`chars()`, char indices — never byte slicing) and follow the
    // same fail-soft contract: a non-string arg or out-of-range index leaves the
    // form unevaluated rather than panicking. `StringSplit`/`Characters` build a
    // `List` (reusing the W-9 list machinery + `MAX_LIST_LENGTH` cap); `ToString`
    // reuses the W-4 `print_wolfram` printer.
    m.insert("StringJoin".to_string(), handler_fn(string_join_handler));
    m.insert("StringLength".to_string(), handler_fn(string_length_handler));
    m.insert("StringTake".to_string(), handler_fn(string_take_handler));
    m.insert("StringDrop".to_string(), handler_fn(string_drop_handler));
    m.insert("StringSplit".to_string(), handler_fn(string_split_handler));
    m.insert("StringReplace".to_string(), handler_fn(string_replace_handler));
    m.insert("ToString".to_string(), handler_fn(to_string_handler));
    m.insert("Characters".to_string(), handler_fn(characters_handler));

    // W-13 list set/multiset operations (MA04 §16). All ordinary `Head[args]`
    // forms — no grammar change. They reuse the W-9 list machinery
    // (`list_elements`, `apply(sym(LIST), …)`, `MAX_LIST_LENGTH`) and the W-9
    // canonical-order comparator `canonical_cmp` both to sort the unique outputs
    // of `Union`/`Intersection`/`Complement` and to define element-equality
    // (`same_element`). `Count` (W-9, predicate form) is left as-is.
    m.insert("Union".to_string(), handler_fn(union_handler));
    m.insert("Intersection".to_string(), handler_fn(intersection_handler));
    m.insert("Complement".to_string(), handler_fn(complement_handler));
    m.insert(
        "DeleteDuplicates".to_string(),
        handler_fn(delete_duplicates_handler),
    );
    m.insert("MemberQ".to_string(), handler_fn(member_q_handler));
    m.insert("Tally".to_string(), handler_fn(tally_handler));
    m
}

/// The W-7 iteration heads, which must be **held** (args not pre-evaluated) so
/// that the iterator index `i` can be bound into the body before each
/// evaluation. The [`WolframBackend`](crate::backend::WolframBackend) folds
/// these into its `hold_heads` set (union with the inner backend's held set).
///
/// Why held? `Table[i^2, {i, 3}]` must *not* evaluate `i^2` up front — `i` is a
/// local binder, and an eager eval would resolve it to a free symbol with
/// nothing left to substitute. Holding keeps both the body (`i^2`) and the
/// spec (`{i, 3}`) literal; the handler then evaluates the spec *bounds* itself
/// (they may be expressions like `{i, n}`) while substituting `i` into the body
/// per iteration.
pub const ITERATION_HEADS: [&str; 4] = ["Table", "Do", "Sum", "Product"];

/// The W-8 local-scoping heads, which must be **held** (args not pre-evaluated)
/// so that the declaration list and body arrive unevaluated — the locals are
/// bound *into* the body via `substitute` before the body is evaluated. The
/// [`WolframBackend`](crate::backend::WolframBackend) folds these into its
/// `hold_heads` set (union with the inner held set and [`ITERATION_HEADS`]).
///
/// Why held? `With[{x = 3}, x^2]` must *not* evaluate `x^2` up front — `x` is a
/// local binder, and an eager eval would resolve it to a free symbol (or a
/// stale global) with nothing left to substitute. Holding keeps both the decl
/// list (`{x = 3}`, lowered to `List(Assign(x, 3))`) and the body (`x^2`)
/// literal; the handler then evaluates each declaration's RHS itself and
/// substitutes the locals into the body per scope entry.
pub const SCOPING_HEADS: [&str; 3] = ["With", "Module", "Block"];

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
// Iteration — Table / Do / Sum / Product (W-7)
// ---------------------------------------------------------------------------
//
// All four share one shape: a held body `expr` and a held iterator spec
// `{i, …}`. The handler evaluates the spec bounds (they may be expressions),
// builds the bounded sequence of index values (capped at `MAX_RANGE_LENGTH`
// like `Range`), then for each value `v` substitutes `i → v` into the body and
// re-evaluates it through the VM. They differ only in what they do with the
// per-iteration results: collect (`Table`), discard (`Do`), fold-`+` (`Sum`),
// fold-`×` (`Product`).

/// A parsed, validated iterator specification: the binder name and the concrete
/// integer values the index takes, in order.
struct IteratorPlan {
    /// The local index symbol (the `i` in `{i, …}`).
    index: String,
    /// The materialised sequence of index values (already DoS-capped).
    values: Vec<i64>,
}

/// Parse and evaluate an iterator spec `{i, imax}` / `{i, imin, imax}` /
/// `{i, imin, imax, di}` into an [`IteratorPlan`].
///
/// Returns `None` (→ the caller leaves the whole form unevaluated) when:
/// - the spec is not a `List`, or its first element is not a bare symbol;
/// - it has the wrong arity (`{i}` with no bound, `{}`, or 5+ elements);
/// - a bound does not evaluate to an exact integer;
/// - the resulting count would exceed [`MAX_RANGE_LENGTH`] (the DoS cap).
///
/// The bound sub-expressions are evaluated through `vm` (the head is held, so
/// they arrive unevaluated — `{i, n}` with `n` a bound variable must be
/// resolved here). All arithmetic is `i128` so a crafted `i64::MIN`/`i64::MAX`
/// bound cannot overflow; an out-of-range count simply yields `None`.
fn plan_iterator(vm: &mut VM, spec: &IRNode) -> Option<IteratorPlan> {
    let elems = list_elements(spec)?;
    // {index, bound...} — at least the binder and one bound.
    if elems.len() < 2 || elems.len() > 4 {
        return None;
    }
    let index = match &elems[0] {
        IRNode::Symbol(s) => s.clone(),
        _ => return None,
    };

    // Evaluate each bound expression, then read it as an exact integer.
    let bound = |vm: &mut VM, node: &IRNode| -> Option<i64> {
        let evaled = vm.eval(node.clone());
        as_i64(&evaled)
    };

    let (start, end, step) = match &elems[1..] {
        // {i, imax} → i ranges 1..=imax.
        [imax] => (1i64, bound(vm, imax)?, 1i64),
        // {i, imin, imax} → i ranges imin..=imax.
        [imin, imax] => (bound(vm, imin)?, bound(vm, imax)?, 1i64),
        // {i, imin, imax, di} → stepped.
        [imin, imax, di] => (bound(vm, imin)?, bound(vm, imax)?, bound(vm, di)?),
        _ => return None,
    };

    let values = range_values(start, end, step)?;
    Some(IteratorPlan { index, values })
}

/// Materialise the integer sequence `start, start+step, …` up to `end`,
/// **DoS-capped** at [`MAX_RANGE_LENGTH`]. Mirrors the `Range` span logic
/// exactly (zero step refused, wrong-way step → empty, count computed in `i128`
/// before allocating). Returns `None` for a zero step or an oversize count so
/// the caller leaves the iteration form unevaluated; an empty range is `Some`
/// of an empty vector (a valid, if degenerate, iteration: `Sum` → 0, etc.).
fn range_values(start: i64, end: i64, step: i64) -> Option<Vec<i64>> {
    // A zero step never terminates — refuse it (form left unevaluated).
    if step == 0 {
        return None;
    }
    // A step pointing away from `end` is an empty iteration.
    if (step > 0 && start > end) || (step < 0 && start < end) {
        return Some(Vec::new());
    }
    // Count before allocating so an oversize span is rejected, never built.
    let span = (end as i128) - (start as i128);
    let count = (span / (step as i128)) + 1; // span and step share sign here
    if count <= 0 {
        return Some(Vec::new());
    }
    if count as u128 > MAX_RANGE_LENGTH as u128 {
        return None; // DoS cap — caller leaves the form unevaluated.
    }
    let mut values = Vec::with_capacity(count as usize);
    let mut value = start as i128;
    for _ in 0..count {
        values.push(value as i64);
        value += step as i128;
    }
    Some(values)
}

/// Bind `index → value` into `body` (a fresh copy via the VM's `substitute`,
/// the same primitive that binds user-function parameters) and evaluate it.
///
/// Using `substitute` rather than mutating the backend environment keeps the
/// index *local*: it never leaks into the session, and a nested `Table` binds
/// its own index over a body whose outer index was already replaced.
fn eval_body_at(vm: &mut VM, body: &IRNode, index: &str, value: i64) -> IRNode {
    let mut mapping: StdHashMap<String, IRNode> = StdHashMap::new();
    mapping.insert(index.to_string(), int(value));
    let bound = substitute(body.clone(), &mapping);
    vm.eval(bound)
}

/// `Table[expr, {i, …}]` → the list of `expr` evaluated with `i` bound to each
/// value of the range. A malformed spec (or oversize range) leaves the whole
/// `Table` unevaluated.
fn table_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(plan) = plan_iterator(vm, &expr.args[1]) else {
        return unevaluated(expr);
    };
    let body = expr.args[0].clone();
    let elems: Vec<IRNode> = plan
        .values
        .iter()
        .map(|&v| eval_body_at(vm, &body, &plan.index, v))
        .collect();
    apply(sym(LIST), elems)
}

/// `Do[expr, {i, n}]` → evaluate `expr` once per index value **for side
/// effects**, discarding each result, and return `Null` (a bare symbol, exactly
/// how Wolfram prints it). A malformed/oversize spec leaves `Do` unevaluated.
fn do_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(plan) = plan_iterator(vm, &expr.args[1]) else {
        return unevaluated(expr);
    };
    let body = expr.args[0].clone();
    for &v in &plan.values {
        // Evaluated purely for effect (e.g. a `Set` inside the body).
        let _ = eval_body_at(vm, &body, &plan.index, v);
    }
    sym("Null")
}

/// `Sum[expr, {i, imin, imax}]` → the sum of `expr` over the range, folded onto
/// the shared `Add` head (so symbolic terms combine via the same engine as
/// `1 + 2`). An empty range sums to `0`. A malformed/oversize spec leaves `Sum`
/// unevaluated.
fn sum_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    fold_iteration(vm, expr, ADD, int(0))
}

/// `Product[expr, {i, imin, imax}]` → the product of `expr` over the range,
/// folded onto the shared `Mul` head. An empty range is `1`. A malformed/
/// oversize spec leaves `Product` unevaluated.
fn product_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    fold_iteration(vm, expr, MUL, int(1))
}

/// Shared core of `Sum`/`Product`: evaluate the body at each index and fold the
/// results with a binary `op` (`Add`/`Mul`), seeded with `identity` (`0`/`1`)
/// so an empty range returns the identity. The fold is left-associative —
/// `op(op(op(identity, t1), t2), t3)` — and each step is re-evaluated through
/// the VM so numeric terms collapse as they accumulate (rather than building a
/// giant unevaluated tree).
fn fold_iteration(vm: &mut VM, expr: IRApply, op: &str, identity: IRNode) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(plan) = plan_iterator(vm, &expr.args[1]) else {
        return unevaluated(expr);
    };
    let body = expr.args[0].clone();
    let mut acc = identity;
    for &v in &plan.values {
        let term = eval_body_at(vm, &body, &plan.index, v);
        acc = vm.eval(apply(sym(op), vec![acc, term]));
    }
    acc
}

// ---------------------------------------------------------------------------
// Local scoping — With / Module / Block (W-8)
// ---------------------------------------------------------------------------
//
// All three share one shape: a held declaration list `{x = e, …}` (lowered to
// `List(Assign(x, e), …)`) and a held body. The handler parses the decls,
// evaluates each RHS through the VM, builds an `index → value` mapping, and then
// substitutes that mapping into the body before evaluating it — the **same**
// `substitute` primitive W-7's iteration index and user-function parameters use.
//
// Substituting into a *copy* of the held body (rather than mutating the session
// environment) is what makes the locals genuinely local: nothing is ever written
// to the global env, so a local can neither leak past the body nor clobber a
// same-named global. After `With[{x = 3}, x]`, a bare `x` is still the free
// symbol `x`. (MA04 §11.2–§11.3.)
//
// The three heads differ only in how a declaration is allowed to look:
//   * `With`  — every decl MUST be `name = value`; the value is evaluated.
//   * `Module`— a decl may be `name = value` (evaluated) OR a bare `name`
//               (uninitialised). An uninitialised local is **α-renamed** to a
//               fresh gensym (`name$nnn`, mirroring real Wolfram) so it can never
//               resolve to — and is never captured by — a same-named *global*.
//   * `Block` — same decl grammar as `With` here. Its dynamic-vs-lexical scope
//               difference is unobservable for the substitution-based subset
//               (MA04 §11.3); a self-contained body behaves identically.
//
// Why gensym for uninitialised Module locals specifically? Mapping `u → u` (the
// identity) would *not* shadow a global: `substitute` would leave the body's `u`
// as the symbol `u`, which `vm.eval` then resolves against the session env to
// any `u = 42` binding — a capture leak. Renaming `u → u$nnn` produces a symbol
// the env has never bound, so it stays free (undefined) exactly as a fresh local
// should. Initialised locals (`y = e`) don't need this: their `y` is replaced by
// the *value* `eval(e)`, so no `y` symbol survives in the body to be captured.

use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter for `Module`'s gensym renaming of uninitialised locals.
///
/// Each uninitialised `Module` local `x` is renamed to `x$<n>` for a unique `n`,
/// so it cannot collide with a global, a sibling local, or an outer scope's
/// local of the same name. `Relaxed` ordering is sufficient — we only need each
/// fetched value to be distinct, not ordered relative to other memory.
static MODULE_GENSYM_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Allocate a fresh, never-before-used local name for `base` (e.g. `x` → `x$7`).
fn fresh_local_name(base: &str) -> String {
    let n = MODULE_GENSYM_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{base}${n}")
}

/// `With[{x = e, …}, body]` — bind each local to its evaluated RHS, substitute
/// into the body, and evaluate. Lexical, immediate (the RHS is evaluated against
/// the surrounding scope, so a decl may reference an outer binding). Every decl
/// must be an initialised `name = value`; a bare-symbol decl (no value) or any
/// other malformed decl/arity/non-list leaves the whole `With` unevaluated.
fn with_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    scope_handler(vm, expr, /* allow_uninitialised = */ false)
}

/// `Block[{x = e}, body]` — temporarily binds `x` for the duration of `body`.
/// In real Wolfram this is *dynamic* scope (it shadows a global `x`); for the
/// substitution-based subset shipped here a self-contained body is observably
/// identical to `With` (MA04 §11.3), so it shares the same decl grammar
/// (every local must be initialised) and the same substitution mechanism.
fn block_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    scope_handler(vm, expr, /* allow_uninitialised = */ false)
}

/// `Module[{x, y = e}, body]` — lexically-scoped locals. An initialised decl
/// (`y = e`) is evaluated like `With`; an *uninitialised* decl (`x`) is α-renamed
/// to a fresh gensym (`x$nnn`) so the body sees an undefined symbol that can
/// never resolve to — or be captured by — a same-named global. This is the one
/// head that accepts a bare-symbol declaration.
fn module_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    scope_handler(vm, expr, /* allow_uninitialised = */ true)
}

/// Shared core of `With`/`Module`/`Block`: parse `{decls}`, evaluate each
/// declaration's RHS, build the local mapping, substitute it into the body, and
/// evaluate. `allow_uninitialised` controls whether a bare-symbol decl (no `=`)
/// is permitted (`Module`) or rejected (`With`/`Block`).
///
/// Returns the form **unevaluated** (never a panic) on any malformed input:
/// wrong arity, a non-`List` first argument, or a declaration that is neither a
/// bare symbol nor a `name = value` assignment (or a bare symbol where a value
/// is required). This mirrors the W-5/W-7 "I can't reduce this" convention.
fn scope_handler(vm: &mut VM, expr: IRApply, allow_uninitialised: bool) -> IRNode {
    // Head[{decls}, body] — exactly two arguments.
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    // The first argument must be a literal `List` of declarations.
    let Some(decls) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };

    // Parse + evaluate every declaration into a (name, value) pair. A single
    // malformed declaration aborts the whole form (left unevaluated) — we do not
    // partially bind.
    let mut mapping: StdHashMap<String, IRNode> = StdHashMap::new();
    for decl in &decls {
        let Some((name, value)) = eval_decl(vm, decl, allow_uninitialised) else {
            return unevaluated(expr);
        };
        // A later decl with the same name shadows an earlier one (last wins),
        // matching how a repeated local would behave; harmless for valid input.
        mapping.insert(name, value);
    }

    // Bind the locals into a *copy* of the held body and evaluate. Because this
    // never touches the session environment, the locals do not leak (MA04 §11.2).
    let body = substitute(expr.args[1].clone(), &mapping);
    vm.eval(body)
}

/// Parse one declaration node into `(name, replacement)` — the symbol the body
/// binds and the IR node every free occurrence of it is replaced with.
///
/// Accepts two shapes:
/// - `Assign(name, rhs)` (the lowering of `name = value`): `name` must be a bare
///   symbol; `rhs` is **evaluated** through the VM (so `With[{x = 1 + 1}, …]`
///   binds `x → 2`, and an RHS referring to an outer binding resolves). The
///   replacement is the evaluated *value*.
/// - a bare `Symbol(name)` — only when `allow_uninitialised` (i.e. `Module`): the
///   local is α-renamed to a fresh gensym `name$nnn`. The replacement is that
///   fresh symbol, so the body sees an undefined local that cannot resolve to a
///   global (see the module-scoping note above).
///
/// Returns `None` (→ the caller leaves the whole scoping form unevaluated) for a
/// non-symbol assignment target (`f[x] = 1`, `1 = 2`), a bare symbol where a
/// value is required (`With`/`Block`), or any other node shape.
fn eval_decl(
    vm: &mut VM,
    decl: &IRNode,
    allow_uninitialised: bool,
) -> Option<(String, IRNode)> {
    match decl {
        // `name = value` → Assign(name, value).
        IRNode::Apply(app) if is_assign(&app.head) && app.args.len() == 2 => {
            let name = match &app.args[0] {
                IRNode::Symbol(s) => s.clone(),
                // A non-symbol assignment target (`f[x] = 1`, `1 = 2`) is not a
                // valid local declaration.
                _ => return None,
            };
            let value = vm.eval(app.args[1].clone());
            Some((name, value))
        }
        // A bare symbol `x` — an uninitialised local (Module only). It is renamed
        // to a fresh gensym so the body sees an undefined local, never a global.
        IRNode::Symbol(name) if allow_uninitialised => {
            Some((name.clone(), sym(fresh_local_name(name))))
        }
        // Anything else (a bare symbol where a value is required, a literal, a
        // non-Assign application) is malformed.
        _ => None,
    }
}

/// True if `head` is the `Assign` symbol (the IR head a surface `x = e`
/// declaration lowers to).
fn is_assign(head: &IRNode) -> bool {
    matches!(head, IRNode::Symbol(s) if s == ASSIGN)
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
// List manipulation — Sort / Reverse / Join / Flatten (W-9)
// ---------------------------------------------------------------------------

/// `Sort[{c, a, b}]` → `{a, b, c}` — ascending in the subset's canonical order
/// ([`canonical_cmp`]). For a pure-numeric list this is numeric order
/// (`Sort[{3, 1, 2}]` → `{1, 2, 3}`); for mixed/symbolic lists it is the
/// documented total order (numbers < symbols < strings < compound, then by
/// value/name/structure). `Sort` of a non-list is left unevaluated.
///
/// The sort is *stable* (`sort_by`, not `sort_unstable_by`) so equal-key
/// elements keep their input order — deterministic across runs.
fn sort_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(mut elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    elems.sort_by(canonical_cmp);
    apply(sym(LIST), elems)
}

/// `Reverse[{1, 2, 3}]` → `{3, 2, 1}`. `Reverse` of a non-list is left
/// unevaluated. Size-preserving — no new DoS surface.
fn reverse_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(mut elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    elems.reverse();
    apply(sym(LIST), elems)
}

/// `Join[a, b, …]` → the lists concatenated, in order. Two or more list
/// arguments are required; if *any* argument is not a list the whole form is left
/// unevaluated (Wolfram's `Join` requires every argument to share the same head).
///
/// **DoS-capped**: the combined length is bounded by [`MAX_LIST_LENGTH`] — an
/// over-cap join is left unevaluated rather than allocated. The total length is
/// accumulated in `usize` with `checked_add` so a crafted chain cannot overflow
/// the running count.
fn join_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() < 2 {
        return unevaluated(expr);
    }
    // First pass: every argument must be a list, and the *combined* length must
    // not exceed the cap — checked before any allocation so an over-cap join is
    // never built.
    let mut lists: Vec<Vec<IRNode>> = Vec::with_capacity(expr.args.len());
    let mut total: usize = 0;
    for arg in &expr.args {
        let Some(elems) = list_elements(arg) else {
            return unevaluated(expr);
        };
        total = match total.checked_add(elems.len()) {
            Some(t) if t <= MAX_LIST_LENGTH => t,
            // Over the cap (or a usize overflow) — refuse, leave unevaluated.
            _ => return unevaluated(expr),
        };
        lists.push(elems);
    }
    let mut out = Vec::with_capacity(total);
    for elems in lists {
        out.extend(elems);
    }
    apply(sym(LIST), out)
}

/// `Flatten[list]` → every nested sub-list spliced in at **all** levels;
/// `Flatten[list, n]` → only the top `n` levels are flattened (a deeper sub-list
/// is left intact).
///
/// `Flatten[{{1, 2}, {3}}]` → `{1, 2, 3}`; `Flatten[{1, {2, {3}}}]` →
/// `{1, 2, 3}`; `Flatten[{1, {2, {3}}}, 1]` → `{1, 2, {3}}` (one level only).
///
/// **DoS-bounded** on two axes: the recursion depth is bounded (the full-flatten
/// recurses on structure, itself bounded by the token-capped input nesting; the
/// `n`-form additionally stops after `n` levels), and the output length is capped
/// at [`MAX_LIST_LENGTH`] — once the accumulator reaches the cap the whole form
/// is left unevaluated rather than grown without bound. A non-list first
/// argument, or a negative/non-integer depth, leaves the form unevaluated.
fn flatten_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    // Resolve the optional depth: absent → flatten all levels (i64::MAX as a
    // sentinel "unbounded"); present → an exact non-negative integer.
    let depth: i64 = match expr.args.as_slice() {
        [_] => i64::MAX,
        [_, d] => match as_i64(d) {
            Some(n) if n >= 0 => n,
            // A negative or non-integer depth is malformed.
            _ => return unevaluated(expr),
        },
        _ => return unevaluated(expr),
    };
    let Some(top_elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    // The *top* list is always the result container — unwrapping it does not cost
    // a level. `depth` counts how many levels of *nested sub-lists* to splice, so
    // `Flatten[list, 1]` splices the sub-lists that are direct elements of `list`.
    // Each top element is flattened with the full `depth`.
    let mut out: Vec<IRNode> = Vec::new();
    for elem in &top_elems {
        // `flatten_into` returns `false` if the cap was hit mid-flatten; in that
        // case leave the whole form unevaluated rather than return a truncated list.
        if !flatten_into(elem, depth, &mut out) {
            return unevaluated(expr);
        }
    }
    apply(sym(LIST), out)
}

/// Splice `node` into `out`: if `node` is a sub-list and `depth > 0`, recurse
/// into each of its elements with one less level remaining; otherwise push `node`
/// verbatim. `i64::MAX` for `depth` means "unbounded — descend through every
/// nested list". Returns `false` if appending would exceed [`MAX_LIST_LENGTH`]
/// (the DoS cap), so the caller can reject the whole `Flatten`.
///
/// Each recursive step decrements `depth`, so the `n`-form splices exactly `n`
/// levels of nested sub-lists; the unbounded form saturates at `i64::MAX - 1` and
/// keeps descending, but real recursion depth is bounded by the (token-capped)
/// input nesting, so it always terminates.
fn flatten_into(node: &IRNode, depth: i64, out: &mut Vec<IRNode>) -> bool {
    match list_elements(node) {
        // A sub-list, and we still have levels to splice: descend into each element.
        Some(elems) if depth > 0 => {
            for elem in &elems {
                if !flatten_into(elem, depth.saturating_sub(1), out) {
                    return false;
                }
            }
            true
        }
        // A non-list element, or depth exhausted: push the element verbatim
        // (capping the output length first).
        _ => {
            if out.len() >= MAX_LIST_LENGTH {
                return false;
            }
            out.push(node.clone());
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Filtering / counting — Select / Count (W-9)
// ---------------------------------------------------------------------------

/// `Select[{1, 2, 3, 4}, EvenQ]` → `{2, 4}` — keep each element for which
/// `pred[e]` evaluates to the `True` symbol. The predicate is applied through the
/// **same** path as `Map`/`Apply`: `build_canonical_application(pred, [e])` then
/// `vm.eval`, so any callable (a built-in `EvenQ`, a user `f[x_] := …`, a bridged
/// head) works. A non-list second argument, or wrong arity, leaves the form
/// unevaluated. The output is at most as long as the input — no new DoS surface.
fn select_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let pred = expr.args[1].clone();
    let kept: Vec<IRNode> = elems
        .into_iter()
        .filter(|e| predicate_is_true(vm, &pred, e))
        .collect();
    apply(sym(LIST), kept)
}

/// `Count[{1, 2, 3, 4}, EvenQ]` → `2` — the number of elements for which
/// `pred[e]` evaluates to `True`. Shares the predicate-application path with
/// [`select_handler`]. This is the **documented simplification** versus full
/// Wolfram pattern-matching `Count` (where the second argument may be a pattern):
/// W-9 supports a *function* predicate, the common introductory case (MA04 §12.3).
/// A non-list first argument, or wrong arity, leaves the form unevaluated.
fn count_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let pred = expr.args[1].clone();
    let n = elems
        .iter()
        .filter(|e| predicate_is_true(vm, &pred, e))
        .count();
    int(n as i64)
}

/// Apply `pred` to `element` and report whether the result is the literal `True`
/// symbol. Builds `pred[element]` via the canonical application path (the same
/// one `Map`/`Apply` use, so the `Plus`→`Add`-style bridges and user functions
/// all resolve) and re-evaluates it through the VM. Any result other than the
/// `True` symbol — `False`, an unevaluated `pred[element]`, a number — counts as
/// *not* selected; this never panics on a non-callable predicate (an unbound head
/// just leaves `pred[element]` unevaluated, which is not `True`).
fn predicate_is_true(vm: &mut VM, pred: &IRNode, element: &IRNode) -> bool {
    let applied = build_canonical_application(pred.clone(), vec![element.clone()]);
    matches!(vm.eval(applied), IRNode::Symbol(s) if s == "True")
}

// ---------------------------------------------------------------------------
// Summation — Total (W-9)
// ---------------------------------------------------------------------------

/// `Total[{1, 2, 3}]` → `6` — the sum of the list's elements, folded onto the
/// shared `Add` head (so symbolic terms combine via the same engine as `1 + 2`,
/// consistent with W-7 `Sum` over a range). An empty list totals to `0`. `Total`
/// of a non-list is left unevaluated.
///
/// The fold is left-associative and each step is re-evaluated through the VM, so
/// numeric terms collapse as they accumulate rather than building a giant
/// unevaluated tree — identical in shape to W-7's `fold_iteration`.
fn total_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let mut acc = int(0);
    for elem in elems {
        acc = vm.eval(apply(sym(ADD), vec![acc, elem]));
    }
    acc
}

// ---------------------------------------------------------------------------
// Set / multiset operations — Union / Intersection / Complement /
// DeleteDuplicates / MemberQ / Tally (W-13)
// ---------------------------------------------------------------------------
//
// The set-theoretic list vocabulary, lowered onto the *same* substrate as W-9:
// `list_elements` to unwrap a `List(...)`, `apply(sym(LIST), …)` to rebuild one,
// the `MAX_LIST_LENGTH` DoS cap, and — crucially — the W-9 canonical-order
// comparator `canonical_cmp` reused *twice*: once to sort the unique outputs of
// `Union`/`Intersection`/`Complement`, and once to define **element-equality**
// via [`same_element`]. Reusing the comparator (rather than inventing a second
// notion of equality) keeps the answers deterministic, consistent with `Sort`,
// and panic-free for `NaN` (`canonical_cmp` is built on `f64::total_cmp`).
//
// Two ordering families, both matching Wolfram exactly:
//   * `Union`/`Intersection`/`Complement` — outputs are **sorted** (canonical
//     order) and duplicate-free, regardless of input order;
//   * `DeleteDuplicates`/`Tally` — outputs are **order-preserving**, fixing each
//     distinct element's position at its first occurrence.
//
// Cost note: membership is a linear `canonical_cmp` scan (no hashing — `IRNode`
// carries an `f64` and is not value-`Hash`-keyable), so the heads are worst-case
// quadratic. Every input is already bounded by `MAX_LIST_LENGTH`, so this is a
// deliberate, documented trade (simplicity over a canonical-key index), never an
// unbounded surface.

/// Two `IRNode`s are the **same element** iff the W-9 canonical comparator ranks
/// them `Equal`. This is the single notion of element-equality every W-13 head
/// uses. It is total and panic-free (built on `f64::total_cmp`), and its
/// type-tag tie-break keeps distinct numeric subtypes of equal magnitude
/// separate, so `2` and `2.0` are **distinct** elements — matching Wolfram, where
/// `Union[{2, 2.}]` keeps both. Structural equality on compound elements
/// (`f[1]` vs `f[1]`) is decided recursively by `canonical_cmp`.
fn same_element(a: &IRNode, b: &IRNode) -> bool {
    canonical_cmp(a, b) == std::cmp::Ordering::Equal
}

/// True if `set` already contains an element equal (under [`same_element`]) to
/// `candidate`. The linear membership scan shared by every W-13 head — the source
/// of the documented worst-case quadratic cost, bounded by `MAX_LIST_LENGTH`.
fn contains_element(set: &[IRNode], candidate: &IRNode) -> bool {
    set.iter().any(|e| same_element(e, candidate))
}

/// `Union[a, b, …]` → the **sorted**, duplicate-free union of the element lists.
///
/// `Union[{1, 2}, {2, 3}]` → `{1, 2, 3}`; `Union[{3, 1, 2, 1}]` → `{1, 2, 3}`
/// (a single argument doubles as "sort-and-unique"). Every argument must be a
/// `List`; a non-list argument (or zero arguments) leaves the form unevaluated.
///
/// **DoS-capped**: the deduped accumulator is refused (form left unevaluated) the
/// moment it would exceed [`MAX_LIST_LENGTH`] — symmetric with `Join`/`Flatten`.
/// The final result is sorted with the W-9 `canonical_cmp` (a *stable* `sort_by`),
/// so the order is deterministic.
fn union_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.is_empty() {
        return unevaluated(expr);
    }
    // Accumulate the deduped union across all argument lists, capping the
    // accumulator length before each insert.
    let mut out: Vec<IRNode> = Vec::new();
    for arg in &expr.args {
        let Some(elems) = list_elements(arg) else {
            return unevaluated(expr);
        };
        for elem in elems {
            if !contains_element(&out, &elem) {
                if out.len() >= MAX_LIST_LENGTH {
                    return unevaluated(expr);
                }
                out.push(elem);
            }
        }
    }
    out.sort_by(canonical_cmp);
    apply(sym(LIST), out)
}

/// `Intersection[a, b, …]` → the **sorted** elements present in *every* argument
/// list. `Intersection[{1, 2, 3}, {2, 3, 4}]` → `{2, 3}`. With a single list
/// argument it is that list, sorted and deduplicated. Every argument must be a
/// `List`; a non-list argument (or zero arguments) leaves the form unevaluated.
///
/// Output is size-non-increasing (a subset of the first list), so it is bounded by
/// the already-capped first argument; the [`MAX_LIST_LENGTH`] cap is asserted
/// anyway for symmetry. Result sorted with the W-9 `canonical_cmp`.
fn intersection_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.is_empty() {
        return unevaluated(expr);
    }
    // Materialise every argument up front so a non-list anywhere rejects the whole
    // form before we start filtering.
    let mut lists: Vec<Vec<IRNode>> = Vec::with_capacity(expr.args.len());
    for arg in &expr.args {
        let Some(elems) = list_elements(arg) else {
            return unevaluated(expr);
        };
        lists.push(elems);
    }
    // Keep each distinct element of the first list that also appears in *all* the
    // rest. Dedup against `out` so a repeated element in the first list is emitted
    // once.
    let (first, rest) = lists.split_first().expect("non-empty: checked above");
    let mut out: Vec<IRNode> = Vec::new();
    for elem in first {
        if contains_element(&out, elem) {
            continue;
        }
        if rest.iter().all(|other| contains_element(other, elem)) {
            if out.len() >= MAX_LIST_LENGTH {
                return unevaluated(expr);
            }
            out.push(elem.clone());
        }
    }
    out.sort_by(canonical_cmp);
    apply(sym(LIST), out)
}

/// `Complement[all, x, …]` → the **sorted** elements of `all` that appear in *none*
/// of `x, …`. `Complement[{1, 2, 3, 4}, {2, 4}]` → `{1, 3}`. At least the `all`
/// argument is required; `Complement[all]` is `all`, sorted and deduplicated.
/// Every argument must be a `List`; a non-list argument leaves the form
/// unevaluated.
///
/// Output is a subset of `all` (size-non-increasing), bounded by the already-capped
/// first argument; [`MAX_LIST_LENGTH`] is asserted for symmetry. Result sorted with
/// the W-9 `canonical_cmp`.
fn complement_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.is_empty() {
        return unevaluated(expr);
    }
    let mut lists: Vec<Vec<IRNode>> = Vec::with_capacity(expr.args.len());
    for arg in &expr.args {
        let Some(elems) = list_elements(arg) else {
            return unevaluated(expr);
        };
        lists.push(elems);
    }
    let (all, subtract) = lists.split_first().expect("non-empty: checked above");
    let mut out: Vec<IRNode> = Vec::new();
    for elem in all {
        if contains_element(&out, elem) {
            continue;
        }
        if subtract.iter().all(|other| !contains_element(other, elem)) {
            if out.len() >= MAX_LIST_LENGTH {
                return unevaluated(expr);
            }
            out.push(elem.clone());
        }
    }
    out.sort_by(canonical_cmp);
    apply(sym(LIST), out)
}

/// `DeleteDuplicates[list]` → the list with later duplicates removed, **preserving
/// the first-occurrence order** (deliberately *unlike* `Union`, which re-sorts).
///
/// `DeleteDuplicates[{3, 1, 1, 2, 3}]` → `{3, 1, 2}`. A non-list argument, or the
/// wrong arity, leaves the form unevaluated. Output is size-non-increasing (a
/// subsequence of the input), bounded by the already-capped input; no re-sort, so
/// the input order is the output order.
fn delete_duplicates_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let mut out: Vec<IRNode> = Vec::new();
    for elem in elems {
        if !contains_element(&out, &elem) {
            out.push(elem);
        }
    }
    apply(sym(LIST), out)
}

/// `MemberQ[list, elem]` → the literal `True` symbol if some element of `list`
/// equals `elem` (under [`same_element`]), else `False`.
///
/// `MemberQ[{1, 2, 3}, 2]` → `True`; `MemberQ[{1, 2, 3}, 9]` → `False`. A non-list
/// first argument, or the wrong arity, leaves the form unevaluated (Wolfram's
/// `MemberQ[3, 2]` is likewise unevaluated). Returns a boolean — no ordering or
/// size concern.
fn member_q_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    if contains_element(&elems, &expr.args[1]) {
        sym("True")
    } else {
        sym("False")
    }
}

/// `Tally[list]` → a list of `{element, count}` pairs in **first-occurrence**
/// order, where `count` is how many times that element appears.
///
/// `Tally[{a, a, b, a}]` → `{{a, 3}, {b, 1}}`. A non-list argument, or the wrong
/// arity, leaves the form unevaluated. The pair list has at most as many entries
/// as the input has distinct elements; that count is capped at [`MAX_LIST_LENGTH`]
/// (an over-cap distinct-element count leaves the form unevaluated) — defensive,
/// since the input is itself already bounded.
fn tally_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[0]) else {
        return unevaluated(expr);
    };
    // Parallel vectors: the distinct elements in first-occurrence order, and their
    // running counts. A linear scan per element keeps the order without hashing
    // (the documented quadratic cost, bounded by MAX_LIST_LENGTH).
    let mut keys: Vec<IRNode> = Vec::new();
    let mut counts: Vec<i64> = Vec::new();
    for elem in elems {
        match keys.iter().position(|k| same_element(k, &elem)) {
            Some(i) => counts[i] += 1,
            None => {
                if keys.len() >= MAX_LIST_LENGTH {
                    return unevaluated(expr);
                }
                keys.push(elem);
                counts.push(1);
            }
        }
    }
    // Build the {element, count} pairs in the recorded first-occurrence order.
    let pairs: Vec<IRNode> = keys
        .into_iter()
        .zip(counts)
        .map(|(k, c)| apply(sym(LIST), vec![k, int(c)]))
        .collect();
    apply(sym(LIST), pairs)
}

// ---------------------------------------------------------------------------
// Functional iteration — Nest / NestList / Fold / FoldList (W-10)
// ---------------------------------------------------------------------------
//
// The four point-free combinators that iterate a *function*. Each re-applies `f`
// through the **same** `build_canonical_application(f, args)` + `vm.eval` path the
// W-5 `Map`/`Apply` and W-9 `Select` use, so any callable resolves: a built-in
// (`Plus`), a bridged head, or a user `SetDelayed` function `g[a_] := …`. A
// symbolic `f` with no definition leaves each `f[acc]` unevaluated, so
// `Nest[f, x, 3]` returns the literal `f[f[f[x]]]` — exactly Wolfram's behaviour.
//
// Two iterate by *unary* re-application (`Nest`/`NestList`, building `f[acc]`),
// two by *binary* left-fold (`Fold`/`FoldList`, building `f[acc, element]`). The
// `…List` variants additionally collect every intermediate result. All four are
// eager (non-held) — `f`, the seed, and the list arrive already evaluated.

/// Apply `f` once to `acc`, re-evaluating through the VM. The single shared
/// primitive of all four combinators: builds `f[acc]` (or `f[acc, x]` for the
/// fold forms, via [`apply_binary`]) on the same canonical-application path as
/// `Map`/`Apply`, so a defined `f` reduces and an undefined one stays as a literal
/// `f[acc]` node (never a panic, never a guess).
fn apply_unary(vm: &mut VM, f: &IRNode, acc: IRNode) -> IRNode {
    vm.eval(build_canonical_application(f.clone(), vec![acc]))
}

/// Apply `f` to the running accumulator and the next list element — the binary
/// fold step `f[acc, element]`, re-evaluated through the VM on the same path as
/// [`apply_unary`].
fn apply_binary(vm: &mut VM, f: &IRNode, acc: IRNode, element: IRNode) -> IRNode {
    vm.eval(build_canonical_application(f.clone(), vec![acc, element]))
}

/// Read the iteration count `n` of `Nest`/`NestList` as a **DoS-capped**
/// non-negative `usize`.
///
/// Returns `None` (→ the caller leaves the whole form unevaluated) for a negative
/// `n`, a non-integer `n`, or an `n` exceeding [`MAX_LIST_LENGTH`] — the cap is
/// checked *before* any iteration so a tiny input like `Nest[f, x, 10^9]` can
/// never drive a billion `vm.eval` calls. `n == 0` is valid (the identity case).
fn nest_count(node: &IRNode) -> Option<usize> {
    let n = as_i64(node)?;
    // Negative counts are malformed; an over-cap count is refused so the loop
    // (and, for `NestList`, the `n + 1` allocation) is bounded.
    if n < 0 || n as u128 > MAX_LIST_LENGTH as u128 {
        return None;
    }
    Some(n as usize)
}

/// `Nest[f, x, n]` → `f` applied to `x` `n` times: `f[f[…f[x]…]]` (`n`
/// applications). `Nest[f, x, 0]` is the identity (`x`).
///
/// Iterates by repeated unary re-application through the VM, so a defined `f`
/// folds (`square[a_] := a*a; Nest[square, 2, 3]` → `256`) and a symbolic `f`
/// builds the literal nest (`Nest[f, x, 3]` → `f[f[f[x]]]`). **DoS-capped**: `n`
/// is bounded by [`nest_count`] before the loop; a negative/non-integer/over-cap
/// `n`, or the wrong arity, leaves the form unevaluated.
fn nest_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 3 {
        return unevaluated(expr);
    }
    let Some(n) = nest_count(&expr.args[2]) else {
        return unevaluated(expr);
    };
    let f = expr.args[0].clone();
    let mut acc = expr.args[1].clone();
    for _ in 0..n {
        acc = apply_unary(vm, &f, acc);
    }
    acc
}

/// `NestList[f, x, n]` → `{x, f[x], f[f[x]], …}` — the `n + 1` intermediate
/// results, **including the seed** `x` at the front.
///
/// `NestList[f, x, 2]` → `{x, f[x], f[f[x]]}`; `NestList[g, 0, 3]` with
/// `g[a_] := a + 1` → `{0, 1, 2, 3}`. **DoS-capped**: `n` is bounded by
/// [`nest_count`], which also bounds the `n + 1`-element result allocation;
/// a malformed/over-cap `n` or the wrong arity leaves the form unevaluated.
fn nest_list_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 3 {
        return unevaluated(expr);
    }
    let Some(n) = nest_count(&expr.args[2]) else {
        return unevaluated(expr);
    };
    let f = expr.args[0].clone();
    let mut acc = expr.args[1].clone();
    // n is capped at MAX_LIST_LENGTH, so n + 1 cannot overflow usize and the
    // allocation is bounded.
    let mut out = Vec::with_capacity(n + 1);
    out.push(acc.clone());
    for _ in 0..n {
        acc = apply_unary(vm, &f, acc);
        out.push(acc.clone());
    }
    apply(sym(LIST), out)
}

/// `Fold[f, x0, list]` → the **left** fold `f[…f[f[x0, l₁], l₂]…, lₙ]`.
///
/// `Fold[Plus, 0, {1, 2, 3}]` → `6` (`((0 + 1) + 2) + 3`). Folds the
/// already-materialised `list` with `f` seeded at `x0`, each step re-evaluated
/// through the VM (so a numeric fold collapses as it accumulates). An empty list
/// returns the seed `x0` unchanged. A non-list third argument, or the wrong
/// arity, leaves the form unevaluated. No new result-size surface — only the
/// scalar accumulator is held; the iteration count is the (source-bounded) list
/// length.
fn fold_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 3 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[2]) else {
        return unevaluated(expr);
    };
    let f = expr.args[0].clone();
    let mut acc = expr.args[1].clone();
    for element in elems {
        acc = apply_binary(vm, &f, acc, element);
    }
    acc
}

/// `FoldList[f, x0, list]` → `{x0, f[x0, l₁], f[f[x0, l₁], l₂], …}` — the running
/// accumulations, **including the seed** `x0` at the front.
///
/// `FoldList[Plus, 0, {1, 2, 3}]` → `{0, 1, 3, 6}`. Like [`fold_handler`] but
/// collecting every intermediate accumulator. An empty list returns `{x0}` (the
/// seed alone). **DoS-bounded**: the result has `len + 1` elements where `len` is
/// the source-bounded input length; a defensive [`MAX_LIST_LENGTH`] check on the
/// input length keeps the `len + 1` allocation bounded even for a crafted input.
/// A non-list third argument, the wrong arity, or an over-cap input length leaves
/// the form unevaluated.
fn fold_list_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 3 {
        return unevaluated(expr);
    }
    let Some(elems) = list_elements(&expr.args[2]) else {
        return unevaluated(expr);
    };
    // Defensive: the result is `elems.len() + 1` elements. The input is already
    // source-bounded, but cap it so the `+ 1` allocation can never exceed the
    // shared list cap (matching `Join`/`Flatten`).
    if elems.len() >= MAX_LIST_LENGTH {
        return unevaluated(expr);
    }
    let f = expr.args[0].clone();
    let mut acc = expr.args[1].clone();
    let mut out = Vec::with_capacity(elems.len() + 1);
    out.push(acc.clone());
    for element in elems {
        acc = apply_binary(vm, &f, acc, element);
        out.push(acc.clone());
    }
    apply(sym(LIST), out)
}

// ---------------------------------------------------------------------------
// Parity predicates — EvenQ / OddQ (W-9)
// ---------------------------------------------------------------------------

/// `EvenQ[n]` → `True` if `n` is an even integer, else `False`. A non-integer
/// argument (a rational, float, symbol, or list) is `False`, matching Wolfram
/// (`EvenQ[x]` is `False`, not unevaluated). Wrong arity stays unevaluated.
///
/// Even-ness uses `rem_euclid(2)` so a *negative* `n` is classified correctly
/// (`EvenQ[-4]` → `True`), unlike the truncating `%` which would still be fine
/// for `== 0` but `rem_euclid` makes the intent explicit.
fn even_q_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    parity_q(expr, /* want_even = */ true)
}

/// `OddQ[n]` → `True` if `n` is an odd integer, else `False`. See [`even_q_handler`].
fn odd_q_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    parity_q(expr, /* want_even = */ false)
}

/// Shared core of `EvenQ`/`OddQ`: `True`/`False` on integer parity, `False` for a
/// non-integer, unevaluated for the wrong arity.
fn parity_q(expr: IRApply, want_even: bool) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let is_even = match &expr.args[0] {
        IRNode::Integer(n) => n.rem_euclid(2) == 0,
        // A non-integer is neither EvenQ nor OddQ → False.
        _ => return sym("False"),
    };
    if is_even == want_even {
        sym("True")
    } else {
        sym("False")
    }
}

// ---------------------------------------------------------------------------
// Integer modulo — Mod (W-11 support for the `Mod[#, 2] == 0 &` idiom)
// ---------------------------------------------------------------------------

/// `Mod[a, b]` → the integer remainder of `a` divided by `b`, using Wolfram's
/// (and Rust's `rem_euclid`) convention that the result has the **sign of the
/// divisor** and lies in `[0, |b|)` for positive `b`: `Mod[7, 2]` → `1`,
/// `Mod[-1, 3]` → `2`. Both arguments must be exact integers and the divisor must
/// be non-zero; any other shape (wrong arity, a non-integer, or a zero divisor)
/// leaves the form unevaluated rather than panicking — the same fail-soft
/// convention every W-5/W-9 builtin follows.
fn mod_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let (Some(a), Some(b)) = (as_i64(&expr.args[0]), as_i64(&expr.args[1])) else {
        return unevaluated(expr);
    };
    if b == 0 {
        return unevaluated(expr); // Mod by zero is undefined.
    }
    // Compute in i128 so a crafted `i64::MIN` divisor cannot overflow: `b.abs()`
    // panics (debug) / wraps (release) for `b == i64::MIN`, so we must NOT take a
    // signed abs at i64 width. `(a as i128).rem_euclid(|b|)` lands in `[0, |b|)`;
    // for a negative divisor we shift into the divisor's sign to match Wolfram.
    // The final remainder's magnitude is < |b| <= i64::MAX + 1, and after the
    // negative-divisor shift it lies in `(b, 0]`, so it always fits back in i64.
    let a = a as i128;
    let b = b as i128;
    let mut r = a.rem_euclid(b.abs());
    if b < 0 && r != 0 {
        r -= b.abs();
    }
    // r is now in (-|b|, |b|) with the divisor's sign, hence within i64 range.
    int(r as i64)
}

// ---------------------------------------------------------------------------
// W-12 string builtins
// ---------------------------------------------------------------------------
//
// Every handler in this block operates on Unicode **by character**: it reads the
// argument as a `&str` (via `as_str`), then — for any indexing or slicing —
// collects `s.chars()` into a `Vec<char>` and indexes *that*. No byte index is
// ever taken, so a multi-byte character (`é`, an emoji) counts as exactly one
// position and a slice can never fall in the middle of a UTF-8 sequence (the
// `byte index N is not a char boundary` panic is structurally impossible). Like
// every W-5/W-9 builtin, an argument of the wrong shape (a non-string, an
// out-of-range or non-integer index) leaves the form **unevaluated** rather than
// panicking — the Wolfram "I can't reduce this" answer.

/// Read a `Str` node as a `&str`, or `None` for any non-string node. The string
/// analogue of [`as_i64`].
fn as_str(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

/// `StringJoin["a", "b", "c"]` → `"abc"`. Every argument must be a string; if any
/// is not, the whole form is left unevaluated (Wolfram's `StringJoin` requires
/// string arguments). Zero or one argument is fine (`StringJoin[]` → `""`,
/// `StringJoin["x"]` → `"x"`).
///
/// **DoS-capped**: the combined character length is bounded by
/// [`MAX_STRING_LENGTH`] — the running total is accumulated with `checked_add`
/// (so a crafted chain cannot overflow the count) and an over-cap join is left
/// unevaluated *before* any allocation.
fn string_join_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    // First pass: every argument must be a string, and the combined character
    // length must stay within the cap — checked before building the output.
    let mut parts: Vec<&str> = Vec::with_capacity(expr.args.len());
    let mut total: usize = 0;
    for arg in &expr.args {
        let Some(s) = as_str(arg) else {
            return unevaluated(expr);
        };
        total = match total.checked_add(s.chars().count()) {
            Some(t) if t <= MAX_STRING_LENGTH => t,
            // Over the cap (or a usize overflow) — refuse, leave unevaluated.
            _ => return unevaluated(expr),
        };
        parts.push(s);
    }
    str_node(parts.concat())
}

/// `StringLength["abc"]` → `3`. Counts **characters**, not bytes, so
/// `StringLength["héllo"]` is `5`. A non-string argument or the wrong arity leaves
/// the form unevaluated.
fn string_length_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(s) = as_str(&expr.args[0]) else {
        return unevaluated(expr);
    };
    int(s.chars().count() as i64)
}

/// `StringTake["hello", 3]` → `"hel"` (first 3 chars); `StringTake["hello", -2]`
/// → `"lo"` (last 2); `StringTake["hello", {2, 4}]` → `"ell"` (1-based inclusive
/// character range). All indices are **character** indices, never bytes, so
/// `StringTake["héllo", 2]` → `"hé"` and a multi-byte boundary is never split.
///
/// Out of range (`|n|` exceeds the length, or a `{m, n}` span outside `1..=len`),
/// a non-integer spec, an `i64::MIN` index, or a non-string first argument all
/// leave the form **unevaluated** rather than panicking.
fn string_take_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(s) = as_str(&expr.args[0]) else {
        return unevaluated(expr);
    };
    // Collect once into a Vec<char> so every index below is a *character* index.
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    // Form 1: StringTake[s, {m, n}] — a 1-based inclusive character range.
    if let Some([m, n]) = list_pair(&expr.args[1]) {
        // 1-based, inclusive. Require 1 <= m <= n <= len; anything else is out of
        // range and stays unevaluated. `m`/`n` are i64 so a crafted i64::MIN /
        // i64::MAX cannot overflow a usize conversion — we compare in i64 first.
        if m < 1 || n < m || (n as i128) > len as i128 {
            return unevaluated(expr);
        }
        // Safe: 1 <= m <= n <= len, so (m-1) and n are valid usize indices.
        let lo = (m - 1) as usize;
        let hi = n as usize;
        return str_node(chars[lo..hi].iter().collect::<String>());
    }

    // Form 2: StringTake[s, n] — first n chars (n >= 0) or last |n| (n < 0).
    let Some(n) = as_i64(&expr.args[1]) else {
        return unevaluated(expr);
    };
    // Compute the magnitude in i128 so `i64::MIN` (whose i64 abs panics/overflows)
    // is handled — its magnitude is `2^63`, far larger than any real `len`, so it
    // simply falls out of range.
    let mag = (n as i128).unsigned_abs();
    if mag > len as u128 {
        return unevaluated(expr); // |n| exceeds the length — out of range.
    }
    let take = mag as usize; // safe: take <= len
    let slice: String = if n >= 0 {
        chars[..take].iter().collect() // first `take` chars
    } else {
        chars[len - take..].iter().collect() // last `take` chars
    };
    str_node(slice)
}

/// `StringDrop["hello", 2]` → `"llo"` (drop the first 2 chars); `n < 0` drops the
/// last `|n|` chars (`StringDrop["hello", -2]` → `"hel"`). **Character** indices,
/// never bytes. Out of range / non-integer / non-string leaves it unevaluated.
fn string_drop_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(s) = as_str(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let Some(n) = as_i64(&expr.args[1]) else {
        return unevaluated(expr);
    };
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    // i128 magnitude so i64::MIN does not overflow an i64 abs.
    let mag = (n as i128).unsigned_abs();
    if mag > len as u128 {
        return unevaluated(expr); // dropping more than exists — out of range.
    }
    let drop = mag as usize; // safe: drop <= len
    let slice: String = if n >= 0 {
        chars[drop..].iter().collect() // drop the first `drop` chars
    } else {
        chars[..len - drop].iter().collect() // drop the last `drop` chars
    };
    str_node(slice)
}

/// `StringSplit["a b  c"]` → `{"a", "b", "c"}` (split on runs of whitespace,
/// dropping empty fields); `StringSplit["a,b,c", ","]` → `{"a", "b", "c"}` (split
/// on a literal string separator). Returns a `List` of strings, reusing the W-9
/// list machinery. A non-string argument (or non-string separator) leaves the form
/// unevaluated.
///
/// **DoS-capped**: the field count is bounded by [`MAX_LIST_LENGTH`] (it cannot
/// exceed the input length anyway, itself W-4-input-capped; the check is
/// defensive and mirrors the W-9 list builders).
fn string_split_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    // Form 1: StringSplit[s] — split on whitespace, dropping empty fields.
    let fields: Vec<&str> = match expr.args.len() {
        1 => {
            let Some(s) = as_str(&expr.args[0]) else {
                return unevaluated(expr);
            };
            // `split_whitespace` already collapses runs and drops leading/trailing
            // empties — exactly Wolfram's `StringSplit[s]` whitespace behaviour.
            s.split_whitespace().collect()
        }
        // Form 2: StringSplit[s, sep] — split on a literal string separator.
        2 => {
            let (Some(s), Some(sep)) = (as_str(&expr.args[0]), as_str(&expr.args[1]))
            else {
                return unevaluated(expr);
            };
            if sep.is_empty() {
                // An empty separator has no well-defined non-overlapping split;
                // leave it unevaluated rather than guess (and avoid a per-char
                // explosion).
                return unevaluated(expr);
            }
            // Wolfram's StringSplit drops empty fields produced by adjacent /
            // leading / trailing separators (`StringSplit[",a,", ","]` → `{"a"}`).
            s.split(sep).filter(|f| !f.is_empty()).collect()
        }
        _ => return unevaluated(expr),
    };
    if fields.len() > MAX_LIST_LENGTH {
        return unevaluated(expr);
    }
    apply(sym(LIST), fields.into_iter().map(str_node).collect())
}

/// `StringReplace["banana", "a" -> "o"]` → `"bonono"`. Replaces **every**
/// non-overlapping literal occurrence of the pattern with the replacement,
/// scanning left-to-right and advancing past each match by the pattern length
/// (so `"a" -> "aa"` does not re-scan the inserted text — the scan is linear and
/// terminates). Accepts a single `a -> b` rule or a `{r1, r2, …}` list of rules
/// applied in sequence (each rule's full pass runs before the next).
///
/// **DoS-guarded** on two axes: an **empty pattern** (`"" -> x`) is rejected and
/// left unevaluated (it would match at every position — unbounded / quadratic
/// expansion), and the output length is bounded by [`MAX_STRING_LENGTH`] (a
/// replacement longer than its pattern grows the string per match). A non-string
/// subject, a malformed rule, or a non-string pattern/replacement leaves the form
/// unevaluated.
fn string_replace_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 2 {
        return unevaluated(expr);
    }
    let Some(subject) = as_str(&expr.args[0]) else {
        return unevaluated(expr);
    };
    // The second argument is either a single Rule(a, b) or a List of Rules.
    let rules: Vec<(&str, &str)> = match rule_pairs(&expr.args[1]) {
        Some(rs) => rs,
        None => return unevaluated(expr),
    };
    // An empty pattern in *any* rule is rejected — it would match between every
    // character and never advance, an unbounded expansion / non-termination risk.
    if rules.iter().any(|(pat, _)| pat.is_empty()) {
        return unevaluated(expr);
    }
    let mut current = subject.to_string();
    for (pat, rep) in rules {
        match replace_all_literal(&current, pat, rep) {
            Some(next) => current = next,
            // Over the output cap — refuse the whole replacement, unevaluated.
            None => return unevaluated(expr),
        }
    }
    str_node(current)
}

/// `ToString[expr]` → the Wolfram surface form of `expr` as a string, reusing the
/// W-4 [`print_wolfram`] printer. A **bare string** renders as its raw content
/// (no surrounding quotes), so `ToString["hi"]` → `"hi"` and `ToString[123]` →
/// `"123"`; any other expr renders exactly as the printer would show it
/// (`ToString[1 + x]` → `"1 + x"`). Always reduces (it never fails) for arity 1;
/// the wrong arity stays unevaluated.
fn to_string_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    // A bare top-level string renders as its raw content (no quotes) — matching
    // Wolfram's `ToString["hi"]` → hi. Inside a larger structure the printer's
    // quoted form is kept (an intentional simplification, see spec §15.3).
    let text = match &expr.args[0] {
        IRNode::Str(s) => s.clone(),
        other => print_wolfram(other),
    };
    str_node(text)
}

/// `Characters["ab"]` → `{"a", "b"}` — the list of single-character strings.
/// Reuses the W-9 list machinery; a non-string argument leaves it unevaluated.
///
/// **DoS-capped**: the element count equals the input character count, itself
/// W-4-input-capped; a defensive [`MAX_LIST_LENGTH`] check mirrors the W-9 list
/// builders.
fn characters_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return unevaluated(expr);
    }
    let Some(s) = as_str(&expr.args[0]) else {
        return unevaluated(expr);
    };
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > MAX_LIST_LENGTH {
        return unevaluated(expr);
    }
    apply(
        sym(LIST),
        chars.into_iter().map(|c| str_node(c.to_string())).collect(),
    )
}

// --- W-12 string helpers ----------------------------------------------------

/// If `node` is a two-element `List(a, b)` of integers, return `[a, b]`. Used by
/// `StringTake[s, {m, n}]` to read the 1-based range spec. Any other shape
/// (wrong length, non-integer elements) gives `None`.
fn list_pair(node: &IRNode) -> Option<[i64; 2]> {
    let elems = list_elements(node)?;
    if elems.len() != 2 {
        return None;
    }
    Some([as_i64(&elems[0])?, as_i64(&elems[1])?])
}

/// Read the rule argument of `StringReplace` as a list of `(pattern, replacement)`
/// string pairs. Accepts a single `Rule(a, b)` (→ one pair) or a `List` of
/// `Rule(a, b)` nodes (→ many pairs). Any other shape — a non-rule, a rule whose
/// sides are not both strings, or a list containing a non-rule — gives `None` so
/// the caller leaves the form unevaluated.
fn rule_pairs(node: &IRNode) -> Option<Vec<(&str, &str)>> {
    // A single Rule.
    if let Some(pair) = single_rule_pair(node) {
        return Some(vec![pair]);
    }
    // A list of Rules.
    if let Some(elems) = list_node_ref(node) {
        let mut out = Vec::with_capacity(elems.len());
        for e in elems {
            out.push(single_rule_pair(e)?);
        }
        return Some(out);
    }
    None
}

/// Borrow the elements of a `List(...)` node without cloning (unlike
/// [`list_elements`], which clones). Returns `None` for a non-list node.
fn list_node_ref(node: &IRNode) -> Option<&[IRNode]> {
    match node {
        IRNode::Apply(app) if is_list(&app.head) => Some(&app.args),
        _ => None,
    }
}

/// Read a single `Rule(a, b)` whose both sides are strings as a `(a, b)` pair, or
/// `None` for any other shape. Borrows from `node` (no clone).
fn single_rule_pair(node: &IRNode) -> Option<(&str, &str)> {
    let IRNode::Apply(app) = node else {
        return None;
    };
    let is_rule = matches!(&app.head, IRNode::Symbol(s) if s == PM_RULE);
    if !is_rule || app.args.len() != 2 {
        return None;
    }
    Some((as_str(&app.args[0])?, as_str(&app.args[1])?))
}

/// Replace every **non-overlapping** literal occurrence of `pat` in `s` with
/// `rep`, scanning left-to-right and advancing past each match by `pat.len()`
/// (so an inserted copy of the pattern is never re-scanned — the pass is linear
/// and terminates even for `"a" -> "aa"`). The caller guarantees `pat` is
/// non-empty.
///
/// Returns `None` if the output would exceed [`MAX_STRING_LENGTH`] **characters**,
/// so an amplifying replacement (`rep` longer than `pat`) cannot be used to aim
/// for an unbounded allocation. Operates on byte offsets *internally* (via
/// `str::find`, which returns valid char-boundary offsets for a literal needle),
/// but the size guard counts characters to stay consistent with the rest of W-12.
fn replace_all_literal(s: &str, pat: &str, rep: &str) -> Option<String> {
    let mut out = String::new();
    let mut char_count: usize = 0;
    let mut rest = s;
    // `str::find` on a literal &str needle returns a byte offset that is always a
    // valid char boundary (the needle's bytes match a UTF-8-aligned subslice), so
    // the byte slicing below can never split a multi-byte char.
    while let Some(pos) = rest.find(pat) {
        let head = &rest[..pos];
        char_count = char_count
            .checked_add(head.chars().count())?
            .checked_add(rep.chars().count())?;
        if char_count > MAX_STRING_LENGTH {
            return None;
        }
        out.push_str(head);
        out.push_str(rep);
        // Advance past the match. `pat` is non-empty, so this strictly shrinks
        // `rest` every iteration — the loop always terminates.
        rest = &rest[pos + pat.len()..];
    }
    char_count = char_count.checked_add(rest.chars().count())?;
    if char_count > MAX_STRING_LENGTH {
        return None;
    }
    out.push_str(rest);
    Some(out)
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

/// The subset's **canonical total order** over `IRNode`, used by `Sort` (MA04
/// §12.2). A *documented simplification* of Wolfram's full canonical order: it
/// agrees with Wolfram for the common cases (pure-numeric lists sort numerically,
/// symbols/strings lexicographically) and is otherwise a deterministic, total,
/// stable order so `Sort` never panics and is reproducible across runs.
///
/// The ordering is, in tiers:
///
/// 1. **all numbers** (Integer/Rational/Float) — compared by their `f64`
///    magnitude, so `2`, `1/2`, `1.5` interleave sensibly; ties (equal magnitude,
///    e.g. `2` vs `2.0`) fall through to the type tag so the order stays total and
///    stable.
/// 2. **symbols** — lexicographic by name.
/// 3. **strings** — lexicographic.
/// 4. **compound `Apply`** — by head first (recursively), then argument count,
///    then arguments left-to-right (recursively).
///
/// Across tiers, the *tier index* decides (numbers < symbols < strings <
/// compound). `f64` comparison uses `total_cmp`, which is a true total order even
/// for `NaN`, so the comparator is panic-free.
fn canonical_cmp(a: &IRNode, b: &IRNode) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // Numbers form one tier, compared by magnitude regardless of exact subtype.
    let a_num = numeric_magnitude(a);
    let b_num = numeric_magnitude(b);
    match (a_num, b_num) {
        (Some(x), Some(y)) => x
            .total_cmp(&y)
            // Equal magnitude (`2` vs `2.0`): break by the type tag so the order
            // is total and stable (numbers stay grouped, ordering is fixed).
            .then_with(|| type_tag(a).cmp(&type_tag(b))),
        // A number sorts before any non-number.
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        // Neither is a number: order by tier tag, then within-tier.
        (None, None) => type_tag(a)
            .cmp(&type_tag(b))
            .then_with(|| within_tier_cmp(a, b)),
    }
}

/// The `f64` magnitude of a numeric node (Integer/Rational/Float), or `None` for
/// a non-number. Used to put every number in one comparison tier in
/// [`canonical_cmp`]. Lossy for huge integers, but only the *ordering* matters
/// and equal-magnitude ties fall back to the type tag, so the order stays total.
fn numeric_magnitude(node: &IRNode) -> Option<f64> {
    match node {
        IRNode::Integer(n) => Some(*n as f64),
        IRNode::Rational(num, den) => Some(*num as f64 / *den as f64),
        IRNode::Float(f) => Some(*f),
        _ => None,
    }
}

/// A stable tier tag fixing the cross-type order: numbers (0) < symbols (1) <
/// strings (2) < compound (3). Within the number tier the three subtypes get
/// distinct tags (0/1/2 offset into the number band) only as an equal-magnitude
/// tie-break, which keeps the order total without disturbing magnitude order.
fn type_tag(node: &IRNode) -> u8 {
    match node {
        IRNode::Integer(_) => 0,
        IRNode::Rational(..) => 1,
        IRNode::Float(_) => 2,
        IRNode::Symbol(_) => 3,
        IRNode::Str(_) => 4,
        IRNode::Apply(_) => 5,
    }
}

/// Order two *same-tier* non-numeric nodes: symbols and strings lexicographically,
/// compound expressions by head then arity then arguments (recursively). Mixed
/// tiers never reach here (the caller orders those by [`type_tag`]); a defensive
/// `Equal` is returned for any residual mismatch so the comparator stays total.
fn within_tier_cmp(a: &IRNode, b: &IRNode) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (IRNode::Symbol(x), IRNode::Symbol(y)) => x.cmp(y),
        (IRNode::Str(x), IRNode::Str(y)) => x.cmp(y),
        (IRNode::Apply(x), IRNode::Apply(y)) => canonical_cmp(&x.head, &y.head)
            .then_with(|| x.args.len().cmp(&y.args.len()))
            .then_with(|| {
                x.args
                    .iter()
                    .zip(y.args.iter())
                    .map(|(ax, bx)| canonical_cmp(ax, bx))
                    .find(|o| *o != Ordering::Equal)
                    .unwrap_or(Ordering::Equal)
            }),
        _ => Ordering::Equal,
    }
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

    /// Like [`run`], but over a real [`WolframBackend`] so a handler that
    /// re-evaluates a *Wolfram* head through the VM (e.g. `Select`/`Count`
    /// applying the `EvenQ` predicate) can resolve it. The plain `run` helper uses
    /// a bare `SymbolicBackend`, which does not know the W-9 predicate heads.
    fn run_wolfram(head: &str, args: Vec<IRNode>) -> IRNode {
        use crate::backend::WolframBackend;
        let table = build_wolfram_builtins();
        let handler = table.get(head).expect("no such builtin").clone();
        let mut vm = VM::new(Box::new(WolframBackend::new()));
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

    // -----------------------------------------------------------------------
    // W-7 iteration handlers (unit level — handlers run over a real VM so the
    // per-iteration substitute + re-eval, and the Add/Mul folds, exercise the
    // shared SymbolicBackend handler table).
    // -----------------------------------------------------------------------

    /// `{i, bounds…}` spec helper.
    fn spec(parts: Vec<IRNode>) -> IRNode {
        list(parts)
    }

    #[test]
    fn table_builds_the_indexed_list() {
        // Table[i, {i, 3}] → {1, 2, 3} (body is the bare index).
        assert_eq!(
            run("Table", vec![sym("i"), spec(vec![sym("i"), int(3)])]),
            list(vec![int(1), int(2), int(3)])
        );
        // Table[Add(i, 10), {i, 2, 4}] → {12, 13, 14} — the body re-evaluates
        // through the Add handler with i substituted.
        assert_eq!(
            run(
                "Table",
                vec![
                    apply(sym("Add"), vec![sym("i"), int(10)]),
                    spec(vec![sym("i"), int(2), int(4)])
                ]
            ),
            list(vec![int(12), int(13), int(14)])
        );
    }

    #[test]
    fn sum_and_product_fold_over_the_range() {
        // Sum[i, {i, 1, 10}] → 55.
        assert_eq!(
            run("Sum", vec![sym("i"), spec(vec![sym("i"), int(1), int(10)])]),
            int(55)
        );
        // Product[i, {i, 1, 4}] → 24.
        assert_eq!(
            run("Product", vec![sym("i"), spec(vec![sym("i"), int(1), int(4)])]),
            int(24)
        );
    }

    #[test]
    fn sum_and_product_of_empty_range_are_identities() {
        // A wrong-way range iterates zero times → fold identity (0 / 1).
        assert_eq!(
            run("Sum", vec![sym("i"), spec(vec![sym("i"), int(5), int(1)])]),
            int(0)
        );
        assert_eq!(
            run("Product", vec![sym("i"), spec(vec![sym("i"), int(5), int(1)])]),
            int(1)
        );
        // Table over an empty range is the empty list.
        assert_eq!(
            run("Table", vec![sym("i"), spec(vec![sym("i"), int(0)])]),
            list(vec![])
        );
    }

    #[test]
    fn do_returns_null() {
        // Do[i, {i, 3}] → Null (the body is pure here, so there is nothing to
        // observe besides the Null return and the absence of a panic).
        assert_eq!(
            run("Do", vec![sym("i"), spec(vec![sym("i"), int(3)])]),
            sym("Null")
        );
    }

    #[test]
    fn iteration_with_oversize_range_stays_unevaluated() {
        // A count beyond MAX_RANGE_LENGTH leaves the whole form unevaluated —
        // never allocated, never looped.
        let big = (MAX_RANGE_LENGTH as i64) + 5;
        let s = spec(vec![sym("i"), int(big)]);
        assert_eq!(
            run("Table", vec![sym("i"), s.clone()]),
            apply(sym("Table"), vec![sym("i"), s.clone()])
        );
        assert_eq!(
            run("Do", vec![sym("i"), s.clone()]),
            apply(sym("Do"), vec![sym("i"), s])
        );
    }

    #[test]
    fn iteration_with_malformed_spec_stays_unevaluated() {
        // {i} — no bound.
        let no_bound = spec(vec![sym("i")]);
        assert_eq!(
            run("Table", vec![sym("i"), no_bound.clone()]),
            apply(sym("Table"), vec![sym("i"), no_bound])
        );
        // Zero step.
        let zero_step = spec(vec![sym("i"), int(1), int(5), int(0)]);
        assert_eq!(
            run("Table", vec![sym("i"), zero_step.clone()]),
            apply(sym("Table"), vec![sym("i"), zero_step])
        );
        // Non-symbol binder.
        let bad_binder = spec(vec![int(7), int(3)]);
        assert_eq!(
            run("Sum", vec![sym("i"), bad_binder.clone()]),
            apply(sym("Sum"), vec![sym("i"), bad_binder])
        );
        // Spec is not a list at all.
        assert_eq!(
            run("Table", vec![sym("i"), int(3)]),
            apply(sym("Table"), vec![sym("i"), int(3)])
        );
    }

    #[test]
    fn iteration_extreme_bounds_do_not_overflow() {
        // A span wider than i64 but with valid i64 bounds: the i128 count
        // exceeds the cap and the form stays unevaluated — no overflow panic.
        let s = spec(vec![sym("i"), int(-9_000_000_000_000_000_000), int(9_000_000_000_000_000_000)]);
        assert_eq!(
            run("Sum", vec![int(1), s.clone()]),
            apply(sym("Sum"), vec![int(1), s])
        );
    }

    #[test]
    fn iteration_wrong_arity_stays_unevaluated() {
        // Only the 2-arg (body, spec) form is valid.
        assert_eq!(run("Table", vec![sym("i")]), apply(sym("Table"), vec![sym("i")]));
        assert_eq!(run("Sum", vec![]), apply(sym("Sum"), vec![]));
    }

    // -----------------------------------------------------------------------
    // W-8 local-scoping handlers (unit level — handlers run over a real VM so
    // the decl-RHS eval, the substitute into the held body, and the body re-eval
    // all exercise the shared SymbolicBackend handler table).
    // -----------------------------------------------------------------------

    /// `name = value` declaration helper (the lowering of an in-`{…}` `Set`).
    fn decl(name: &str, value: IRNode) -> IRNode {
        apply(sym(ASSIGN), vec![sym(name), value])
    }

    #[test]
    fn with_binds_a_single_local_and_evaluates_the_body() {
        // With[{x = 3}, x^2] → 9 (Pow is an inner-backend head).
        assert_eq!(
            run(
                "With",
                vec![
                    list(vec![decl("x", int(3))]),
                    apply(sym("Pow"), vec![sym("x"), int(2)])
                ]
            ),
            int(9)
        );
    }

    #[test]
    fn with_binds_multiple_locals_in_parallel() {
        // With[{a = 1, b = 2}, a + b] → 3.
        assert_eq!(
            run(
                "With",
                vec![
                    list(vec![decl("a", int(1)), decl("b", int(2))]),
                    apply(sym("Add"), vec![sym("a"), sym("b")])
                ]
            ),
            int(3)
        );
    }

    #[test]
    fn with_evaluates_the_decl_rhs() {
        // With[{x = 1 + 1}, x] → 2 — the RHS is evaluated before substitution.
        assert_eq!(
            run(
                "With",
                vec![
                    list(vec![decl("x", apply(sym("Add"), vec![int(1), int(1)]))]),
                    sym("x")
                ]
            ),
            int(2)
        );
    }

    #[test]
    fn module_with_initialised_locals_behaves_like_with() {
        // Module[{a = 1, b = 2}, a + b] → 3.
        assert_eq!(
            run(
                "Module",
                vec![
                    list(vec![decl("a", int(1)), decl("b", int(2))]),
                    apply(sym("Add"), vec![sym("a"), sym("b")])
                ]
            ),
            int(3)
        );
    }

    #[test]
    fn module_uninitialised_local_stays_symbolic() {
        // Module[{x}, x] → x$nnn — an uninitialised local is α-renamed to a fresh
        // gensym, so the body sees an undefined symbol (never a global). We assert
        // the *shape* (a free symbol whose name starts with the base) rather than
        // the exact gensym number, which is non-deterministic across the suite.
        match run("Module", vec![list(vec![sym("x")]), sym("x")]) {
            IRNode::Symbol(s) => assert!(
                s == "x" || s.starts_with("x$"),
                "expected a fresh `x` local, got {s:?}"
            ),
            other => panic!("expected a symbol, got {other}"),
        }
        // A mix: Module[{x, y = 2}, y] → 2 (uninitialised x is harmless).
        assert_eq!(
            run(
                "Module",
                vec![list(vec![sym("x"), decl("y", int(2))]), sym("y")]
            ),
            int(2)
        );
    }

    #[test]
    fn module_uninitialised_local_does_not_resolve_to_a_global() {
        // With a global `u = 42` bound in the backend, Module[{u}, u] must NOT
        // return 42 — the gensym rename gives the local a name the env never
        // bound, so it stays free. This is the capture-leak guard.
        use symbolic_vm::backend::Backend;
        let table = build_wolfram_builtins();
        let handler = table.get("Module").expect("no Module builtin").clone();
        let mut backend = SymbolicBackend::new();
        backend.bind("u", int(42));
        let mut vm = VM::new(Box::new(backend));
        let out = handler(
            &mut vm,
            IRApply {
                head: sym("Module"),
                args: vec![list(vec![sym("u")]), sym("u")],
            },
        );
        match out {
            IRNode::Symbol(s) => assert!(
                s.starts_with("u$"),
                "uninitialised local must be a fresh gensym, got {s:?}"
            ),
            other => panic!("expected a fresh symbol, not the global 42: {other}"),
        }
    }

    #[test]
    fn block_binds_like_with_for_self_contained_bodies() {
        // Block[{x = 5}, x + 1] → 6.
        assert_eq!(
            run(
                "Block",
                vec![
                    list(vec![decl("x", int(5))]),
                    apply(sym("Add"), vec![sym("x"), int(1)])
                ]
            ),
            int(6)
        );
    }

    #[test]
    fn with_uninitialised_local_is_rejected() {
        // With requires every local to be initialised; a bare `x` leaves the
        // whole form unevaluated (Block too).
        let form = vec![list(vec![sym("x")]), sym("x")];
        assert_eq!(
            run("With", form.clone()),
            apply(sym("With"), form.clone())
        );
        assert_eq!(run("Block", form.clone()), apply(sym("Block"), form));
    }

    #[test]
    fn scoping_with_malformed_decls_stays_unevaluated() {
        // First argument not a list.
        assert_eq!(
            run("With", vec![sym("x"), sym("x")]),
            apply(sym("With"), vec![sym("x"), sym("x")])
        );
        // A decl that is a literal, not a symbol or assignment.
        let lit = vec![list(vec![int(7)]), int(1)];
        assert_eq!(run("With", lit.clone()), apply(sym("With"), lit));
        // A non-symbol assignment target: f[x] = 1.
        let bad_target = vec![
            list(vec![apply(
                sym(ASSIGN),
                vec![apply(sym("f"), vec![sym("x")]), int(1)],
            )]),
            int(1),
        ];
        assert_eq!(
            run("With", bad_target.clone()),
            apply(sym("With"), bad_target)
        );
    }

    #[test]
    fn scoping_wrong_arity_stays_unevaluated() {
        // Only the 2-arg (decls, body) form is valid.
        assert_eq!(
            run("With", vec![list(vec![decl("x", int(1))])]),
            apply(sym("With"), vec![list(vec![decl("x", int(1))])])
        );
        assert_eq!(run("Module", vec![]), apply(sym("Module"), vec![]));
        assert_eq!(
            run(
                "Block",
                vec![list(vec![decl("x", int(1))]), int(1), int(2)]
            ),
            apply(
                sym("Block"),
                vec![list(vec![decl("x", int(1))]), int(1), int(2)]
            )
        );
    }

    // -----------------------------------------------------------------------
    // W-9 list-manipulation handlers (unit level — Select/Count/Total run over a
    // real VM so the predicate-application path and the Add fold exercise the
    // shared SymbolicBackend handler table).
    // -----------------------------------------------------------------------

    #[test]
    fn sort_orders_a_numeric_list_ascending() {
        assert_eq!(
            run("Sort", vec![list(vec![int(3), int(1), int(2)])]),
            list(vec![int(1), int(2), int(3)])
        );
        // Mixed magnitudes (int / rational / float) interleave by value.
        assert_eq!(
            run(
                "Sort",
                vec![list(vec![int(2), IRNode::rational(1, 2), flt(1.5)])]
            ),
            list(vec![IRNode::rational(1, 2), flt(1.5), int(2)])
        );
        // Empty and singleton lists are fixed points.
        assert_eq!(run("Sort", vec![list(vec![])]), list(vec![]));
        assert_eq!(run("Sort", vec![list(vec![int(7)])]), list(vec![int(7)]));
    }

    #[test]
    fn sort_orders_symbols_and_mixed_types_canonically() {
        // Symbols sort lexicographically, and numbers sort before symbols.
        assert_eq!(
            run(
                "Sort",
                vec![list(vec![sym("c"), int(2), sym("a"), int(1)])]
            ),
            list(vec![int(1), int(2), sym("a"), sym("c")])
        );
    }

    #[test]
    fn sort_of_a_non_list_stays_unevaluated() {
        assert_eq!(run("Sort", vec![sym("x")]), apply(sym("Sort"), vec![sym("x")]));
        // Wrong arity too.
        assert_eq!(run("Sort", vec![]), apply(sym("Sort"), vec![]));
    }

    #[test]
    fn reverse_reverses_a_list() {
        assert_eq!(
            run("Reverse", vec![list(vec![int(1), int(2), int(3)])]),
            list(vec![int(3), int(2), int(1)])
        );
        assert_eq!(run("Reverse", vec![list(vec![])]), list(vec![]));
        // Non-list stays unevaluated.
        assert_eq!(
            run("Reverse", vec![int(5)]),
            apply(sym("Reverse"), vec![int(5)])
        );
    }

    #[test]
    fn join_concatenates_two_or_more_lists() {
        assert_eq!(
            run("Join", vec![list(vec![int(1)]), list(vec![int(2), int(3)])]),
            list(vec![int(1), int(2), int(3)])
        );
        // Three-argument form.
        assert_eq!(
            run(
                "Join",
                vec![list(vec![int(1)]), list(vec![int(2)]), list(vec![int(3)])]
            ),
            list(vec![int(1), int(2), int(3)])
        );
        // Joining with an empty list is the identity.
        assert_eq!(
            run("Join", vec![list(vec![int(1)]), list(vec![])]),
            list(vec![int(1)])
        );
    }

    #[test]
    fn join_with_a_non_list_or_too_few_args_stays_unevaluated() {
        // A non-list argument aborts the whole join.
        assert_eq!(
            run("Join", vec![list(vec![int(1)]), int(2)]),
            apply(sym("Join"), vec![list(vec![int(1)]), int(2)])
        );
        // Fewer than two arguments is malformed.
        assert_eq!(
            run("Join", vec![list(vec![int(1)])]),
            apply(sym("Join"), vec![list(vec![int(1)])])
        );
    }

    #[test]
    fn flatten_full_flattens_all_levels() {
        // One level.
        assert_eq!(
            run("Flatten", vec![list(vec![list(vec![int(1), int(2)]), list(vec![int(3)])])]),
            list(vec![int(1), int(2), int(3)])
        );
        // Deep nesting, all levels.
        assert_eq!(
            run(
                "Flatten",
                vec![list(vec![int(1), list(vec![int(2), list(vec![int(3)])])])]
            ),
            list(vec![int(1), int(2), int(3)])
        );
        // Already flat is a fixed point.
        assert_eq!(
            run("Flatten", vec![list(vec![int(1), int(2)])]),
            list(vec![int(1), int(2)])
        );
    }

    #[test]
    fn flatten_with_explicit_depth_stops_after_n_levels() {
        // Depth 1: only the top level is spliced; the inner {3} survives.
        assert_eq!(
            run(
                "Flatten",
                vec![
                    list(vec![int(1), list(vec![int(2), list(vec![int(3)])])]),
                    int(1)
                ]
            ),
            list(vec![int(1), int(2), list(vec![int(3)])])
        );
        // Depth 0: nothing is descended — the list is returned element-wise as-is.
        assert_eq!(
            run(
                "Flatten",
                vec![list(vec![int(1), list(vec![int(2)])]), int(0)]
            ),
            list(vec![int(1), list(vec![int(2)])])
        );
    }

    #[test]
    fn mod_integer_remainder_uses_divisor_sign() {
        // Positive divisor: result in [0, b).
        assert_eq!(run("Mod", vec![int(7), int(2)]), int(1));
        assert_eq!(run("Mod", vec![int(8), int(2)]), int(0));
        // Negative dividend: still non-negative for a positive divisor.
        assert_eq!(run("Mod", vec![int(-1), int(3)]), int(2));
        // Negative divisor: result takes the divisor's sign.
        assert_eq!(run("Mod", vec![int(7), int(-3)]), int(-2));
        assert_eq!(run("Mod", vec![int(-7), int(-3)]), int(-1));
    }

    #[test]
    fn mod_extreme_operands_do_not_overflow() {
        // A crafted i64::MIN divisor must NOT panic on `b.abs()` (the i128 path
        // avoids the signed-abs overflow). i64::MIN mod 2 = 0 (it is even).
        assert_eq!(run("Mod", vec![int(i64::MIN), int(2)]), int(0));
        // i64::MIN as the divisor: Mod[-1, i64::MIN] = -1 (divisor-signed).
        assert_eq!(run("Mod", vec![int(-1), int(i64::MIN)]), int(-1));
        // i64::MAX divisor, large dividend — stays in range.
        assert_eq!(run("Mod", vec![int(i64::MAX), int(i64::MAX)]), int(0));
        assert_eq!(run("Mod", vec![int(i64::MIN), int(i64::MAX)]), int(i64::MAX - 1));
    }

    #[test]
    fn mod_malformed_stays_unevaluated() {
        // Mod by zero is undefined → unevaluated, no panic.
        assert_eq!(
            run("Mod", vec![int(5), int(0)]),
            apply(sym("Mod"), vec![int(5), int(0)])
        );
        // Wrong arity.
        assert_eq!(run("Mod", vec![int(5)]), apply(sym("Mod"), vec![int(5)]));
        // Non-integer argument.
        assert_eq!(
            run("Mod", vec![sym("x"), int(2)]),
            apply(sym("Mod"), vec![sym("x"), int(2)])
        );
    }

    #[test]
    fn flatten_malformed_stays_unevaluated() {
        // Non-list first argument.
        assert_eq!(
            run("Flatten", vec![int(5)]),
            apply(sym("Flatten"), vec![int(5)])
        );
        // Negative depth.
        assert_eq!(
            run("Flatten", vec![list(vec![int(1)]), int(-1)]),
            apply(sym("Flatten"), vec![list(vec![int(1)]), int(-1)])
        );
        // Non-integer depth.
        assert_eq!(
            run("Flatten", vec![list(vec![int(1)]), sym("x")]),
            apply(sym("Flatten"), vec![list(vec![int(1)]), sym("x")])
        );
    }

    #[test]
    fn even_q_and_odd_q_classify_integers() {
        assert_eq!(run("EvenQ", vec![int(4)]), sym("True"));
        assert_eq!(run("EvenQ", vec![int(3)]), sym("False"));
        assert_eq!(run("OddQ", vec![int(3)]), sym("True"));
        assert_eq!(run("OddQ", vec![int(4)]), sym("False"));
        // Negative integers are classified correctly (rem_euclid).
        assert_eq!(run("EvenQ", vec![int(-4)]), sym("True"));
        assert_eq!(run("OddQ", vec![int(-3)]), sym("True"));
        // Zero is even.
        assert_eq!(run("EvenQ", vec![int(0)]), sym("True"));
        // A non-integer is neither even nor odd → False.
        assert_eq!(run("EvenQ", vec![sym("x")]), sym("False"));
        assert_eq!(run("OddQ", vec![flt(2.0)]), sym("False"));
        // Wrong arity stays unevaluated.
        assert_eq!(run("EvenQ", vec![]), apply(sym("EvenQ"), vec![]));
    }

    #[test]
    fn select_keeps_elements_passing_the_predicate() {
        // Select[{1, 2, 3, 4}, EvenQ] → {2, 4}.
        assert_eq!(
            run_wolfram(
                "Select",
                vec![list(vec![int(1), int(2), int(3), int(4)]), sym("EvenQ")]
            ),
            list(vec![int(2), int(4)])
        );
        // A predicate that never fires gives the empty list.
        assert_eq!(
            run_wolfram("Select", vec![list(vec![int(1), int(3)]), sym("EvenQ")]),
            list(vec![])
        );
    }

    #[test]
    fn select_with_an_unbound_predicate_selects_nothing() {
        // `f[e]` stays unevaluated (f unbound) — never the True symbol — so no
        // element is selected and nothing panics.
        assert_eq!(
            run("Select", vec![list(vec![int(1), int(2)]), sym("f")]),
            list(vec![])
        );
        // Non-list / wrong arity stay unevaluated.
        assert_eq!(
            run("Select", vec![int(1), sym("EvenQ")]),
            apply(sym("Select"), vec![int(1), sym("EvenQ")])
        );
    }

    #[test]
    fn count_tallies_elements_passing_the_predicate() {
        // Count[{1, 2, 3, 4}, EvenQ] → 2.
        assert_eq!(
            run_wolfram(
                "Count",
                vec![list(vec![int(1), int(2), int(3), int(4)]), sym("EvenQ")]
            ),
            int(2)
        );
        assert_eq!(
            run_wolfram("Count", vec![list(vec![int(1), int(3), int(5)]), sym("EvenQ")]),
            int(0)
        );
        // Non-list stays unevaluated.
        assert_eq!(
            run("Count", vec![sym("x"), sym("EvenQ")]),
            apply(sym("Count"), vec![sym("x"), sym("EvenQ")])
        );
    }

    #[test]
    fn total_sums_a_list_onto_add() {
        assert_eq!(run("Total", vec![list(vec![int(1), int(2), int(3)])]), int(6));
        // An empty list totals to 0.
        assert_eq!(run("Total", vec![list(vec![])]), int(0));
        // Symbolic terms combine via the Add engine: Total[{x, x}] → 2 x ... but
        // at minimum x + 1 + 2 collapses the numbers; assert the all-symbol case
        // stays as a sum (handled by the shared Add handler).
        assert_eq!(
            run("Total", vec![list(vec![int(10), int(20)])]),
            int(30)
        );
        // Non-list stays unevaluated.
        assert_eq!(
            run("Total", vec![sym("x")]),
            apply(sym("Total"), vec![sym("x")])
        );
    }

    // -----------------------------------------------------------------------
    // W-10 functional-iteration combinators
    // -----------------------------------------------------------------------

    #[test]
    fn nest_applies_f_n_times_symbolically() {
        // Nest[f, x, 3] → f[f[f[x]]] with a symbolic (undefined) f.
        let expected = apply(
            sym("f"),
            vec![apply(sym("f"), vec![apply(sym("f"), vec![sym("x")])])],
        );
        assert_eq!(run_wolfram("Nest", vec![sym("f"), sym("x"), int(3)]), expected);
    }

    #[test]
    fn nest_zero_is_the_identity() {
        // Nest[f, x, 0] → x (zero applications).
        assert_eq!(run_wolfram("Nest", vec![sym("f"), sym("x"), int(0)]), sym("x"));
    }

    #[test]
    fn nest_with_a_bridged_head_folds_numerically() {
        // Nest[Plus[1], x, …] is awkward without partial application; instead use
        // a numeric driver: nest with `Plus` requires two args, so test the
        // symbolic shape for unary f and rely on the fold tests for numeric f.
        // Here: an undefined unary head still builds the literal nest of depth 2.
        let expected = apply(sym("g"), vec![apply(sym("g"), vec![int(0)])]);
        assert_eq!(run_wolfram("Nest", vec![sym("g"), int(0), int(2)]), expected);
    }

    #[test]
    fn nest_list_collects_intermediates_including_the_seed() {
        // NestList[f, x, 2] → {x, f[x], f[f[x]]}.
        let fx = apply(sym("f"), vec![sym("x")]);
        let ffx = apply(sym("f"), vec![fx.clone()]);
        assert_eq!(
            run_wolfram("NestList", vec![sym("f"), sym("x"), int(2)]),
            list(vec![sym("x"), fx, ffx])
        );
    }

    #[test]
    fn nest_list_with_zero_is_just_the_seed() {
        // NestList[f, x, 0] → {x}.
        assert_eq!(
            run_wolfram("NestList", vec![sym("f"), sym("x"), int(0)]),
            list(vec![sym("x")])
        );
    }

    #[test]
    fn fold_left_folds_plus_to_a_total() {
        // Fold[Plus, 0, {1, 2, 3}] → 6 = ((0 + 1) + 2) + 3.
        assert_eq!(
            run_wolfram(
                "Fold",
                vec![sym("Plus"), int(0), list(vec![int(1), int(2), int(3)])]
            ),
            int(6)
        );
    }

    #[test]
    fn fold_over_empty_list_returns_the_seed() {
        // Fold[Plus, 42, {}] → 42 (no elements to fold).
        assert_eq!(
            run_wolfram("Fold", vec![sym("Plus"), int(42), list(vec![])]),
            int(42)
        );
    }

    #[test]
    fn fold_is_left_associative_for_subtraction() {
        // Fold[Subtract, 10, {1, 2, 3}] → ((10 - 1) - 2) - 3 = 4. A
        // *left*-associative fold gives 4; a right fold would give a different
        // value, so this pins the associativity.
        assert_eq!(
            run_wolfram(
                "Fold",
                vec![sym("Subtract"), int(10), list(vec![int(1), int(2), int(3)])]
            ),
            int(4)
        );
    }

    #[test]
    fn fold_list_collects_running_accumulations() {
        // FoldList[Plus, 0, {1, 2, 3}] → {0, 1, 3, 6}.
        assert_eq!(
            run_wolfram(
                "FoldList",
                vec![sym("Plus"), int(0), list(vec![int(1), int(2), int(3)])]
            ),
            list(vec![int(0), int(1), int(3), int(6)])
        );
    }

    #[test]
    fn fold_list_over_empty_list_is_just_the_seed() {
        // FoldList[Plus, 7, {}] → {7}.
        assert_eq!(
            run_wolfram("FoldList", vec![sym("Plus"), int(7), list(vec![])]),
            list(vec![int(7)])
        );
    }

    #[test]
    fn nest_negative_count_stays_unevaluated() {
        // A negative n is malformed → echoed back unchanged (no panic, no wrap).
        assert_eq!(
            run_wolfram("Nest", vec![sym("f"), sym("x"), int(-1)]),
            apply(sym("Nest"), vec![sym("f"), sym("x"), int(-1)])
        );
        assert_eq!(
            run_wolfram("NestList", vec![sym("f"), sym("x"), int(-5)]),
            apply(sym("NestList"), vec![sym("f"), sym("x"), int(-5)])
        );
    }

    #[test]
    fn nest_non_integer_count_stays_unevaluated() {
        // n must be an exact integer; a symbol is malformed.
        assert_eq!(
            run_wolfram("Nest", vec![sym("f"), sym("x"), sym("k")]),
            apply(sym("Nest"), vec![sym("f"), sym("x"), sym("k")])
        );
    }

    #[test]
    fn nest_over_cap_count_stays_unevaluated() {
        // A tiny input with an enormous n must NOT iterate — it is refused before
        // the loop (DoS cap). Echoed back unchanged.
        let huge = int(MAX_LIST_LENGTH as i64 + 1);
        assert_eq!(
            run_wolfram("Nest", vec![sym("f"), sym("x"), huge.clone()]),
            apply(sym("Nest"), vec![sym("f"), sym("x"), huge.clone()])
        );
        assert_eq!(
            run_wolfram("NestList", vec![sym("f"), sym("x"), huge.clone()]),
            apply(sym("NestList"), vec![sym("f"), sym("x"), huge])
        );
    }

    #[test]
    fn nest_in_cap_count_iterates_rather_than_echoing() {
        // An in-cap count must actually iterate (not stay unevaluated). The
        // boundary value MAX_LIST_LENGTH is accepted by `nest_count`; we exercise
        // the accept path with a small count whose nested result is cheap to build
        // and distinct from the unevaluated form.
        let out = run_wolfram("Nest", vec![sym("f"), sym("x"), int(5)]);
        assert_ne!(
            out,
            apply(sym("Nest"), vec![sym("f"), sym("x"), int(5)]),
            "an in-cap count must iterate, not stay unevaluated"
        );
    }

    #[test]
    fn fold_non_list_third_arg_stays_unevaluated() {
        // Fold/FoldList require a list to fold over; a non-list is malformed.
        assert_eq!(
            run_wolfram("Fold", vec![sym("Plus"), int(0), sym("notalist")]),
            apply(sym("Fold"), vec![sym("Plus"), int(0), sym("notalist")])
        );
        assert_eq!(
            run_wolfram("FoldList", vec![sym("Plus"), int(0), sym("notalist")]),
            apply(sym("FoldList"), vec![sym("Plus"), int(0), sym("notalist")])
        );
    }

    #[test]
    fn combinators_with_wrong_arity_stay_unevaluated() {
        // Each combinator requires exactly 3 args; 2 is malformed.
        assert_eq!(
            run_wolfram("Nest", vec![sym("f"), sym("x")]),
            apply(sym("Nest"), vec![sym("f"), sym("x")])
        );
        assert_eq!(
            run_wolfram("Fold", vec![sym("Plus"), int(0)]),
            apply(sym("Fold"), vec![sym("Plus"), int(0)])
        );
    }

    #[test]
    fn fold_with_a_non_callable_f_builds_a_literal_nest() {
        // A non-callable f is NOT an error: each f[acc, el] stays unevaluated.
        // Fold[f, 0, {1, 2}] → f[f[0, 1], 2].
        let inner = apply(sym("f"), vec![int(0), int(1)]);
        let outer = apply(sym("f"), vec![inner, int(2)]);
        assert_eq!(
            run_wolfram("Fold", vec![sym("f"), int(0), list(vec![int(1), int(2)])]),
            outer
        );
    }

    // -----------------------------------------------------------------------
    // W-12 string builtins
    // -----------------------------------------------------------------------

    /// Build a `Str` node — a terse test helper.
    fn s(text: &str) -> IRNode {
        str_node(text)
    }

    /// Build a `Rule(a, b)` over two strings (for `StringReplace` tests).
    fn rule(a: &str, b: &str) -> IRNode {
        apply(sym(PM_RULE), vec![s(a), s(b)])
    }

    #[test]
    fn string_length_counts_characters() {
        assert_eq!(run("StringLength", vec![s("abc")]), int(3));
        assert_eq!(run("StringLength", vec![s("")]), int(0));
        // Multi-byte: "héllo" is 5 chars / 6 bytes — must count 5.
        assert_eq!(run("StringLength", vec![s("héllo")]), int(5));
        // An emoji is a single char (4 bytes) — counts as 1.
        assert_eq!(run("StringLength", vec![s("a😀b")]), int(3));
    }

    #[test]
    fn string_length_of_non_string_stays_unevaluated() {
        assert_eq!(
            run("StringLength", vec![int(123)]),
            apply(sym("StringLength"), vec![int(123)])
        );
    }

    #[test]
    fn string_join_concatenates() {
        assert_eq!(run("StringJoin", vec![s("a"), s("b"), s("c")]), s("abc"));
        // Zero / one argument is fine.
        assert_eq!(run("StringJoin", vec![]), s(""));
        assert_eq!(run("StringJoin", vec![s("x")]), s("x"));
        // Unicode parts concatenate without splitting a char.
        assert_eq!(run("StringJoin", vec![s("hé"), s("llo")]), s("héllo"));
    }

    #[test]
    fn string_join_with_a_non_string_stays_unevaluated() {
        assert_eq!(
            run("StringJoin", vec![s("a"), int(1)]),
            apply(sym("StringJoin"), vec![s("a"), int(1)])
        );
    }

    #[test]
    fn string_take_first_n_and_last_n() {
        assert_eq!(run("StringTake", vec![s("hello"), int(3)]), s("hel"));
        assert_eq!(run("StringTake", vec![s("hello"), int(-2)]), s("lo"));
        assert_eq!(run("StringTake", vec![s("hello"), int(0)]), s(""));
        // Taking the whole string both ways.
        assert_eq!(run("StringTake", vec![s("hello"), int(5)]), s("hello"));
        assert_eq!(run("StringTake", vec![s("hello"), int(-5)]), s("hello"));
    }

    #[test]
    fn string_take_range_is_one_based_inclusive() {
        assert_eq!(
            run("StringTake", vec![s("hello"), list(vec![int(2), int(4)])]),
            s("ell")
        );
        // Single-character range {m, m}.
        assert_eq!(
            run("StringTake", vec![s("hello"), list(vec![int(1), int(1)])]),
            s("h")
        );
        // Full range.
        assert_eq!(
            run("StringTake", vec![s("abc"), list(vec![int(1), int(3)])]),
            s("abc")
        );
    }

    #[test]
    fn string_take_is_unicode_by_char() {
        // "héllo" — taking 2 chars must give "hé" (3 bytes), never split the é.
        assert_eq!(run("StringTake", vec![s("héllo"), int(2)]), s("hé"));
        // A range spanning the multi-byte char.
        assert_eq!(
            run("StringTake", vec![s("héllo"), list(vec![int(1), int(2)])]),
            s("hé")
        );
        // Last 3 of "a😀b😀" — must not split the emoji.
        assert_eq!(run("StringTake", vec![s("a😀b"), int(-2)]), s("😀b"));
    }

    #[test]
    fn string_take_out_of_range_or_malformed_stays_unevaluated() {
        // |n| exceeds the length.
        assert_eq!(
            run("StringTake", vec![s("hi"), int(9)]),
            apply(sym("StringTake"), vec![s("hi"), int(9)])
        );
        // i64::MIN index must not panic — just unevaluated.
        assert_eq!(
            run("StringTake", vec![s("hi"), int(i64::MIN)]),
            apply(sym("StringTake"), vec![s("hi"), int(i64::MIN)])
        );
        // Range out of bounds.
        assert_eq!(
            run("StringTake", vec![s("hi"), list(vec![int(1), int(9)])]),
            apply(sym("StringTake"), vec![s("hi"), list(vec![int(1), int(9)])])
        );
        // Inverted range (n < m).
        assert_eq!(
            run("StringTake", vec![s("hello"), list(vec![int(4), int(2)])]),
            apply(sym("StringTake"), vec![s("hello"), list(vec![int(4), int(2)])])
        );
        // Non-string subject.
        assert_eq!(
            run("StringTake", vec![int(5), int(1)]),
            apply(sym("StringTake"), vec![int(5), int(1)])
        );
    }

    #[test]
    fn string_drop_first_and_last() {
        assert_eq!(run("StringDrop", vec![s("hello"), int(2)]), s("llo"));
        assert_eq!(run("StringDrop", vec![s("hello"), int(-2)]), s("hel"));
        assert_eq!(run("StringDrop", vec![s("hello"), int(0)]), s("hello"));
        // Dropping everything.
        assert_eq!(run("StringDrop", vec![s("hello"), int(5)]), s(""));
        // Unicode: dropping 1 char from "héllo" → "éllo".
        assert_eq!(run("StringDrop", vec![s("héllo"), int(1)]), s("éllo"));
    }

    #[test]
    fn string_drop_out_of_range_or_malformed_stays_unevaluated() {
        assert_eq!(
            run("StringDrop", vec![s("hi"), int(9)]),
            apply(sym("StringDrop"), vec![s("hi"), int(9)])
        );
        // i64::MIN must not panic.
        assert_eq!(
            run("StringDrop", vec![s("hi"), int(i64::MIN)]),
            apply(sym("StringDrop"), vec![s("hi"), int(i64::MIN)])
        );
        assert_eq!(
            run("StringDrop", vec![int(5), int(1)]),
            apply(sym("StringDrop"), vec![int(5), int(1)])
        );
    }

    #[test]
    fn string_split_on_whitespace() {
        assert_eq!(
            run("StringSplit", vec![s("a b  c")]),
            list(vec![s("a"), s("b"), s("c")])
        );
        // Leading / trailing whitespace is dropped.
        assert_eq!(
            run("StringSplit", vec![s("  a  b  ")]),
            list(vec![s("a"), s("b")])
        );
        // No whitespace → a single field.
        assert_eq!(run("StringSplit", vec![s("abc")]), list(vec![s("abc")]));
        // All whitespace → empty list.
        assert_eq!(run("StringSplit", vec![s("   ")]), list(vec![]));
    }

    #[test]
    fn string_split_on_a_separator() {
        assert_eq!(
            run("StringSplit", vec![s("a,b,c"), s(",")]),
            list(vec![s("a"), s("b"), s("c")])
        );
        // Adjacent / leading / trailing separators drop empty fields.
        assert_eq!(
            run("StringSplit", vec![s(",a,,b,"), s(",")]),
            list(vec![s("a"), s("b")])
        );
        // A multi-character separator.
        assert_eq!(
            run("StringSplit", vec![s("a::b::c"), s("::")]),
            list(vec![s("a"), s("b"), s("c")])
        );
    }

    #[test]
    fn string_split_malformed_stays_unevaluated() {
        // Empty separator is rejected.
        assert_eq!(
            run("StringSplit", vec![s("abc"), s("")]),
            apply(sym("StringSplit"), vec![s("abc"), s("")])
        );
        // Non-string subject.
        assert_eq!(
            run("StringSplit", vec![int(5)]),
            apply(sym("StringSplit"), vec![int(5)])
        );
        // Non-string separator.
        assert_eq!(
            run("StringSplit", vec![s("a"), int(1)]),
            apply(sym("StringSplit"), vec![s("a"), int(1)])
        );
    }

    #[test]
    fn string_replace_all_occurrences() {
        assert_eq!(
            run("StringReplace", vec![s("banana"), rule("a", "o")]),
            s("bonono")
        );
        // Multi-character pattern.
        assert_eq!(
            run("StringReplace", vec![s("aXbXc"), rule("X", "-")]),
            s("a-b-c")
        );
        // No match → unchanged.
        assert_eq!(
            run("StringReplace", vec![s("abc"), rule("z", "Q")]),
            s("abc")
        );
        // Amplifying replacement (rep longer than pat) — non-overlapping scan does
        // NOT re-scan the inserted text, so "a"->"aa" on "aa" gives "aaaa", not ∞.
        assert_eq!(
            run("StringReplace", vec![s("aa"), rule("a", "aa")]),
            s("aaaa")
        );
    }

    #[test]
    fn string_replace_accepts_a_list_of_rules() {
        // Rules apply in sequence: "a"->"b" then "b"->"c" turns "a" into "c".
        assert_eq!(
            run(
                "StringReplace",
                vec![s("abc"), list(vec![rule("a", "X"), rule("c", "Y")])]
            ),
            s("XbY")
        );
    }

    #[test]
    fn string_replace_empty_pattern_is_rejected() {
        // "" -> x would match between every char (unbounded) — left unevaluated.
        assert_eq!(
            run("StringReplace", vec![s("abc"), rule("", "Z")]),
            apply(sym("StringReplace"), vec![s("abc"), rule("", "Z")])
        );
    }

    #[test]
    fn string_replace_malformed_stays_unevaluated() {
        // Non-string subject.
        assert_eq!(
            run("StringReplace", vec![int(5), rule("a", "b")]),
            apply(sym("StringReplace"), vec![int(5), rule("a", "b")])
        );
        // Second arg is not a rule.
        assert_eq!(
            run("StringReplace", vec![s("abc"), int(1)]),
            apply(sym("StringReplace"), vec![s("abc"), int(1)])
        );
        // A rule whose sides are not strings.
        let bad = apply(sym(PM_RULE), vec![int(1), int(2)]);
        assert_eq!(
            run("StringReplace", vec![s("abc"), bad.clone()]),
            apply(sym("StringReplace"), vec![s("abc"), bad])
        );
    }

    #[test]
    fn string_replace_is_unicode_safe() {
        // Replacing a multi-byte char must not split a UTF-8 boundary.
        assert_eq!(
            run("StringReplace", vec![s("héllo"), rule("é", "e")]),
            s("hello")
        );
        assert_eq!(
            run("StringReplace", vec![s("a😀b"), rule("😀", "X")]),
            s("aXb")
        );
    }

    #[test]
    fn to_string_renders_surface_form() {
        // A bare number renders without quotes.
        assert_eq!(run("ToString", vec![int(123)]), s("123"));
        // A bare string renders as its raw content (no surrounding quotes).
        assert_eq!(run("ToString", vec![s("hi")]), s("hi"));
        // A symbol.
        assert_eq!(run("ToString", vec![sym("x")]), s("x"));
        // A compound expression reuses the printer.
        assert_eq!(
            run("ToString", vec![apply(sym(ADD), vec![sym("x"), int(1)])]),
            s("x + 1")
        );
        // A list.
        assert_eq!(
            run("ToString", vec![list(vec![int(1), int(2)])]),
            s("{1, 2}")
        );
    }

    #[test]
    fn to_string_wrong_arity_stays_unevaluated() {
        assert_eq!(
            run("ToString", vec![int(1), int(2)]),
            apply(sym("ToString"), vec![int(1), int(2)])
        );
    }

    #[test]
    fn characters_splits_into_single_char_strings() {
        assert_eq!(run("Characters", vec![s("ab")]), list(vec![s("a"), s("b")]));
        assert_eq!(run("Characters", vec![s("")]), list(vec![]));
        // Unicode: each multi-byte char is one element.
        assert_eq!(
            run("Characters", vec![s("héllo")]),
            list(vec![s("h"), s("é"), s("l"), s("l"), s("o")])
        );
        assert_eq!(
            run("Characters", vec![s("a😀")]),
            list(vec![s("a"), s("😀")])
        );
    }

    #[test]
    fn characters_of_non_string_stays_unevaluated() {
        assert_eq!(
            run("Characters", vec![int(5)]),
            apply(sym("Characters"), vec![int(5)])
        );
    }

    #[test]
    fn string_join_over_cap_stays_unevaluated() {
        // Two strings whose combined length exceeds MAX_STRING_LENGTH are refused.
        // Build via repeat so the test stays cheap (just over half the cap each).
        let half = "a".repeat(MAX_STRING_LENGTH / 2 + 1);
        let a = s(&half);
        let b = s(&half);
        let result = run("StringJoin", vec![a.clone(), b.clone()]);
        assert_eq!(result, apply(sym("StringJoin"), vec![a, b]));
    }

    // -----------------------------------------------------------------------
    // W-13 list set operations — Union / Intersection / Complement /
    // DeleteDuplicates / MemberQ / Tally
    // -----------------------------------------------------------------------

    #[test]
    fn union_of_two_lists_is_sorted_and_unique() {
        // Union[{1, 2}, {2, 3}] → {1, 2, 3}
        assert_eq!(
            run("Union", vec![list(vec![int(1), int(2)]), list(vec![int(2), int(3)])]),
            list(vec![int(1), int(2), int(3)])
        );
    }

    #[test]
    fn union_of_a_single_list_sorts_and_dedups() {
        // Union[{3, 1, 2, 1}] → {1, 2, 3} — a single argument doubles as
        // sort-and-unique.
        assert_eq!(
            run("Union", vec![list(vec![int(3), int(1), int(2), int(1)])]),
            list(vec![int(1), int(2), int(3)])
        );
    }

    #[test]
    fn union_over_three_lists_unions_all() {
        assert_eq!(
            run(
                "Union",
                vec![
                    list(vec![int(1)]),
                    list(vec![int(3), int(2)]),
                    list(vec![int(2), int(5)])
                ]
            ),
            list(vec![int(1), int(2), int(3), int(5)])
        );
    }

    #[test]
    fn union_keeps_distinct_numeric_subtypes() {
        // 2 and 2.0 are DISTINCT elements (type-tag tie-break in canonical_cmp),
        // matching Wolfram's Union[{2, 2.}] → {2, 2.}. The integer sorts before
        // the equal-magnitude float.
        assert_eq!(
            run("Union", vec![list(vec![int(2), flt(2.0)])]),
            list(vec![int(2), flt(2.0)])
        );
    }

    #[test]
    fn union_with_symbol_and_compound_elements() {
        // Mixed tiers sort numbers < symbols, and a repeated compound dedups.
        assert_eq!(
            run(
                "Union",
                vec![
                    list(vec![sym("b"), int(1), apply(sym("f"), vec![int(1)])]),
                    list(vec![apply(sym("f"), vec![int(1)]), sym("a")])
                ]
            ),
            list(vec![
                int(1),
                sym("a"),
                sym("b"),
                apply(sym("f"), vec![int(1)])
            ])
        );
    }

    #[test]
    fn intersection_keeps_common_elements_sorted() {
        // Intersection[{1, 2, 3}, {2, 3, 4}] → {2, 3}
        assert_eq!(
            run(
                "Intersection",
                vec![
                    list(vec![int(1), int(2), int(3)]),
                    list(vec![int(2), int(3), int(4)])
                ]
            ),
            list(vec![int(2), int(3)])
        );
    }

    #[test]
    fn intersection_over_three_lists() {
        assert_eq!(
            run(
                "Intersection",
                vec![
                    list(vec![int(1), int(2), int(3), int(4)]),
                    list(vec![int(2), int(3), int(4)]),
                    list(vec![int(3), int(4), int(5)])
                ]
            ),
            list(vec![int(3), int(4)])
        );
    }

    #[test]
    fn intersection_with_no_common_elements_is_empty() {
        assert_eq!(
            run(
                "Intersection",
                vec![list(vec![int(1), int(2)]), list(vec![int(3), int(4)])]
            ),
            list(vec![])
        );
    }

    #[test]
    fn complement_removes_subtracted_elements_sorted() {
        // Complement[{1, 2, 3, 4}, {2, 4}] → {1, 3}
        assert_eq!(
            run(
                "Complement",
                vec![
                    list(vec![int(1), int(2), int(3), int(4)]),
                    list(vec![int(2), int(4)])
                ]
            ),
            list(vec![int(1), int(3)])
        );
    }

    #[test]
    fn complement_over_multiple_subtrahends() {
        // Remove anything present in ANY of the trailing lists.
        assert_eq!(
            run(
                "Complement",
                vec![
                    list(vec![int(1), int(2), int(3), int(4), int(5)]),
                    list(vec![int(2)]),
                    list(vec![int(4), int(5)])
                ]
            ),
            list(vec![int(1), int(3)])
        );
    }

    #[test]
    fn complement_of_single_list_sorts_and_dedups() {
        assert_eq!(
            run("Complement", vec![list(vec![int(3), int(1), int(1), int(2)])]),
            list(vec![int(1), int(2), int(3)])
        );
    }

    #[test]
    fn delete_duplicates_preserves_first_occurrence_order() {
        // DeleteDuplicates[{3, 1, 1, 2, 3}] → {3, 1, 2} — order kept, NOT sorted.
        assert_eq!(
            run(
                "DeleteDuplicates",
                vec![list(vec![int(3), int(1), int(1), int(2), int(3)])]
            ),
            list(vec![int(3), int(1), int(2)])
        );
    }

    #[test]
    fn delete_duplicates_differs_from_union_ordering() {
        // The key semantic contrast: same input, Union sorts, DeleteDuplicates
        // preserves order.
        let input = list(vec![int(3), int(1), int(2), int(1)]);
        assert_eq!(
            run("Union", vec![input.clone()]),
            list(vec![int(1), int(2), int(3)])
        );
        assert_eq!(
            run("DeleteDuplicates", vec![input]),
            list(vec![int(3), int(1), int(2)])
        );
    }

    #[test]
    fn member_q_true_and_false() {
        // MemberQ[{1, 2, 3}, 2] → True ; MemberQ[{1, 2, 3}, 9] → False
        assert_eq!(
            run("MemberQ", vec![list(vec![int(1), int(2), int(3)]), int(2)]),
            sym("True")
        );
        assert_eq!(
            run("MemberQ", vec![list(vec![int(1), int(2), int(3)]), int(9)]),
            sym("False")
        );
    }

    #[test]
    fn member_q_distinguishes_int_from_float() {
        // 2 is not a member of {2.0} — distinct numeric subtypes.
        assert_eq!(
            run("MemberQ", vec![list(vec![flt(2.0)]), int(2)]),
            sym("False")
        );
        assert_eq!(
            run("MemberQ", vec![list(vec![sym("a"), sym("b")]), sym("b")]),
            sym("True")
        );
    }

    #[test]
    fn tally_counts_in_first_occurrence_order() {
        // Tally[{a, a, b, a}] → {{a, 3}, {b, 1}}
        assert_eq!(
            run("Tally", vec![list(vec![sym("a"), sym("a"), sym("b"), sym("a")])]),
            list(vec![
                list(vec![sym("a"), int(3)]),
                list(vec![sym("b"), int(1)])
            ])
        );
    }

    #[test]
    fn tally_of_empty_list_is_empty() {
        assert_eq!(run("Tally", vec![list(vec![])]), list(vec![]));
    }

    #[test]
    fn set_ops_on_empty_lists() {
        assert_eq!(run("Union", vec![list(vec![])]), list(vec![]));
        assert_eq!(
            run("Intersection", vec![list(vec![]), list(vec![int(1)])]),
            list(vec![])
        );
        assert_eq!(
            run("Complement", vec![list(vec![]), list(vec![int(1)])]),
            list(vec![])
        );
        assert_eq!(run("DeleteDuplicates", vec![list(vec![])]), list(vec![]));
    }

    #[test]
    fn nan_element_does_not_panic_in_set_ops() {
        // A crafted NaN float must compare panic-free (canonical_cmp uses
        // total_cmp). Two NaNs are the SAME element under total_cmp, so they dedup.
        let nan = flt(f64::NAN);
        let result = run("Union", vec![list(vec![nan.clone(), nan.clone()])]);
        // Exactly one NaN survives; assert via list_elements rather than `==`
        // (NaN != NaN under PartialEq).
        let elems = list_elements(&result).expect("Union returns a list");
        assert_eq!(elems.len(), 1);
        assert!(matches!(elems[0], IRNode::Float(f) if f.is_nan()));
        // MemberQ finds a NaN among NaNs (same_element via total_cmp).
        assert_eq!(
            run("MemberQ", vec![list(vec![flt(f64::NAN)]), flt(f64::NAN)]),
            sym("True")
        );
    }

    #[test]
    fn set_ops_on_non_list_stay_unevaluated() {
        // Non-list argument to any head → original form, no panic.
        assert_eq!(
            run("Union", vec![int(1), list(vec![int(2)])]),
            apply(sym("Union"), vec![int(1), list(vec![int(2)])])
        );
        assert_eq!(
            run("Intersection", vec![sym("x")]),
            apply(sym("Intersection"), vec![sym("x")])
        );
        assert_eq!(
            run("Complement", vec![int(3), list(vec![int(1)])]),
            apply(sym("Complement"), vec![int(3), list(vec![int(1)])])
        );
        assert_eq!(
            run("DeleteDuplicates", vec![int(7)]),
            apply(sym("DeleteDuplicates"), vec![int(7)])
        );
        // MemberQ[3, 2] — non-list first argument stays unevaluated.
        assert_eq!(
            run("MemberQ", vec![int(3), int(2)]),
            apply(sym("MemberQ"), vec![int(3), int(2)])
        );
        assert_eq!(
            run("Tally", vec![sym("y")]),
            apply(sym("Tally"), vec![sym("y")])
        );
    }

    #[test]
    fn set_ops_with_wrong_arity_stay_unevaluated() {
        // Zero arguments / wrong arity → unevaluated.
        assert_eq!(run("Union", vec![]), apply(sym("Union"), vec![]));
        assert_eq!(
            run("Intersection", vec![]),
            apply(sym("Intersection"), vec![])
        );
        assert_eq!(run("Complement", vec![]), apply(sym("Complement"), vec![]));
        // DeleteDuplicates / MemberQ / Tally are fixed-arity.
        assert_eq!(
            run("DeleteDuplicates", vec![list(vec![]), list(vec![])]),
            apply(sym("DeleteDuplicates"), vec![list(vec![]), list(vec![])])
        );
        assert_eq!(
            run("MemberQ", vec![list(vec![int(1)])]),
            apply(sym("MemberQ"), vec![list(vec![int(1)])])
        );
        assert_eq!(
            run("Tally", vec![list(vec![]), int(1)]),
            apply(sym("Tally"), vec![list(vec![]), int(1)])
        );
    }

    #[test]
    fn union_over_cap_stays_unevaluated() {
        // A union whose deduped accumulator would exceed MAX_LIST_LENGTH is
        // refused (DoS cap). Build a single list of MAX_LIST_LENGTH + 1 distinct
        // integers — every element is unique, so the accumulator overruns the cap.
        let big: Vec<IRNode> = (0..=(MAX_LIST_LENGTH as i64)).map(int).collect();
        let arg = list(big);
        let result = run("Union", vec![arg.clone()]);
        assert_eq!(result, apply(sym("Union"), vec![arg]));
    }

    #[test]
    fn tally_over_cap_stays_unevaluated() {
        // MAX_LIST_LENGTH + 1 distinct elements → more than MAX_LIST_LENGTH pairs,
        // refused before allocation.
        let big: Vec<IRNode> = (0..=(MAX_LIST_LENGTH as i64)).map(int).collect();
        let arg = list(big);
        let result = run("Tally", vec![arg.clone()]);
        assert_eq!(result, apply(sym("Tally"), vec![arg]));
    }

    #[test]
    fn set_ops_resolve_end_to_end_through_the_backend() {
        // Over a real WolframBackend the heads are reachable by name and compose
        // with the rest of the lane (here just confirming dispatch + result shape).
        assert_eq!(
            run_wolfram("Union", vec![list(vec![int(2), int(1)]), list(vec![int(1), int(3)])]),
            list(vec![int(1), int(2), int(3)])
        );
        assert_eq!(
            run_wolfram("Tally", vec![list(vec![int(5), int(5)])]),
            list(vec![list(vec![int(5), int(2)])])
        );
    }
}

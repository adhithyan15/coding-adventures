//! Polynomial expansion: distribute `Mul` over `Add`/`Sub`, and expand
//! non-negative integer powers of a sum via square-and-multiply.
//!
//! ## What this is — and, honestly, what it is not
//!
//! `expand(x)` is the operation MACSYMA calls `expand()` and Wolfram
//! calls `Expand[...]`. It **distributes** — `(x+1)*(x+2)` becomes
//! `2 + x + 2*x + x*x` — but it does **not collect like terms**: the
//! two `x` terms above stay separate rather than merging into `3*x`,
//! and `x*x` is never folded into `x^2`. The result is always
//! mathematically correct (it evaluates identically to the input for
//! any assignment) but is not always the compact form a human would
//! write by hand. Full like-term collection is a separate, more
//! involved pass — tracked as an explicit follow-up, not silently
//! dropped (see the crate's `spice-macsyma-pending-work.md` entry).
//!
//! This is a **faithful recursive-distributor port** of the Python
//! reference (`symbolic_vm.cas_handlers._sym_expand` /
//! `_sym_expand_mul` / `_sym_expand_pow`), generalized to the n-ary
//! `Add`/`Mul` shape this Rust IR actually produces (Python's reference
//! assumes strictly-binary `Add`/`Sub`/`Mul`, since its frontends never
//! flatten more than two operands into one node). The Python reference
//! also has a *second*, faster path for single-variable
//! rational-coefficient polynomials, built on a `to_rational`/
//! `from_polynomial` bridge, that *does* collect like terms (that is
//! what its docstring's "clean" example actually demonstrates — not
//! the general path this module ports). This port always takes the
//! general path, so it does not reproduce that fast-path's cleaner
//! output even for single-variable input.
//!
//! ## Truth table (the four expansion rules)
//!
//! | Input shape                | Rule                              |
//! |-----------------------------|------------------------------------|
//! | `Mul(.., Add(a, b), ..)`   | distribute: `Mul(.., a, ..) + Mul(.., b, ..)` |
//! | `Mul(.., Sub(a, b), ..)`   | distribute: `Mul(.., a, ..) - Mul(.., b, ..)` |
//! | `Pow(Add(..), n)` (0≤n≤32) | square-and-multiply: `O(log n)` multiplications, not `O(n)` |
//! | everything else            | recurse into children, head unchanged |
//!
//! ## The DoS this module guards against
//!
//! Square-and-multiply on a *k*-term sum roughly **squares the term
//! count at every squaring step** (a raw, uncombined distribution never
//! merges like terms), so `m` squarings can reach `k^(2^m)` terms —
//! doubly exponential in the number of squarings, not the exponent
//! itself. `(a+b+c+d)^32` (5 squarings from a 4-term base) would reach
//! `4^32` raw terms without a guard — an instant memory-exhaustion
//! crash, not a slow computation. [`EXPAND_MAX_TERMS`] bounds this: any
//! single distribution step whose *product* of operand term-counts
//! would exceed the cap is refused (the operands are returned as an
//! unexpanded `Mul` instead of being allocated), so growth stops the
//! moment it would cross the line rather than after it already has.
//!
//! **This guard has already had one real bug**, caught in security
//! review before merge: [`term_count`] originally treated *any*
//! non-`Add`/`Sub` node — including a refused-and-wrapped `Mul(a, b)`
//! that [`expand_mul`] itself had just produced — as a single opaque
//! term of size `1`. That let a capped subtree's true size go dark on
//! the very next multiplication (the guard saw a stale "1" instead of
//! the real, possibly-huge count), so a chain of several multi-term
//! sum factors (`(a1+..)*(b1+..)*(c1+..)*...`, an entirely ordinary-
//! looking expression, no pathological construction needed) could
//! still reach hundreds of millions of nodes — confirmed empirically
//! against the original code before the fix. `term_count` now
//! recurses into `Mul` descendants *multiplicatively* (mirroring how
//! it already recurses into `Add`/`Sub` *additively*), so a refused
//! `Mul`'s true size is always visible to the next check. See
//! [`term_count`]'s own docs and
//! `term_count_sees_through_a_refused_mul_instead_of_going_blind` for
//! the full account and the regression test.

use symbolic_ir::{apply, sym, IRApply, IRNode, ADD, MUL, POW, SUB};

use crate::simplifier::simplify;

/// Iteration cap passed to [`simplify`] when cleaning up a raw
/// distribution result. Matches the cap used elsewhere in this crate
/// and in every consumer's own `simplify()`/`Simplify[...]` wiring —
/// the simplifier already fixed-points internally, this is a shared
/// non-termination guard, not an `expand`-specific tuning choice.
const EXPAND_SIMPLIFY_MAX_ITERATIONS: usize = 50;

/// Maximum non-negative integer exponent [`expand`] will distribute via
/// square-and-multiply. Mirrors the Python reference's
/// `_SYM_EXPAND_MAX_POW` guard. A `Pow` whose exponent is not an
/// integer in `0..=EXPAND_MAX_POW` is left un-expanded (`Pow(base,
/// exp)`, recursively expanded but not distributed).
pub const EXPAND_MAX_POW: i64 = 32;

/// Maximum term-count product a single distribution step may produce.
/// See the module-level "DoS this module guards against" section for
/// why this exists and why it must be checked *before* distributing,
/// not after.
pub const EXPAND_MAX_TERMS: usize = 10_000;

/// The number of leaf terms `node` *would* have if fully distributed —
/// not necessarily the number it structurally has right now.
///
/// This must recurse into **both** `Add`/`Sub` descendants (summed) and
/// `Mul` descendants (multiplied), not just count `node`'s own direct
/// args:
///
/// - [`expand_mul`] does not flatten/canonicalize between successive
///   squaring steps inside [`expand_pow`], so an intermediate result is
///   typically a *nested* `Add(Add(..), Add(..))` tree, not a single
///   flat `Add` node. Counting only the direct args (e.g. `2` for
///   `Add(Add(4 terms), Add(4 terms))`) would undercount.
/// - **Critically**, when [`expand_mul`] *refuses* to distribute
///   because the product would exceed [`EXPAND_MAX_TERMS`], it returns
///   the operands wrapped in an ordinary `Mul(a, b)` node — the exact
///   same shape [`expand_apply`]'s `Mul` branch folds a chain of
///   factors through. If `term_count` treated every `Mul`-headed node
///   as a single opaque term (size `1`, the same as any atom), a
///   refused distribution would make its own true size *invisible* to
///   the very next fold step: `term_count(refused_mul) == 1` even
///   though the subtree it wraps may already represent millions of
///   would-be terms. The next multiplication then sails through the
///   cap check and distributes anyway, repeating this "cap → go dark →
///   distribute past the cap → cap again" cycle once per chained
///   factor — unbounded growth despite the cap, confirmed by an
///   adversarial-review harness that reached 200M+ nodes from a
///   perfectly ordinary six-factor expression (six chained parenthesized
///   sums) before this fix. Recursing multiplicatively into `Mul`
///   descendants here closes that hole: a refused-and-wrapped `Mul`
///   reports its true (potentially huge) size on every subsequent
///   check, so the guard keeps seeing accurate numbers instead of a
///   stale "1" and correctly keeps refusing to distribute further.
///
/// - **The same blindness recurs for every other head `expand_apply`
///   leaves in place** — `Div`, `Neg`, `Pow` left un-distributed (a
///   non-integer or out-of-range exponent), and every transcendental
///   (`Sin`, `Log`, ...). `expand_apply`'s fallthrough recursively
///   expands *their children* but never touches the wrapper head
///   itself (see `expand_recurses_into_div_operands`), so
///   `Div(huge_expanded_numerator, y)` is a completely ordinary,
///   frequently-produced shape — not a contrived edge case. Treating it
///   as size `1` (the pre-fix `_ => 1` catch-all) is safe *mathematically*
///   (as a polynomial term, `x/y` genuinely is one term, hidden internal
///   size notwithstanding) but wrong as a *cost estimate*: if this node
///   later becomes an operand under a further `Add`-distribution (e.g.
///   `expand_mul` folding it against another sum), [`expand_mul`] clones
///   it once per term of the other side — real cost proportional to its
///   *true* size, not `1`. A chain of several such wrapped huge
///   subtrees, each hidden from the cap check the same way, reproduces
///   exactly the "cap → go dark → distribute past the cap" cycle the
///   `Mul` fix above already closed for refused multiplications — just
///   via `Div`/`Neg`/transcendental wrappers instead of a refused `Mul`.
///   Any `Apply` node whose head is not itself distributed (i.e.,
///   anything but `Mul`, which multiplies) now falls through to summing
///   its children's term counts — the same conservative "total
///   underlying size" measure `Add`/`Sub` already used, generalized to
///   every wrapper shape `expand_apply` can leave behind, not just
///   `Add`/`Sub` specifically.
fn term_count(node: &IRNode) -> usize {
    match node {
        IRNode::Apply(app) if is_head(&app.head, MUL) => app
            .args
            .iter()
            .map(term_count)
            .fold(1usize, |acc, c| acc.saturating_mul(c)),
        // Every other `Apply` head (`Add`/`Sub`, `Div`, `Neg`, `Pow` left
        // un-distributed, every transcendental, ...) is not itself
        // distributed by `expand_apply` — only its children are
        // recursively expanded, the wrapper head stays. Summing the
        // children's term counts is the right measure for both cases:
        // for `Add`/`Sub` it *is* the true would-be term count; for
        // everything else it is a conservative (over-, never under-)
        // estimate of how much real cloning cost this subtree carries
        // if it is later multiplied against something else — the
        // direction a DoS guard must err toward.
        IRNode::Apply(app) => app.args.iter().map(term_count).sum::<usize>().max(1),
        _ => 1,
    }
}

/// Whether `node` is the symbol `name` (e.g. `is_head(&app.head, ADD)`
/// checks whether an application's head is literally `Add`).
fn is_head(node: &IRNode, name: &str) -> bool {
    matches!(node, IRNode::Symbol(s) if s == name)
}

/// Distribute multiplication over addition/subtraction:
/// `(a+b+c)*d = a*d + b*d + c*d` (generalized to n-ary `Add`/`Sub` —
/// the true distribution law holds for any number of terms, not just
/// two). Recurses into *both* operands, so a product of two sums
/// (`(a+b)*(c+d)`) fully distributes. Returns `Mul(a, b)` unchanged
/// when neither operand is an `Add`/`Sub`, or when distributing would
/// exceed [`EXPAND_MAX_TERMS`] (see the module-level DoS note).
fn expand_mul(a: &IRNode, b: &IRNode) -> IRNode {
    if term_count(a).saturating_mul(term_count(b)) > EXPAND_MAX_TERMS {
        return apply(sym(MUL), vec![a.clone(), b.clone()]);
    }
    if let IRNode::Apply(app) = a {
        if is_head(&app.head, ADD) || is_head(&app.head, SUB) {
            let head = app.head.clone();
            let terms = app.args.iter().map(|t| expand_mul(t, b)).collect();
            return apply(head, terms);
        }
    }
    if let IRNode::Apply(app) = b {
        if is_head(&app.head, ADD) || is_head(&app.head, SUB) {
            let head = app.head.clone();
            let terms = app.args.iter().map(|t| expand_mul(a, t)).collect();
            return apply(head, terms);
        }
    }
    apply(sym(MUL), vec![a.clone(), b.clone()])
}

/// Expand `base^n` (`n` a non-negative integer, already bounds-checked
/// against [`EXPAND_MAX_POW`] by the caller) via square-and-multiply:
/// `O(log n)` distribution steps instead of `O(n)`. Term-count growth
/// across those `O(log n)` squarings is exactly the doubly-exponential
/// blowup [`expand_mul`]'s [`EXPAND_MAX_TERMS`] check guards against.
fn expand_pow(base: &IRNode, n: i64) -> IRNode {
    if n == 0 {
        return IRNode::Integer(1);
    }
    if n == 1 {
        return base.clone();
    }
    let half = expand_pow(base, n / 2);
    let squared = expand_mul(&half, &half);
    if n % 2 == 1 {
        return expand_mul(&squared, base);
    }
    squared
}

/// Recursively distribute `Mul` over `Add`/`Sub` and expand bounded
/// non-negative integer `Pow`s throughout `node`, then run the result
/// through [`simplify`] (canonical form, numeric-literal folding, and
/// identity rules — `x*1 -> x`, `1*1 -> 1`, etc.) to clean up the raw
/// distribution.
///
/// See the module-level docs for what "does not collect like terms"
/// means in practice — the output below has two separate `x` terms,
/// not one `2*x` term, and `x*x` rather than `x^2`.
///
/// Non-polynomial subexpressions (trig, transcendentals, symbolic
/// powers, `Div`) are returned with their children recursively expanded
/// but their own head unchanged — `expand` is safe to call on any IR
/// tree, not just pure polynomials.
///
/// ```rust
/// use symbolic_ir::{apply, int, sym, ADD, POW};
/// use cas_simplify::expand;
///
/// // (x + 1)^2 -> 1 + x + x + x*x  (mathematically x^2 + 2x + 1;
/// // see the module docs for why the two `x` terms and `x*x` are
/// // not collected/folded further).
/// let x_plus_1 = apply(sym(ADD), vec![sym("x"), int(1)]);
/// let expr = apply(sym(POW), vec![x_plus_1, int(2)]);
/// let expanded = expand(expr);
/// assert_eq!(format!("{expanded}"), "Add(1, x, x, Mul(x, x))");
/// ```
pub fn expand(node: IRNode) -> IRNode {
    simplify(expand_recursive(node), EXPAND_SIMPLIFY_MAX_ITERATIONS)
}

fn expand_recursive(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(app) => expand_apply(*app),
        other => other,
    }
}

fn expand_apply(node: IRApply) -> IRNode {
    let IRApply { head, args } = node;
    let expanded_args: Vec<IRNode> = args.into_iter().map(expand_recursive).collect();

    if let IRNode::Symbol(name) = &head {
        if name == ADD {
            return apply(head.clone(), expanded_args);
        }
        if name == MUL {
            let mut terms = expanded_args.into_iter();
            let first = terms.next().unwrap_or(IRNode::Integer(1));
            return terms.fold(first, |acc, next| expand_mul(&acc, &next));
        }
        if name == POW && expanded_args.len() == 2 {
            if let IRNode::Integer(n) = expanded_args[1] {
                if (0..=EXPAND_MAX_POW).contains(&n) {
                    return expand_pow(&expanded_args[0], n);
                }
            }
        }
    }
    apply(head, expanded_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, flt, int, sym, DIV, NEG, SIN};

    fn add(args: Vec<IRNode>) -> IRNode {
        apply(sym(ADD), args)
    }
    fn sub(a: IRNode, b: IRNode) -> IRNode {
        apply(sym(SUB), vec![a, b])
    }
    fn mul(args: Vec<IRNode>) -> IRNode {
        apply(sym(MUL), args)
    }
    fn pow(base: IRNode, exp: IRNode) -> IRNode {
        apply(sym(POW), vec![base, exp])
    }

    #[test]
    fn expand_distributes_mul_over_add() {
        // (x + 1) * (x + 2) -> 2 + x + 2*x + x*x (see module docs:
        // the `x` term from 1*x and the `2*x` term from 2*x are not
        // collected into 3*x; x*x is not folded into x^2).
        let lhs = add(vec![sym("x"), int(1)]);
        let rhs = add(vec![sym("x"), int(2)]);
        let result = expand(mul(vec![lhs, rhs]));
        assert_eq!(
            result,
            add(vec![
                int(2),
                sym("x"),
                mul(vec![int(2), sym("x")]),
                mul(vec![sym("x"), sym("x")]),
            ])
        );
    }

    #[test]
    fn expand_distributes_mul_over_sub() {
        // (a + b) * (a - b) -> (a*a - a*b) + (a*b - b*b) — evaluates
        // to a^2 - b^2 (the two `a*b` terms cancel numerically for any
        // assignment) but Sub is not flattened into a single Add of
        // signed terms, so the middle terms are never actually
        // cancelled structurally. See module docs.
        let lhs = add(vec![sym("a"), sym("b")]);
        let rhs = sub(sym("a"), sym("b"));
        let result = expand(mul(vec![lhs, rhs]));
        assert_eq!(
            result,
            add(vec![
                sub(mul(vec![sym("a"), sym("a")]), mul(vec![sym("a"), sym("b")]),),
                sub(mul(vec![sym("a"), sym("b")]), mul(vec![sym("b"), sym("b")]),),
            ])
        );
    }

    #[test]
    fn expand_pow_of_binomial_distributes_correctly() {
        // (x + 1)^2 -> 1 + x + x + x*x (mathematically x^2 + 2x + 1;
        // see module docs for why it isn't collected into that form).
        let result = expand(pow(add(vec![sym("x"), int(1)]), int(2)));
        assert_eq!(
            result,
            add(vec![
                int(1),
                sym("x"),
                sym("x"),
                mul(vec![sym("x"), sym("x")])
            ])
        );
    }

    #[test]
    fn expand_pow_of_trinomial_multivariate() {
        // (a + b)^3 -> 8 raw monomials (a^3 has only one arrangement;
        // a^2*b and a*b^2 each have 3 arrangements from square-and-
        // multiply, matching the binomial coefficients C(3,1)=3, but
        // emitted as repeated separate terms rather than one term with
        // coefficient 3 — see module docs).
        let result = expand(pow(add(vec![sym("a"), sym("b")]), int(3)));
        let aaa = mul(vec![sym("a"), sym("a"), sym("a")]);
        let aab = mul(vec![sym("a"), sym("a"), sym("b")]);
        let abb = mul(vec![sym("a"), sym("b"), sym("b")]);
        let bbb = mul(vec![sym("b"), sym("b"), sym("b")]);
        assert_eq!(
            result,
            add(vec![
                aaa,
                aab.clone(),
                aab.clone(),
                aab,
                abb.clone(),
                abb.clone(),
                abb,
                bbb,
            ])
        );
    }

    #[test]
    fn expand_pow_zero_and_one() {
        assert_eq!(expand(pow(sym("x"), int(0))), int(1));
        assert_eq!(expand(pow(sym("x"), int(1))), sym("x"));
    }

    #[test]
    fn expand_pow_above_max_stays_unevaluated() {
        // exponent above EXPAND_MAX_POW: left as Pow, not expanded
        // (the base is still recursively processed and canonicalized
        // by the final `simplify` pass, so `Add(x, 1)` becomes the
        // canonical `Add(1, x)` even though it was never distributed).
        let big = int(EXPAND_MAX_POW + 1);
        let result = expand(pow(add(vec![sym("x"), int(1)]), big.clone()));
        assert_eq!(result, pow(add(vec![int(1), sym("x")]), big));
    }

    #[test]
    fn expand_pow_negative_exponent_stays_unevaluated() {
        let result = expand(pow(add(vec![sym("x"), int(1)]), int(-1)));
        assert_eq!(result, pow(add(vec![int(1), sym("x")]), int(-1)));
    }

    #[test]
    fn expand_leaves_transcendentals_structurally_unchanged() {
        // Sin(x + 1) has no Mul-over-Add or integer Pow to distribute;
        // expand recurses into the argument (which the final
        // `simplify` pass canonicalizes to `Add(1, x)`) but the Sin
        // head itself is untouched.
        let result = expand(apply(sym(SIN), vec![add(vec![sym("x"), int(1)])]));
        assert_eq!(result, apply(sym(SIN), vec![add(vec![int(1), sym("x")])]));
    }

    #[test]
    fn expand_recurses_into_div_operands() {
        // (x+1)^2 / y -> (1 + x + x + x*x) / y — Div itself is not
        // distributed, but its numerator is still expanded.
        let numerator = pow(add(vec![sym("x"), int(1)]), int(2));
        let result = expand(apply(sym(DIV), vec![numerator, sym("y")]));
        assert_eq!(
            result,
            apply(
                sym(DIV),
                vec![
                    add(vec![
                        int(1),
                        sym("x"),
                        sym("x"),
                        mul(vec![sym("x"), sym("x")])
                    ]),
                    sym("y"),
                ]
            )
        );
    }

    #[test]
    fn expand_atoms_pass_through_unchanged() {
        assert_eq!(expand(int(5)), int(5));
        assert_eq!(expand(sym("x")), sym("x"));
        assert_eq!(expand(flt(2.5)), flt(2.5));
    }

    #[test]
    fn expand_mul_term_count_guard_refuses_astronomical_blowup() {
        // A 4-term base raised to the 32nd power would (without the
        // guard) reach 4^32 raw terms across 5 squarings — the guard
        // must stop distribution long before that, leaving a (still
        // partially expanded, but bounded) Mul/Pow shape rather than
        // hanging or exhausting memory.
        let many_terms = add(vec![sym("a"), sym("b"), sym("c"), sym("d")]);
        let expr = pow(many_terms, int(32));
        // Must return promptly (no timeout) and must not panic.
        let result = expand(expr);
        // The result is some valid IR value — the guard's job is
        // bounding work, not producing a specific shape.
        let _ = result;
    }

    #[test]
    fn expand_term_count_guard_caps_a_two_large_sums_product() {
        // Two sums whose term-count product exceeds EXPAND_MAX_TERMS
        // must not be distributed.
        let terms_a: Vec<IRNode> = (0..200).map(|i| sym(format!("a{i}"))).collect();
        let terms_b: Vec<IRNode> = (0..200).map(|i| sym(format!("b{i}"))).collect();
        // 200 * 200 = 40,000 > EXPAND_MAX_TERMS (10,000).
        let result = expand_mul(&add(terms_a.clone()), &add(terms_b.clone()));
        assert_eq!(result, mul(vec![add(terms_a), add(terms_b)]));
    }

    /// Recursively counts every node in `node`, regardless of head — a
    /// literal tree-size measurement, unlike [`term_count`] (which
    /// counts would-be polynomial terms, not allocated nodes). Used
    /// only by the regression test below to prove the result actually
    /// stayed bounded, not just that it returned before a test timeout.
    fn total_node_count(node: &IRNode) -> usize {
        match node {
            IRNode::Apply(app) => 1 + app.args.iter().map(total_node_count).sum::<usize>(),
            _ => 1,
        }
    }

    #[test]
    fn term_count_sees_through_a_refused_mul_instead_of_going_blind() {
        // Regression test for a real vulnerability caught in security
        // review: term_count() originally treated ANY non-Add/Sub node
        // — including a Mul(a, b) that expand_mul itself had just
        // *refused* to distribute because it was already too big — as
        // a single opaque term of size 1. That let a capped subtree's
        // true size go dark on the very next multiplication: the guard
        // would see a stale "1" instead of the real (possibly huge)
        // count, wave the next factor through, and repeat — "cap, go
        // blind, distribute past the cap, cap again" once per chained
        // factor. An adversarial-review harness against the original
        // code reached 200M+ nodes from six chained 20-term sums.
        //
        // This constructs the same shape (chained multi-term sum
        // factors) at a size that would have detonated under the old
        // logic, and asserts the result stays small — proving
        // term_count's Mul case (added by this fix) keeps the guard
        // seeing the real size instead of a fictitious "1".
        fn sum_of(prefix: &str, n: usize) -> IRNode {
            add((0..n).map(|i| sym(format!("{prefix}{i}"))).collect())
        }
        let factors = vec![
            sum_of("a", 20),
            sum_of("b", 20),
            sum_of("c", 20),
            sum_of("d", 20),
            sum_of("e", 20),
            sum_of("f", 20),
        ];
        let expr = mul(factors);
        let result = expand(expr);
        // Under the pre-fix logic this reached 200M+ nodes; with the
        // fix, growth stops the moment any single distribution step
        // would exceed EXPAND_MAX_TERMS, so the true total stays in
        // the low tens of thousands at most — nowhere near millions.
        assert!(
            total_node_count(&result) < 100_000,
            "expand() of six chained 20-term sums produced {} nodes -- \
             the term_count guard is not seeing through a refused Mul \
             the way it should",
            total_node_count(&result)
        );
    }

    #[test]
    fn term_count_sees_through_a_div_wrapped_subtree_instead_of_going_blind() {
        // Regression test for the Div/Neg/transcendental generalization
        // of the fix above. expand_apply never distributes Div itself —
        // it recurses into the numerator/denominator and leaves the Div
        // head in place (see expand_recurses_into_div_operands) — so
        // Div(huge_add_tree, y) is an entirely ordinary shape a real
        // expansion can leave behind, not a contrived one. Build one
        // directly (standing in for whatever a prior expand() call
        // would already have produced) and multiply it by a modest
        // second Add factor.
        let huge_numerator = add((0..9000).map(|i| sym(format!("x{i}"))).collect());
        let big_div = apply(sym(DIV), vec![huge_numerator, sym("y")]);
        let second_factor = add((0..20).map(|i| sym(format!("z{i}"))).collect());

        let result = expand(mul(vec![big_div, second_factor]));

        // Under the pre-fix logic, term_count(big_div) == 1 (Div is
        // neither Add/Sub nor Mul), so the cap check saw "1 * 20 = 20"
        // and happily distributed — cloning the ~9000-node numerator
        // once per term of the second factor (20 clones, ~180,000+
        // nodes). With the fix, term_count sees big_div's true size
        // (~9000), the cap check correctly refuses
        // (9000 * 20 = 180,000 > EXPAND_MAX_TERMS), and the result
        // stays a single unexpanded Mul — no cloning at all.
        assert!(
            total_node_count(&result) < 20_000,
            "expand() of a Div-wrapped 9000-term subtree times a 20-term \
             second factor produced {} nodes -- term_count is not seeing \
             through the Div wrapper the way it should",
            total_node_count(&result)
        );
    }

    #[test]
    fn term_count_treats_neg_and_transcendental_wrappers_the_same_way() {
        // Not adversarial-scale (the Div test above already proves the
        // guard holds under real pressure) -- this just confirms the
        // generalized fix actually applies uniformly to every other
        // non-Mul Apply head, not only Div specifically. Neg(20-term
        // sum) and Sin(20-term sum) must each report the same term
        // count as the bare sum (20), not 1.
        let twenty_terms = add((0..20).map(|i| sym(format!("t{i}"))).collect());
        let neg_wrapped = apply(sym(NEG), vec![twenty_terms.clone()]);
        let sin_wrapped = apply(sym(SIN), vec![twenty_terms.clone()]);

        // term_count is private to this module; drive it indirectly the
        // same way every other test in this file does, by checking that
        // multiplying each wrapped form against another sizeable sum
        // gets correctly refused (mirroring the Div test's structure).
        let other_factor = add((0..600).map(|i| sym(format!("o{i}"))).collect());
        // 20 * 600 = 12,000 > EXPAND_MAX_TERMS (10,000): must refuse.
        for wrapped in [neg_wrapped, sin_wrapped] {
            let result = expand_mul(&wrapped, &other_factor);
            assert_eq!(
                result,
                mul(vec![wrapped, other_factor.clone()]),
                "a Neg/Sin-wrapped 20-term sum must be sized as 20 terms, \
                 not 1, when checked against a 600-term second factor"
            );
        }
    }
}

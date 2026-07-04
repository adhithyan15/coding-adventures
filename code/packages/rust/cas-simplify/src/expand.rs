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

/// The number of leaf terms `node` contributes to a `Mul` distribution.
///
/// This must recurse into `Add`/`Sub` descendants, not just count
/// `node`'s own direct args: [`expand_mul`] does not flatten/canonicalize
/// between successive squaring steps inside [`expand_pow`], so an
/// intermediate result is typically a *nested* `Add(Add(..), Add(..))`
/// tree, not a single flat `Add` node. Counting only the direct args
/// (e.g. `2` for `Add(Add(4 terms), Add(4 terms))`) would drastically
/// undercount the true term count and let the very blowup this guard
/// exists to prevent slip through uncapped — confirmed by a test that
/// hung and had to be killed before this fix.
fn term_count(node: &IRNode) -> usize {
    match node {
        IRNode::Apply(app) if is_head(&app.head, ADD) || is_head(&app.head, SUB) => {
            app.args.iter().map(term_count).sum::<usize>().max(1)
        }
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
    use symbolic_ir::{apply, flt, int, sym, DIV, SIN};

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
}

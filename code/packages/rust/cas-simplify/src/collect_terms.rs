//! Collect-like-terms pass: groups the structurally-repeated monomials
//! [`crate::expand`]'s raw distributor leaves behind and sums their
//! coefficients — the "known remaining gap, honestly scoped out" of the
//! original `expand` port (see `expand.rs`'s module docs and
//! `code/specs/spice-macsyma-pending-work.md`'s `Expand` entry).
//!
//! ## What "collecting" means here
//!
//! `expand()`'s distributor is a *faithful* port of the reference
//! recursive distributor: `(x+1)^2` becomes `1 + x + x + x*x`, not the
//! "clean" `1 + 2*x + x^2` a human would write. This pass turns the former
//! into the latter, as a bottom-up rewrite over the already-expanded tree:
//!
//! 1. **Flatten** an `Add`/`Sub` subtree into a flat list of *signed*
//!    terms (`a - b + c` → `[(+, a), (-, b), (+, c)]`), recursing through
//!    nested `Add`/`Sub` — [`expand_mul`](crate::expand) does not flatten
//!    between successive square-and-multiply steps, so an intermediate
//!    result is typically a *nested* `Add(Add(..), Add(..))` tree, not one
//!    flat `Add`.
//! 2. **Monomialize** each term: split off its numeric coefficient
//!    (reusing [`crate::numeric_fold`]'s exact-rational accumulator,
//!    [`Acc`](crate::numeric_fold::Acc), so the same GCD-reduced,
//!    float-contamination-aware arithmetic backs both passes) and
//!    decompose its remaining factors into `(base, exponent)` pairs —
//!    `x*x` and `x^2` both become the same monomial `[(x, 2)]`, closing
//!    the *second* gap `expand`'s module docs call out ("`x*x` is never
//!    folded into `x^2`") as a byproduct of the same decomposition, not a
//!    separate fix.
//! 3. **Group** terms sharing the same monomial (the same sorted `(base,
//!    exponent)` set — `x^2*y` and `x*y^2` are *different* monomials, even
//!    though they share the same base *set*) and sum their coefficients —
//!    the actual "like terms" collection. A group whose summed
//!    coefficient is exactly zero is dropped entirely: this is where a
//!    genuine cancellation (e.g. the cross terms in `(a+b)*(a-b)`) really
//!    disappears, rather than surviving as an explicit `0` term.
//! 4. **Rebuild** each surviving group as `coefficient * base1^exp1 *
//!    base2^exp2 * ...` and wrap the results back up as `Add`, a single
//!    term, or `0`.
//!
//! A bare `Mul` (not part of a surrounding sum) is monomialized the same
//! way on its own, so `expand(x*x)` returns `x^2`, not `x*x` — the
//! power-folding half of the fix applies uniformly, not only to terms
//! that happen to sit inside an `Add`.
//!
//! ## Why this doesn't reopen the DoS `expand`'s sibling guard closes
//!
//! This pass only ever runs on [`expand_recursive`](crate::expand)'s
//! *output* — already bounded by `expand`'s own `EXPAND_MAX_TERMS` at
//! every distribution step it took to build. Flattening + monomializing +
//! grouping is `O(n log n)` in the number of (already-bounded) terms it's
//! handed — a sort-then-merge, not a quadratic scan — and it never
//! recurses into, or attempts to further distribute, a subtree that
//! `expand_mul` already refused to expand: an unexpanded `Mul`/`Pow`/other
//! wrapper is treated as one opaque monomial factor (the same fallback
//! [`factor_to_base_exp`] uses for anything that isn't a bare `Pow` with
//! an integer exponent), so this pass cannot re-open growth the term-count
//! cap already closed off.

use symbolic_ir::{apply, sym, IRNode, ADD, MUL, POW, SUB};

use crate::expand::is_head;
use crate::numeric_fold::{node_to_acc, Acc};

/// One base raised to one exponent inside a monomial, tagged with a
/// pre-computed sort/group key (`base`'s `Debug` string) so grouping never
/// re-derives it.
type BaseTerm = (String, IRNode, i64);

/// Recursively collect like terms throughout `node`.
///
/// Every `Add`/`Sub`-headed subtree is flattened, regrouped by monomial,
/// and rebuilt (see the module docs for the four-step algorithm); every
/// `Mul`-headed subtree is monomialized on its own so repeated factors
/// fold into a power even outside of a sum; everything else is left with
/// its head unchanged, children recursively collected.
pub fn collect_terms(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(app) => {
            let head = collect_terms(app.head);
            let args: Vec<IRNode> = app.args.into_iter().map(collect_terms).collect();

            if let IRNode::Symbol(name) = &head {
                if name == ADD || name == SUB {
                    return collect_additive(name, &args);
                }
                if name == MUL {
                    let (coef, monomial) = monomialize_factors(&args);
                    return rebuild_term(coef, monomial);
                }
            }
            apply(head, args)
        }
        other => other,
    }
}

/// Flatten an `Add`/`Sub` node's immediate args into signed terms, group by
/// monomial, and rebuild.
fn collect_additive(head_name: &str, args: &[IRNode]) -> IRNode {
    let mut signed: Vec<(Acc, Vec<BaseTerm>)> = Vec::new();
    for (i, a) in args.iter().enumerate() {
        // Add: every arg contributes positively. Sub(a, b) is exactly
        // `a - b` — the second (and only the second) of its two args is
        // negated.
        let sign: i64 = if head_name == SUB && i == 1 { -1 } else { 1 };
        collect_signed_terms(a, sign, &mut signed);
    }
    rebuild_additive(signed)
}

/// Recursively flatten `node` (scaled by `sign`) into `(coefficient,
/// monomial)` pairs, descending through nested `Add`/`Sub` so an
/// intermediate square-and-multiply tree (`Add(Add(..), Add(..))`) is
/// treated as one flat sum, not two separate ones.
fn collect_signed_terms(node: &IRNode, sign: i64, out: &mut Vec<(Acc, Vec<BaseTerm>)>) {
    if let IRNode::Apply(app) = node {
        if is_head(&app.head, ADD) {
            for a in &app.args {
                collect_signed_terms(a, sign, out);
            }
            return;
        }
        if is_head(&app.head, SUB) && app.args.len() == 2 {
            collect_signed_terms(&app.args[0], sign, out);
            collect_signed_terms(&app.args[1], -sign, out);
            return;
        }
    }
    let (coef, monomial) = monomialize(node);
    let signed_coef = if sign < 0 {
        coef.combine(Acc::Rat(-1, 1), true)
    } else {
        coef
    };
    out.push((signed_coef, monomial));
}

/// Decompose an arbitrary single term into `(coefficient, monomial)` —
/// unwraps a `Mul`'s own factor list, or treats `node` itself as the
/// single factor of a bare (coefficient-1) monomial.
fn monomialize(node: &IRNode) -> (Acc, Vec<BaseTerm>) {
    if let IRNode::Apply(app) = node {
        if is_head(&app.head, MUL) {
            return monomialize_factors(&app.args);
        }
    }
    monomialize_factors(std::slice::from_ref(node))
}

/// Decompose an already-known factor list (a `Mul`'s args, or a
/// single-element slice standing in for a bare term) into `(coefficient,
/// monomial)`: numeric-literal factors fold into one exact-rational
/// coefficient (via [`node_to_acc`]/[`Acc::combine`], the same machinery
/// [`crate::numeric_fold`] uses); every other factor is decomposed into a
/// `(base, exponent)` pair via [`factor_to_base_exp`], and repeated bases
/// (`x*x`, or `x` and `x^2` together) have their exponents summed into one
/// entry — this *is* the "`x*x` → `x^2`" half of the fix, applied
/// uniformly to any repeated base, not just a literal `x*x`.
///
/// Flattens *nested* `Mul`s first (`flatten_mul_factors`): `expand_mul`
/// only ever wraps exactly two operands per call and never re-flattens
/// against an operand that is itself already a `Mul` — square-and-multiply
/// in particular routinely leaves a raw `Mul(Mul(a, a), a)` behind for
/// `a^3`, not the flat `Mul(a, a, a)` this function would otherwise need
/// to see to recognise all three factors as the same base.
fn monomialize_factors(raw_factors: &[IRNode]) -> (Acc, Vec<BaseTerm>) {
    let mut flat: Vec<&IRNode> = Vec::new();
    for factor in raw_factors {
        flatten_mul_factor(factor, &mut flat);
    }

    let mut coef = Acc::identity(true);
    let mut bases: Vec<BaseTerm> = Vec::new();
    for factor in flat {
        if let Some(val) = node_to_acc(factor) {
            coef = coef.combine(val, true);
            continue;
        }
        let (base, exp) = factor_to_base_exp(factor);
        let key = format!("{base:?}");
        if let Some(existing) = bases.iter_mut().find(|(k, _, _)| *k == key) {
            existing.2 += exp;
        } else {
            bases.push((key, base, exp));
        }
    }
    (coef, bases)
}

/// Recursively unwrap nested `Mul` structure, pushing every non-`Mul` leaf
/// factor onto `out` — see [`monomialize_factors`]'s doc comment for why
/// this nesting exists in `expand`'s raw output and must be flattened
/// before per-base exponent-summing can see every occurrence of a base.
fn flatten_mul_factor<'a>(node: &'a IRNode, out: &mut Vec<&'a IRNode>) {
    if let IRNode::Apply(app) = node {
        if is_head(&app.head, MUL) {
            for a in &app.args {
                flatten_mul_factor(a, out);
            }
            return;
        }
    }
    out.push(node);
}

/// Split `factor` into `(base, exponent)`: a `Pow(base, n)` with an
/// integer exponent `n` (of either sign — a negative exponent is left
/// exact, not specially handled, since summing exponents works the same
/// way regardless of sign) yields `(base, n)`; anything else — a bare
/// symbol, an unexpanded `Mul`/`Pow`/`Div`/transcendental `expand_mul`
/// refused or left alone, a `Pow` with a non-integer or symbolic exponent
/// — is treated as one opaque factor of its own, exponent `1`.
fn factor_to_base_exp(factor: &IRNode) -> (IRNode, i64) {
    if let IRNode::Apply(app) = factor {
        if is_head(&app.head, POW) && app.args.len() == 2 {
            if let IRNode::Integer(n) = app.args[1] {
                return (app.args[0].clone(), n);
            }
        }
    }
    (factor.clone(), 1)
}

/// Group `signed_terms` by monomial signature, sum coefficients, drop
/// exact-zero groups, and rebuild the survivors as `Add`/a single
/// term/`0`.
///
/// Sorts by signature first and merges adjacent runs in one linear pass —
/// `O(n log n)`, not the `O(n²)` a linear find-or-insert scan would cost
/// at `EXPAND_MAX_TERMS` scale (see the module docs' DoS note).
fn rebuild_additive(signed_terms: Vec<(Acc, Vec<BaseTerm>)>) -> IRNode {
    let mut keyed: Vec<(String, Acc, Vec<BaseTerm>)> = signed_terms
        .into_iter()
        .map(|(coef, monomial)| (monomial_signature(&monomial), coef, monomial))
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));

    let mut terms: Vec<IRNode> = Vec::new();
    let mut iter = keyed.into_iter();
    if let Some((mut cur_sig, mut cur_coef, mut cur_monomial)) = iter.next() {
        for (sig, coef, monomial) in iter {
            if sig == cur_sig {
                cur_coef = cur_coef.combine(coef, false);
            } else {
                if !cur_coef.is_identity(false) {
                    terms.push(rebuild_term(cur_coef, cur_monomial));
                }
                cur_sig = sig;
                cur_coef = coef;
                cur_monomial = monomial;
            }
        }
        if !cur_coef.is_identity(false) {
            terms.push(rebuild_term(cur_coef, cur_monomial));
        }
    }

    match terms.len() {
        0 => IRNode::Integer(0),
        1 => terms.into_iter().next().expect("len checked above"),
        _ => apply(sym(ADD), terms),
    }
}

/// The grouping key for a monomial: its sorted `(base_key, exponent)`
/// pairs joined into one string. Includes exponents (not just which bases
/// appear), so `x^2*y` and `x*y^2` — same base *set*, different powers —
/// are correctly treated as different monomials.
fn monomial_signature(monomial: &[BaseTerm]) -> String {
    let mut parts: Vec<String> = monomial
        .iter()
        .filter(|(_, _, exp)| *exp != 0)
        .map(|(key, _, exp)| format!("{key}^{exp}"))
        .collect();
    parts.sort();
    parts.join("*")
}

/// Rebuild one collected term: `coefficient * base1^exp1 * base2^exp2 *
/// ...`, omitting a coefficient of exactly `1` and a base's exponent of
/// exactly `1` (both the bare, unwrapped forms), and collapsing to a bare
/// numeric literal when the monomial has no surviving factors. Factors are
/// ordered by each base's own key string (`x` before `y`) — but this is
/// **not** the final user-facing order: [`crate::expand`] always runs
/// [`crate::simplify`] on `collect_terms`'s output afterward, and
/// `simplify`'s `canonical` pass re-sorts every `Mul`'s args by its own
/// `(type-rank, debug-string)` key (bare symbols, rank 3, before `Apply`
/// nodes like a `Pow`, rank 4 — regardless of the symbol's name), so
/// `expand()`'s actual output for e.g. `3*a^2*b` ends up `Mul(3, b,
/// Pow(a, 2))`, not `Mul(3, Pow(a, 2), b)`. Calling `collect_terms`
/// directly (as this module's own tests do) skips that later re-sort and
/// sees the order this function actually produces.
fn rebuild_term(coef: Acc, monomial: Vec<BaseTerm>) -> IRNode {
    if coef.is_identity(false) {
        // Exact zero coefficient: the whole term vanishes regardless of
        // its factors (0 * anything == 0). Only reachable defensively —
        // `rebuild_additive` already filters zero-coefficient groups
        // before calling this — but a bare `Mul` dispatched straight from
        // `collect_terms` (never grouped/filtered) could in principle
        // carry a literal zero factor, so this stays a real check, not
        // dead code.
        return coef.into_irnode();
    }

    let mut sorted = monomial;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let factor_nodes: Vec<IRNode> = sorted
        .into_iter()
        .filter(|(_, _, exp)| *exp != 0)
        .map(|(_, base, exp)| {
            if exp == 1 {
                base
            } else {
                apply(sym(POW), vec![base, IRNode::Integer(exp)])
            }
        })
        .collect();

    if factor_nodes.is_empty() {
        return coef.into_irnode();
    }
    if coef.is_identity(true) {
        return if factor_nodes.len() == 1 {
            factor_nodes.into_iter().next().expect("len checked above")
        } else {
            apply(sym(MUL), factor_nodes)
        };
    }
    let mut mul_args = vec![coef.into_irnode()];
    mul_args.extend(factor_nodes);
    apply(sym(MUL), mul_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbolic_ir::{apply, int, sym, DIV, SIN};

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
    fn repeated_symbol_terms_combine_into_one_coefficient() {
        // x + x -> 2*x
        let result = collect_terms(add(vec![sym("x"), sym("x")]));
        assert_eq!(result, mul(vec![int(2), sym("x")]));
    }

    #[test]
    fn repeated_multiplication_folds_into_a_power_standalone() {
        // x*x -> x^2, even with no surrounding Add.
        let result = collect_terms(mul(vec![sym("x"), sym("x")]));
        assert_eq!(result, pow(sym("x"), int(2)));
    }

    #[test]
    fn repeated_multiplication_folds_into_a_power_inside_a_sum() {
        // 1 + x + x + x*x -> 1 + 2*x + x^2
        let raw = add(vec![
            int(1),
            sym("x"),
            sym("x"),
            mul(vec![sym("x"), sym("x")]),
        ]);
        let result = collect_terms(raw);
        assert_eq!(
            result,
            add(vec![int(1), mul(vec![int(2), sym("x")]), pow(sym("x"), int(2))])
        );
    }

    #[test]
    fn opposite_signed_like_terms_cancel_to_zero() {
        // a*b - a*b -> 0
        let ab = mul(vec![sym("a"), sym("b")]);
        let result = collect_terms(sub(ab.clone(), ab));
        assert_eq!(result, int(0));
    }

    #[test]
    fn difference_of_squares_collects_cleanly() {
        // (a*a - a*b) + (a*b - b*b) -> a^2 - b^2, cross terms cancelled.
        let a = sym("a");
        let b = sym("b");
        let raw = add(vec![
            sub(mul(vec![a.clone(), a.clone()]), mul(vec![a.clone(), b.clone()])),
            sub(mul(vec![a.clone(), b.clone()]), mul(vec![b.clone(), b.clone()])),
        ]);
        let result = collect_terms(raw);
        assert_eq!(
            result,
            add(vec![pow(a, int(2)), mul(vec![int(-1), pow(b, int(2))])])
        );
    }

    #[test]
    fn same_base_set_different_exponents_are_different_monomials() {
        // x^2*y + x*y^2 must NOT collapse into one term — same base set,
        // different powers. Grouping orders by monomial signature string
        // ("x^1*y^2" sorts before "x^2*y^1" — the exponent digit differs
        // first), so xy2 comes first in collect_terms's own output (see
        // rebuild_term's doc comment on why this differs from the final
        // order expand()'s later canonical pass would produce).
        let x = sym("x");
        let y = sym("y");
        let x2y = mul(vec![pow(x.clone(), int(2)), y.clone()]);
        let xy2 = mul(vec![x.clone(), pow(y.clone(), int(2))]);
        let result = collect_terms(add(vec![x2y.clone(), xy2.clone()]));
        assert_eq!(result, add(vec![xy2, x2y]));
    }

    #[test]
    fn binomial_cube_collects_to_the_textbook_coefficients() {
        // (a+b)^3's raw 8-monomial expansion collects to a^3 + 3a^2b + 3ab^2 + b^3.
        let a = sym("a");
        let b = sym("b");
        let aaa = mul(vec![a.clone(), a.clone(), a.clone()]);
        let aab = mul(vec![a.clone(), a.clone(), b.clone()]);
        let abb = mul(vec![a.clone(), b.clone(), b.clone()]);
        let bbb = mul(vec![b.clone(), b.clone(), b.clone()]);
        let raw = add(vec![
            aaa,
            aab.clone(),
            aab.clone(),
            aab,
            abb.clone(),
            abb.clone(),
            abb,
            bbb,
        ]);
        // Grouping orders by monomial signature string: "a^1*b^2" <
        // "a^2*b^1" < "a^3" < "b^3" (exponent/base digits differ in that
        // order) — so a*b^2 comes first, then a^2*b, then the two pure
        // powers. Within each mixed term, factors sort by base key ("a"
        // before "b"), which is collect_terms's own order, not
        // necessarily what expand()'s later canonical pass would produce
        // (see rebuild_term's doc comment).
        let result = collect_terms(raw);
        assert_eq!(
            result,
            add(vec![
                mul(vec![int(3), a.clone(), pow(b.clone(), int(2))]),
                mul(vec![int(3), pow(a.clone(), int(2)), b.clone()]),
                pow(a, int(3)),
                pow(b, int(3)),
            ])
        );
    }

    #[test]
    fn exact_rational_coefficients_sum_precisely() {
        // (1/2)*x + (1/3)*x -> (5/6)*x, via the shared exact-rational Acc.
        let half_x = mul(vec![IRNode::rational(1, 2), sym("x")]);
        let third_x = mul(vec![IRNode::rational(1, 3), sym("x")]);
        let result = collect_terms(add(vec![half_x, third_x]));
        assert_eq!(result, mul(vec![IRNode::rational(5, 6), sym("x")]));
    }

    #[test]
    fn a_float_coefficient_contaminates_the_sum_to_float() {
        // Mirrors numeric_fold's own float-contamination rule: one float
        // anywhere in the accumulation makes the whole result float.
        let result = collect_terms(add(vec![
            mul(vec![symbolic_ir::flt(1.5), sym("x")]),
            sym("x"),
        ]));
        assert_eq!(result, mul(vec![symbolic_ir::flt(2.5), sym("x")]));
    }

    #[test]
    fn negative_integer_exponents_combine_the_same_way() {
        // x^-1 * x^-1 -> x^-2 (Pow with a negative exponent is a real,
        // already-supported shape -- expand_apply only distributes
        // 0..=EXPAND_MAX_POW, so a negative-exponent Pow is left alone by
        // the distributor and can appear as an ordinary Mul factor).
        let x_inv = pow(sym("x"), int(-1));
        let result = collect_terms(mul(vec![x_inv.clone(), x_inv]));
        assert_eq!(result, pow(sym("x"), int(-2)));
    }

    #[test]
    fn recurses_into_div_and_transcendental_children() {
        // Div/Sin aren't distributed, but their children still collect.
        let inner = add(vec![sym("x"), sym("x")]);
        let div_result = collect_terms(apply(sym(DIV), vec![inner.clone(), sym("y")]));
        assert_eq!(
            div_result,
            apply(sym(DIV), vec![mul(vec![int(2), sym("x")]), sym("y")])
        );
        let sin_result = collect_terms(apply(sym(SIN), vec![inner]));
        assert_eq!(
            sin_result,
            apply(sym(SIN), vec![mul(vec![int(2), sym("x")])])
        );
    }

    #[test]
    fn an_opaque_non_pow_factor_is_treated_as_a_single_base() {
        // Sin(x)*Sin(x) -> Sin(x)^2 -- any repeated non-Pow factor
        // consolidates the same way a repeated symbol does.
        let sin_x = apply(sym(SIN), vec![sym("x")]);
        let result = collect_terms(mul(vec![sin_x.clone(), sin_x.clone()]));
        assert_eq!(result, pow(sin_x, int(2)));
    }

    #[test]
    fn atoms_and_bare_symbols_pass_through_unchanged() {
        assert_eq!(collect_terms(int(5)), int(5));
        assert_eq!(collect_terms(sym("x")), sym("x"));
    }

    #[test]
    fn all_terms_cancelling_collapses_the_whole_sum_to_zero() {
        // x + (-1)*x -> 0, not a leftover Add or a stray 0 term.
        let result = collect_terms(add(vec![sym("x"), mul(vec![int(-1), sym("x")])]));
        assert_eq!(result, int(0));
    }
}


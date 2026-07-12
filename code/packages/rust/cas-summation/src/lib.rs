// A doc comment continues a bullet-list paragraph into a following prose
// paragraph without re-indenting; the wording is intentional, so allow it.
#![allow(clippy::doc_lazy_continuation)]
use std::cmp::Ordering;

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, EXP, LOG, MUL, NEG, POW, SUB};

pub mod gosper;
pub mod series_closed_forms;

pub const SUM: &str = "Sum";
pub const PRODUCT: &str = "Product";
pub const GAMMA_FUNC: &str = "GammaFunc";

pub use gosper::{try_gosper_sum, MAX_POLY_DEGREE};
pub use series_closed_forms::{bernoulli_rational, try_closed_form_series};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    numer: i64,
    denom: i64,
}

impl Rational {
    pub fn new(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "Rational denominator cannot be zero");
        let (mut numer, mut denom) = if denom < 0 {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let g = gcd(numer.unsigned_abs(), denom as u64) as i64;
        numer /= g;
        denom /= g;
        Self { numer, denom }
    }

    pub fn to_ir(self) -> IRNode {
        if self.denom == 1 {
            int(self.numer)
        } else {
            rat(self.numer, self.denom)
        }
    }
}

impl std::ops::Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numer * rhs.denom + rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }
}

impl std::ops::Sub for Rational {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numer * rhs.denom - rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }
}

impl std::ops::Mul for Rational {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }
}

impl std::ops::Div for Rational {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }
}

impl std::ops::Neg for Rational {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.numer, self.denom)
    }
}

pub fn rational_value(node: &IRNode) -> Option<Rational> {
    match node {
        IRNode::Integer(value) => Some(Rational::new(*value, 1)),
        IRNode::Rational(numer, denom) => Some(Rational::new(*numer, *denom)),
        _ => None,
    }
}

pub fn is_constant_in(node: &IRNode, k: &IRNode) -> bool {
    if node == k {
        return false;
    }
    match node {
        IRNode::Apply(apply_node) => apply_node.args.iter().all(|arg| is_constant_in(arg, k)),
        _ => true,
    }
}

pub fn geometric_sum_ir(
    coeff: IRNode,
    base: IRNode,
    lo: IRNode,
    hi: Option<IRNode>,
    is_infinite: bool,
) -> IRNode {
    let sum_part = if is_infinite {
        let one_minus_base = binary(SUB, int(1), base.clone());
        if matches!(lo, IRNode::Integer(0)) {
            binary(DIV, int(1), one_minus_base)
        } else {
            binary(DIV, binary(POW, base, lo), one_minus_base)
        }
    } else {
        let hi = hi.expect("hi must be provided for finite geometric sums");
        let span_plus_1 = binary(ADD, binary(SUB, hi, lo.clone()), int(1));
        let numerator = binary(SUB, binary(POW, base.clone(), span_plus_1), int(1));
        let denominator = binary(SUB, base.clone(), int(1));
        let ratio_part = binary(DIV, numerator, denominator);
        binary(MUL, binary(POW, base, lo), ratio_part)
    };

    if matches!(coeff, IRNode::Integer(1)) {
        sum_part
    } else {
        binary(MUL, coeff, sum_part)
    }
}

pub fn faulhaber_ir(m: i64, n: IRNode) -> Option<IRNode> {
    Some(match m {
        0 => n,
        1 => binary(DIV, binary(MUL, n.clone(), binary(ADD, n, int(1))), int(2)),
        2 => binary(
            DIV,
            binary(
                MUL,
                n.clone(),
                binary(
                    MUL,
                    binary(ADD, n.clone(), int(1)),
                    binary(ADD, binary(MUL, int(2), n), int(1)),
                ),
            ),
            int(6),
        ),
        3 => {
            let half = binary(DIV, binary(MUL, n.clone(), binary(ADD, n, int(1))), int(2));
            binary(POW, half, int(2))
        }
        4 => {
            let inner = binary(
                SUB,
                binary(
                    ADD,
                    binary(MUL, int(3), binary(POW, n.clone(), int(2))),
                    binary(MUL, int(3), n.clone()),
                ),
                int(1),
            );
            let two_n_plus_1 = binary(ADD, binary(MUL, int(2), n.clone()), int(1));
            binary(
                DIV,
                binary(
                    MUL,
                    n.clone(),
                    binary(
                        MUL,
                        binary(ADD, n, int(1)),
                        binary(MUL, two_n_plus_1, inner),
                    ),
                ),
                int(30),
            )
        }
        5 => {
            let inner = binary(
                SUB,
                binary(
                    ADD,
                    binary(MUL, int(2), binary(POW, n.clone(), int(2))),
                    binary(MUL, int(2), n.clone()),
                ),
                int(1),
            );
            binary(
                DIV,
                binary(
                    MUL,
                    binary(POW, n.clone(), int(2)),
                    binary(MUL, binary(POW, binary(ADD, n, int(1)), int(2)), inner),
                ),
                int(12),
            )
        }
        _ => return None,
    })
}

pub fn poly_sum_ir(m: i64, coeff: Rational, lo_val: i64, hi: IRNode) -> Option<IRNode> {
    let s_hi = faulhaber_ir(m, hi)?;
    let lo_minus_1 = lo_val - 1;
    let s_lo = if lo_minus_1 <= 0 {
        None
    } else {
        faulhaber_ir(m, int(lo_minus_1))
    };
    let mut diff = match s_lo {
        Some(s_lo) => binary(SUB, s_hi, s_lo),
        None => s_hi,
    };
    if lo_val == 0 && m == 0 {
        diff = binary(ADD, diff, int(1));
    }
    if coeff == Rational::new(1, 1) {
        Some(diff)
    } else {
        Some(binary(MUL, coeff.to_ir(), diff))
    }
}

pub fn try_special_infinite(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if is_int(lo, 1) && match_inv_k_pow(f, k, 2) {
        return Some(binary(DIV, binary(POW, sym("%pi"), int(2)), int(6)));
    }
    if is_int(lo, 1) && match_inv_k_pow(f, k, 4) {
        return Some(binary(DIV, binary(POW, sym("%pi"), int(4)), int(90)));
    }
    if is_int(lo, 0) && match_leibniz(f, k) {
        return Some(binary(DIV, sym("%pi"), int(4)));
    }
    if is_int(lo, 0) && match_inv_factorial(f, k) {
        return Some(sym("%e"));
    }
    if is_int(lo, 0) {
        if let Some(x) = match_exp_series(f, k) {
            return Some(unary(EXP, x));
        }
    }
    None
}

pub fn evaluate_sum<E>(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode, mut eval_fn: E) -> IRNode
where
    E: FnMut(IRNode) -> IRNode,
{
    // Wrap as a trait object so the recursive call inside
    // ``evaluate_sum_inner`` doesn't monomorphise to ever-deepening
    // ``&mut &mut ...`` layers and trip the compiler's recursion limit.
    let closure: &mut dyn FnMut(IRNode) -> IRNode = &mut eval_fn;
    evaluate_sum_inner(f, k, lo, hi, closure, false)
}

/// Track B2 (Rust port of Phase 40 + Phase 46 from Python ``symbolic-vm``
/// ``sum_handler``).  The ``apart_retried`` flag prevents infinite
/// recursion when the retry path falls back into the same pipeline —
/// Apart is attempted at most once per top-level call.
///
/// Apart-retry telescope chain
/// ---------------------------
/// When the structural telescope detector and every other narrow
/// recogniser fall through to the unevaluated ``Sum(...)`` shape AND
/// the summand is a proper rational ``Div(P(k), Q(k))``, we expand once
/// via ``Apart(f, k)`` (dispatched through the user-provided
/// ``eval_fn`` — typically a ``symbolic-vm`` VM with the Apart handler
/// installed) and retry ``evaluate_sum`` on the partial-fraction
/// decomposed shape.
///
/// The classic case is ``∑ 1/(k·(k+1))``: Apart emits
/// ``Add(Div(1, k), Neg(Div(1, k+1)))`` which the Phase 40+46
/// Add-with-negation normaliser rewrites to ``Sub(1/k, 1/(k+1))`` so the
/// telescope detector fires and Phase 41 emits ``1`` at infinity (since
/// ``1/(k+1) → 0``).  When ``eval_fn`` does not dispatch Apart the
/// attempt returns ``Apply(Apart, [f, k])`` which structurally differs
/// from ``f``, but the recursive retry won't close on it so the original
/// unevaluated Sum is preserved.
fn evaluate_sum_inner(
    f: IRNode,
    k: IRNode,
    lo: IRNode,
    hi: IRNode,
    eval_fn: &mut dyn FnMut(IRNode) -> IRNode,
    apart_retried: bool,
) -> IRNode {
    let inf_upper = is_inf(&hi);

    if is_constant_in(&f, &k) {
        let count = binary(ADD, binary(SUB, hi.clone(), lo.clone()), int(1));
        return eval_fn(binary(MUL, f, count));
    }

    if let Some((coeff, base)) = try_geometric(&f, &k) {
        let raw = geometric_sum_ir(coeff, base, lo.clone(), Some(hi.clone()), inf_upper);
        return eval_fn(raw);
    }

    if let Some((coeff, m)) = try_power_of_k(&f, &k) {
        if let IRNode::Integer(lo_int) = lo {
            if lo_int >= 0 && !inf_upper {
                if let Some(raw) = poly_sum_ir(m, coeff, lo_int, hi.clone()) {
                    return eval_fn(raw);
                }
            }
        }
    }

    // Phase 39 (finite) + Phase 41/42 (infinite) telescoping sums.
    //
    // Detect `f = g(k+1) − g(k)` (or its antisymmetric `g(k) − g(k+1)`)
    // and emit a closed form:
    // - Phase 39 (finite hi): g(hi+1) − g(lo) / g(lo) − g(hi+1).
    // - Phase 41+42 (hi = %inf): emit −g(lo) (standard) or g(lo)
    //   (antisymmetric) when g(k) provably vanishes at infinity per
    //   `g_vanishes_at_infinity` (constant numerator + positive-degree
    //   polynomial denominator, or any proper rational with
    //   deg(num) < deg(den)).  Otherwise fall through to the
    //   unevaluated SUM at the bottom.
    if let Some((g_expr, sign)) = try_telescoping(&f, &k, eval_fn) {
        if inf_upper {
            if g_vanishes_at_infinity(&g_expr, &k) {
                let g_at_lo = substitute(&g_expr, &k, &lo);
                let closed = if sign > 0 {
                    // ∑[g(k+1) − g(k)] from lo to ∞ = −g(lo)
                    apply(sym(NEG), vec![g_at_lo])
                } else {
                    // ∑[g(k) − g(k+1)] from lo to ∞ = g(lo)
                    g_at_lo
                };
                return eval_fn(closed);
            }
            // Limit not provably zero — fall through.
        } else {
            let hi_plus_one = binary(ADD, hi.clone(), int(1));
            let g_at_hi_plus_one = substitute(&g_expr, &k, &hi_plus_one);
            let g_at_lo = substitute(&g_expr, &k, &lo);
            let closed = if sign > 0 {
                binary(SUB, g_at_hi_plus_one, g_at_lo)
            } else {
                binary(SUB, g_at_lo, g_at_hi_plus_one)
            };
            return eval_fn(closed);
        }
    }

    if inf_upper {
        if let Some(raw) = try_special_infinite(&f, &k, &lo) {
            return eval_fn(raw);
        }
    }

    // Track I2 — closed-form transcendental infinite sums.  Recognises the
    // canonical zeta(2m), eta(2m), eta(1) = log(2), e_series, exp/cos/sin/
    // cosh/sinh Taylor series.  Mirrors the Python dispatch insertion point
    // (step 5a): placed after `try_special_infinite` so its pre-existing
    // patterns (Basel zeta(2)/zeta(4), Leibniz π/4) keep their IR shapes
    // and tests; `try_closed_form_series` only fires on patterns the
    // legacy handler refuses (e.g. ``Σ 1/k⁶``, the eta family, sin/cos/
    // sinh/cosh).
    if inf_upper {
        if let Some(raw) = series_closed_forms::try_closed_form_series(&f, &k, &lo, &hi) {
            return eval_fn(raw);
        }
    }

    if let (IRNode::Integer(lo_int), IRNode::Integer(hi_int)) = (&lo, &hi) {
        if (0..=999).contains(&(hi_int - lo_int)) {
            let mut total = Rational::new(0, 1);
            let mut ok = true;
            for value in *lo_int..=*hi_int {
                let term = substitute(&f, &k, &int(value));
                let evaluated = eval_fn(term);
                if let Some(r) = rational_value(&evaluated) {
                    total = total + r;
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                return total.to_ir();
            }
        }
    }

    // Track H2 — Gosper hypergeometric closed-form attempt.  Runs after
    // all narrow recognisers (constant, geometric, Faulhaber, telescoping,
    // special infinite series, small-range numeric) but before the
    // Apart-retry telescope chain and the unevaluated fallthrough.
    //
    // Mirrors the Python dispatch insertion point in
    // `cas_summation.summation` (step 5b): Gosper only runs for *finite*
    // upper bounds because it returns `T(hi+1) − T(lo)` which is only
    // meaningful when `hi+1` is a real value.  Infinite upper bounds
    // belong to the dedicated limit-aware paths above (telescope at ∞,
    // classic series).  This guard also preserves the Phase 41
    // fall-through contract for non-vanishing telescopes.
    if !inf_upper {
        if let Some(gosper_result) = gosper::try_gosper_sum(&f, &k, &lo, &hi) {
            return eval_fn(gosper_result);
        }
    }

    // Track B2 — Apart-retry telescope chain.  Mirrors the Python
    // ``sum_handler`` Phase 40 / Phase 46 retry path: when every direct
    // rule above fails on a rational summand, expand once via
    // ``Apart(f, k)`` (dispatched through the user-provided ``eval_fn``)
    // and try the whole pipeline again.  The ``apart_retried`` guard
    // pins the retry to at most one round, matching Python's structural
    // one-shot behaviour and guaranteeing termination.
    if !apart_retried && head_is_div(&f) {
        let apart_attempt = eval_fn(apply(sym(APART), vec![f.clone(), k.clone()]));
        if apart_attempt != f {
            // Apart emits two-term partial fractions as
            // ``Add(a, Neg(b))`` or ``Add(a, Div(-c, d))``.  The
            // structural telescope detector keys off ``Sub`` heads, so
            // normalise via the existing helper before retrying.
            let normalised = normalise_add_neg_to_sub(&apart_attempt);
            let retry = evaluate_sum_inner(normalised, k.clone(), lo.clone(), hi.clone(), eval_fn, true);
            // Only return when the retry actually closed (i.e. it is
            // no longer the unevaluated Sum head).  Otherwise fall
            // through and return the original unevaluated form below.
            if !is_unevaluated_sum(&retry) {
                return retry;
            }
        }
    }

    apply(sym(SUM), vec![f, k, lo, hi])
}

const APART: &str = "Apart";

fn head_is_div(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(a) if head_is(&a.head, DIV))
}

fn is_unevaluated_sum(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(a) if head_is(&a.head, SUM))
}

pub fn evaluate_product<E>(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode, mut eval_fn: E) -> IRNode
where
    E: FnMut(IRNode) -> IRNode,
{
    if let Some(raw) = evaluate_product_expr(f.clone(), k.clone(), lo.clone(), hi.clone()) {
        return eval_fn(raw);
    }

    if let (IRNode::Integer(lo_int), IRNode::Integer(hi_int)) = (&lo, &hi) {
        if (0..=20).contains(&(hi_int - lo_int)) {
            let mut total = Rational::new(1, 1);
            let mut ok = true;
            for value in *lo_int..=*hi_int {
                let term = substitute(&f, &k, &int(value));
                let evaluated = eval_fn(term);
                if let Some(r) = rational_value(&evaluated) {
                    total = total * r;
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                return total.to_ir();
            }
        }
    }

    apply(sym(PRODUCT), vec![f, k, lo, hi])
}

pub fn evaluate_product_expr(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode) -> Option<IRNode> {
    if is_constant_in(&f, &k) {
        let span = binary(ADD, binary(SUB, hi, lo), int(1));
        return Some(binary(POW, f, span));
    }
    if is_int(&lo, 1) && f == k {
        return Some(gamma(hi));
    }
    if is_int(&lo, 1) {
        if let Some(coeff) = split_linear_coeff(&f, &k) {
            if coeff == Rational::new(1, 1) {
                return Some(gamma(hi));
            }
            return Some(binary(
                MUL,
                binary(POW, coeff.to_ir(), hi.clone()),
                gamma(hi),
            ));
        }
    }
    None
}

fn try_geometric(f: &IRNode, k: &IRNode) -> Option<(IRNode, IRNode)> {
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, POW)
            && node.args.len() == 2
            && node.args[1] == *k
            && is_constant_in(&node.args[0], k)
            && node.args[0] != *k
        {
            return Some((int(1), node.args[0].clone()));
        }
        if head_is(&node.head, MUL) && node.args.len() == 2 {
            for (coeff, pow) in [
                (&node.args[0], &node.args[1]),
                (&node.args[1], &node.args[0]),
            ] {
                if let IRNode::Apply(pow_node) = pow {
                    if head_is(&pow_node.head, POW)
                        && pow_node.args.len() == 2
                        && pow_node.args[1] == *k
                        && pow_node.args[0] != *k
                        && is_constant_in(&pow_node.args[0], k)
                        && is_constant_in(coeff, k)
                    {
                        return Some((coeff.clone(), pow_node.args[0].clone()));
                    }
                }
            }
        }
    }
    None
}

fn try_power_of_k(f: &IRNode, k: &IRNode) -> Option<(Rational, i64)> {
    if f == k {
        return Some((Rational::new(1, 1), 1));
    }
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, POW) && node.args.len() == 2 && node.args[0] == *k {
            if let IRNode::Integer(m) = node.args[1] {
                if (0..=5).contains(&m) {
                    return Some((Rational::new(1, 1), m));
                }
            }
        }
        if head_is(&node.head, MUL) && node.args.len() == 2 {
            for (coeff, other) in [
                (&node.args[0], &node.args[1]),
                (&node.args[1], &node.args[0]),
            ] {
                if let Some(c) = rational_value(coeff) {
                    if other == k {
                        return Some((c, 1));
                    }
                    if let IRNode::Apply(other_node) = other {
                        if head_is(&other_node.head, POW)
                            && other_node.args.len() == 2
                            && other_node.args[0] == *k
                        {
                            if let IRNode::Integer(m) = other_node.args[1] {
                                if (0..=5).contains(&m) {
                                    return Some((c, m));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Phase 39: Detect a *structurally telescoping* summand
/// `f = g(k+1) − g(k)` (or its antisymmetric `g(k) − g(k+1)`).
///
/// Returns `(g_expr, sign)` where:
/// - `g_expr` is the expression representing `g(k)` — the "minus" half
///   of the SUB shape.  The closed form is then `g(hi+1) − g(lo)` for
///   `sign = +1` and `g(lo) − g(hi+1)` for `sign = -1`.
/// - Detection is purely structural: substitute `k → k+1` in one half
///   of the SUB and compare against the other half after `eval_fn`
///   normalisation.  No partial-fraction expansion is attempted — the
///   classic `1/(k(k+1))` form needs an explicit `Apart` step first.
/// Phase 40+46 (Rust port): Detect whether `node` represents a negation
/// and, if so, return the corresponding positive magnitude (the thing
/// that, prepended with a unary minus, equals `node`).
///
/// Two recognised shapes:
///
///   1.  Top-level `Neg(x)`                       → `x`
///   2.  `Div(c, d)` with literal `c < 0`         → `Div(|c|, d)`
///
/// Case 2 is the Phase 46 widening — Python's `Apart` of
/// `5/(k(k+1))` returns `Add(Div(-5, k+1), Div(5, k))` with the
/// negation folded into the numerator.  Even without `Apart` on the
/// Rust side, users who write the equivalent shape directly get the
/// benefit of the widened telescope detector.
///
/// Returns `None` when `node` is not a recognised negation.
fn extract_negation(node: &IRNode) -> Option<IRNode> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    // Case 1: top-level Neg wrapper.
    if head_is(&apply_node.head, NEG) && apply_node.args.len() == 1 {
        return Some(apply_node.args[0].clone());
    }
    // Case 2: Div with a negative literal numerator.
    if head_is(&apply_node.head, DIV) && apply_node.args.len() == 2 {
        let numer = &apply_node.args[0];
        let denom = &apply_node.args[1];
        match numer {
            IRNode::Integer(v) if *v < 0 => {
                return Some(binary(DIV, int(-v), denom.clone()));
            }
            IRNode::Rational(n, d) if *n < 0 => {
                // The IRNode::Rational invariant keeps denom > 0, so
                // negating the numerator alone flips the sign cleanly.
                return Some(binary(DIV, rat(-n, *d), denom.clone()));
            }
            IRNode::Float(v) if *v < 0.0 => {
                return Some(binary(DIV, IRNode::Float(-v), denom.clone()));
            }
            _ => {}
        }
    }
    None
}

/// Phase 40+46 (Rust port): Rewrite two-term `Add` nodes containing a
/// (recognised) negation into the equivalent `Sub` shape.  Used by
/// `try_telescoping` as a fallback when the direct `Sub` match fails.
///
/// Input shape                              | Output
/// -----------------------------------------+----------------------
/// `Add(a, Neg(b))`                         | `Sub(a, b)`
/// `Add(Neg(b), a)`                         | `Sub(a, b)`
/// `Add(a, Div(-c, d))` (Phase 46)          | `Sub(a, Div(c, d))`
/// `Add(Div(-c, d), a)` (Phase 46)          | `Sub(a, Div(c, d))`
/// `Add(Neg(a), Neg(b))`                    | unchanged
/// anything else                            | unchanged
fn normalise_add_neg_to_sub(node: &IRNode) -> IRNode {
    let apply_node = match node {
        IRNode::Apply(a) if head_is(&a.head, ADD) && a.args.len() == 2 => a,
        _ => return canonicalise_add_operand_order(node),
    };
    let left = &apply_node.args[0];
    let right = &apply_node.args[1];
    let left_pos = extract_negation(left);
    let right_pos = extract_negation(right);
    let rewritten = match (left_pos, right_pos) {
        // Both sides genuinely negative — no telescope to expose.
        (Some(_), Some(_)) => node.clone(),
        (_, Some(rp)) => binary(SUB, left.clone(), rp),
        (Some(lp), _) => binary(SUB, right.clone(), lp),
        (None, None) => node.clone(),
    };
    canonicalise_add_operand_order(&rewritten)
}

/// Track B2 (Rust port of Python ``_canonicalise_add_operand_order``):
/// deep-rewrite every ``Add`` so that numeric literals appear *last*
/// among its arguments.
///
/// The symbolic VM doesn't currently impose a canonical operand order on
/// ``Add``, so two structurally distinct trees can represent the same
/// mathematical expression — e.g. ``Add(k, 1)`` vs ``Add(1, k)``.  The
/// Phase 39 telescope detector relies on structural equality after
/// ``eval_fn``, so these must look identical for the Apart-rewritten
/// summand to match the substituted half (``Apart`` emits ``Add(1, k)``
/// while substitution produces ``Add(k, 1)``).
///
/// Walks the tree and sorts each ``Add``'s arguments so that integer /
/// rational / float literals come *last*, preserving the relative order
/// of non-literal children.
fn canonicalise_add_operand_order(node: &IRNode) -> IRNode {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return node.clone(),
    };
    let new_args: Vec<IRNode> = apply_node
        .args
        .iter()
        .map(canonicalise_add_operand_order)
        .collect();
    if head_is(&apply_node.head, ADD) && new_args.len() >= 2 {
        let (literals, non_literals): (Vec<IRNode>, Vec<IRNode>) =
            new_args.iter().cloned().partition(|arg| {
                matches!(
                    arg,
                    IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_)
                )
            });
        if !literals.is_empty() && !non_literals.is_empty() {
            let mut reordered = non_literals;
            reordered.extend(literals);
            if reordered != new_args {
                return apply(apply_node.head.clone(), reordered);
            }
        }
    }
    if new_args != apply_node.args {
        return apply(apply_node.head.clone(), new_args);
    }
    node.clone()
}

fn try_telescoping<E>(f: &IRNode, k: &IRNode, eval_fn: &mut E) -> Option<(IRNode, i32)>
where
    E: ?Sized + FnMut(IRNode) -> IRNode,
{
    // Phase 46: if f is an Add-with-negation shape, normalise to Sub
    // first so the existing structural match below fires.  No-op when
    // f is already a Sub or a non-Add shape.
    let normalised: IRNode;
    let f_ref: &IRNode = match f {
        IRNode::Apply(a) if head_is(&a.head, ADD) && a.args.len() == 2 => {
            normalised = normalise_add_neg_to_sub(f);
            &normalised
        }
        _ => f,
    };
    let node = match f_ref {
        IRNode::Apply(node) if head_is(&node.head, SUB) && node.args.len() == 2 => node,
        _ => return None,
    };
    let left = &node.args[0];
    let right = &node.args[1];
    let k_plus_one = binary(ADD, k.clone(), int(1));
    // Standard orientation: f = g(k+1) − g(k).  Check whether
    // substituting k → k+1 in `right` yields `left` (after normalisation).
    let right_shifted = substitute(right, k, &k_plus_one);
    if eval_fn(right_shifted) == eval_fn(left.clone()) {
        return Some((right.clone(), 1));
    }
    // Antisymmetric: f = g(k) − g(k+1).
    let left_shifted = substitute(left, k, &k_plus_one);
    if eval_fn(left_shifted) == eval_fn(right.clone()) {
        return Some((left.clone(), -1));
    }
    None
}

/// Phase 41+42: True when `node` is a polynomial in `k` of strictly
/// positive degree.  Used by `g_vanishes_at_infinity` to decide whether
/// a denominator grows without bound as `k → ∞`.
fn is_positive_degree_polynomial_in_k(node: &IRNode, k: &IRNode) -> bool {
    if node == k {
        return true;
    }
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    let head_str = match &apply_node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return false,
    };
    // k^n with n ≥ 1.
    if head_str == POW && apply_node.args.len() == 2 {
        let base = &apply_node.args[0];
        let exp = &apply_node.args[1];
        if base == k {
            if let IRNode::Integer(n) = exp {
                if *n >= 1 {
                    return true;
                }
            }
        }
    }
    if head_str == ADD && apply_node.args.len() >= 2 {
        if !apply_node
            .args
            .iter()
            .any(|a| is_positive_degree_polynomial_in_k(a, k))
        {
            return false;
        }
        return apply_node
            .args
            .iter()
            .all(|a| is_constant_in(a, k) || is_positive_degree_polynomial_in_k(a, k));
    }
    if head_str == MUL && apply_node.args.len() >= 2 {
        let mut has_positive = false;
        for arg in &apply_node.args {
            if is_constant_in(arg, k) {
                continue;
            }
            if is_positive_degree_polynomial_in_k(arg, k) {
                has_positive = true;
                continue;
            }
            return false;
        }
        return has_positive;
    }
    false
}

/// Phase 42: Return the polynomial degree of `node` in `k`, or `None`
/// for non-polynomial shapes.
fn polynomial_degree_in_k(node: &IRNode, k: &IRNode) -> Option<i64> {
    if is_constant_in(node, k) {
        return Some(0);
    }
    if node == k {
        return Some(1);
    }
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    let head_str = match &apply_node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return None,
    };
    if head_str == POW && apply_node.args.len() == 2 {
        let base = &apply_node.args[0];
        let exp = &apply_node.args[1];
        if base == k {
            if let IRNode::Integer(n) = exp {
                if *n >= 0 {
                    return Some(*n);
                }
            }
        }
        return None;
    }
    if head_str == NEG && apply_node.args.len() == 1 {
        return polynomial_degree_in_k(&apply_node.args[0], k);
    }
    if head_str == ADD || head_str == SUB {
        let mut max_deg: i64 = 0;
        for arg in &apply_node.args {
            {
                let d = polynomial_degree_in_k(arg, k)?;
                if d > max_deg {
                    max_deg = d;
                }
            }
        }
        return Some(max_deg);
    }
    if head_str == MUL {
        let mut sum_deg: i64 = 0;
        for arg in &apply_node.args {
            {
                let d = polynomial_degree_in_k(arg, k)?;
                sum_deg += d
            }
        }
        return Some(sum_deg);
    }
    None
}

/// Phase 41+42: True when `g(k)` provably tends to 0 as `k → ∞`.
///
/// Two-tier recognition:
///   1. Phase 41 fast path: `Div(c, h(k))` with `c` constant in `k` and
///      `h(k)` recognised as a positive-degree polynomial.
///   2. Phase 42 widening: `Div(P(k), Q(k))` with both pure polynomials
///      and `deg(P) < deg(Q)`.
///
/// Anything else (transcendental, improper rational, non-Div) returns
/// `false`.
/// Phase 43 helper: return the sign (+1 / -1) of the leading
/// coefficient of `node` as a polynomial in `k`, or `None` for
/// non-polynomial / degree-0 / unknown-sign shapes.
///
/// Required by `h_diverges_at_infinity` so we don't claim
/// `exp(-k)` or `2^(-k)` diverge (they vanish: `Mul(-1, k)` has
/// negative leading coefficient).
fn polynomial_leading_coeff_sign_in_k(node: &IRNode, k: &IRNode) -> Option<i64> {
    if is_constant_in(node, k) {
        return None;
    }
    if node == k {
        return Some(1);
    }
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    let head_str = match &apply_node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return None,
    };
    // k^n (n >= 1) → +1.
    if head_str == POW && apply_node.args.len() == 2 {
        let base = &apply_node.args[0];
        let exp = &apply_node.args[1];
        if base == k {
            if let IRNode::Integer(n) = exp {
                if *n >= 1 {
                    return Some(1);
                }
            }
        }
        return None;
    }
    // Neg(p) flips sign.
    if head_str == NEG && apply_node.args.len() == 1 {
        return polynomial_leading_coeff_sign_in_k(&apply_node.args[0], k).map(|s| -s);
    }
    // Mul(...): multiply signs of constant + k-bearing factors.
    if head_str == MUL {
        let mut sign: i64 = 1;
        let mut any_k_bearing = false;
        for arg in &apply_node.args {
            if is_constant_in(arg, k) {
                let val = rational_value(arg)?;
                if val.numer == 0 {
                    return None;
                }
                if val.numer < 0 {
                    sign = -sign;
                }
                continue;
            }
            let inner = polynomial_leading_coeff_sign_in_k(arg, k)?;
            if inner < 0 {
                sign = -sign;
            }
            any_k_bearing = true;
        }
        if any_k_bearing {
            return Some(sign);
        }
        return None;
    }
    // Add(...): dominated by highest-degree term; tied → refuse.
    if head_str == ADD {
        let mut max_deg: i64 = -1;
        let mut leader_sign: Option<i64> = None;
        let mut tied_at_max = false;
        for arg in &apply_node.args {
            let deg = polynomial_degree_in_k(arg, k)?;
            if deg == 0 {
                continue;
            }
            match deg.cmp(&max_deg) {
                Ordering::Greater => {
                    max_deg = deg;
                    leader_sign = polynomial_leading_coeff_sign_in_k(arg, k);
                    tied_at_max = false;
                }
                Ordering::Equal => {
                    tied_at_max = true;
                }
                Ordering::Less => {}
            }
        }
        if tied_at_max {
            return None;
        }
        return leader_sign;
    }
    None
}

/// Phase 43: True when `node` provably diverges to ±∞ as `k → ∞`.
///
/// Union of Phase 41/42 positive-degree polynomial + three transcendental
/// cases:
///   1. `Exp(h(k))` with h positive-degree AND positive leading coeff.
///   2. `Pow(b, h(k))` with rational |b| > 1 AND h positive-degree with
///      positive leading coefficient.
///   3. `Mul(...)` where at least one factor diverges and the rest are
///      constant-in-k or also diverging.  Recursive.
fn h_diverges_at_infinity(node: &IRNode, k: &IRNode) -> bool {
    // Phase 41/42 fast path.
    if is_positive_degree_polynomial_in_k(node, k) {
        return true;
    }
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    let head_str = match &apply_node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return false,
    };
    // Phase 43: Exp(h(k)) with h positive-degree and positive leading coeff.
    if head_str == EXP && apply_node.args.len() == 1 {
        let inner = &apply_node.args[0];
        if is_positive_degree_polynomial_in_k(inner, k) {
            return polynomial_leading_coeff_sign_in_k(inner, k) == Some(1);
        }
        return false;
    }
    // Phase 43: Pow(b, h(k)) with |b| > 1 rational and h → +∞.
    if head_str == POW && apply_node.args.len() == 2 {
        let base = &apply_node.args[0];
        let exp = &apply_node.args[1];
        if is_constant_in(base, k) {
            if let Some(base_val) = rational_value(base) {
                // |base| > 1 iff |numer| > denom (denom > 0 in
                // normalised Rational).  Compare in u64-space so that
                // `numer == i64::MIN` doesn't truncate after
                // `unsigned_abs()`.
                let abs_numer: u64 = base_val.numer.unsigned_abs();
                let denom_u: u64 = base_val.denom as u64; // denom > 0
                if abs_numer > denom_u
                    && is_positive_degree_polynomial_in_k(exp, k)
                    && polynomial_leading_coeff_sign_in_k(exp, k) == Some(1)
                {
                    return true;
                }
            }
        }
    }
    // Phase 43: Mul(...) — at least one factor diverges; others constant
    // in k or also diverging.  Recursive.
    if head_str == MUL && apply_node.args.len() >= 2 {
        let mut has_divergent = false;
        for arg in &apply_node.args {
            if is_constant_in(arg, k) {
                continue;
            }
            if h_diverges_at_infinity(arg, k) {
                has_divergent = true;
                continue;
            }
            return false;
        }
        return has_divergent;
    }
    // Phase 44: Log(h(k)) where h(k) → +∞.  Three sub-cases:
    //   - Polynomial h: positive leading coefficient required.
    //   - Exp inner: always positive; defer to recursion.
    //   - Pow(b, h') inner: require b > 1 strictly (Pow(-2, k) value
    //     oscillates in sign, log((-2)^k) not real).
    // Anything else (Log(const), Log(Sin), Log(Mul(...))) refused.
    if head_str == LOG && apply_node.args.len() == 1 {
        let inner = &apply_node.args[0];
        if is_positive_degree_polynomial_in_k(inner, k) {
            return polynomial_leading_coeff_sign_in_k(inner, k) == Some(1);
        }
        if let IRNode::Apply(inner_apply) = inner {
            let inner_head = match &inner_apply.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => return false,
            };
            if inner_head == EXP {
                return h_diverges_at_infinity(inner, k);
            }
            if inner_head == POW && inner_apply.args.len() == 2 {
                let base = &inner_apply.args[0];
                if is_constant_in(base, k) {
                    if let Some(base_val) = rational_value(base) {
                        // Strictly positive base > 1: numer > denom AND
                        // numer > 0 (denom > 0 in normalised Rational).
                        if base_val.numer > base_val.denom && base_val.numer > 0 {
                            return h_diverges_at_infinity(inner, k);
                        }
                    }
                }
            }
        }
        return false;
    }
    false
}

/// Phase 49 (Rust port): True when ``node`` is *provably* uniformly
/// bounded in ``k``.  Used by ``g_vanishes_at_infinity`` to recognise
/// shapes like ``sin(k)/k²``.
///
///   node shape                   | Provably bounded?
///   -----------------------------|----------------------------
///   constant in k                | yes (trivially)
///   Sin(any) / Cos(any)          | yes (|sin|, |cos| ≤ 1)
///   Mul(bounded, bounded)        | yes (recursive)
///   Add(bounded, bounded)        | yes (recursive)
///   Neg(bounded)                 | yes
///   anything else                | no (conservative)
fn is_bounded_in_k(node: &IRNode, k: &IRNode) -> bool {
    if is_constant_in(node, k) {
        return true;
    }
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    let head_name = match &apply_node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return false,
    };
    // Sin / Cos bounded by 1 in modulus.
    if head_name == "Sin" && apply_node.args.len() == 1 {
        return true;
    }
    if head_name == "Cos" && apply_node.args.len() == 1 {
        return true;
    }
    // Closure under Mul / Add / Neg.
    if head_name == MUL {
        return apply_node.args.iter().all(|a| is_bounded_in_k(a, k));
    }
    if head_name == ADD {
        return apply_node.args.iter().all(|a| is_bounded_in_k(a, k));
    }
    if head_name == NEG && apply_node.args.len() == 1 {
        return is_bounded_in_k(&apply_node.args[0], k);
    }
    false
}

/// Phase 50 (Rust port): True when ``node = Log(h(k))`` with
/// ``h(k) → +∞``.  Sign-aware via ``h_diverges_at_infinity``.
fn is_log_of_diverging_in_k(node: &IRNode, k: &IRNode) -> bool {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    if !matches!(&apply_node.head, IRNode::Symbol(s) if s == LOG) || apply_node.args.len() != 1 {
        return false;
    }
    h_diverges_at_infinity(node, k)
}

/// Phase 51 (Rust port): Return the effective polynomial half-degree
/// of ``Sqrt(P(k))`` when ``P`` is a positive-degree polynomial with
/// positive leading coefficient.  Returns ``None`` otherwise.
///
/// Returns ``deg(P) * 2`` (twice the half-degree) as an i64 to avoid
/// fractional arithmetic.  Callers compare with ``2 * den_deg`` to
/// preserve the inequality ``den_deg > deg(P)/2``.
fn sqrt_effective_half_degree_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !matches!(&apply_node.head, IRNode::Symbol(s) if s == "Sqrt") || apply_node.args.len() != 1 {
        return None;
    }
    let inner = &apply_node.args[0];
    let inner_deg = polynomial_degree_in_k(inner, k)?;
    if inner_deg < 1 {
        return None;
    }
    if polynomial_leading_coeff_sign_in_k(inner, k) != Some(1) {
        return None;
    }
    Some(inner_deg) // = 2 * (inner_deg / 2)
}

/// Phase 52 (Rust port): Return ``Some((bounded_aggregate, poly_degree))``
/// when ``node = Mul(bounded_factors, polynomial_factors)`` in ``k``;
/// ``None`` otherwise.
///
/// Used by ``g_vanishes_at_infinity`` to recognise that ``sin(k)·k/k³``
/// vanishes (bounded × deg 1 over deg 3).  The bounded part must contain
/// at least one non-constant-in-k factor — otherwise Phase 49 would catch
/// the whole numerator as a single bounded expression.
///
/// Algorithm:
///   1. Require ``node = Mul(...)``.
///   2. Partition each factor into bounded vs polynomial buckets.
///      Factors that are neither bounded nor polynomial → ``None``.
///   3. Require ≥ 1 non-constant-in-k bounded factor.
///   4. Sum the polynomial factors' degrees.
///   5. Return ``Some((aggregate, summed_poly_degree))``.
fn split_bounded_polynomial_factor(
    node: &IRNode,
    k: &IRNode,
) -> Option<(IRNode, i64)> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut bounded_factors: Vec<IRNode> = Vec::new();
    let mut poly_deg: i64 = 0;
    let mut has_non_constant_bounded = false;
    for arg in &apply_node.args {
        if is_bounded_in_k(arg, k) {
            if !is_constant_in(arg, k) {
                has_non_constant_bounded = true;
            }
            bounded_factors.push(arg.clone());
            continue;
        }
        {
            let d = polynomial_degree_in_k(arg, k)?;
            poly_deg += d
        }
    }
    // Pure polynomial — Phase 42 will handle it; no non-constant bounded factor.
    if !has_non_constant_bounded {
        return None;
    }
    if bounded_factors.is_empty() {
        return None;
    }
    let bounded_aggregate = if bounded_factors.len() == 1 {
        bounded_factors.remove(0)
    } else {
        apply(sym(MUL), bounded_factors)
    };
    Some((bounded_aggregate, poly_deg))
}

/// Phase 53 (Rust port): Return ``Some(sqrt_inner_deg + 2 * poly_deg_sum)``
/// when ``node = Mul(Sqrt(P), polynomial_factors)``; ``None`` otherwise.
///
/// The numerator ``Sqrt(P(k)) · Q(k)`` has effective growth rate
/// ``deg(P)/2 + deg(Q)``.  Returning ``deg(P) + 2·deg(Q)`` (= 2× the
/// effective degree) lets the caller compare against ``2 * den_deg``
/// to avoid float arithmetic.
///
/// Requirements:
///   - ``node = Mul(...)`` — Phase 51 handles the plain ``Sqrt(P)`` case.
///   - Exactly one factor passes ``sqrt_effective_half_degree_x2``.
///   - All remaining factors are polynomials in ``k``.
fn sqrt_poly_numerator_effective_degree_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut sqrt_inner_deg: Option<i64> = None;
    let mut poly_deg_sum: i64 = 0;
    for arg in &apply_node.args {
        // Try the Sqrt(P) shape first.
        if let Some(eff) = sqrt_effective_half_degree_x2(arg, k) {
            // Only one Sqrt factor allowed — bail on a second.
            if sqrt_inner_deg.is_some() {
                return None;
            }
            sqrt_inner_deg = Some(eff);
            continue;
        }
        // Otherwise must be polynomial in k.
        {
            let d = polynomial_degree_in_k(arg, k)?;
            poly_deg_sum += d
        }
    }
    // Must have found exactly one Sqrt factor.
    let sid = sqrt_inner_deg?;
    Some(sid + 2 * poly_deg_sum)
}

/// Phase 54 helper: split a `Mul` node into exactly one `Log(diverging)`
/// factor and a polynomial part in `k`.
///
/// Returns `Some((log_factor_ref, poly_deg_sum))` when:
///   - `node = Mul(...)`
///   - Exactly one factor passes `is_log_of_diverging_in_k`
///   - All remaining factors are polynomials in `k`
///
/// Returns `None` on any other shape (zero or two log factors, non-poly
/// non-log factor, or non-Mul node).
///
/// Used by `g_vanishes_at_infinity` (Phase 54) to recognise that
/// `log(h(k)) · P(k) / Q(k)` vanishes when `deg(Q) > deg(P)`.
fn split_log_polynomial_factor<'a>(node: &'a IRNode, k: &IRNode) -> Option<(&'a IRNode, i64)> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut log_factor: Option<&IRNode> = None;
    let mut poly_deg_sum: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            // Only one Log(diverging) factor is allowed.
            if log_factor.is_some() {
                return None;
            }
            log_factor = Some(arg);
            continue;
        }
        {
            let d = polynomial_degree_in_k(arg, k)?;
            poly_deg_sum += d
        }
    }
    let lf = log_factor?; // Must have found exactly one Log(diverging) factor.
    Some((lf, poly_deg_sum))
}

/// Phase 55 helper: return true when `node` is a `Mul` with exactly one
/// `Log(diverging)` factor and all remaining factors bounded in `k`.
///
/// The bounded part is uniformly bounded (|f| ≤ C) and `log(h(k))` grows
/// sub-polynomially, so their product is dominated by any polynomial or
/// faster-growing denominator.  This is the bounded-times-log complement
/// of Phase 52 (bounded × polynomial) and Phase 54 (log × polynomial).
///
/// Requirements:
///   - `node = Mul(...)`
///   - Exactly one factor passes `is_log_of_diverging_in_k`
///   - All remaining factors pass `is_bounded_in_k`
///   - Any other factor → return false
fn is_bounded_times_log_in_k(node: &IRNode, k: &IRNode) -> bool {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    if !head_is(&apply_node.head, MUL) {
        return false;
    }
    let mut log_count = 0usize;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            continue;
        }
        if is_bounded_in_k(arg, k) {
            continue;
        }
        // Factor is neither Log(diverging) nor bounded — unrecognised.
        return false;
    }
    log_count == 1
}

/// Phase 56 (Rust port): Return the ``Sqrt`` inner polynomial degree
/// (×2 to stay exact) when ``node`` is a ``Mul`` with exactly one
/// ``Sqrt(positive-leading polynomial)`` factor and all remaining
/// factors bounded in ``k``; ``None`` otherwise.
///
/// Mirror of ``is_bounded_times_log_in_k`` for sqrt instead of log.
/// Returns ``deg(P)`` (= 2 × half-degree) so callers can compare with
/// ``2 * den_deg`` in integer arithmetic.
fn bounded_times_sqrt_inner_deg(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut sqrt_inner_deg: Option<i64> = None;
    for arg in &apply_node.args {
        if let Some(deg) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_inner_deg.is_some() {
                // Two Sqrt factors — refuse (conservative).
                return None;
            }
            sqrt_inner_deg = Some(deg);
            continue;
        }
        if is_bounded_in_k(arg, k) {
            continue;
        }
        // Neither Sqrt(positive-poly) nor bounded → unrecognised.
        return None;
    }
    sqrt_inner_deg
}

/// Phase 57 (Rust port): Return the ``Sqrt`` inner polynomial degree (×2 to
/// stay exact in integer arithmetic) when ``node`` is a ``Mul`` with
/// **exactly one** ``Log(diverging)`` factor AND **exactly one**
/// ``Sqrt(positive-leading polynomial)`` factor, plus any number of bounded
/// factors; ``None`` otherwise.
///
/// Combines sub-polynomial ``Log`` growth with half-polynomial ``Sqrt``
/// growth.  Effective ``log(k)·k^{deg(P)/2}`` is strictly dominated by
/// ``k^{deg(P)/2+ε}`` for any ``ε > 0`` since ``log(k) = o(k^ε)``.
/// Caller compares ``2 * den_deg > deg(P)`` using the same ×2 integer
/// idiom as Phase 56's ``bounded_times_sqrt_inner_deg``.
///
/// Requires **both** Log and Sqrt — one-only patterns fall through to
/// Phase 55 (bounded × Log) or Phase 56 (bounded × Sqrt).  Two-of-either
/// is refused (conservative; combined growth-rate logic would be needed).
///
/// Algorithm:
///   1. Require ``node = Mul(...)``.
///   2. For each factor:
///      - ``Log(diverging)`` → count; refuse (return None) if count > 1.
///      - ``Sqrt(positive-poly)`` → record ×2 degree; refuse if second one.
///      - ``is_bounded_in_k`` → accept.
///      - otherwise → return None.
///   3. Require ``log_count == 1`` and ``sqrt_inner_deg.is_some()``.
fn bounded_log_sqrt_inner_deg(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut log_count = 0usize;
    let mut sqrt_inner_deg: Option<i64> = None;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 1 {
                // Two or more Log factors — refuse (conservative).
                return None;
            }
            continue;
        }
        if let Some(deg) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_inner_deg.is_some() {
                // Two Sqrt factors — refuse (conservative).
                return None;
            }
            sqrt_inner_deg = Some(deg);
            continue;
        }
        if is_bounded_in_k(arg, k) {
            continue;
        }
        // Unrecognised factor.
        return None;
    }
    if log_count != 1 {
        return None;
    }
    sqrt_inner_deg
}

/// Phase 58 (Rust port): Return the total polynomial degree when ``node`` is
/// a ``Mul`` with exactly one ``Log(diverging)`` factor, any polynomial
/// factors (total degree ``m``), and any bounded (non-polynomial) factors;
/// ``None`` otherwise.
///
/// Fills the gap between:
/// - **Phase 54** — ``Mul(Log, polynomial_only)``; refuses bounded factors.
/// - **Phase 55** — ``Mul(bounded, Log)``; refuses polynomial factors.
/// - **Phase 57** — ``Mul(bounded, Log, Sqrt)``; the Sqrt specialisation.
///
/// Effective growth ``log(k)·k^m = o(k^{m+ε})`` for any ``ε > 0``.
/// Caller compares ``den_deg > poly_deg`` (strictly) for polynomial
/// denominators, or short-circuits on non-polynomial diverging denominator.
///
/// Sqrt factors are refused here — use ``bounded_log_sqrt_inner_deg``
/// (Phase 57) for those.
///
/// Algorithm:
///   1. Require ``node = Mul(...)``.
///   2. For each factor:
///      - ``is_log_of_diverging_in_k`` → count; bail if count > 1.
///      - ``polynomial_degree_in_k`` → Some(d) → add d to ``poly_deg``.
///      - ``is_bounded_in_k`` → accept silently.
///      - otherwise (Sqrt, Exp, free diverging, …) → return None.
///   3. Require ``log_count == 1``.
///   4. Return ``poly_deg``.
fn bounded_log_poly_degree(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut log_count = 0usize;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 1 {
                // Two or more Log factors — refuse.
                return None;
            }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) {
            poly_deg += deg;
            continue;
        }
        if is_bounded_in_k(arg, k) {
            // Bounded but non-polynomial (e.g. Sin, Cos) — accept silently.
            continue;
        }
        // Sqrt or unrecognised factor — bail (Sqrt handled by Phase 57).
        return None;
    }
    if log_count != 1 {
        return None;
    }
    Some(poly_deg)
}

/// Return `Σ sqrt_inner_deg_x2 + 2·Σ poly_deg` when `node` is a `Mul` whose
/// factors split into any combination of:
///
/// * `Log(diverging)` factors (any count, including zero),
/// * `Sqrt(positive-leading polynomial)` factors (any count),
/// * polynomial factors in `k` (any count),
/// * bounded-in-`k` factors (any count, e.g. `Sin`, `Cos`, constants),
///
/// and at least one of the Log/Sqrt/polynomial factors is present.  Returns
/// `None` when any factor is unrecognised (e.g. `Exp(k)`) or when the
/// numerator is purely bounded.
///
/// **Phase 86 — cleanup.**  Supersedes the hand-written grid of
/// `N-Sqrt × M-Log × polynomial` helpers from Phases 59-85.  The convergence
/// math is identical for every non-negative `(N, M)` pair:
///
/// * Product of `N` `Log(diverging)` factors is sub-polynomial —
///   `log^N(k) = o(k^ε)` for any `ε > 0` — so `N` contributes 0 to the
///   effective growth degree.
/// * Each `Sqrt(P_i)` contributes `deg(P_i)/2` (recorded ×2 to stay in
///   integer arithmetic).
/// * Each polynomial factor `Q_j` contributes its own `deg(Q_j)`
///   (multiplied ×2 here).
/// * Bounded factors contribute 0.
///
/// Effective growth ×2:
///
/// ```text
/// effective_x2 = sum_i sqrt_inner_deg_x2(Sqrt(P_i))
///              + 2 * sum_j deg(Q_j)
/// ```
///
/// The caller compares `2 * den_deg > effective_x2` (polynomial denominator)
/// or short-circuits on non-polynomial diverging denominator.
///
/// Conservative refusals:
///
/// * Empty `Mul` (no recognised growth factor) → `None`.
/// * `Sqrt` of a polynomial whose leading coefficient is negative → `None`.
/// * Any unrecognised factor (Exp, free symbol, …) → `None`.
fn log_sqrt_poly_effective_x2_generic(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut sqrt_inner_deg_x2_sum: i64 = 0;
    let mut poly_deg_sum: i64 = 0;
    let mut found_log = false;
    let mut found_sqrt = false;
    let mut found_poly = false;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            found_log = true;
            continue;
        }
        if let Some(sqrt_deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_inner_deg_x2_sum += sqrt_deg_x2;
            found_sqrt = true;
            continue;
        }
        if is_bounded_in_k(arg, k) {
            // Constants and Sin/Cos/closures — contribute nothing.
            continue;
        }
        if let Some(poly_deg) = polynomial_degree_in_k(arg, k) {
            if poly_deg >= 1 {
                poly_deg_sum += poly_deg;
                found_poly = true;
                continue;
            }
        }
        // Unrecognised factor (Exp, free symbol, …) — bail.
        return None;
    }
    if !found_log && !found_sqrt && !found_poly {
        // Pure-bounded numerator — let Phase 49 handle it.
        return None;
    }
    Some(sqrt_inner_deg_x2_sum + 2 * poly_deg_sum)
}

fn vanishes_at_infinity(node: &IRNode, k: &IRNode) -> bool {
    if is_constant_in(node, k) {
        return rational_value(node).is_some_and(|value| value.numer == 0);
    }
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    let head_name = match &apply_node.head {
        IRNode::Symbol(s) => s.as_str(),
        _ => return false,
    };
    if head_name == NEG && apply_node.args.len() == 1 {
        return vanishes_at_infinity(&apply_node.args[0], k);
    }
    if head_name == ADD {
        return apply_node
            .args
            .iter()
            .all(|arg| vanishes_at_infinity(arg, k));
    }
    if head_name == EXP && apply_node.args.len() == 1 {
        let inner = &apply_node.args[0];
        return polynomial_degree_in_k(inner, k).is_some_and(|degree| degree > 0)
            && polynomial_leading_coeff_sign_in_k(inner, k) == Some(-1);
    }
    if head_name == POW && apply_node.args.len() == 2 {
        let base = &apply_node.args[0];
        let exp = &apply_node.args[1];
        if is_constant_in(base, k) {
            if let Some(base_val) = rational_value(base) {
                let abs_numer: u64 = base_val.numer.unsigned_abs();
                let denom_u: u64 = base_val.denom as u64;
                if abs_numer > denom_u {
                    return polynomial_degree_in_k(exp, k).is_some_and(|degree| degree > 0)
                        && polynomial_leading_coeff_sign_in_k(exp, k) == Some(-1);
                }
            }
        }
    }
    if head_name == MUL {
        let mut has_vanishing = false;
        for arg in &apply_node.args {
            if is_constant_in(arg, k) {
                if rational_value(arg).is_some_and(|value| value.numer == 0) {
                    return true;
                }
                continue;
            }
            if is_bounded_in_k(arg, k) {
                continue;
            }
            if vanishes_at_infinity(arg, k) {
                has_vanishing = true;
                continue;
            }
            return false;
        }
        return has_vanishing;
    }
    false
}

fn g_vanishes_at_infinity(g: &IRNode, k: &IRNode) -> bool {
    if vanishes_at_infinity(g, k) {
        return true;
    }
    let apply_node = match g {
        IRNode::Apply(a) => a,
        _ => return false,
    };
    if !matches!(&apply_node.head, IRNode::Symbol(s) if s == DIV) || apply_node.args.len() != 2 {
        return false;
    }
    let num = &apply_node.args[0];
    let den = &apply_node.args[1];
    // Phase 41/43 fast path: constant numerator + diverging denominator
    // (positive-degree polynomial OR exp / b^k transcendental).
    if is_constant_in(num, k) {
        return h_diverges_at_infinity(den, k);
    }
    // Phase 49: bounded numerator + diverging denominator.  Covers
    // shapes like sin(k)/k² and cos(k)·sin(k)/k³.
    if is_bounded_in_k(num, k) && h_diverges_at_infinity(den, k) {
        return true;
    }
    // Phase 50: Log(diverging) numerator + diverging denominator.
    // log/poly → 0 always (log grows slower than any positive power).
    if is_log_of_diverging_in_k(num, k) && h_diverges_at_infinity(den, k) {
        return true;
    }
    // Phase 51: Sqrt(positive-poly) numerator + polynomial denominator
    // with deg(den) > deg(P)/2.  Compare ``2 * den_deg`` against
    // ``inner_deg`` (which is ``2 * (deg/2)``) to avoid float arithmetic.
    if let Some(inner_deg) = sqrt_effective_half_degree_x2(num, k) {
        if let Some(den_deg) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg > inner_deg {
                return true;
            }
        }
    }
    // Phase 52: Mul(bounded, polynomial) numerator pattern.  When the
    // numerator factors as bounded × polynomial with positive poly degree,
    // the quotient vanishes iff den_deg > poly_deg.  Catches shapes like
    // sin(k)·k/k³ that Phase 49 misses (Mul isn't wholly bounded) and
    // Phase 42 refuses (sin is not polynomial).
    if let Some((_bounded, poly_deg)) = split_bounded_polynomial_factor(num, k) {
        if let Some(den_deg_bp) = polynomial_degree_in_k(den, k) {
            if den_deg_bp > poly_deg {
                return true;
            }
        }
    }
    // Phase 53: Mul(Sqrt(P), polynomial_factors) numerator pattern.
    // Effective growth = deg(P)/2 + deg(Q).  Using ×2 integer arithmetic:
    // vanishes when 2*den_deg > deg(P) + 2*deg(Q).
    if let Some(sqrt_poly_eff) = sqrt_poly_numerator_effective_degree_x2(num, k) {
        if let Some(den_deg_sp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sp > sqrt_poly_eff {
                return true;
            }
        }
    }
    // Phase 54: Mul(Log(diverging), polynomial_factors) numerator pattern.
    // log(h(k)) grows sub-polynomially (o(k^ε) for any ε > 0), so the
    // effective growth degree of log(h) · P(k) equals deg(P).  Vanishes
    // when den_deg > poly_deg (strictly).  Equal degrees are refused:
    // log(k) * constant diverges to ±∞.
    if let Some((_log_factor, poly_deg_lp)) = split_log_polynomial_factor(num, k) {
        if let Some(den_deg_lp) = polynomial_degree_in_k(den, k) {
            if den_deg_lp > poly_deg_lp {
                return true;
            }
        }
    }
    // Phase 55: Mul(bounded, Log(diverging)) numerator + diverging denominator.
    // bounded × log(h(k)) grows sub-polynomially — dominated by any
    // polynomial or faster-growing denominator.  Unlike Phase 54, we compare
    // against `h_diverges_at_infinity` (not a strict degree inequality) because
    // the numerator's effective polynomial degree is 0 (no polynomial factor).
    if is_bounded_times_log_in_k(num, k) && h_diverges_at_infinity(den, k) {
        return true;
    }
    // Phase 56: Mul(bounded, Sqrt(positive-poly)) numerator pattern.
    // Effective growth ``deg(P)/2``; closes when:
    //   - polynomial denominator with ``2 * den_deg > deg(P)``, OR
    //   - non-polynomial diverging denominator (Exp / Pow / Log×poly).
    if let Some(sqrt_inner_deg) = bounded_times_sqrt_inner_deg(num, k) {
        if let Some(den_deg_bs) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_bs > sqrt_inner_deg {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 57: Mul(bounded, Log(diverging), Sqrt(positive-poly)) numerator.
    // Effective growth ``log(k)·k^{deg(P)/2}`` dominated by any
    // ``k^{deg(P)/2+ε}``, so the quotient vanishes when:
    //   - polynomial denominator with ``2 * den_deg > deg(P)`` (×2 idiom), OR
    //   - non-polynomial diverging denominator (Exp / Pow / Log×poly).
    // Requires both Log and Sqrt; one-only falls through to Phase 55 / 56.
    if let Some(bls_inner_deg) = bounded_log_sqrt_inner_deg(num, k) {
        if let Some(den_deg_bls) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_bls > bls_inner_deg {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 58: Mul(bounded, Log(diverging), polynomial) numerator.
    // Effective growth ``log(k)·k^m = o(k^{m+ε})``.  Closes when:
    //   - polynomial denominator with ``den_deg > poly_deg`` (strictly), OR
    //   - non-polynomial diverging denominator (Exp / Pow / Log×poly).
    // Fills the gap between Phase 54 (Log × poly, refuses bounded) and
    // Phase 55 (bounded × Log, refuses poly).  Sqrt refused → Phase 57.
    if let Some(blp_deg) = bounded_log_poly_degree(num, k) {
        if let Some(den_deg_blp) = polynomial_degree_in_k(den, k) {
            if den_deg_blp > blp_deg {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // ---- Generic recogniser (Phase 86 cleanup) ----
    //
    // `Mul(bounded..., Log_1, ..., Log_N, Sqrt_1, ..., Sqrt_M, Poly_1, ..., Poly_K)`
    // over diverging denominator.  The product of any number of
    // `Log(diverging)` factors is still sub-polynomial
    // (`log^N(k) = o(k^ε)`), so `N` drops out of the comparison.  The Sqrt
    // factors contribute `Σ deg(P_i)/2` and the polynomial factors
    // contribute `Σ deg(Q_j)`.  Effective ×2:
    //
    //     effective_x2 = Σ sqrt_inner_deg_x2 + 2·Σ poly_deg
    //
    // Vanishes when `2·den_deg > effective_x2` (polynomial denominator) or
    // short-circuits on non-polynomial diverging denominator.
    //
    // Supersedes the hand-written grid of `N-Sqrt × M-Log × polynomial`
    // helpers (Phases 59-85): the math is identical for every (N, M) ≥ (0, 0).
    // The hardcoded helpers remain in place for now but are preempted by
    // this branch; a follow-up cleanup PR will delete them.
    if let Some(gen_x2) = log_sqrt_poly_effective_x2_generic(num, k) {
        if let Some(den_deg_gen) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_gen > gen_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 42 widening: deg(num) < deg(den) on pure polynomials.
    let num_deg = match polynomial_degree_in_k(num, k) {
        Some(d) => d,
        None => return false,
    };
    let den_deg = match polynomial_degree_in_k(den, k) {
        Some(d) => d,
        None => return false,
    };
    num_deg < den_deg
}

fn split_linear_coeff(f: &IRNode, k: &IRNode) -> Option<Rational> {
    if f == k {
        return Some(Rational::new(1, 1));
    }
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, MUL) && node.args.len() == 2 {
            let a = rational_value(&node.args[0]);
            let b = rational_value(&node.args[1]);
            if let Some(coeff) = a {
                if node.args[1] == *k {
                    return Some(coeff);
                }
            }
            if let Some(coeff) = b {
                if node.args[0] == *k {
                    return Some(coeff);
                }
            }
        }
    }
    None
}

fn match_inv_k_pow(f: &IRNode, k: &IRNode, exp: i64) -> bool {
    if let IRNode::Apply(node) = f {
        if head_is(&node.head, DIV) && node.args.len() == 2 && is_int(&node.args[0], 1) {
            if let IRNode::Apply(denom) = &node.args[1] {
                return head_is(&denom.head, POW)
                    && denom.args.len() == 2
                    && denom.args[0] == *k
                    && is_int(&denom.args[1], exp);
            }
        }
    }
    false
}

fn match_leibniz(f: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(node) = f else {
        return false;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return false;
    }
    let numerator = &node.args[0];
    let denominator = &node.args[1];
    let num_ok = match numerator {
        IRNode::Apply(num) if head_is(&num.head, POW) && num.args.len() == 2 => {
            num.args[1] == *k && (is_int(&num.args[0], -1) || is_neg_one(&num.args[0]))
        }
        _ => false,
    };
    if !num_ok {
        return false;
    }
    let IRNode::Apply(denom) = denominator else {
        return false;
    };
    if !head_is(&denom.head, ADD) || denom.args.len() != 2 {
        return false;
    }
    let (a, b) = (&denom.args[0], &denom.args[1]);
    is_two_k_plus_one(a, b, k) || is_two_k_plus_one(b, a, k)
}

fn match_inv_factorial(f: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(node) = f else {
        return false;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 || !is_int(&node.args[0], 1) {
        return false;
    }
    match_gamma_k_plus_one(&node.args[1], k)
}

fn match_exp_series(f: &IRNode, k: &IRNode) -> Option<IRNode> {
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    let IRNode::Apply(numer) = &node.args[0] else {
        return None;
    };
    if !head_is(&numer.head, POW) || numer.args.len() != 2 || numer.args[1] != *k {
        return None;
    }
    if numer.args[0] == *k || !match_gamma_k_plus_one(&node.args[1], k) {
        return None;
    }
    Some(numer.args[0].clone())
}

fn match_gamma_k_plus_one(node: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(gamma_node) = node else {
        return false;
    };
    if !head_is(&gamma_node.head, GAMMA_FUNC) || gamma_node.args.len() != 1 {
        return false;
    }
    let IRNode::Apply(arg) = &gamma_node.args[0] else {
        return false;
    };
    head_is(&arg.head, ADD) && arg.args.len() == 2 && arg.args[0] == *k && is_int(&arg.args[1], 1)
}

fn substitute(node: &IRNode, from: &IRNode, to: &IRNode) -> IRNode {
    if node == from {
        return to.clone();
    }
    match node {
        IRNode::Apply(apply_node) => apply(
            apply_node.head.clone(),
            apply_node
                .args
                .iter()
                .map(|arg| substitute(arg, from, to))
                .collect(),
        ),
        _ => node.clone(),
    }
}

fn binary(head: &str, a: IRNode, b: IRNode) -> IRNode {
    apply(sym(head), vec![a, b])
}

fn unary(head: &str, a: IRNode) -> IRNode {
    apply(sym(head), vec![a])
}

fn gamma(n: IRNode) -> IRNode {
    unary(GAMMA_FUNC, binary(ADD, n, int(1)))
}

fn head_is(node: &IRNode, name: &str) -> bool {
    matches!(node, IRNode::Symbol(actual) if actual == name)
}

fn is_int(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(actual) if *actual == value)
}

fn is_inf(node: &IRNode) -> bool {
    matches!(node, IRNode::Symbol(name) if name == "inf" || name == "%inf")
}

fn is_neg_one(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(apply_node)
        if head_is(&apply_node.head, NEG)
            && apply_node.args.len() == 1
            && is_int(&apply_node.args[0], 1))
}

fn is_two_k_plus_one(two_k: &IRNode, one: &IRNode, k: &IRNode) -> bool {
    if !is_int(one, 1) {
        return false;
    }
    matches!(two_k, IRNode::Apply(node)
        if head_is(&node.head, MUL)
            && node.args.len() == 2
            && is_int(&node.args[0], 2)
            && node.args[1] == *k)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    match a.cmp(&0) {
        Ordering::Equal => 1,
        Ordering::Greater => a,
        Ordering::Less => unreachable!(),
    }
}

use std::cmp::Ordering;

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, EXP, LOG, MUL, NEG, POW, SUB};

pub const SUM: &str = "Sum";
pub const PRODUCT: &str = "Product";
pub const GAMMA_FUNC: &str = "GammaFunc";

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
    if let Some((g_expr, sign)) = try_telescoping(&f, &k, &mut eval_fn) {
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

    apply(sym(SUM), vec![f, k, lo, hi])
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
        _ => return node.clone(),
    };
    let left = &apply_node.args[0];
    let right = &apply_node.args[1];
    let left_pos = extract_negation(left);
    let right_pos = extract_negation(right);
    match (left_pos, right_pos) {
        // Both sides genuinely negative — no telescope to expose.
        (Some(_), Some(_)) => node.clone(),
        (_, Some(rp)) => binary(SUB, left.clone(), rp),
        (Some(lp), _) => binary(SUB, right.clone(), lp),
        (None, None) => node.clone(),
    }
}

fn try_telescoping<E>(f: &IRNode, k: &IRNode, eval_fn: &mut E) -> Option<(IRNode, i32)>
where
    E: FnMut(IRNode) -> IRNode,
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
            match polynomial_degree_in_k(arg, k) {
                Some(d) => {
                    if d > max_deg {
                        max_deg = d;
                    }
                }
                None => return None,
            }
        }
        return Some(max_deg);
    }
    if head_str == MUL {
        let mut sum_deg: i64 = 0;
        for arg in &apply_node.args {
            match polynomial_degree_in_k(arg, k) {
                Some(d) => sum_deg += d,
                None => return None,
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
        match polynomial_degree_in_k(arg, k) {
            Some(d) => poly_deg += d,
            None => return None,   // Unrecognised factor.
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
        match polynomial_degree_in_k(arg, k) {
            Some(d) => poly_deg_sum += d,
            None => return None, // Neither Sqrt shape nor polynomial.
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
        match polynomial_degree_in_k(arg, k) {
            Some(d) => poly_deg_sum += d,
            None => return None, // Neither Log(diverging) nor polynomial — bail.
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

/// Return `sqrt_inner_deg + 2 * poly_deg` when `node` is a `Mul` with
/// exactly one `Sqrt(positive-leading polynomial P)` factor, any polynomial
/// factors (total degree `poly_deg`), and any number of bounded factors in
/// `k`; `None` otherwise.
///
/// Phase 59 — Bounded × Sqrt(P) × polynomial numerator.
///
/// Fills the gap between:
/// * **Phase 53** — `Mul(Sqrt, polynomial_only)`: refuses bounded factors.
/// * **Phase 56** — `Mul(bounded, Sqrt)`: refuses polynomial factors.
///
/// Effective growth: `C · k^{deg(P)/2 + poly_deg}`.  Using the ×2 integer
/// trick (same as Phase 51, 53, 56, 57): `effective_x2 = deg(P) + 2·poly_deg`.
/// The caller checks `2 * den_deg > effective_x2` to avoid floating-point
/// comparisons involving the half-integer degree.
///
/// Log factors are explicitly refused — that combination belongs to
/// Phase 57 (`bounded × Log × Sqrt`).
///
/// Algorithm:
///   1. Require `node = Mul(...)`.
///   2. For each factor:
///      - `Sqrt(positive-leading polynomial)` → record `×2` degree via
///        `sqrt_effective_half_degree_x2`; refuse if a second appears.
///      - `Log(diverging)` → immediately return `None` (Phase 57 territory).
///      - polynomial in `k` → accumulate degree.
///      - bounded (non-polynomial, non-Sqrt, non-Log) → accept silently.
///      - Anything else → return `None`.
///   3. Require exactly one Sqrt factor.
///   4. Return `sqrt_inner_deg + 2 * poly_deg`.
fn bounded_sqrt_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut sqrt_inner_deg: Option<i64> = None;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        // Sqrt(positive-leading polynomial) factor?
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_inner_deg.is_some() {
                // Two Sqrt factors — refuse.
                return None;
            }
            sqrt_inner_deg = Some(deg_x2);
            continue;
        }
        // Log factor — refuse (belongs to Phase 57 territory).
        if is_log_of_diverging_in_k(arg, k) {
            return None;
        }
        // Polynomial factor?
        if let Some(deg) = polynomial_degree_in_k(arg, k) {
            poly_deg += deg;
            continue;
        }
        // Bounded (non-polynomial, non-Sqrt, non-Log)?
        if is_bounded_in_k(arg, k) {
            continue;
        }
        // Unrecognised factor — bail.
        return None;
    }
    let sid = sqrt_inner_deg?;
    Some(sid + 2 * poly_deg)
}

/// Phase 60 (Rust port): Return ``sqrt_inner_deg + 2·poly_deg`` when ``node``
/// is a ``Mul`` with **exactly one** ``Log(diverging)`` factor, **exactly one**
/// ``Sqrt(positive-leading polynomial P)``, any polynomial factors (total
/// degree ``m``), and any number of bounded factors; ``None`` otherwise.
///
/// Closes the gap left by Phase 57 (``Mul(bounded, Log, Sqrt)``; refuses
/// polynomial factors).
///
/// # Growth analysis
///
/// ``log(k) · k^{deg(P)/2 + m}`` — log is sub-polynomial, so the dominant
/// term is the Sqrt×poly part.  Using the ×2 integer trick:
/// ``effective_x2 = deg(P) + 2·m``.
///
/// Caller checks ``2·den_deg > effective_x2`` (polynomial denominator) or
/// ``h_diverges_at_infinity`` (non-polynomial diverging denominator).
fn bounded_log_sqrt_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut log_count: usize = 0;
    let mut sqrt_inner_deg: Option<i64> = None;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        // Log(diverging) factor?
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 1 {
                // Two or more Log factors — refuse.
                return None;
            }
            continue;
        }
        // Sqrt(positive-poly) factor?
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_inner_deg.is_some() {
                // Two Sqrt factors — refuse (conservative).
                return None;
            }
            sqrt_inner_deg = Some(deg_x2);
            continue;
        }
        // Polynomial factor?
        if let Some(deg) = polynomial_degree_in_k(arg, k) {
            poly_deg += deg;
            continue;
        }
        // Bounded (non-polynomial, non-Sqrt, non-Log)?
        if is_bounded_in_k(arg, k) {
            continue;
        }
        // Unrecognised factor — bail.
        return None;
    }
    if log_count != 1 {
        return None;
    }
    let sid = sqrt_inner_deg?;
    Some(sid + 2 * poly_deg)
}

/// Phase 61 (Rust port): Return ``deg(P1) + deg(P2) + 2·poly_deg`` when
/// ``node`` is a ``Mul`` with **exactly two** ``Sqrt(positive-leading
/// polynomial)`` factors, any polynomial factors (total degree ``m``), and
/// any number of bounded factors; ``None`` otherwise.
///
/// Closes the gap where Phases 51, 53, 56, 59, 60 each require exactly one
/// Sqrt and hard-reject a second.
///
/// # Growth analysis (×2 trick)
///
/// ``Sqrt(P1(k)) · Sqrt(P2(k)) · k^m ≈ k^{deg(P1)/2 + deg(P2)/2 + m}``.
/// ``effective_x2 = deg(P1) + deg(P2) + 2·m``.
/// Caller checks ``2·den_deg > effective_x2``.
///
/// ``Log`` factors are refused (future Log×two-Sqrt phase territory).
fn two_sqrt_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node {
        IRNode::Apply(a) => a,
        _ => return None,
    };
    if !head_is(&apply_node.head, MUL) {
        return None;
    }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        // Sqrt(positive-leading polynomial) factor?
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(deg_x2);
            if sqrt_degs.len() > 2 {
                // Three or more Sqrt factors — refuse.
                return None;
            }
            continue;
        }
        // Log(diverging) factor — refuse.
        if is_log_of_diverging_in_k(arg, k) {
            return None;
        }
        // Polynomial factor?
        if let Some(deg) = polynomial_degree_in_k(arg, k) {
            poly_deg += deg;
            continue;
        }
        // Bounded (non-polynomial, non-Sqrt, non-Log)?
        if is_bounded_in_k(arg, k) {
            continue;
        }
        // Unrecognised factor — bail.
        return None;
    }
    if sqrt_degs.len() != 2 {
        return None;
    }
    Some(sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg)
}

/// Phase 62 — Two-Log × polynomial numerator.
///
/// Returns `2 * poly_deg_sum` when `node` is a `Mul` with **exactly two**
/// `Log(diverging-in-k)` factors, any polynomial factors (total degree `m`),
/// and any bounded factors; `None` otherwise.
///
/// `log(k)² · k^m` grows sub-polynomially, so `effective_x2 = 2 * m`.
/// Caller checks `2 * den_deg > effective_x2`.
///
/// Sqrt factors are refused (belong to the two-Sqrt / log-Sqrt family).
fn two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 2 { return None; }
            continue;
        }
        // Sqrt factor → refuse
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 2 { return None; }
    Some(2 * poly_deg)
}

/// Phase 67 — Three-Log × polynomial numerator.
///
/// Returns `2 * poly_deg_sum` when `node` is a `Mul` with **exactly three**
/// `Log(diverging-in-k)` factors, any polynomial factors (total degree `m`),
/// and any bounded factors; `None` otherwise.
///
/// `log(k)³ · k^m` grows sub-polynomially, so `effective_x2 = 2 * m`.
/// Caller checks `2 * den_deg > effective_x2`.
///
/// Sqrt factors are refused (belong to the sqrt-log family).
fn three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 3 { return None; }
            continue;
        }
        // Sqrt factor → refuse
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 3 { return None; }
    Some(2 * poly_deg)
}

/// Phase 63 — Two-Sqrt × Log × polynomial numerator.
///
/// Returns `deg(P1) + deg(P2) + 2 * poly_deg` when `node` is a `Mul` with
/// **exactly two** Sqrt factors, **exactly one** Log(diverging) factor,
/// any polynomial factors, and any bounded factors; `None` otherwise.
///
/// Log is sub-polynomial and doesn't change the effective degree.
/// Caller checks `2 * den_deg > effective_x2`.
fn two_sqrt_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(deg_x2);
            if sqrt_degs.len() > 2 { return None; }
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 1 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 2 || log_count != 1 { return None; }
    Some(sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg)
}

/// Phase 64 — Two-Log × Sqrt × polynomial numerator.
///
/// Returns `sqrt_inner_deg_x2 + 2 * poly_deg` when `node` is a `Mul` with
/// **exactly two** Log(diverging) factors, **exactly one** Sqrt factor,
/// any polynomial factors, and any bounded factors; `None` otherwise.
///
/// log² is sub-polynomial; effective_x2 = sqrt_deg_x2 + 2 * poly_deg.
/// Caller checks `2 * den_deg > effective_x2`.
fn two_log_sqrt_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 2 { return None; }
            continue;
        }
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt → refuse
            sqrt_deg_x2 = Some(deg_x2);
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 2 { return None; }
    sqrt_deg_x2.map(|d| d + 2 * poly_deg)
}

/// Phase 65 — Two-Sqrt × Two-Log × polynomial numerator.
///
/// Returns `deg(P1) + deg(P2) + 2 * poly_deg` when `node` is a `Mul` with
/// exactly two Sqrt factors, exactly two Log(diverging) factors,
/// any polynomial factors, and any bounded factors; `None` otherwise.
///
/// log² is sub-polynomial; effective_x2 = deg(P1) + deg(P2) + 2 * poly_deg.
/// Caller checks `2 * den_deg > effective_x2`.
fn two_sqrt_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(deg_x2);
            if sqrt_degs.len() > 2 { return None; }
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 2 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 2 || log_count != 2 { return None; }
    Some(sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg)
}

/// Phase 66 — Three-Sqrt × polynomial numerator.
///
/// Returns `deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg` when `node` is a `Mul`
/// with exactly three Sqrt factors, any polynomial factors, and any bounded
/// factors; `None` otherwise.
///
/// Log factors are rejected immediately (use Phase 63/64/65 for sqrt+log combos).
/// effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg.
/// Caller checks `2 * den_deg > effective_x2`.
fn three_sqrt_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(deg_x2) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(deg_x2);
            if sqrt_degs.len() > 3 { return None; }
            continue;
        }
        // Log factors not handled here — bail so Phase 63/64/65 can catch them.
        if is_log_of_diverging_in_k(arg, k) { return None; }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 3 { return None; }
    Some(sqrt_degs[0] + sqrt_degs[1] + sqrt_degs[2] + 2 * poly_deg)
}

/// Return `sqrt_deg1 + sqrt_deg2 + sqrt_deg3 + 2 * poly_deg` when `node` is a
/// `Mul` with **exactly three** `Sqrt(positive-leading polynomial)` factors,
/// **exactly one** `Log(diverging-in-k)` factor, any polynomial factors, and
/// any bounded factors; `None` otherwise.
///
/// Phase 68 — Three-Sqrt × Log × polynomial numerator.
///
/// The Log factor is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = d1 + d2 + d3 + 2·m`.
/// Caller checks `2 * den_deg > effective_x2`.
fn three_sqrt_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(d);
            if sqrt_degs.len() > 3 { return None; }
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 1 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 3 || log_count != 1 { return None; }
    Some(sqrt_degs[0] + sqrt_degs[1] + sqrt_degs[2] + 2 * poly_deg)
}

/// Return `sqrt_deg + 2 * poly_deg` when `node` is a `Mul` with **exactly one**
/// `Sqrt(positive-leading polynomial)` factor, **exactly three**
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors; `None` otherwise.
///
/// Phase 69 — One-Sqrt × Three-Log × polynomial numerator.
///
/// `log³(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = sqrt_deg + 2·poly_deg`.
/// Caller checks `2 * den_deg > effective_x2`.
fn one_sqrt_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 3 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let sd = sqrt_deg?;
    if log_count != 3 { return None; }
    Some(sd + 2 * poly_deg)
}

/// Phase 70 — Three-Sqrt × Two-Log × polynomial numerator.
///
/// `log²(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = d1 + d2 + d3 + 2·poly_deg` where di are the
/// doubled inner degrees of the three Sqrt factors.
/// Caller checks `2 * den_deg > effective_x2`.
fn three_sqrt_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(d);
            if sqrt_degs.len() > 3 { return None; } // more than three Sqrts — refuse
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 2 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 3 || log_count != 2 { return None; }
    Some(sqrt_degs.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 71 — Two-Sqrt × Three-Log × polynomial numerator.
///
/// `log³(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = d1 + d2 + 2·poly_deg` where d1, d2 are the
/// doubled inner degrees of the two Sqrt factors.
/// Caller checks `2 * den_deg > effective_x2`.
fn two_sqrt_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(d);
            if sqrt_degs.len() > 2 { return None; } // more than two Sqrts — refuse
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 3 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 2 || log_count != 3 { return None; }
    Some(sqrt_degs.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 72 — Three-Sqrt × Three-Log × polynomial numerator.
///
/// `log³(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = d1 + d2 + d3 + 2·poly_deg` where d1, d2, d3 are
/// the doubled inner degrees of the three Sqrt factors.
/// Caller checks `2 * den_deg > effective_x2`.
fn three_sqrt_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            sqrt_degs.push(d);
            if sqrt_degs.len() > 3 { return None; } // more than three Sqrts — refuse
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 3 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs.len() != 3 || log_count != 3 { return None; }
    Some(sqrt_degs.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 73 — Four-Log × polynomial numerator.
///
/// Returns `2·poly_deg` when `node` is a `Mul` with exactly four
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors; `None` otherwise.
///
/// `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = 2·poly_deg` (no Sqrt factors).
/// Sqrt factors are refused — use Sqrt × log phases for mixed forms.
/// Caller checks `2 * den_deg > effective_x2`.
fn four_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 4 { return None; }
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt → refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 4 { return None; }
    Some(2 * poly_deg)
}

/// Phase 74 — One-Sqrt × Four-Log × polynomial numerator.
///
/// Returns `sqrt_inner_deg_x2 + 2·poly_deg` when `node` is a `Mul` with
/// exactly one `Sqrt(positive-leading polynomial)` factor, exactly four
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors; `None` otherwise.
///
/// `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg`.
/// Caller checks `2 * den_deg > effective_x2`.
fn one_sqrt_four_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 4 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let d = sqrt_deg_x2?;
    if log_count != 4 { return None; }
    Some(d + 2 * poly_deg)
}

/// Phase 78 — One-Sqrt × Five-Log × polynomial numerator.
///
/// Returns `sqrt_deg_x2 + 2·poly_deg` when `node` is a `Mul` with exactly one
/// `Sqrt(positive-leading polynomial)` factor, exactly five `Log(diverging-in-k)`
/// factors, any polynomial factors, and any bounded factors; `None` otherwise.
///
/// `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// `effective_x2 = sqrt_inner_deg_x2 + 2·m`.
/// Caller checks `2 * den_deg > effective_x2`.
fn one_sqrt_five_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 5 { return None; } // six or more Logs — refuse
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let d = sqrt_deg_x2?;
    if log_count != 5 { return None; }
    Some(d + 2 * poly_deg)
}

/// Phase 75 — Two-Sqrt × Four-Log × polynomial numerator.
///
/// Returns `sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg` when `node` is a `Mul` with
/// exactly two `Sqrt(positive-leading polynomial)` factors, exactly four
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors; `None` otherwise.
///
/// `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to the effective
/// degree.  `effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`.
/// Caller checks `2 * den_deg > effective_x2`.
fn two_sqrt_four_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 4 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 4 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Return `sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2 * poly_deg` when `node`
/// is a `Mul` with **exactly three** `Sqrt(positive-leading polynomial)` factors,
/// **exactly four** `Log(diverging-in-k)` factors, any polynomial factors,
/// and any bounded factors; `None` otherwise.
///
/// Phase 76 — Three-Sqrt × Four-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁴ · k^m ≈ k^{(a+b+c)/2} · log⁴(k) · k^m`.
/// `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0. Using the ×2 integer trick:
/// `effective_x2 = a + b + c + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_four_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 4 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 4 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 77 — Five-Log × polynomial numerator.
///
/// Returns `2 * poly_deg` when `node` is a `Mul` with **exactly five**
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors; `None` otherwise.
///
/// `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0.  Using the ×2 integer trick:
/// `effective_x2 = 2·m`.  Caller checks `2·den_deg > effective_x2`.
///
/// Sqrt factors are explicitly refused so this phase does not shadow the
/// Sqrt-bearing phases (73–76, 78+).
fn five_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 5 { return None; } // six or more Logs — not this phase
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt — refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 5 { return None; }
    Some(2 * poly_deg)
}

/// Phase 79 — Two-Sqrt × Five-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁵ · k^m ≈ k^{(a+b)/2} · log⁵(k) · k^m`.
/// `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0. Using the ×2 integer trick:
/// `effective_x2 = a + b + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_five_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 5 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 5 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Phase 80 — Three-Sqrt × Five-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁵ · k^m ≈ k^{(a+b+c)/2} · log⁵(k) · k^m`.
/// `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0. Using the ×2 integer trick:
/// `effective_x2 = a + b + c + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_five_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 5 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 5 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 81 — Four-Sqrt × Five-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)⁵ · k^m
/// ≈ k^{(a+b+c+d)/2} · log⁵(k) · k^m`.
/// `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0. Using the ×2 integer trick:
/// `effective_x2 = a + b + c + d + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_five_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 5 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 5 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + sqrt_degs_x2[3] + 2 * poly_deg)
}

/// Phase 82 — Five-Sqrt × Five-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a₁)·…·sqrt(k^a₅)·log(k)⁵·k^m ≈ k^{(a₁+…+a₅)/2}·log⁵(k)·k^m`.
/// `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0. Using the ×2 integer trick:
/// `effective_x2 = a₁+a₂+a₃+a₄+a₅ + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_five_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 5 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 5 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2]
         + sqrt_degs_x2[3] + sqrt_degs_x2[4] + 2 * poly_deg)
}

/// Phase 83 — Six-Log × polynomial numerator.
///
/// Effective growth: `log(k)⁶ · k^m`. `log⁶(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn six_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 6 { return None; } // seven or more Logs — not this phase
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt — refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 6 { return None; }
    Some(2 * poly_deg)
}

/// Phase 84 — One-Sqrt × Six-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)⁶ · k^m`. `log⁶(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_six_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 6 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 6 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 89 — Seven-Log × polynomial numerator.
///
/// Effective growth: `log(k)⁷ · k^m`. `log⁷(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Sqrt factors are explicitly refused so Sqrt-bearing phases handle them.
/// Caller checks `2·den_deg > effective_x2`.
fn seven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 7 { return None; } // eight or more Logs — not this phase
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt — refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 7 { return None; }
    Some(2 * poly_deg)
}

/// Phase 95 — Eight-Log × polynomial numerator (zero Sqrt).
///
/// Effective growth: `log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Sqrt factors are explicitly refused.
/// Caller checks `2·den_deg > effective_x2`.
fn eight_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 8 { return None; } // nine or more Logs — not this phase
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt — refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 8 { return None; }
    Some(2 * poly_deg)
}

/// Phase 101 — Nine-Log × polynomial numerator.
///
/// Effective growth: `log(k)⁹ · k^m`. `log⁹(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Zero Sqrt factors required; Sqrt presence causes immediate refusal.
/// Caller checks `2·den_deg > effective_x2`.
fn nine_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 9 { return None; } // ten or more Logs — not this phase
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt — refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 9 { return None; }
    Some(2 * poly_deg)
}

/// Phase 102 — One-Sqrt × Nine-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)⁹ · k^m`. `log⁹(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_nine_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 9 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_deg_x2.is_none() || log_count != 9 { return None; }
    Some(sqrt_deg_x2.unwrap() + 2 * poly_deg)
}

/// Phase 103 — Two-Sqrt × Nine-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁹ · k^m`.
/// `effective_x2 = a + b + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_nine_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 9 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 9 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 104 — Three-Sqrt × Nine-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁹ · k^m`.
/// `effective_x2 = a + b + c + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_nine_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 9 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 9 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 105 — Four-Sqrt × Nine-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a)×4 · log(k)⁹ · k^m`.
/// `effective_x2 = a+b+c+d + 2·m`. Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_nine_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 9 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 9 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 106 — Five-Sqrt × Nine-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a)×5 · log(k)⁹ · k^m`.
/// `effective_x2 = a+b+c+d+e + 2·m`. Caller checks `2·den_deg > effective_x2`.
/// Completes the Nine-Log family (Phases 101–106).
fn five_sqrt_nine_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 9 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 9 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

fn ten_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 10 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 10 { return None; }
    Some(2 * poly_deg)
}

fn one_sqrt_ten_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; }
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 10 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_deg_x2.is_none() || log_count != 10 { return None; }
    Some(sqrt_deg_x2.unwrap() + 2 * poly_deg)
}

fn two_sqrt_ten_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 10 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 10 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

fn three_sqrt_ten_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 10 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 10 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

fn four_sqrt_ten_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 10 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 10 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

fn five_sqrt_ten_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 10 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 10 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 113 — Eleven-Log × polynomial numerator (no Sqrt factors).
///
/// Effective growth: `log(k)¹¹ · k^m`. `log¹¹(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn eleven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 11 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 11 { return None; }
    Some(2 * poly_deg)
}

/// Phase 114 — One-Sqrt × Eleven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹¹ · k^m`. `log¹¹(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_eleven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 11 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 11 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 115 — Two-Sqrt × Eleven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹¹ · k^m`. `log¹¹(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_eleven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 11 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 11 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 116 — Three-Sqrt × Eleven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹¹ · k^m`. `log¹¹(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_eleven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 11 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 11 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 117 — Four-Sqrt × Eleven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹¹ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_eleven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 11 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 11 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 118 — Five-Sqrt × Eleven-Log × polynomial numerator.
/// Completes the Eleven-Log family (Phases 113-118).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹¹ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_eleven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 11 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 11 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 119 — Twelve-Log × polynomial numerator (no Sqrt factors).
///
/// Effective growth: `log(k)¹² · k^m`. `log¹²(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn twelve_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 12 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 12 { return None; }
    Some(2 * poly_deg)
}

/// Phase 120 — One-Sqrt × Twelve-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹² · k^m`. `log¹²(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_twelve_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 12 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 12 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 121 — Two-Sqrt × Twelve-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹² · k^m`. `log¹²(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_twelve_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 12 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 12 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 122 — Three-Sqrt × Twelve-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹² · k^m`. `log¹²(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_twelve_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 12 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 12 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 123 — Four-Sqrt × Twelve-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹² · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_twelve_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 12 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 12 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 124 — Five-Sqrt × Twelve-Log × polynomial numerator.
/// Completes the Twelve-Log family (Phases 119-124).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹² · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_twelve_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 12 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 12 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 125 — Thirteen-Log × polynomial numerator (no Sqrt factors).
///
/// Effective growth: `log(k)¹³ · k^m`. `log¹³(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn thirteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 13 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 13 { return None; }
    Some(2 * poly_deg)
}

/// Phase 126 — One-Sqrt × Thirteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹³ · k^m`. `log¹³(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_thirteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 13 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 13 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 127 — Two-Sqrt × Thirteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹³ · k^m`. `log¹³(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_thirteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 13 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 13 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 128 — Three-Sqrt × Thirteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹³ · k^m`. `log¹³(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_thirteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 13 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 13 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 129 — Four-Sqrt × Thirteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹³ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_thirteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 13 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 13 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 130 — Five-Sqrt × Thirteen-Log × polynomial numerator.
/// Completes the Thirteen-Log family (Phases 125-130).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹³ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_thirteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 13 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 13 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 131 — Fourteen-Log × polynomial numerator.
///
/// Effective growth: `log(k)¹⁴ · k^m`. `log¹⁴(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn fourteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 14 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 14 { return None; }
    Some(2 * poly_deg)
}

/// Phase 132 — One-Sqrt × Fourteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹⁴ · k^m`. `log¹⁴(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_fourteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 14 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 14 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 133 — Two-Sqrt × Fourteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹⁴ · k^m`. `log¹⁴(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_fourteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 14 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 14 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 134 — Three-Sqrt × Fourteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹⁴ · k^m`. `log¹⁴(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_fourteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 14 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 14 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 135 — Four-Sqrt × Fourteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹⁴ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_fourteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 14 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 14 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 136 — Five-Sqrt × Fourteen-Log × polynomial numerator.
/// Completes the Fourteen-Log family (Phases 131-136).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹⁴ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_fourteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 14 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 14 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 137 — Fifteen-Log × polynomial numerator.
///
/// Effective growth: `log(k)¹⁵ · k^m`. `log¹⁵(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
/// Sqrt factors are explicitly refused so this does not shadow Phase 138 onward.
fn fifteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 15 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 15 { return None; }
    Some(2 * poly_deg)
}

/// Phase 138 — One-Sqrt × Fifteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹⁵ · k^m`. `log¹⁵(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_fifteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 15 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 15 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 139 — Two-Sqrt × Fifteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹⁵ · k^m`. `log¹⁵(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_fifteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 15 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 15 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 140 — Three-Sqrt × Fifteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹⁵ · k^m`. `log¹⁵(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_fifteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 15 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 15 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 141 — Four-Sqrt × Fifteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹⁵ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_fifteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 15 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 15 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 142 — Five-Sqrt × Fifteen-Log × polynomial numerator.
/// Completes the Fifteen-Log family (Phases 137-142).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹⁵ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_fifteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 15 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 15 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 143 — Sixteen-Log × polynomial numerator.
///
/// Effective growth: `log(k)¹⁶ · k^m`. `log¹⁶(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
/// Sqrt factors are explicitly refused so this does not shadow Phase 144 onward.
fn sixteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 16 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 16 { return None; }
    Some(2 * poly_deg)
}

/// Phase 144 — One-Sqrt × Sixteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹⁶ · k^m`. `log¹⁶(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_sixteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 16 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 16 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 145 — Two-Sqrt × Sixteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹⁶ · k^m`. `log¹⁶(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_sixteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 16 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 16 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 146 — Three-Sqrt × Sixteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹⁶ · k^m`. `log¹⁶(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_sixteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 16 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 16 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 147 — Four-Sqrt × Sixteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹⁶ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_sixteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 16 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 16 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 148 — Five-Sqrt × Sixteen-Log × polynomial numerator.
/// Completes the Sixteen-Log family (Phases 143-148).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹⁶ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_sixteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 16 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 16 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 149 — Seventeen-Log × polynomial numerator.
///
/// Effective growth: `log(k)¹⁷ · k^m`. `log¹⁷(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
/// Sqrt factors are explicitly refused so this does not shadow Phase 150 onward.
fn seventeen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 17 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 17 { return None; }
    Some(2 * poly_deg)
}

/// Phase 150 — One-Sqrt × Seventeen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹⁷ · k^m`. `log¹⁷(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_seventeen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 17 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 17 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 151 — Two-Sqrt × Seventeen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹⁷ · k^m`. `log¹⁷(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_seventeen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 17 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 17 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 152 — Three-Sqrt × Seventeen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹⁷ · k^m`. `log¹⁷(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_seventeen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 17 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 17 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 153 — Four-Sqrt × Seventeen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹⁷ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_seventeen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 17 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 17 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 154 — Five-Sqrt × Seventeen-Log × polynomial numerator.
/// Completes the Seventeen-Log family (Phases 149-154).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹⁷ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_seventeen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 17 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 17 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 155 — Eighteen-Log × polynomial numerator.
///
/// Effective growth: `log(k)¹⁸ · k^m`. `log¹⁸(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
/// Sqrt factors are explicitly refused so this does not shadow Phase 156 onward.
fn eighteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 18 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 18 { return None; }
    Some(2 * poly_deg)
}

/// Phase 156 — One-Sqrt × Eighteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹⁸ · k^m`. `log¹⁸(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_eighteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 18 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 18 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 157 — Two-Sqrt × Eighteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹⁸ · k^m`. `log¹⁸(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_eighteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 18 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 18 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 158 — Three-Sqrt × Eighteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹⁸ · k^m`. `log¹⁸(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_eighteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 18 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 18 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 159 — Four-Sqrt × Eighteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹⁸ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_eighteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 18 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 18 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 160 — Five-Sqrt × Eighteen-Log × polynomial numerator.
/// Completes the Eighteen-Log family (Phases 155-160).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹⁸ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_eighteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 18 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 18 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 161 — Nineteen-Log × polynomial numerator.
/// Opens the Nineteen-Log family (Phases 161-166).
///
/// Effective growth: `log(k)¹⁹ · k^m`. `log¹⁹(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn nineteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 19 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 19 { return None; }
    Some(2 * poly_deg)
}

/// Phase 162 — One-Sqrt × Nineteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)¹⁹ · k^m`. `log¹⁹(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_nineteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 19 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 19 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 163 — Two-Sqrt × Nineteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)¹⁹ · k^m`. `log¹⁹(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_nineteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 19 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 19 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 164 — Three-Sqrt × Nineteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)¹⁹ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_nineteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 19 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 19 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 165 — Four-Sqrt × Nineteen-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)¹⁹ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_nineteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 19 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 19 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 166 — Five-Sqrt × Nineteen-Log × polynomial numerator.
/// Completes the Nineteen-Log family (Phases 161-166).
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)¹⁹ · k^m`.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_nineteen_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; }
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 19 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 19 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

fn twenty_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 20 { return None; } // twenty-one or more Logs — not this phase
            continue;
        }
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; } // Sqrt — refuse
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 20 { return None; }
    Some(2 * poly_deg)
}

/// Phase 168 — One-Sqrt × Twenty-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)²⁰ · k^m`. `log²⁰(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_twenty_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 20 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 20 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 169 — Two-Sqrt × Twenty-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)²⁰ · k^m`. `log²⁰(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_twenty_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 20 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 20 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Phase 170 — Three-Sqrt × Twenty-Log × polynomial numerator.
///
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_twenty_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 20 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 20 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 171 — Four-Sqrt × Twenty-Log × polynomial numerator.
///
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_twenty_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 20 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 20 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + sqrt_degs_x2[3] + 2 * poly_deg)
}

/// Phase 172 — Five-Sqrt × Twenty-Log × polynomial numerator.
///
/// Using the ×2 integer trick: `effective_x2 = a₁ + a₂ + a₃ + a₄ + a₅ + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_twenty_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 20 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 20 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2]
        + sqrt_degs_x2[3] + sqrt_degs_x2[4] + 2 * poly_deg)
}

/// Phase 173 — Twenty-One-Log × polynomial numerator.
/// Returns `2 * poly_deg` when `node` is a `Mul` with exactly **21**
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors. Returns `None` otherwise. Sqrt factors are explicitly refused.
fn twenty_one_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 21 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 21 { return None; }
    Some(2 * poly_deg)
}

/// Phase 174 — One-Sqrt × Twenty-One-Log × polynomial numerator.
/// Returns `sqrt_deg_x2 + 2 * poly_deg`.
fn one_sqrt_twenty_one_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 21 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 21 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 175 — Two-Sqrt × Twenty-One-Log × polynomial numerator.
fn two_sqrt_twenty_one_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 21 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 21 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Phase 176 — Three-Sqrt × Twenty-One-Log × polynomial numerator.
fn three_sqrt_twenty_one_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 21 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 21 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 177 — Four-Sqrt × Twenty-One-Log × polynomial numerator.
fn four_sqrt_twenty_one_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 21 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 21 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + sqrt_degs_x2[3] + 2 * poly_deg)
}

/// Phase 178 — Five-Sqrt × Twenty-One-Log × polynomial numerator.
/// Completes the Twenty-One-Log family (Phases 173-178).
fn five_sqrt_twenty_one_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 21 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 21 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2]
        + sqrt_degs_x2[3] + sqrt_degs_x2[4] + 2 * poly_deg)
}



/// Phase 179 — Twenty-Two-Log × polynomial numerator.
/// Returns `2 * poly_deg` when `node` is a `Mul` with exactly **22**
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors. Returns `None` otherwise. Sqrt factors are explicitly refused.
fn twenty_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 22 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 22 { return None; }
    Some(2 * poly_deg)
}

/// Phase 180 — One-Sqrt × Twenty-Two-Log × polynomial numerator.
/// Returns `sqrt_deg_x2 + 2 * poly_deg`.
fn one_sqrt_twenty_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 22 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 22 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 181 — Two-Sqrt × Twenty-Two-Log × polynomial numerator.
fn two_sqrt_twenty_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 22 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 22 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Phase 182 — Three-Sqrt × Twenty-Two-Log × polynomial numerator.
fn three_sqrt_twenty_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 22 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 22 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 183 — Four-Sqrt × Twenty-Two-Log × polynomial numerator.
fn four_sqrt_twenty_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 22 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 22 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + sqrt_degs_x2[3] + 2 * poly_deg)
}

/// Phase 184 — Five-Sqrt × Twenty-Two-Log × polynomial numerator.
/// Completes the Twenty-Two-Log family (Phases 179-184).
fn five_sqrt_twenty_two_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 22 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 22 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2]
        + sqrt_degs_x2[3] + sqrt_degs_x2[4] + 2 * poly_deg)
}



/// Phase 185 — Twenty-Three-Log × polynomial numerator.
/// Returns `2 * poly_deg` when `node` is a `Mul` with exactly **23**
/// `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
/// factors. Returns `None` otherwise. Sqrt factors are explicitly refused.
fn twenty_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if sqrt_effective_half_degree_x2(arg, k).is_some() { return None; }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 23 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if log_count != 23 { return None; }
    Some(2 * poly_deg)
}

/// Phase 186 — One-Sqrt × Twenty-Three-Log × polynomial numerator.
/// Returns `sqrt_deg_x2 + 2 * poly_deg`.
fn one_sqrt_twenty_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 23 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 23 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 187 — Two-Sqrt × Twenty-Three-Log × polynomial numerator.
fn two_sqrt_twenty_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 23 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 23 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Phase 188 — Three-Sqrt × Twenty-Three-Log × polynomial numerator.
fn three_sqrt_twenty_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 23 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 23 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 189 — Four-Sqrt × Twenty-Three-Log × polynomial numerator.
fn four_sqrt_twenty_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 23 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 23 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + sqrt_degs_x2[3] + 2 * poly_deg)
}

/// Phase 190 — Five-Sqrt × Twenty-Three-Log × polynomial numerator.
/// Completes the Twenty-Three-Log family (Phases 185-190).
fn five_sqrt_twenty_three_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 23 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 23 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2]
        + sqrt_degs_x2[3] + sqrt_degs_x2[4] + 2 * poly_deg)
}



/// Phase 96 — One-Sqrt × Eight-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_eight_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 8 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 8 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 97 — Two-Sqrt × Eight-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_eight_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 8 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 8 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 100 — Five-Sqrt × Eight-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a)·…·sqrt(k^e)·log(k)⁸·k^m`. `log⁸(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`. Completes the Eight-Log family (Phases 95–100).
fn five_sqrt_eight_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 8 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 8 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 99 — Four-Sqrt × Eight-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_eight_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 8 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 8 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 98 — Three-Sqrt × Eight-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_eight_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 8 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 8 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 90 — One-Sqrt × Seven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · log(k)⁷ · k^m`. `log⁷(k)` is sub-polynomial (`o(k^ε)`),
/// contributing 0. Using the ×2 integer trick: `effective_x2 = a + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn one_sqrt_seven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_deg_x2: Option<i64> = None;
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_deg_x2.is_some() { return None; } // second Sqrt — refuse
            sqrt_deg_x2 = Some(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 7 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    let s = sqrt_deg_x2?;
    if log_count != 7 { return None; }
    Some(s + 2 * poly_deg)
}

/// Phase 91 — Two-Sqrt × Seven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁷ · k^m ≈ k^{(a+b)/2} · log⁷(k) · k^m`.
/// `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = a + b + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn two_sqrt_seven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 2 { return None; } // third Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 7 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 2 || log_count != 7 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg)
}

/// Phase 92 — Three-Sqrt × Seven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁷ · k^m`.
/// `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn three_sqrt_seven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 3 { return None; } // fourth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 7 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 3 || log_count != 7 { return None; }
    Some(sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg)
}

/// Phase 93 — Four-Sqrt × Seven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a)·sqrt(k^b)·sqrt(k^c)·sqrt(k^d)·log(k)⁷·k^m`.
/// `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn four_sqrt_seven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 4 { return None; } // fifth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 7 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 4 || log_count != 7 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

/// Phase 94 — Five-Sqrt × Seven-Log × polynomial numerator.
///
/// Effective growth: `sqrt(k^a)·sqrt(k^b)·sqrt(k^c)·sqrt(k^d)·sqrt(k^e)·log(k)⁷·k^m`.
/// `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
/// Using the ×2 integer trick: `effective_x2 = a + b + c + d + e + 2·m`.
/// Caller checks `2·den_deg > effective_x2`.
fn five_sqrt_seven_log_poly_effective_x2(node: &IRNode, k: &IRNode) -> Option<i64> {
    let apply_node = match node { IRNode::Apply(a) => a, _ => return None };
    if !head_is(&apply_node.head, MUL) { return None; }
    let mut sqrt_degs_x2: Vec<i64> = Vec::new();
    let mut log_count: usize = 0;
    let mut poly_deg: i64 = 0;
    for arg in &apply_node.args {
        if let Some(d) = sqrt_effective_half_degree_x2(arg, k) {
            if sqrt_degs_x2.len() >= 5 { return None; } // sixth Sqrt — refuse
            sqrt_degs_x2.push(d);
            continue;
        }
        if is_log_of_diverging_in_k(arg, k) {
            log_count += 1;
            if log_count > 7 { return None; }
            continue;
        }
        if let Some(deg) = polynomial_degree_in_k(arg, k) { poly_deg += deg; continue; }
        if is_bounded_in_k(arg, k) { continue; }
        return None;
    }
    if sqrt_degs_x2.len() != 5 || log_count != 7 { return None; }
    Some(sqrt_degs_x2.iter().sum::<i64>() + 2 * poly_deg)
}

fn g_vanishes_at_infinity(g: &IRNode, k: &IRNode) -> bool {
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
    // Phase 59: `Mul(bounded, Sqrt(positive-poly), polynomial)` numerator.
    // Bounded × Sqrt × polynomial: effective_x2 = sqrt_inner_deg + 2·poly_deg.
    // Vanishes when `2·den_deg > effective_x2` (polynomial) or non-polynomial diverging denom.
    // Closes the gap between Phase 53 (Sqrt×poly, refuses bounded) and
    // Phase 56 (bounded×Sqrt, refuses poly).  Log factors are refused → Phase 57.
    if let Some(bsp_x2) = bounded_sqrt_poly_effective_x2(num, k) {
        if let Some(den_deg_bsp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_bsp > bsp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 60: `Mul(bounded, Log(diverging), Sqrt(positive-poly), polynomial)`
    // numerator.  Closes the gap left by Phase 57 (bounded×Log×Sqrt, refuses
    // polynomial factors).  effective_x2 = sqrt_inner_deg + 2·poly_deg.
    // Vanishes when `2·den_deg > effective_x2` (polynomial) or non-polynomial
    // diverging denominator.
    if let Some(blsp_x2) = bounded_log_sqrt_poly_effective_x2(num, k) {
        if let Some(den_deg_blsp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_blsp > blsp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 61: `Mul(Sqrt(P1), Sqrt(P2), polynomial..., bounded...)` numerator.
    // Closes the gap where all prior Sqrt phases require exactly one Sqrt.
    // effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    // Vanishes when `2·den_deg > effective_x2` (polynomial) or non-polynomial
    // diverging denominator.
    if let Some(tsp_x2) = two_sqrt_poly_effective_x2(num, k) {
        if let Some(den_deg_tsp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_tsp > tsp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 62: Mul(Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // log²(k) is sub-polynomial; effective_x2 = 2 * poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(tlp_x2) = two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_tlp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_tlp > tlp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 63: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), polynomial..., bounded...) numerator.
    // Two Sqrt + one Log; log is sub-polynomial; effective_x2 = deg(P1) + deg(P2) + 2*poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(tslp_x2) = two_sqrt_log_poly_effective_x2(num, k) {
        if let Some(den_deg_tslp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_tslp > tslp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 64: Mul(Log(diverging), Log(diverging), Sqrt(P), polynomial..., bounded...) numerator.
    // Two Logs + one Sqrt; log² sub-polynomial; effective_x2 = sqrt_deg_x2 + 2 * poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(tlsp_x2) = two_log_sqrt_poly_effective_x2(num, k) {
        if let Some(den_deg_tlsp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_tlsp > tlsp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 65: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // Two Sqrts + two Logs; log² sub-polynomial; effective_x2 = deg(P1) + deg(P2) + 2 * poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(ts2l_x2) = two_sqrt_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_ts2l) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_ts2l > ts2l_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 66: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...) numerator.
    // Three Sqrt factors; log factors refused (use Phase 63/64/65 for sqrt+log combos).
    // effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2 * poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(tsp_x2) = three_sqrt_poly_effective_x2(num, k) {
        if let Some(den_deg_tsp) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_tsp > tsp_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 67: Mul(Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // Three Log factors; sqrt factors refused. log³ sub-polynomial; effective_x2 = 2 * poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(tlp3_x2) = three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_tlp3) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_tlp3 > tlp3_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 68: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), polynomial..., bounded...) numerator.
    // Three Sqrt factors + one Log; Log is sub-polynomial — effective_x2 = d1+d2+d3 + 2*poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(ts3l_x2) = three_sqrt_log_poly_effective_x2(num, k) {
        if let Some(den_deg_ts3l) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_ts3l > ts3l_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 69: Mul(Sqrt(P), Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // One Sqrt factor + three Log factors; log³ sub-polynomial — effective_x2 = sqrt_deg + 2*poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l3_x2) = one_sqrt_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l3) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l3 > s1l3_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 70: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // Three Sqrt factors + two Log factors; log² sub-polynomial — effective_x2 = d1+d2+d3 + 2*poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(ts2l_x2) = three_sqrt_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_ts2l) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_ts2l > ts2l_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 71: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // Two Sqrt factors + three Log factors; log³ sub-polynomial — effective_x2 = d1+d2 + 2*poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(ts2l3_x2) = two_sqrt_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_ts2l3) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_ts2l3 > ts2l3_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 72: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
    // Three Sqrt factors + three Log factors; log³ sub-polynomial — effective_x2 = d1+d2+d3 + 2*poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(ts3l3_x2) = three_sqrt_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_ts3l3) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_ts3l3 > ts3l3_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 73: Mul(Log(diverging)×4, polynomial..., bounded...) numerator.
    // Four Log factors; log⁴ sub-polynomial — effective_x2 = 2·poly_deg. Sqrt refused.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(flp4_x2) = four_log_poly_effective_x2(num, k) {
        if let Some(den_deg_flp4) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_flp4 > flp4_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 74: Mul(Sqrt(P), Log(diverging)×4, polynomial..., bounded...) numerator.
    // One Sqrt + four Log factors; log⁴ sub-polynomial — effective_x2 = d + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l4_x2) = one_sqrt_four_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l4) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l4 > s1l4_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 75: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×4, polynomial..., bounded...) numerator.
    // Two Sqrt + four Log factors; log⁴ sub-polynomial — effective_x2 = d1 + d2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s2l4_x2) = two_sqrt_four_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l4) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l4 > s2l4_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 76: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging)×4, polynomial..., bounded...) numerator.
    // Three Sqrt + four Log factors; log⁴ sub-polynomial — effective_x2 = d1 + d2 + d3 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s3l4_x2) = three_sqrt_four_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l4) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l4 > s3l4_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 77: Mul(Log(diverging)×5, polynomial..., bounded...) numerator.
    // Five Log factors; no Sqrt; log⁵ sub-polynomial — effective_x2 = 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(fl5_x2) = five_log_poly_effective_x2(num, k) {
        if let Some(den_deg_fl5) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_fl5 > fl5_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 78: Mul(Sqrt(P), Log(diverging)×5, polynomial..., bounded...) numerator.
    // One Sqrt + five Log factors; log⁵ sub-polynomial — effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l5_x2) = one_sqrt_five_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l5) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l5 > s1l5_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 79: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×5, polynomial..., bounded...) numerator.
    // Two Sqrt + five Log factors; log⁵ sub-polynomial — effective_x2 = d1 + d2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s2l5_x2) = two_sqrt_five_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l5) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l5 > s2l5_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 80: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging)×5, polynomial..., bounded...) numerator.
    // Three Sqrt + five Log factors; log⁵ sub-polynomial — effective_x2 = d1 + d2 + d3 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s3l5_x2) = three_sqrt_five_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l5) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l5 > s3l5_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 81: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(diverging)×5, polynomial..., bounded...) numerator.
    // Four Sqrt + five Log factors; log⁵ sub-polynomial — effective_x2 = d1+d2+d3+d4 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s4l5_x2) = four_sqrt_five_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l5) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l5 > s4l5_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 82: Mul(Sqrt(P1)...(P5), Log(diverging)×5, polynomial..., bounded...) numerator.
    // Five Sqrt + five Log factors; log⁵ sub-polynomial — effective_x2 = d1+d2+d3+d4+d5 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s5l5_x2) = five_sqrt_five_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l5) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l5 > s5l5_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 83: Mul(Log(diverging)×6, polynomial..., bounded...) numerator — zero Sqrt factors.
    // log⁶ sub-polynomial — effective_x2 = 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(sl6_x2) = six_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl6) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl6 > sl6_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 84: Mul(Sqrt(P), Log(diverging)×6, polynomial..., bounded...) numerator.
    // One Sqrt + six Log factors; log⁶ sub-polynomial — effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l6_x2) = one_sqrt_six_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l6) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l6 > s1l6_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 89: Mul(Log(diverging)×7, polynomial..., bounded...) numerator — zero Sqrt factors.
    // Seven Log + polynomial; log⁷ sub-polynomial — effective_x2 = 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(sl7_x2) = seven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl7) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl7 > sl7_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 90: Mul(Sqrt(P), Log(diverging)×7, polynomial..., bounded...) numerator.
    // One Sqrt + seven Log factors; log⁷ sub-polynomial — effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l7_x2) = one_sqrt_seven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l7) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l7 > s1l7_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 91: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×7, polynomial..., bounded...) numerator.
    // Two Sqrt + seven Log factors; log⁷ sub-polynomial — effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s2l7_x2) = two_sqrt_seven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l7) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l7 > s2l7_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 92: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging)×7, polynomial..., bounded...) numerator.
    // Three Sqrt + seven Log factors; log⁷ sub-polynomial — effective_x2 = sqrt1+sqrt2+sqrt3_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s3l7_x2) = three_sqrt_seven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l7) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l7 > s3l7_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 93: Mul(Sqrt(P1)..Sqrt(P4), Log(diverging)×7, polynomial..., bounded...) numerator.
    // Four Sqrt + seven Log factors; log⁷ sub-polynomial — effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s4l7_x2) = four_sqrt_seven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l7) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l7 > s4l7_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 94: Mul(Sqrt(P1)..Sqrt(P5), Log(diverging)×7, polynomial..., bounded...) numerator.
    // Five Sqrt + seven Log factors; log⁷ sub-polynomial — effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s5l7_x2) = five_sqrt_seven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l7) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l7 > s5l7_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 95: Mul(Log(diverging)×8, polynomial..., bounded...) numerator.
    // Zero Sqrt + eight Log factors; log⁸ sub-polynomial — effective_x2 = 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(sl8_x2) = eight_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl8) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl8 > sl8_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 96: Mul(Sqrt(P), Log(diverging)×8, polynomial..., bounded...) numerator.
    // One Sqrt + eight Log factors; log⁸ sub-polynomial — effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l8_x2) = one_sqrt_eight_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l8) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l8 > s1l8_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 97: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×8, polynomial..., bounded...) numerator.
    // Two Sqrt + eight Log factors; log⁸ sub-polynomial — effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s2l8_x2) = two_sqrt_eight_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l8) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l8 > s2l8_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 190: Mul(Sqrt(P1)×5, Log(h1)×23, ...) numerator.
    if let Some(s5l23_x2) = five_sqrt_twenty_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l23) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l23 > s5l23_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 189: Mul(Sqrt(P1)×4, Log(h1)×23, ...) numerator.
    if let Some(s4l23_x2) = four_sqrt_twenty_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l23) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l23 > s4l23_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 188: Mul(Sqrt(P1)×3, Log(h1)×23, ...) numerator.
    if let Some(s3l23_x2) = three_sqrt_twenty_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l23) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l23 > s3l23_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 187: Mul(Sqrt(P), Sqrt(P2), Log(h1)×23, ...) numerator.
    if let Some(s2l23_x2) = two_sqrt_twenty_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l23) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l23 > s2l23_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 186: Mul(Sqrt(P), Log(h1)×23, ...) numerator.
    if let Some(s1l23_x2) = one_sqrt_twenty_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l23) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l23 > s1l23_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 185: Mul(Log(h1)×23, ...) numerator.
    if let Some(sl23_x2) = twenty_three_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl23) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl23 > sl23_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 184: Mul(Sqrt(P1)×5, Log(h1)×22, ...) numerator.
    if let Some(s5l22_x2) = five_sqrt_twenty_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l22) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l22 > s5l22_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 183: Mul(Sqrt(P1)×4, Log(h1)×22, ...) numerator.
    if let Some(s4l22_x2) = four_sqrt_twenty_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l22) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l22 > s4l22_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 182: Mul(Sqrt(P1)×3, Log(h1)×22, ...) numerator.
    if let Some(s3l22_x2) = three_sqrt_twenty_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l22) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l22 > s3l22_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 181: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×22, ...) numerator.
    if let Some(s2l22_x2) = two_sqrt_twenty_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l22) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l22 > s2l22_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 180: Mul(Sqrt(P), Log(h1)×22, ...) numerator.
    if let Some(s1l22_x2) = one_sqrt_twenty_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l22) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l22 > s1l22_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 179: Mul(Log(h1)×22, ...) numerator.
    if let Some(sl22_x2) = twenty_two_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl22) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl22 > sl22_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 178: Mul(Sqrt(P1)×5, Log(h1)×21, ...) numerator.
    if let Some(s5l21_x2) = five_sqrt_twenty_one_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l21) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l21 > s5l21_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 177: Mul(Sqrt(P1)×4, Log(h1)×21, ...) numerator.
    if let Some(s4l21_x2) = four_sqrt_twenty_one_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l21) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l21 > s4l21_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 176: Mul(Sqrt(P1)×3, Log(h1)×21, ...) numerator.
    if let Some(s3l21_x2) = three_sqrt_twenty_one_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l21) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l21 > s3l21_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 175: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×21, ...) numerator.
    if let Some(s2l21_x2) = two_sqrt_twenty_one_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l21) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l21 > s2l21_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 174: Mul(Sqrt(P), Log(h1)×21, ...) numerator.
    if let Some(s1l21_x2) = one_sqrt_twenty_one_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l21) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l21 > s1l21_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 173: Mul(Log(h1)×21, ...) numerator.
    if let Some(sl21_x2) = twenty_one_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl21) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl21 > sl21_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 172: Mul(Sqrt(P1)×5, Log(diverging)×20, polynomial..., bounded...) numerator.
    if let Some(s5l20_x2) = five_sqrt_twenty_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l20) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l20 > s5l20_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 171: Mul(Sqrt(P1)×4, Log(diverging)×20, polynomial..., bounded...) numerator.
    if let Some(s4l20_x2) = four_sqrt_twenty_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l20) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l20 > s4l20_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 170: Mul(Sqrt(P1)×3, Log(diverging)×20, polynomial..., bounded...) numerator.
    if let Some(s3l20_x2) = three_sqrt_twenty_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l20) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l20 > s3l20_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 169: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×20, polynomial..., bounded...) numerator.
    if let Some(s2l20_x2) = two_sqrt_twenty_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l20) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l20 > s2l20_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 168: Mul(Sqrt(P), Log(diverging)×20, polynomial..., bounded...) numerator.
    if let Some(s1l20_x2) = one_sqrt_twenty_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l20) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l20 > s1l20_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 167: Mul(Log(diverging)×20, polynomial..., bounded...) numerator.
    if let Some(sl20_x2) = twenty_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl20) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl20 > sl20_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 166: Mul(Sqrt(P1)×5, Log(diverging)×19, polynomial..., bounded...) numerator.
    if let Some(s5l19_x2) = five_sqrt_nineteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l19) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l19 > s5l19_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 165: Mul(Sqrt(P1)×4, Log(diverging)×19, polynomial..., bounded...) numerator.
    if let Some(s4l19_x2) = four_sqrt_nineteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l19) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l19 > s4l19_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 164: Mul(Sqrt(P1)×3, Log(diverging)×19, polynomial..., bounded...) numerator.
    if let Some(s3l19_x2) = three_sqrt_nineteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l19) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l19 > s3l19_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 163: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×19, polynomial..., bounded...) numerator.
    if let Some(s2l19_x2) = two_sqrt_nineteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l19) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l19 > s2l19_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 162: Mul(Sqrt(P), Log(diverging)×19, polynomial..., bounded...) numerator.
    if let Some(s1l19_x2) = one_sqrt_nineteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l19) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l19 > s1l19_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 161: Mul(Log(diverging)×19, polynomial..., bounded...) numerator.
    if let Some(sl19_x2) = nineteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl19) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl19 > sl19_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 160: Mul(Sqrt(P1)×5, Log(diverging)×18, polynomial..., bounded...) numerator.
    if let Some(s5l18_x2) = five_sqrt_eighteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l18) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l18 > s5l18_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 159: Mul(Sqrt(P1)×4, Log(diverging)×18, polynomial..., bounded...) numerator.
    if let Some(s4l18_x2) = four_sqrt_eighteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l18) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l18 > s4l18_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 158: Mul(Sqrt(P1)×3, Log(diverging)×18, polynomial..., bounded...) numerator.
    if let Some(s3l18_x2) = three_sqrt_eighteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l18) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l18 > s3l18_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 157: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×18, polynomial..., bounded...) numerator.
    if let Some(s2l18_x2) = two_sqrt_eighteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l18) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l18 > s2l18_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 156: Mul(Sqrt(P), Log(diverging)×18, polynomial..., bounded...) numerator.
    if let Some(s1l18_x2) = one_sqrt_eighteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l18) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l18 > s1l18_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 155: Mul(Log(diverging)×18, polynomial..., bounded...) numerator.
    if let Some(sl18_x2) = eighteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl18) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl18 > sl18_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 154: Mul(Sqrt(P1)×5, Log(diverging)×17, polynomial..., bounded...) numerator.
    if let Some(s5l17_x2) = five_sqrt_seventeen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l17) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l17 > s5l17_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 153: Mul(Sqrt(P1)×4, Log(diverging)×17, polynomial..., bounded...) numerator.
    if let Some(s4l17_x2) = four_sqrt_seventeen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l17) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l17 > s4l17_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 152: Mul(Sqrt(P1)×3, Log(diverging)×17, polynomial..., bounded...) numerator.
    if let Some(s3l17_x2) = three_sqrt_seventeen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l17) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l17 > s3l17_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 151: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×17, polynomial..., bounded...) numerator.
    if let Some(s2l17_x2) = two_sqrt_seventeen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l17) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l17 > s2l17_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 150: Mul(Sqrt(P), Log(diverging)×17, polynomial..., bounded...) numerator.
    if let Some(s1l17_x2) = one_sqrt_seventeen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l17) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l17 > s1l17_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 149: Mul(Log(diverging)×17, polynomial..., bounded...) numerator.
    if let Some(sl17_x2) = seventeen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl17) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl17 > sl17_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 148: Mul(Sqrt(P1)×5, Log(diverging)×16, polynomial..., bounded...) numerator.
    if let Some(s5l16_x2) = five_sqrt_sixteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l16) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l16 > s5l16_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 147: Mul(Sqrt(P1)×4, Log(diverging)×16, polynomial..., bounded...) numerator.
    if let Some(s4l16_x2) = four_sqrt_sixteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l16) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l16 > s4l16_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 146: Mul(Sqrt(P1)×3, Log(diverging)×16, polynomial..., bounded...) numerator.
    if let Some(s3l16_x2) = three_sqrt_sixteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l16) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l16 > s3l16_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 145: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×16, polynomial..., bounded...) numerator.
    if let Some(s2l16_x2) = two_sqrt_sixteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l16) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l16 > s2l16_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 144: Mul(Sqrt(P), Log(diverging)×16, polynomial..., bounded...) numerator.
    if let Some(s1l16_x2) = one_sqrt_sixteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l16) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l16 > s1l16_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 143: Mul(Log(diverging)×16, polynomial..., bounded...) numerator.
    if let Some(sl16_x2) = sixteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl16) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl16 > sl16_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 142: Mul(Sqrt(P1)×5, Log(diverging)×15, polynomial..., bounded...) numerator.
    if let Some(s5l15_x2) = five_sqrt_fifteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l15) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l15 > s5l15_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 141: Mul(Sqrt(P1)×4, Log(diverging)×15, polynomial..., bounded...) numerator.
    if let Some(s4l15_x2) = four_sqrt_fifteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l15) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l15 > s4l15_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 140: Mul(Sqrt(P1)×3, Log(diverging)×15, polynomial..., bounded...) numerator.
    if let Some(s3l15_x2) = three_sqrt_fifteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l15) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l15 > s3l15_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 139: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×15, polynomial..., bounded...) numerator.
    if let Some(s2l15_x2) = two_sqrt_fifteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l15) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l15 > s2l15_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 138: Mul(Sqrt(P), Log(diverging)×15, polynomial..., bounded...) numerator.
    if let Some(s1l15_x2) = one_sqrt_fifteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l15) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l15 > s1l15_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 137: Mul(Log(diverging)×15, polynomial..., bounded...) numerator.
    if let Some(sl15_x2) = fifteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl15) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl15 > sl15_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 136: Mul(Sqrt(P1)×5, Log(diverging)×14, polynomial..., bounded...) numerator.
    if let Some(s5l14_x2) = five_sqrt_fourteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l14) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l14 > s5l14_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 135: Mul(Sqrt(P1)×4, Log(diverging)×14, polynomial..., bounded...) numerator.
    if let Some(s4l14_x2) = four_sqrt_fourteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l14) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l14 > s4l14_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 134: Mul(Sqrt(P1)×3, Log(diverging)×14, polynomial..., bounded...) numerator.
    if let Some(s3l14_x2) = three_sqrt_fourteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l14) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l14 > s3l14_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 133: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×14, polynomial..., bounded...) numerator.
    if let Some(s2l14_x2) = two_sqrt_fourteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l14) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l14 > s2l14_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 132: Mul(Sqrt(P), Log(diverging)×14, polynomial..., bounded...) numerator.
    if let Some(s1l14_x2) = one_sqrt_fourteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l14) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l14 > s1l14_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 131: Mul(Log(diverging)×14, polynomial..., bounded...) numerator.
    if let Some(sl14_x2) = fourteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl14) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl14 > sl14_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 130: Mul(Sqrt(P1)×5, Log(diverging)×13, polynomial..., bounded...) numerator.
    if let Some(s5l13_x2) = five_sqrt_thirteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l13) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l13 > s5l13_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 129: Mul(Sqrt(P1)×4, Log(diverging)×13, polynomial..., bounded...) numerator.
    if let Some(s4l13_x2) = four_sqrt_thirteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l13) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l13 > s4l13_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 128: Mul(Sqrt(P1)×3, Log(diverging)×13, polynomial..., bounded...) numerator.
    if let Some(s3l13_x2) = three_sqrt_thirteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l13) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l13 > s3l13_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 127: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×13, polynomial..., bounded...) numerator.
    if let Some(s2l13_x2) = two_sqrt_thirteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l13) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l13 > s2l13_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 126: Mul(Sqrt(P), Log(diverging)×13, polynomial..., bounded...) numerator.
    if let Some(s1l13_x2) = one_sqrt_thirteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l13) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l13 > s1l13_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 125: Mul(Log(diverging)×13, polynomial..., bounded...) numerator.
    if let Some(sl13_x2) = thirteen_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl13) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl13 > sl13_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 124: Mul(Sqrt(P1)×5, Log(diverging)×12, polynomial..., bounded...) numerator.
    if let Some(s5l12_x2) = five_sqrt_twelve_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l12) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l12 > s5l12_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 123: Mul(Sqrt(P1)×4, Log(diverging)×12, polynomial..., bounded...) numerator.
    if let Some(s4l12_x2) = four_sqrt_twelve_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l12) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l12 > s4l12_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 122: Mul(Sqrt(P1)×3, Log(diverging)×12, polynomial..., bounded...) numerator.
    if let Some(s3l12_x2) = three_sqrt_twelve_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l12) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l12 > s3l12_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 121: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×12, polynomial..., bounded...) numerator.
    if let Some(s2l12_x2) = two_sqrt_twelve_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l12) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l12 > s2l12_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 120: Mul(Sqrt(P), Log(diverging)×12, polynomial..., bounded...) numerator.
    if let Some(s1l12_x2) = one_sqrt_twelve_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l12) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l12 > s1l12_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 119: Mul(Log(diverging)×12, polynomial..., bounded...) numerator.
    if let Some(sl12_x2) = twelve_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl12) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl12 > sl12_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 118: Mul(Sqrt(P1)×5, Log(diverging)×11, polynomial..., bounded...) numerator.
    if let Some(s5l11_x2) = five_sqrt_eleven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l11) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l11 > s5l11_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 117: Mul(Sqrt(P1)×4, Log(diverging)×11, polynomial..., bounded...) numerator.
    if let Some(s4l11_x2) = four_sqrt_eleven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l11) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l11 > s4l11_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 116: Mul(Sqrt(P1)×3, Log(diverging)×11, polynomial..., bounded...) numerator.
    if let Some(s3l11_x2) = three_sqrt_eleven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l11) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l11 > s3l11_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 115: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×11, polynomial..., bounded...) numerator.
    if let Some(s2l11_x2) = two_sqrt_eleven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l11) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l11 > s2l11_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 114: Mul(Sqrt(P), Log(diverging)×11, polynomial..., bounded...) numerator.
    if let Some(s1l11_x2) = one_sqrt_eleven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l11) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l11 > s1l11_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 113: Mul(Log(diverging)×11, polynomial..., bounded...) numerator.
    if let Some(sl11_x2) = eleven_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl11) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl11 > sl11_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 112: Mul(Sqrt(P1)×5, Log(diverging)×10, polynomial..., bounded...) numerator.
    if let Some(s5l10_x2) = five_sqrt_ten_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l10) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l10 > s5l10_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 111: Mul(Sqrt(P1)×4, Log(diverging)×10, polynomial..., bounded...) numerator.
    if let Some(s4l10_x2) = four_sqrt_ten_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l10) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l10 > s4l10_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 110: Mul(Sqrt(P1)×3, Log(diverging)×10, polynomial..., bounded...) numerator.
    if let Some(s3l10_x2) = three_sqrt_ten_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l10) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l10 > s3l10_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 109: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×10, polynomial..., bounded...) numerator.
    if let Some(s2l10_x2) = two_sqrt_ten_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l10) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l10 > s2l10_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 108: Mul(Sqrt(P), Log(diverging)×10, polynomial..., bounded...) numerator.
    if let Some(s1l10_x2) = one_sqrt_ten_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l10) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l10 > s1l10_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 107: Mul(Log(diverging)×10, polynomial..., bounded...) numerator.
    if let Some(sl10_x2) = ten_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl10) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl10 > sl10_x2 { return true; }
        } else if h_diverges_at_infinity(den, k) { return true; }
    }
    // Phase 106: Mul(Sqrt(P1)×5, Log(diverging)×9, polynomial..., bounded...) numerator.
    // Five Sqrt + nine Log factors; log⁹ sub-polynomial — effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s5l9_x2) = five_sqrt_nine_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l9) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l9 > s5l9_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 105: Mul(Sqrt(P1)×4, Log(diverging)×9, polynomial..., bounded...) numerator.
    // Four Sqrt + nine Log factors; log⁹ sub-polynomial — effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s4l9_x2) = four_sqrt_nine_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l9) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l9 > s4l9_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 104: Mul(Sqrt(P1)×3, Log(diverging)×9, polynomial..., bounded...) numerator.
    // Three Sqrt + nine Log factors; log⁹ sub-polynomial — effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s3l9_x2) = three_sqrt_nine_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l9) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l9 > s3l9_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 103: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×9, polynomial..., bounded...) numerator.
    // Two Sqrt + nine Log factors; log⁹ sub-polynomial — effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s2l9_x2) = two_sqrt_nine_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s2l9) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s2l9 > s2l9_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 102: Mul(Sqrt(P), Log(diverging)×9, polynomial..., bounded...) numerator.
    // One Sqrt + nine Log factors; log⁹ sub-polynomial — effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s1l9_x2) = one_sqrt_nine_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s1l9) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s1l9 > s1l9_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 101: Mul(Log(diverging)×9, polynomial..., bounded...) numerator.
    // Zero Sqrt + nine Log factors; log⁹ sub-polynomial — effective_x2 = 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(sl9_x2) = nine_log_poly_effective_x2(num, k) {
        if let Some(den_deg_sl9) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_sl9 > sl9_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 100: Mul(Sqrt×5, Log(diverging)×8, polynomial..., bounded...) numerator.
    // Five Sqrt + eight Log factors; log⁸ sub-polynomial — effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s5l8_x2) = five_sqrt_eight_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s5l8) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s5l8 > s5l8_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 99: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(diverging)×8, polynomial..., bounded...) numerator.
    // Four Sqrt + eight Log factors; log⁸ sub-polynomial — effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s4l8_x2) = four_sqrt_eight_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s4l8) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s4l8 > s4l8_x2 {
                return true;
            }
        } else if h_diverges_at_infinity(den, k) {
            return true;
        }
    }
    // Phase 98: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging)×8, polynomial..., bounded...) numerator.
    // Three Sqrt + eight Log factors; log⁸ sub-polynomial — effective_x2 = sum(sqrt_deg_x2) + 2·poly_deg.
    // Closes when 2 * den_deg > effective_x2 or non-polynomial diverging denom.
    if let Some(s3l8_x2) = three_sqrt_eight_log_poly_effective_x2(num, k) {
        if let Some(den_deg_s3l8) = polynomial_degree_in_k(den, k) {
            if 2 * den_deg_s3l8 > s3l8_x2 {
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

//! Cubic-equation closed form using rational roots and Cardano's formula.
//!
//! For `a*x^3 + b*x^2 + c*x + d = 0` with rational coefficients:
//!
//! 1. Try the rational-root theorem. When a rational root is found, deflate
//!    the cubic and solve the residual quadratic.
//! 2. If no rational root exists, use Cardano's formula for the depressed
//!    cubic `t^3 + p*t + q = 0`.
//!
//! The casus irreducibilis branch (`D_cardano < 0`) intentionally returns an
//! empty solution list, matching the Python reference's "leave unevaluated"
//! behavior.

use symbolic_ir::{apply, sym, IRNode, ADD, DIV, MUL, NEG, SQRT, SUB};

use crate::frac::Frac;
use crate::quadratic::{solve_quadratic, I_UNIT};
use crate::SolveResult;

/// Cube-root head used by Cardano symbolic fallback expressions.
pub const CBRT: &str = "Cbrt";

/// Solve `a*x^3 + b*x^2 + c*x + d = 0` over rationals, with complex IR roots
/// where the closed form naturally produces them.
///
/// When `a == 0`, this delegates to [`solve_quadratic`]. Repeated roots are
/// deduplicated. Cubics in casus irreducibilis return `Solutions(vec![])` so
/// callers can treat the original expression as unevaluated.
pub fn solve_cubic(a: Frac, b: Frac, c: Frac, d: Frac) -> SolveResult {
    if a.is_zero() {
        return solve_quadratic(b, c, d);
    }

    if let Some(root) = find_rational_root(a, b, c, d) {
        let b2 = b + a * root;
        let c2 = c + b2 * root;
        let remainder = d + c2 * root;
        if remainder.is_zero() {
            let root_ir = root.to_irnode();
            let SolveResult::Solutions(remaining) = solve_quadratic(a, b2, c2) else {
                return SolveResult::Solutions(vec![root_ir]);
            };
            return SolveResult::Solutions(dedup_roots(
                std::iter::once(root_ir).chain(remaining).collect(),
            ));
        }
    }

    let three = Frac::from_int(3);
    let four = Frac::from_int(4);
    let twenty_seven = Frac::from_int(27);
    let a_inv = Frac::one() / a;

    let p = c * a_inv - b * b * a_inv * a_inv / three;
    let q = d * a_inv - b * c * a_inv * a_inv / three
        + Frac::from_int(2) * b * b * b * a_inv * a_inv * a_inv / twenty_seven;
    let shift = -b / (three * a);
    let d_card = q * q / four + p * p * p / twenty_seven;

    if d_card > Frac::zero() {
        return SolveResult::Solutions(cardano_one_real_two_complex(q, shift, d_card));
    }

    if d_card == Frac::zero() {
        return SolveResult::Solutions(cardano_repeated(p, q, shift));
    }

    SolveResult::Solutions(vec![])
}

fn cardano_one_real_two_complex(q: Frac, shift: Frac, d_card: Frac) -> Vec<IRNode> {
    let neg_q_half = -q / Frac::from_int(2);

    if let Some(sqrt_d) = try_exact_sqrt(d_card) {
        let a_term = neg_q_half + sqrt_d;
        let b_term = neg_q_half - sqrt_d;
        if let (Some(cbrt_a), Some(cbrt_b)) = (try_exact_cbrt(a_term), try_exact_cbrt(b_term)) {
            let t1 = cbrt_a + cbrt_b;
            let x1 = t1 + shift;
            let half_sum = -(cbrt_a + cbrt_b) / Frac::from_int(2);
            let half_diff = (cbrt_a - cbrt_b) / Frac::from_int(2);
            let mut roots = vec![x1.to_irnode()];

            if half_diff.is_zero() {
                let repeated = (half_sum + shift).to_irnode();
                roots.push(repeated.clone());
                roots.push(repeated);
            } else {
                let real_part = (half_sum + shift).to_irnode();
                let imag = imag_term(half_diff);
                roots.push(apply(sym(ADD), vec![real_part.clone(), imag.clone()]));
                roots.push(apply(sym(SUB), vec![real_part, imag]));
            }
            return roots;
        }
    }

    let sqrt_d_ir = sqrt_ir(d_card);
    let neg_q_half_ir = neg_q_half.to_irnode();
    let (cbrt_a, cbrt_b) = if neg_q_half.is_zero() {
        (
            apply(sym(CBRT), vec![sqrt_d_ir.clone()]),
            apply(sym(CBRT), vec![apply(sym(NEG), vec![sqrt_d_ir])]),
        )
    } else {
        (
            apply(
                sym(CBRT),
                vec![apply(
                    sym(ADD),
                    vec![neg_q_half_ir.clone(), sqrt_d_ir.clone()],
                )],
            ),
            apply(
                sym(CBRT),
                vec![apply(sym(SUB), vec![neg_q_half_ir, sqrt_d_ir])],
            ),
        )
    };

    let t1 = apply(sym(ADD), vec![cbrt_a.clone(), cbrt_b.clone()]);
    let x1 = add_shift(t1, shift);

    let minus_t1_half = apply(
        sym(DIV),
        vec![
            apply(
                sym(NEG),
                vec![apply(sym(ADD), vec![cbrt_a.clone(), cbrt_b.clone()])],
            ),
            IRNode::Integer(2),
        ],
    );
    let real_part = add_shift(minus_t1_half, shift);
    let diff = apply(sym(SUB), vec![cbrt_a, cbrt_b]);
    let imag_part = apply(
        sym(MUL),
        vec![
            apply(
                sym(DIV),
                vec![
                    apply(
                        sym(MUL),
                        vec![diff, apply(sym(SQRT), vec![IRNode::Integer(3)])],
                    ),
                    IRNode::Integer(2),
                ],
            ),
            sym(I_UNIT),
        ],
    );

    vec![
        x1,
        apply(sym(ADD), vec![real_part.clone(), imag_part.clone()]),
        apply(sym(SUB), vec![real_part, imag_part]),
    ]
}

fn cardano_repeated(p: Frac, q: Frac, shift: Frac) -> Vec<IRNode> {
    if p.is_zero() && q.is_zero() {
        return vec![shift.to_irnode()];
    }

    let neg_q_half = -q / Frac::from_int(2);
    if let Some(cbrt_val) = try_exact_cbrt(neg_q_half) {
        let t1 = Frac::from_int(2) * cbrt_val;
        let t2 = -cbrt_val;
        return dedup_roots(vec![(t1 + shift).to_irnode(), (t2 + shift).to_irnode()]);
    }

    let cbrt = apply(sym(CBRT), vec![neg_q_half.to_irnode()]);
    vec![
        add_shift(
            apply(sym(MUL), vec![IRNode::Integer(2), cbrt.clone()]),
            shift,
        ),
        add_shift(apply(sym(NEG), vec![cbrt]), shift),
    ]
}

fn find_rational_root(a: Frac, b: Frac, c: Frac, d: Frac) -> Option<Frac> {
    let lcm = fraction_lcm(&[a, b, c, d]);
    let scaled_a = a.numer * (lcm / a.denom);
    let scaled_d = d.numer * (lcm / d.denom);

    if scaled_d == 0 {
        return Some(Frac::zero());
    }

    for p in divisors(scaled_d.unsigned_abs()) {
        for q in divisors(scaled_a.unsigned_abs()) {
            for sign in [1, -1] {
                let candidate = Frac::new(sign * p as i64, q as i64);
                if eval_cubic(a, b, c, d, candidate).is_zero() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn eval_cubic(a: Frac, b: Frac, c: Frac, d: Frac, x: Frac) -> Frac {
    a * x * x * x + b * x * x + c * x + d
}

fn divisors(n: u64) -> Vec<u64> {
    if n == 0 {
        return vec![0];
    }

    let mut divs = Vec::new();
    let mut i = 1;
    while i * i <= n {
        if n % i == 0 {
            divs.push(i);
            if i != n / i {
                divs.push(n / i);
            }
        }
        i += 1;
    }
    divs.sort_unstable();
    divs
}

fn fraction_lcm(fracs: &[Frac]) -> i64 {
    fracs.iter().fold(1, |acc, f| lcm(acc, f.denom))
}

fn lcm(a: i64, b: i64) -> i64 {
    a / gcd(a.unsigned_abs(), b.unsigned_abs()) as i64 * b
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

fn try_exact_sqrt(value: Frac) -> Option<Frac> {
    if value.numer < 0 {
        return None;
    }
    match (isqrt(value.numer as u64), isqrt(value.denom as u64)) {
        (Some(n), Some(d)) => Some(Frac::new(n as i64, d as i64)),
        _ => None,
    }
}

fn try_exact_cbrt(value: Frac) -> Option<Frac> {
    if value.is_zero() {
        return Some(Frac::zero());
    }
    let sign = if value.numer < 0 { -1 } else { 1 };
    match (
        icbrt(value.numer.unsigned_abs()),
        icbrt(value.denom.unsigned_abs()),
    ) {
        (Some(n), Some(d)) => Some(Frac::new(sign * n as i64, d as i64)),
        _ => None,
    }
}

fn isqrt(n: u64) -> Option<u64> {
    let r = (n as f64).sqrt() as u64;
    for candidate in r.saturating_sub(1)..=r + 1 {
        if candidate * candidate == n {
            return Some(candidate);
        }
    }
    None
}

fn icbrt(n: u64) -> Option<u64> {
    if n == 0 {
        return Some(0);
    }
    let r = (n as f64).cbrt() as u64;
    for candidate in r.saturating_sub(1)..=r + 1 {
        if candidate * candidate * candidate == n {
            return Some(candidate);
        }
    }
    None
}

fn sqrt_ir(value: Frac) -> IRNode {
    try_exact_sqrt(value).map_or_else(
        || apply(sym(SQRT), vec![value.to_irnode()]),
        |sqrt| sqrt.to_irnode(),
    )
}

fn imag_term(coef: Frac) -> IRNode {
    if coef == Frac::one() {
        return sym(I_UNIT);
    }
    if coef == -Frac::one() {
        return apply(sym(NEG), vec![sym(I_UNIT)]);
    }
    apply(sym(MUL), vec![coef.to_irnode(), sym(I_UNIT)])
}

fn add_shift(node: IRNode, shift: Frac) -> IRNode {
    if shift.is_zero() {
        return node;
    }

    if shift.denom == 1 && shift.numer < 0 {
        apply(sym(SUB), vec![node, IRNode::Integer(-shift.numer)])
    } else {
        apply(sym(ADD), vec![node, shift.to_irnode()])
    }
}

fn dedup_roots(roots: Vec<IRNode>) -> Vec<IRNode> {
    let mut deduped = Vec::new();
    for root in roots {
        if !deduped.contains(&root) {
            deduped.push(root);
        }
    }
    deduped
}

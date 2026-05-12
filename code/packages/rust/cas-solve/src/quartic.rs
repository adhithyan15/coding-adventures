//! Quartic-equation solving via rational deflation and Ferrari's method.
//!
//! For `a*x^4 + b*x^3 + c*x^2 + d*x + e = 0` with rational coefficients:
//!
//! 1. Try the rational-root theorem. When a rational root is found, deflate
//!    the quartic and solve the residual cubic.
//! 2. If no rational root exists, depress the quartic. Biquadratic cases are
//!    solved through the quadratic solver; general cases use a resolvent cubic
//!    and factor into two quadratics when the resolvent has a usable rational
//!    root.
//!
//! Quartics whose resolvent cannot be represented by a rational IR root return
//! an empty solution list, matching the Python reference's unevaluated
//! fallback behavior.

use symbolic_ir::{apply, sym, IRNode, ADD, NEG, SQRT, SUB};

use crate::cubic::solve_cubic;
use crate::frac::Frac;
use crate::quadratic::solve_quadratic;
use crate::SolveResult;

/// Solve `a*x^4 + b*x^3 + c*x^2 + d*x + e = 0` over rationals.
///
/// When `a == 0`, delegates to [`solve_cubic`]. Repeated roots are deduplicated.
/// Cases outside the bounded Ferrari path return `Solutions(vec![])`.
pub fn solve_quartic(a: Frac, b: Frac, c: Frac, d: Frac, e: Frac) -> SolveResult {
    if a.is_zero() {
        return solve_cubic(b, c, d, e);
    }

    if let Some(root) = find_rational_root_quartic(a, b, c, d, e) {
        let b2 = b + a * root;
        let c2 = c + b2 * root;
        let d2 = d + c2 * root;
        let remainder = e + d2 * root;
        if remainder.is_zero() {
            let root_ir = root.to_irnode();
            let SolveResult::Solutions(remaining) = solve_cubic(a, b2, c2, d2) else {
                return SolveResult::Solutions(vec![root_ir]);
            };
            return SolveResult::Solutions(dedup_roots(
                std::iter::once(root_ir).chain(remaining).collect(),
            ));
        }
    }

    let two = Frac::from_int(2);
    let four = Frac::from_int(4);
    let eight = Frac::from_int(8);
    let sixteen = Frac::from_int(16);
    let two_fifty_six = Frac::from_int(256);
    let a2 = a * a;
    let a3 = a2 * a;
    let a4 = a3 * a;
    let b2 = b * b;
    let b3 = b2 * b;
    let b4 = b2 * b2;

    let p = c / a - Frac::from_int(3) * b2 / (eight * a2);
    let q = b3 / (eight * a3) - b * c / (two * a2) + d / a;
    let r_coef = Frac::from_int(-3) * b4 / (two_fifty_six * a4) + b2 * c / (sixteen * a3)
        - b * d / (four * a2)
        + e / a;
    let shift = -b / (four * a);

    if q.is_zero() {
        let SolveResult::Solutions(u_roots) = solve_quadratic(Frac::one(), p, r_coef) else {
            return SolveResult::Solutions(vec![]);
        };
        let mut roots = Vec::with_capacity(u_roots.len() * 2);
        for root in u_roots {
            let t = apply(sym(SQRT), vec![root]);
            roots.push(add_shift(t.clone(), shift));
            roots.push(add_shift(apply(sym(NEG), vec![t]), shift));
        }
        return SolveResult::Solutions(dedup_roots(roots));
    }

    let resolvent = solve_cubic(
        Frac::from_int(8),
        Frac::from_int(8) * p,
        two * p * p - Frac::from_int(8) * r_coef,
        -(q * q),
    );
    let SolveResult::Solutions(resolvent_roots) = resolvent else {
        return SolveResult::Solutions(vec![]);
    };
    let Some(m) = resolvent_roots.iter().find_map(frac_from_ir) else {
        return SolveResult::Solutions(vec![]);
    };
    if m.is_zero() {
        return SolveResult::Solutions(vec![]);
    }

    let alpha = p / two + m * m / two - q / (two * m);
    let beta = p / two + m * m / two + q / (two * m);
    let mut roots = Vec::new();
    if let SolveResult::Solutions(roots1) = solve_quadratic(Frac::one(), m, alpha) {
        roots.extend(roots1.into_iter().map(|root| add_shift(root, shift)));
    }
    if let SolveResult::Solutions(roots2) = solve_quadratic(Frac::one(), -m, beta) {
        roots.extend(roots2.into_iter().map(|root| add_shift(root, shift)));
    }
    SolveResult::Solutions(dedup_roots(roots))
}

fn find_rational_root_quartic(a: Frac, b: Frac, c: Frac, d: Frac, e: Frac) -> Option<Frac> {
    let lcm = fraction_lcm(&[a, b, c, d, e]);
    let scaled_a = a.numer * (lcm / a.denom);
    let scaled_e = e.numer * (lcm / e.denom);

    if scaled_e == 0 {
        return Some(Frac::zero());
    }

    for p in divisors(scaled_e.unsigned_abs()) {
        for q in divisors(scaled_a.unsigned_abs()) {
            for sign in [1, -1] {
                let candidate = Frac::new(sign * p as i64, q as i64);
                if eval_quartic(a, b, c, d, e, candidate).is_zero() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn eval_quartic(a: Frac, b: Frac, c: Frac, d: Frac, e: Frac, x: Frac) -> Frac {
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x3 * x;
    a * x4 + b * x3 + c * x2 + d * x + e
}

fn frac_from_ir(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(value) => Some(Frac::from_int(*value)),
        IRNode::Rational(numer, denom) => Some(Frac::new(*numer, *denom)),
        _ => None,
    }
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

fn add_shift(node: IRNode, shift: Frac) -> IRNode {
    if shift.is_zero() {
        return node;
    }
    if shift.numer < 0 {
        return apply(sym(SUB), vec![node, (-shift).to_irnode()]);
    }
    apply(sym(ADD), vec![node, shift.to_irnode()])
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

//! Polynomial inequality solving in one real variable.
//!
//! This is the Rust package-level counterpart to Python Phase 27.  It accepts
//! `Less`, `Greater`, `LessEqual`, or `GreaterEqual` IR nodes, normalizes
//! `lhs op rhs` to a rational univariate polynomial, and emits interval
//! predicates over the requested variable.

use symbolic_ir::{
    apply, flt, int, sym, IRNode, ADD, AND, GREATER, GREATER_EQUAL, LESS, LESS_EQUAL, MUL, NEG,
    POW, SUB,
};

use crate::frac::Frac;
use crate::{
    nsolve_poly, solve_cubic, solve_linear, solve_quadratic, solve_quartic, Complex, SolveResult,
};

const MAX_INEQUALITY_DEGREE: usize = 4;
const REAL_ROOT_TOL: f64 = 1e-8;

/// Try to solve a polynomial inequality in one real variable.
///
/// Returns:
/// - `Some(vec![condition, ...])` for a supported polynomial inequality.
/// - `Some(vec![])` when the solution set is empty.
/// - `None` for non-inequality or non-polynomial inputs.
///
/// The all-real solution set is represented by `GreaterEqual(0, 0)`, matching
/// the Python and TypeScript ports.
pub fn try_solve_inequality(ineq: &IRNode, variable: &IRNode) -> Option<Vec<IRNode>> {
    let IRNode::Symbol(var_name) = variable else {
        return None;
    };
    let IRNode::Apply(apply_node) = ineq else {
        return None;
    };
    if apply_node.args.len() != 2 {
        return None;
    }
    let IRNode::Symbol(head) = &apply_node.head else {
        return None;
    };
    if !matches!(head.as_str(), LESS | GREATER | LESS_EQUAL | GREATER_EQUAL) {
        return None;
    }

    let normalized = apply(
        sym(SUB),
        vec![apply_node.args[0].clone(), apply_node.args[1].clone()],
    );
    let coeffs = extract_polynomial(&normalized, var_name, MAX_INEQUALITY_DEGREE)?;
    let want_positive = matches!(head.as_str(), GREATER | GREATER_EQUAL);
    let strict = matches!(head.as_str(), LESS | GREATER);
    Some(solve_polynomial_sign(
        &coeffs,
        variable,
        want_positive,
        strict,
    ))
}

fn solve_polynomial_sign(
    coeffs_input: &[Frac],
    variable: &IRNode,
    want_positive: bool,
    strict: bool,
) -> Vec<IRNode> {
    let coeffs = trim_polynomial(coeffs_input);
    let degree = coeffs.len() - 1;
    if degree == 0 {
        return if sign_matches_frac(coeffs[0], want_positive, strict) {
            vec![all_reals_sentinel()]
        } else {
            vec![]
        };
    }

    let roots = real_boundary_roots(&coeffs);
    if roots.is_empty() {
        return if sign_matches_float(evaluate_polynomial(&coeffs, 0.0), want_positive, strict) {
            vec![all_reals_sentinel()]
        } else {
            vec![]
        };
    }

    let mut intervals = Vec::new();
    let root_values: Vec<f64> = roots.iter().map(|root| root.value).collect();
    for (index, sample) in interval_samples(&root_values).into_iter().enumerate() {
        if !sign_matches_float(evaluate_polynomial(&coeffs, sample), want_positive, true) {
            continue;
        }
        let lower = index
            .checked_sub(1)
            .and_then(|i| roots.get(i))
            .map(|r| &r.node);
        let upper = roots.get(index).map(|r| &r.node);
        intervals.push(make_interval(variable, lower, upper, strict, strict));
    }

    if !strict && intervals.len() == roots.len() + 1 {
        vec![all_reals_sentinel()]
    } else {
        intervals
    }
}

#[derive(Debug, Clone)]
struct BoundaryRoot {
    value: f64,
    node: IRNode,
}

fn real_boundary_roots(coeffs_ascending: &[Frac]) -> Vec<BoundaryRoot> {
    let exact_roots: Vec<BoundaryRoot> = exact_polynomial_roots(coeffs_ascending)
        .into_iter()
        .filter_map(|node| numeric_value(&node).map(|value| BoundaryRoot { value, node }))
        .collect();

    let numeric_coeffs: Vec<Complex> = coeffs_ascending
        .iter()
        .rev()
        .map(|coef| Complex::new(frac_to_f64(*coef), 0.0))
        .collect();
    let mut roots = Vec::new();
    for root in nsolve_poly(&numeric_coeffs, 200, 1e-12) {
        if root.im.abs() > REAL_ROOT_TOL {
            continue;
        }
        if roots
            .iter()
            .any(|candidate: &BoundaryRoot| (candidate.value - root.re).abs() < 1e-7)
        {
            continue;
        }
        let exact = exact_roots
            .iter()
            .find(|candidate| (candidate.value - root.re).abs() < 1e-7)
            .map(|candidate| candidate.node.clone());
        roots.push(BoundaryRoot {
            value: root.re,
            node: exact.unwrap_or_else(|| flt(root.re)),
        });
    }

    if roots.is_empty() && coeffs_ascending.len() == 2 {
        let root = -coeffs_ascending[0] / coeffs_ascending[1];
        roots.push(BoundaryRoot {
            value: frac_to_f64(root),
            node: root.to_irnode(),
        });
    }

    roots.sort_by(|lhs, rhs| lhs.value.total_cmp(&rhs.value));
    roots
}

fn exact_polynomial_roots(coeffs_ascending: &[Frac]) -> Vec<IRNode> {
    let coeffs = trim_polynomial(coeffs_ascending);
    match coeffs.len() - 1 {
        1 => match solve_linear(coeffs[1], coeffs[0]) {
            SolveResult::Solutions(roots) => roots,
            SolveResult::All => Vec::new(),
        },
        2 => match solve_quadratic(coeffs[2], coeffs[1], coeffs[0]) {
            SolveResult::Solutions(roots) => roots,
            SolveResult::All => Vec::new(),
        },
        3 => match solve_cubic(coeffs[3], coeffs[2], coeffs[1], coeffs[0]) {
            SolveResult::Solutions(roots) => roots,
            SolveResult::All => Vec::new(),
        },
        4 => match solve_quartic(coeffs[4], coeffs[3], coeffs[2], coeffs[1], coeffs[0]) {
            SolveResult::Solutions(roots) => roots,
            SolveResult::All => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn interval_samples(roots: &[f64]) -> Vec<f64> {
    if roots.is_empty() {
        return vec![0.0];
    }
    let mut samples = vec![roots[0] - (roots[0].abs() * 0.5).max(1.0)];
    for pair in roots.windows(2) {
        samples.push((pair[0] + pair[1]) / 2.0);
    }
    let last = *roots.last().expect("non-empty roots");
    samples.push(last + (last.abs() * 0.5).max(1.0));
    samples
}

fn make_interval(
    variable: &IRNode,
    lower: Option<&IRNode>,
    upper: Option<&IRNode>,
    lower_strict: bool,
    upper_strict: bool,
) -> IRNode {
    match (lower, upper) {
        (None, None) => all_reals_sentinel(),
        (None, Some(hi)) => apply(
            sym(if upper_strict { LESS } else { LESS_EQUAL }),
            vec![variable.clone(), hi.clone()],
        ),
        (Some(lo), None) => apply(
            sym(if lower_strict { GREATER } else { GREATER_EQUAL }),
            vec![variable.clone(), lo.clone()],
        ),
        (Some(lo), Some(hi)) => apply(
            sym(AND),
            vec![
                apply(
                    sym(if lower_strict { GREATER } else { GREATER_EQUAL }),
                    vec![variable.clone(), lo.clone()],
                ),
                apply(
                    sym(if upper_strict { LESS } else { LESS_EQUAL }),
                    vec![variable.clone(), hi.clone()],
                ),
            ],
        ),
    }
}

fn all_reals_sentinel() -> IRNode {
    apply(sym(GREATER_EQUAL), vec![int(0), int(0)])
}

fn sign_matches_frac(value: Frac, want_positive: bool, strict: bool) -> bool {
    let cmp = value.cmp(&Frac::zero());
    if want_positive {
        if strict {
            cmp.is_gt()
        } else {
            !cmp.is_lt()
        }
    } else if strict {
        cmp.is_lt()
    } else {
        !cmp.is_gt()
    }
}

fn sign_matches_float(value: f64, want_positive: bool, strict: bool) -> bool {
    let cmp = if value.abs() < 1e-9 {
        0
    } else if value < 0.0 {
        -1
    } else {
        1
    };
    if want_positive {
        if strict {
            cmp > 0
        } else {
            cmp >= 0
        }
    } else if strict {
        cmp < 0
    } else {
        cmp <= 0
    }
}

fn evaluate_polynomial(coeffs: &[Frac], x: f64) -> f64 {
    coeffs
        .iter()
        .rev()
        .fold(0.0, |acc, coef| acc.mul_add(x, frac_to_f64(*coef)))
}

fn extract_polynomial(node: &IRNode, variable: &str, max_degree: usize) -> Option<Vec<Frac>> {
    if let Some(constant) = node_to_frac(node) {
        return Some(vec![constant]);
    }

    if let IRNode::Symbol(name) = node {
        if name != variable {
            return None;
        }
        return Some(vec![Frac::zero(), Frac::one()]);
    }

    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    let IRNode::Symbol(head) = &apply_node.head else {
        return None;
    };

    match head.as_str() {
        ADD => {
            let mut result = vec![Frac::zero()];
            for arg in &apply_node.args {
                let poly = extract_polynomial(arg, variable, max_degree)?;
                result = add_polynomials(&result, &poly);
                if result.len() - 1 > max_degree {
                    return None;
                }
            }
            Some(trim_polynomial(&result))
        }
        SUB if apply_node.args.len() == 2 => {
            let lhs = extract_polynomial(&apply_node.args[0], variable, max_degree)?;
            let rhs = extract_polynomial(&apply_node.args[1], variable, max_degree)?;
            Some(trim_polynomial(&add_polynomials(
                &lhs,
                &scale_polynomial(&rhs, Frac::from_int(-1)),
            )))
        }
        NEG if apply_node.args.len() == 1 => {
            let poly = extract_polynomial(&apply_node.args[0], variable, max_degree)?;
            Some(scale_polynomial(&poly, Frac::from_int(-1)))
        }
        MUL => {
            let mut result = vec![Frac::one()];
            for arg in &apply_node.args {
                let poly = extract_polynomial(arg, variable, max_degree)?;
                result = multiply_polynomials(&result, &poly, max_degree)?;
            }
            Some(trim_polynomial(&result))
        }
        POW if apply_node.args.len() == 2 => {
            let IRNode::Integer(exponent) = &apply_node.args[1] else {
                return None;
            };
            if *exponent < 0 || *exponent as usize > max_degree {
                return None;
            }
            let base = extract_polynomial(&apply_node.args[0], variable, max_degree)?;
            let mut result = vec![Frac::one()];
            for _ in 0..*exponent {
                result = multiply_polynomials(&result, &base, max_degree)?;
            }
            Some(trim_polynomial(&result))
        }
        _ => None,
    }
}

fn add_polynomials(lhs: &[Frac], rhs: &[Frac]) -> Vec<Frac> {
    let len = lhs.len().max(rhs.len());
    trim_polynomial(
        &(0..len)
            .map(|i| {
                lhs.get(i).copied().unwrap_or_else(Frac::zero)
                    + rhs.get(i).copied().unwrap_or_else(Frac::zero)
            })
            .collect::<Vec<_>>(),
    )
}

fn scale_polynomial(poly: &[Frac], scalar: Frac) -> Vec<Frac> {
    trim_polynomial(&poly.iter().map(|coef| *coef * scalar).collect::<Vec<_>>())
}

fn multiply_polynomials(lhs: &[Frac], rhs: &[Frac], max_degree: usize) -> Option<Vec<Frac>> {
    let mut result = vec![Frac::zero(); lhs.len() + rhs.len() - 1];
    for (i, lhs_coef) in lhs.iter().enumerate() {
        for (j, rhs_coef) in rhs.iter().enumerate() {
            if i + j > max_degree {
                return None;
            }
            result[i + j] = result[i + j] + *lhs_coef * *rhs_coef;
        }
    }
    Some(trim_polynomial(&result))
}

fn trim_polynomial(poly: &[Frac]) -> Vec<Frac> {
    let mut end = poly.len().saturating_sub(1);
    while end > 0 && poly[end].is_zero() {
        end -= 1;
    }
    poly[..=end].to_vec()
}

fn numeric_value(node: &IRNode) -> Option<f64> {
    match node {
        IRNode::Integer(value) => Some(*value as f64),
        IRNode::Rational(numer, denom) => Some(*numer as f64 / *denom as f64),
        IRNode::Float(value) => Some(*value),
        IRNode::Apply(apply_node)
            if is_head_name(&apply_node.head, NEG) && apply_node.args.len() == 1 =>
        {
            numeric_value(&apply_node.args[0]).map(|value| -value)
        }
        _ => None,
    }
}

fn node_to_frac(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(value) => Some(Frac::from_int(*value)),
        IRNode::Rational(numer, denom) => Some(Frac::new(*numer, *denom)),
        _ => None,
    }
}

fn frac_to_f64(value: Frac) -> f64 {
    value.numer as f64 / value.denom as f64
}

fn is_head_name(node: &IRNode, expected: &str) -> bool {
    matches!(node, IRNode::Symbol(name) if name == expected)
}

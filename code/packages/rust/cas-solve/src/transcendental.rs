//! Direct transcendental equation solving in one variable.
//!
//! This handles the Phase 26 package-level slice for equations of the form
//! `f(linear) = constant`, preserving symbolic inverse functions and periodic
//! integer families where appropriate.

use symbolic_ir::{
    apply, int, sym, IRNode, ACOS, ACOSH, ADD, ASIN, ASINH, ATAN, ATANH, COS, COSH, DIV, EQUAL,
    EXP, LOG, MUL, NEG, POW, SIN, SINH, SUB, TAN, TANH,
};

use crate::frac::Frac;

pub const FREE_INTEGER: &str = "FreeInteger";

/// Try to solve a direct transcendental equation in one variable.
///
/// Accepts `Equal(lhs, rhs)` or a bare expression treated as `expr = 0`.
/// Returns:
/// - `Some(vec![...])` for supported direct `f(linear) = constant` forms.
/// - `None` when the equation is not in the supported direct form.
pub fn try_solve_transcendental(eq: &IRNode, variable: &IRNode) -> Option<Vec<IRNode>> {
    let IRNode::Symbol(var_name) = variable else {
        return None;
    };

    if let Some((lhs, rhs)) = split_equal(eq) {
        return try_func_eq_const(lhs, rhs, var_name)
            .or_else(|| try_func_eq_const(rhs, lhs, var_name));
    }

    try_func_eq_const(eq, &int(0), var_name)
}

fn try_func_eq_const(
    func_side: &IRNode,
    const_side: &IRNode,
    variable: &str,
) -> Option<Vec<IRNode>> {
    if !is_const_wrt(const_side, variable) {
        return None;
    }

    let IRNode::Apply(apply_node) = func_side else {
        return None;
    };
    if apply_node.args.len() != 1 {
        return None;
    }
    let IRNode::Symbol(head) = &apply_node.head else {
        return None;
    };

    let (a, b) = extract_linear(&apply_node.args[0], variable)?;
    let two_pi_k = apply(
        sym(MUL),
        vec![int(2), apply(sym(MUL), vec![sym("%pi"), sym(FREE_INTEGER)])],
    );

    match head.as_str() {
        SIN => {
            let asin_c = apply(sym(ASIN), vec![const_side.clone()]);
            Some(vec![
                solve_linear_for_value(
                    a,
                    b,
                    apply(sym(ADD), vec![asin_c.clone(), two_pi_k.clone()]),
                ),
                solve_linear_for_value(
                    a,
                    b,
                    apply(
                        sym(ADD),
                        vec![apply(sym(SUB), vec![sym("%pi"), asin_c]), two_pi_k],
                    ),
                ),
            ])
        }
        COS => {
            let acos_c = apply(sym(ACOS), vec![const_side.clone()]);
            Some(vec![
                solve_linear_for_value(
                    a,
                    b,
                    apply(sym(ADD), vec![acos_c.clone(), two_pi_k.clone()]),
                ),
                solve_linear_for_value(
                    a,
                    b,
                    apply(sym(ADD), vec![apply(sym(NEG), vec![acos_c]), two_pi_k]),
                ),
            ])
        }
        TAN => Some(vec![solve_linear_for_value(
            a,
            b,
            apply(
                sym(ADD),
                vec![
                    apply(sym(ATAN), vec![const_side.clone()]),
                    apply(sym(MUL), vec![sym("%pi"), sym(FREE_INTEGER)]),
                ],
            ),
        )]),
        EXP => Some(vec![solve_linear_for_value(
            a,
            b,
            apply(sym(LOG), vec![const_side.clone()]),
        )]),
        LOG => Some(vec![solve_linear_for_value(
            a,
            b,
            apply(sym(EXP), vec![const_side.clone()]),
        )]),
        SINH => Some(vec![solve_linear_for_value(
            a,
            b,
            apply(sym(ASINH), vec![const_side.clone()]),
        )]),
        COSH => {
            let acosh_c = apply(sym(ACOSH), vec![const_side.clone()]);
            Some(vec![
                solve_linear_for_value(a, b, acosh_c.clone()),
                solve_linear_for_value(a, b, apply(sym(NEG), vec![acosh_c])),
            ])
        }
        TANH => Some(vec![solve_linear_for_value(
            a,
            b,
            apply(sym(ATANH), vec![const_side.clone()]),
        )]),
        _ => None,
    }
}

fn split_equal(node: &IRNode) -> Option<(&IRNode, &IRNode)> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    if is_head_name(&apply_node.head, EQUAL) && apply_node.args.len() == 2 {
        Some((&apply_node.args[0], &apply_node.args[1]))
    } else {
        None
    }
}

fn extract_linear(node: &IRNode, variable: &str) -> Option<(Frac, Frac)> {
    let coeffs = extract_polynomial(node, variable, 1)?;
    if coeffs.len() != 2 || coeffs[1].is_zero() {
        return None;
    }
    Some((coeffs[1], coeffs[0]))
}

fn solve_linear_for_value(a: Frac, b: Frac, value: IRNode) -> IRNode {
    let shifted = if b.is_zero() {
        value
    } else {
        apply(sym(SUB), vec![value, b.to_irnode()])
    };
    if a == Frac::one() {
        shifted
    } else {
        apply(sym(DIV), vec![shifted, a.to_irnode()])
    }
}

fn is_const_wrt(node: &IRNode, variable: &str) -> bool {
    match node {
        IRNode::Symbol(name) => name != variable,
        IRNode::Apply(apply_node) => {
            is_const_wrt(&apply_node.head, variable)
                && apply_node
                    .args
                    .iter()
                    .all(|arg| is_const_wrt(arg, variable))
        }
        _ => true,
    }
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

fn node_to_frac(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(value) => Some(Frac::from_int(*value)),
        IRNode::Rational(numer, denom) => Some(Frac::new(*numer, *denom)),
        _ => None,
    }
}

fn is_head_name(node: &IRNode, expected: &str) -> bool {
    matches!(node, IRNode::Symbol(name) if name == expected)
}

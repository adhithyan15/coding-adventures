use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, MUL, NEG, POW, SQRT, SUB};

use crate::algebraic::{factor_over_extension, AlgCoeff, AlgPoly};
use crate::rational::Rational;

pub const ALG_FACTOR: &str = "AlgFactor";

pub fn extract_radical_d(node: &IRNode) -> Option<i64> {
    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    if apply_node.head != sym(SQRT) || apply_node.args.len() != 1 {
        return None;
    }
    let IRNode::Integer(value) = apply_node.args[0] else {
        return None;
    };
    if value <= 0 || is_square(value) {
        return None;
    }
    Some(value)
}

pub fn alg_factor_ir(poly: &IRNode, sqrt_d: &IRNode, variable: &IRNode) -> Option<IRNode> {
    let d = extract_radical_d(sqrt_d)?;
    let coeffs = ir_to_integer_poly(poly, variable)?;
    let factors = factor_over_extension(&coeffs, d)?;
    Some(factors_to_ir(&factors, variable, sqrt_d))
}

pub fn factors_to_ir(factors: &[AlgPoly], variable: &IRNode, sqrt_d: &IRNode) -> IRNode {
    let mut iter = factors
        .iter()
        .map(|factor| alg_poly_to_ir(factor, variable, sqrt_d));
    let Some(first) = iter.next() else {
        return int(1);
    };
    iter.fold(first, |acc, factor| apply(sym(MUL), vec![acc, factor]))
}

pub fn alg_poly_to_ir(poly: &AlgPoly, variable: &IRNode, sqrt_d: &IRNode) -> IRNode {
    let mut terms = Vec::new();
    for (degree, coeff) in poly.iter().enumerate() {
        if coeff.rational.is_zero() && coeff.radical.is_zero() {
            continue;
        }

        let coeff_ir = alg_coeff_to_ir(*coeff, sqrt_d);
        let term = match degree {
            0 => coeff_ir,
            1 => multiply_coeff(coeff_ir, variable.clone()),
            _ => {
                let power = apply(sym(POW), vec![variable.clone(), int(degree as i64)]);
                multiply_coeff(coeff_ir, power)
            }
        };
        terms.push(term);
    }

    if terms.is_empty() {
        int(0)
    } else {
        terms
            .into_iter()
            .reduce(|acc, term| apply(sym(ADD), vec![acc, term]))
            .unwrap()
    }
}

pub fn alg_coeff_to_ir(coeff: AlgCoeff, sqrt_d: &IRNode) -> IRNode {
    let rational = rational_to_ir(coeff.rational);
    if coeff.radical.is_zero() {
        return rational;
    }

    let radical_part = if coeff.radical.is_one() {
        sqrt_d.clone()
    } else if coeff.radical == Rational::from_int(-1) {
        apply(sym(NEG), vec![sqrt_d.clone()])
    } else {
        apply(
            sym(MUL),
            vec![rational_to_ir(coeff.radical), sqrt_d.clone()],
        )
    };

    if coeff.rational.is_zero() {
        radical_part
    } else {
        apply(sym(ADD), vec![rational, radical_part])
    }
}

pub fn ir_to_integer_poly(node: &IRNode, variable: &IRNode) -> Option<Vec<i64>> {
    let coeffs = ir_to_rational_poly(node, variable)?;
    let mut out = Vec::with_capacity(coeffs.len());
    for coeff in coeffs {
        if coeff.denom != 1 {
            return None;
        }
        out.push(coeff.numer);
    }
    trim_trailing_zeros(out)
}

pub fn ir_to_cleared_integer_poly(node: &IRNode, variable: &IRNode) -> Option<Vec<i64>> {
    let coeffs = ir_to_rational_poly(node, variable)?;
    let lcm = coeffs
        .iter()
        .try_fold(1_i64, |acc, coeff| checked_lcm(acc, coeff.denom))?;

    let mut out = Vec::with_capacity(coeffs.len());
    for coeff in coeffs {
        out.push(coeff.numer.checked_mul(lcm / coeff.denom)?);
    }
    trim_trailing_zeros(out)
}

fn ir_to_rational_poly(node: &IRNode, variable: &IRNode) -> Option<Vec<Rational>> {
    if node == variable {
        return Some(vec![Rational::ZERO, Rational::ONE]);
    }

    match node {
        IRNode::Integer(value) => Some(vec![Rational::from_int(*value)]),
        IRNode::Rational(numer, denom) => Some(vec![Rational::new(*numer, *denom)]),
        IRNode::Apply(apply_node) if apply_node.head == sym(ADD) => {
            let mut acc = vec![Rational::ZERO];
            for arg in &apply_node.args {
                acc = poly_add(&acc, &ir_to_rational_poly(arg, variable)?);
            }
            Some(acc)
        }
        IRNode::Apply(apply_node) if apply_node.head == sym(SUB) && apply_node.args.len() == 2 => {
            let lhs = ir_to_rational_poly(&apply_node.args[0], variable)?;
            let rhs = ir_to_rational_poly(&apply_node.args[1], variable)?;
            Some(poly_sub(&lhs, &rhs))
        }
        IRNode::Apply(apply_node) if apply_node.head == sym(MUL) => {
            let mut acc = vec![Rational::ONE];
            for arg in &apply_node.args {
                acc = poly_mul(&acc, &ir_to_rational_poly(arg, variable)?);
            }
            Some(acc)
        }
        IRNode::Apply(apply_node) if apply_node.head == sym(NEG) && apply_node.args.len() == 1 => {
            let poly = ir_to_rational_poly(&apply_node.args[0], variable)?;
            Some(poly.into_iter().map(|coeff| -coeff).collect())
        }
        IRNode::Apply(apply_node) if apply_node.head == sym(POW) && apply_node.args.len() == 2 => {
            let IRNode::Integer(exp) = apply_node.args[1] else {
                return None;
            };
            if exp < 0 {
                return None;
            }
            let base = ir_to_rational_poly(&apply_node.args[0], variable)?;
            let mut acc = vec![Rational::ONE];
            for _ in 0..exp {
                acc = poly_mul(&acc, &base);
            }
            Some(acc)
        }
        _ => None,
    }
}

fn multiply_coeff(coeff: IRNode, term: IRNode) -> IRNode {
    if coeff == int(1) {
        term
    } else if coeff == int(-1) {
        apply(sym(NEG), vec![term])
    } else {
        apply(sym(MUL), vec![coeff, term])
    }
}

fn rational_to_ir(value: Rational) -> IRNode {
    if value.denom == 1 {
        int(value.numer)
    } else {
        rat(value.numer, value.denom)
    }
}

fn trim_trailing_zeros(mut poly: Vec<i64>) -> Option<Vec<i64>> {
    while poly.last() == Some(&0) {
        poly.pop();
    }
    Some(poly)
}

fn checked_lcm(lhs: i64, rhs: i64) -> Option<i64> {
    let gcd = integer_gcd(lhs.unsigned_abs(), rhs.unsigned_abs()) as i64;
    (lhs / gcd).checked_mul(rhs)
}

fn integer_gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn poly_add(lhs: &[Rational], rhs: &[Rational]) -> Vec<Rational> {
    let len = lhs.len().max(rhs.len());
    let mut out = vec![Rational::ZERO; len];
    for index in 0..len {
        out[index] = lhs.get(index).copied().unwrap_or(Rational::ZERO)
            + rhs.get(index).copied().unwrap_or(Rational::ZERO);
    }
    out
}

fn poly_sub(lhs: &[Rational], rhs: &[Rational]) -> Vec<Rational> {
    let len = lhs.len().max(rhs.len());
    let mut out = vec![Rational::ZERO; len];
    for index in 0..len {
        out[index] = lhs.get(index).copied().unwrap_or(Rational::ZERO)
            - rhs.get(index).copied().unwrap_or(Rational::ZERO);
    }
    out
}

fn poly_mul(lhs: &[Rational], rhs: &[Rational]) -> Vec<Rational> {
    if lhs.is_empty() || rhs.is_empty() {
        return vec![Rational::ZERO];
    }
    let mut out = vec![Rational::ZERO; lhs.len() + rhs.len() - 1];
    for (i, lhs_coeff) in lhs.iter().enumerate() {
        for (j, rhs_coeff) in rhs.iter().enumerate() {
            out[i + j] = out[i + j] + *lhs_coeff * *rhs_coeff;
        }
    }
    out
}

fn is_square(value: i64) -> bool {
    let root = (value as f64).sqrt() as i64;
    root * root == value || (root + 1) * (root + 1) == value
}

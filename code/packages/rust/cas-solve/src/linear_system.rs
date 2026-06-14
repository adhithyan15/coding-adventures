//! Exact linear-system solving with Gaussian elimination.
//!
//! Equations may be `Equal(lhs, rhs)` nodes or zero-form expressions. Results
//! are returned as `Rule(var, value)` IR nodes in the same order as `variables`.

use std::collections::HashMap;

use symbolic_ir::{apply, sym, IRNode, ADD, EQUAL, MUL, NEG, POW, RULE, SUB};

use crate::frac::Frac;

/// Solve a square linear system over exact rational coefficients.
///
/// Returns `None` for empty, non-square, singular, or non-linear systems.
pub fn solve_linear_system(equations: &[IRNode], variables: &[IRNode]) -> Option<Vec<IRNode>> {
    let dimension = variables.len();
    if equations.len() != dimension || dimension == 0 {
        return None;
    }

    let mut variable_columns = HashMap::new();
    for (index, variable) in variables.iter().enumerate() {
        if let IRNode::Symbol(name) = variable {
            variable_columns.insert(name.clone(), index);
        } else {
            return None;
        }
    }

    let mut matrix = Vec::with_capacity(dimension);
    for equation in equations {
        matrix.push(equation_to_row(equation, &variable_columns, dimension)?);
    }

    for col in 0..dimension {
        let pivot_row = (col..dimension)
            .max_by(|lhs, rhs| frac_abs_cmp(matrix[*lhs][col], matrix[*rhs][col]))
            .expect("non-empty pivot range");
        if matrix[pivot_row][col].is_zero() {
            return None;
        }
        matrix.swap(col, pivot_row);

        let pivot = matrix[col][col];
        for row in (col + 1)..dimension {
            if matrix[row][col].is_zero() {
                continue;
            }
            let factor = matrix[row][col] / pivot;
            for j in col..=dimension {
                matrix[row][j] = matrix[row][j] - factor * matrix[col][j];
            }
        }
    }

    let mut solution = vec![Frac::zero(); dimension];
    for row in (0..dimension).rev() {
        if matrix[row][row].is_zero() {
            return None;
        }
        let mut rhs = matrix[row][dimension];
        for (col, value) in solution.iter().enumerate().skip(row + 1) {
            rhs = rhs - matrix[row][col] * *value;
        }
        solution[row] = rhs / matrix[row][row];
    }

    Some(
        variables
            .iter()
            .zip(solution)
            .map(|(variable, value)| apply(sym(RULE), vec![variable.clone(), value.to_irnode()]))
            .collect(),
    )
}

fn equation_to_row(
    equation: &IRNode,
    variable_columns: &HashMap<String, usize>,
    dimension: usize,
) -> Option<Vec<Frac>> {
    let normalized;
    let expr = if let IRNode::Apply(apply_node) = equation {
        if is_head_name(&apply_node.head, EQUAL) && apply_node.args.len() == 2 {
            normalized = apply(
                sym(SUB),
                vec![apply_node.args[0].clone(), apply_node.args[1].clone()],
            );
            &normalized
        } else {
            equation
        }
    } else {
        equation
    };

    let linear = linear_eval(expr, variable_columns, dimension)?;
    let mut row = linear.coeffs;
    row.push(-linear.constant);
    Some(row)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinearForm {
    coeffs: Vec<Frac>,
    constant: Frac,
}

fn linear_eval(
    node: &IRNode,
    variable_columns: &HashMap<String, usize>,
    dimension: usize,
) -> Option<LinearForm> {
    if let Some(constant) = node_to_frac(node) {
        return Some(LinearForm {
            coeffs: zero_vector(dimension),
            constant,
        });
    }

    if let IRNode::Symbol(name) = node {
        let column = *variable_columns.get(name)?;
        let mut coeffs = zero_vector(dimension);
        coeffs[column] = Frac::one();
        return Some(LinearForm {
            coeffs,
            constant: Frac::zero(),
        });
    }

    let IRNode::Apply(apply_node) = node else {
        return None;
    };
    let head = match &apply_node.head {
        IRNode::Symbol(name) => name.as_str(),
        _ => return None,
    };

    match head {
        ADD => {
            let mut coeffs = zero_vector(dimension);
            let mut constant = Frac::zero();
            for arg in &apply_node.args {
                let form = linear_eval(arg, variable_columns, dimension)?;
                coeffs = add_vectors(&coeffs, &form.coeffs);
                constant = constant + form.constant;
            }
            Some(LinearForm { coeffs, constant })
        }
        SUB if apply_node.args.len() == 2 => {
            let lhs = linear_eval(&apply_node.args[0], variable_columns, dimension)?;
            let rhs = linear_eval(&apply_node.args[1], variable_columns, dimension)?;
            Some(LinearForm {
                coeffs: sub_vectors(&lhs.coeffs, &rhs.coeffs),
                constant: lhs.constant - rhs.constant,
            })
        }
        NEG if apply_node.args.len() == 1 => {
            let form = linear_eval(&apply_node.args[0], variable_columns, dimension)?;
            Some(LinearForm {
                coeffs: form.coeffs.into_iter().map(|coef| -coef).collect(),
                constant: -form.constant,
            })
        }
        MUL => linear_eval_product(&apply_node.args, variable_columns, dimension),
        POW if apply_node.args.len() == 2 => match &apply_node.args[1] {
            IRNode::Integer(0) => Some(LinearForm {
                coeffs: zero_vector(dimension),
                constant: Frac::one(),
            }),
            IRNode::Integer(1) => linear_eval(&apply_node.args[0], variable_columns, dimension),
            _ => None,
        },
        _ => None,
    }
}

fn linear_eval_product(
    args: &[IRNode],
    variable_columns: &HashMap<String, usize>,
    dimension: usize,
) -> Option<LinearForm> {
    let mut scalar = Frac::one();
    let mut linear_part: Option<LinearForm> = None;

    for arg in args {
        if let Some(value) = node_to_frac(arg) {
            scalar = scalar * value;
            continue;
        }

        let form = linear_eval(arg, variable_columns, dimension)?;
        if is_zero_vector(&form.coeffs) {
            scalar = scalar * form.constant;
        } else if linear_part.is_some() {
            return None;
        } else {
            linear_part = Some(form);
        }
    }

    if let Some(form) = linear_part {
        Some(LinearForm {
            coeffs: form.coeffs.into_iter().map(|coef| coef * scalar).collect(),
            constant: form.constant * scalar,
        })
    } else {
        Some(LinearForm {
            coeffs: zero_vector(dimension),
            constant: scalar,
        })
    }
}

fn node_to_frac(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(value) => Some(Frac::from_int(*value)),
        IRNode::Rational(numer, denom) => Some(Frac::new(*numer, *denom)),
        _ => None,
    }
}

fn zero_vector(length: usize) -> Vec<Frac> {
    vec![Frac::zero(); length]
}

fn add_vectors(lhs: &[Frac], rhs: &[Frac]) -> Vec<Frac> {
    lhs.iter().zip(rhs).map(|(a, b)| *a + *b).collect()
}

fn sub_vectors(lhs: &[Frac], rhs: &[Frac]) -> Vec<Frac> {
    lhs.iter().zip(rhs).map(|(a, b)| *a - *b).collect()
}

fn is_zero_vector(values: &[Frac]) -> bool {
    values.iter().all(Frac::is_zero)
}

fn frac_abs_cmp(lhs: Frac, rhs: Frac) -> std::cmp::Ordering {
    let lhs_abs = (lhs.numer as i128).abs() * rhs.denom as i128;
    let rhs_abs = (rhs.numer as i128).abs() * lhs.denom as i128;
    lhs_abs.cmp(&rhs_abs)
}

fn is_head_name(node: &IRNode, expected: &str) -> bool {
    matches!(node, IRNode::Symbol(name) if name == expected)
}

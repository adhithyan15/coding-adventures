//! Characteristic polynomial helpers for exact-rational matrices.

use std::cmp::Ordering;

use cas_solve::frac::Frac as SolveFrac;
use cas_solve::{solve_cubic, solve_linear, solve_quadratic, solve_quartic, SolveResult};
use symbolic_ir::{apply, int, sym, IRNode, ADD, LIST, MUL, NEG, POW};

use crate::matrix::{matrix, num_cols, num_rows, MatrixError, MatrixResult};
use crate::rowreduce::{matrix_to_fracs, Frac};
use crate::subspaces::nullspace;

/// Return coefficients for `det(lambda I - A)` in ascending power order.
pub fn char_poly_coeffs(m: &IRNode) -> MatrixResult<Vec<IRNode>> {
    char_poly_coeff_fracs(m)?
        .into_iter()
        .map(Frac::to_irnode)
        .collect()
}

/// Return `det(lambda I - A)` as an un-simplified IR polynomial.
pub fn charpoly(m: &IRNode, variable: &IRNode) -> MatrixResult<IRNode> {
    let terms = char_poly_coeff_fracs(m)?.into_iter().enumerate().try_fold(
        Vec::new(),
        |mut terms, (power, coeff)| -> MatrixResult<Vec<IRNode>> {
            if coeff.is_zero() {
                return Ok(terms);
            }
            let coeff_node = coeff.to_irnode()?;
            if power == 0 {
                terms.push(coeff_node);
                return Ok(terms);
            }
            let variable_power = if power == 1 {
                variable.clone()
            } else {
                apply(sym(POW), vec![variable.clone(), int(power as i64)])
            };
            if coeff == Frac::one() {
                terms.push(variable_power);
            } else {
                terms.push(apply(sym(MUL), vec![coeff_node, variable_power]));
            }
            Ok(terms)
        },
    )?;

    Ok(match terms.len() {
        0 => int(0),
        1 => terms.into_iter().next().unwrap(),
        _ => apply(sym(ADD), terms),
    })
}

/// Return `List(List(lambda, multiplicity), ...)` for exact matrices up to 4x4.
pub fn eigenvalues(m: &IRNode) -> MatrixResult<IRNode> {
    let n = num_rows(m)?;
    let ncols = num_cols(m)?;
    if n != ncols {
        return Err(MatrixError(format!(
            "eigenvalues: matrix must be square, got {n}x{ncols}"
        )));
    }
    if n > 4 {
        return Err(MatrixError(
            "eigenvalues: only matrices up to 4x4 are supported in this Rust port".into(),
        ));
    }

    let coeff_fracs = char_poly_coeff_fracs(m)?;
    let coeffs: Vec<SolveFrac> = coeff_fracs
        .iter()
        .copied()
        .map(to_solve_frac)
        .collect::<MatrixResult<Vec<SolveFrac>>>()?;
    let result = solve_characteristic(&coeffs)?;

    let SolveResult::Solutions(mut roots) = result else {
        return Ok(apply(sym("Eigenvalues"), vec![m.clone()]));
    };
    roots.sort_by(compare_eigen_root);
    let root_count = roots.len();
    Ok(apply(
        sym("List"),
        roots
            .into_iter()
            .map(|root| {
                let multiplicity = root_multiplicity(&root, &coeff_fracs, n, root_count) as i64;
                apply(sym("List"), vec![root, int(multiplicity)])
            })
            .collect(),
    ))
}

/// Return `List(List(lambda, multiplicity, List(vector, ...)), ...)`.
///
/// Exact eigenvector bases are returned for rational eigenvalues.  Irrational
/// or complex roots keep the eigenvalue/multiplicity pair but receive an empty
/// vector list, matching the Python reference fallback.
pub fn eigenvectors(m: &IRNode) -> MatrixResult<IRNode> {
    let rows = matrix_to_fracs(m)?;
    let eigs = eigenvalues(m)?;
    let IRNode::Apply(eigs_apply) = eigs else {
        return Ok(apply(sym("Eigenvectors"), vec![m.clone()]));
    };
    if eigs_apply.head != sym(LIST) {
        return Ok(apply(sym("Eigenvectors"), vec![m.clone()]));
    }

    let mut triples = Vec::new();
    for pair in eigs_apply.args {
        let IRNode::Apply(pair_apply) = pair else {
            triples.push(apply(
                sym(LIST),
                vec![pair, int(1), apply(sym(LIST), vec![])],
            ));
            continue;
        };
        let pair_args = pair_apply.args;
        if pair_args.len() < 2 {
            triples.push(apply(
                sym(LIST),
                vec![
                    apply(sym(LIST), pair_args),
                    int(1),
                    apply(sym(LIST), vec![]),
                ],
            ));
            continue;
        }
        let lambda = pair_args[0].clone();
        let multiplicity = pair_args[1].clone();
        let Some(lambda_frac) = ir_to_frac(&lambda) else {
            triples.push(apply(
                sym(LIST),
                vec![lambda, multiplicity, apply(sym(LIST), vec![])],
            ));
            continue;
        };

        let shifted = rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(|(col_index, entry)| {
                        let adjustment = if row_index == col_index {
                            lambda_frac
                        } else {
                            Frac::zero()
                        };
                        (*entry - adjustment).to_irnode()
                    })
                    .collect::<MatrixResult<Vec<IRNode>>>()
            })
            .collect::<MatrixResult<Vec<Vec<IRNode>>>>()?;
        let vectors = nullspace(&matrix(shifted)?)?;
        triples.push(apply(sym(LIST), vec![lambda, multiplicity, vectors]));
    }

    Ok(apply(sym(LIST), triples))
}

pub(crate) fn char_poly_coeff_fracs(m: &IRNode) -> MatrixResult<Vec<Frac>> {
    let n = num_rows(m)?;
    let ncols = num_cols(m)?;
    if n != ncols {
        return Err(MatrixError(format!(
            "char_poly_coeffs: matrix must be square, got {n}x{ncols}"
        )));
    }

    let rows = matrix_to_fracs(m)?;
    let poly_rows: Vec<Vec<Vec<Frac>>> = rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(col_index, entry)| {
                    if row_index == col_index {
                        vec![-*entry, Frac::one()]
                    } else {
                        vec![-*entry]
                    }
                })
                .collect()
        })
        .collect();

    Ok(det_poly(&poly_rows))
}

fn solve_characteristic(coeffs: &[SolveFrac]) -> MatrixResult<SolveResult> {
    let degree = coeffs.len().saturating_sub(1);
    match degree {
        1 => Ok(solve_linear(coeffs[1], coeffs[0])),
        2 => Ok(solve_quadratic(coeffs[2], coeffs[1], coeffs[0])),
        3 => Ok(solve_cubic(coeffs[3], coeffs[2], coeffs[1], coeffs[0])),
        4 => Ok(solve_quartic(
            coeffs[4], coeffs[3], coeffs[2], coeffs[1], coeffs[0],
        )),
        _ => Err(MatrixError(format!(
            "eigenvalues: unsupported characteristic polynomial degree {degree}"
        ))),
    }
}

fn compare_eigen_root(left: &IRNode, right: &IRNode) -> Ordering {
    match (ir_to_frac(left), ir_to_frac(right)) {
        (Some(left), Some(right)) => (left.numer * right.denom).cmp(&(right.numer * left.denom)),
        _ => Ordering::Equal,
    }
}

fn root_multiplicity(
    root: &IRNode,
    coeffs: &[Frac],
    matrix_size: usize,
    root_count: usize,
) -> usize {
    let Some(rational_root) = ir_to_frac(root) else {
        return if root_count == 1 { matrix_size } else { 1 };
    };

    let mut remaining = coeffs.to_vec();
    let mut multiplicity = 0;
    while remaining.len() > 1 {
        let (quotient, remainder) = divide_by_linear_factor(&remaining, rational_root);
        if !remainder.is_zero() {
            break;
        }
        multiplicity += 1;
        remaining = quotient;
    }
    multiplicity.max(1)
}

fn divide_by_linear_factor(coeffs: &[Frac], root: Frac) -> (Vec<Frac>, Frac) {
    let degree = coeffs.len().saturating_sub(1);
    if degree == 0 {
        return (
            Vec::new(),
            coeffs.first().copied().unwrap_or_else(Frac::zero),
        );
    }

    let mut quotient = vec![Frac::zero(); degree];
    quotient[degree - 1] = coeffs[degree];
    for i in (0..degree.saturating_sub(1)).rev() {
        quotient[i] = coeffs[i + 1] + root * quotient[i + 1];
    }
    let remainder = coeffs[0] + root * quotient[0];
    (quotient, remainder)
}

fn ir_to_frac(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(value) => Some(Frac::from_i64(*value)),
        IRNode::Rational(numer, denom) => Some(Frac::new(*numer as i128, *denom as i128)),
        IRNode::Apply(apply) if apply.head == sym(NEG) && apply.args.len() == 1 => {
            ir_to_frac(&apply.args[0]).map(|value| -value)
        }
        _ => None,
    }
}

fn to_solve_frac(value: Frac) -> MatrixResult<SolveFrac> {
    let node = value.to_irnode()?;
    match node {
        IRNode::Integer(n) => Ok(SolveFrac::from_int(n)),
        IRNode::Rational(n, d) => Ok(SolveFrac::new(n, d)),
        other => Err(MatrixError(format!(
            "eigenvalues: expected rational coefficient, got {other:?}"
        ))),
    }
}

fn det_poly(rows: &[Vec<Vec<Frac>>]) -> Vec<Frac> {
    let n = rows.len();
    match n {
        0 => vec![Frac::one()],
        1 => rows[0][0].clone(),
        2 => {
            let a = &rows[0][0];
            let b = &rows[0][1];
            let c = &rows[1][0];
            let d = &rows[1][1];
            poly_sub(&poly_mul(a, d), &poly_mul(b, c))
        }
        _ => rows[0]
            .iter()
            .enumerate()
            .fold(vec![Frac::zero()], |acc, (col, entry)| {
                let product = poly_mul(entry, &det_poly(&minor_poly(rows, 0, col)));
                if col % 2 == 0 {
                    poly_add(&acc, &product)
                } else {
                    poly_sub(&acc, &product)
                }
            }),
    }
}

fn minor_poly(rows: &[Vec<Vec<Frac>>], skip_row: usize, skip_col: usize) -> Vec<Vec<Vec<Frac>>> {
    rows.iter()
        .enumerate()
        .filter(|(row_index, _)| *row_index != skip_row)
        .map(|(_, row)| {
            row.iter()
                .enumerate()
                .filter(|(col_index, _)| *col_index != skip_col)
                .map(|(_, entry)| entry.clone())
                .collect()
        })
        .collect()
}

fn poly_add(left: &[Frac], right: &[Frac]) -> Vec<Frac> {
    let length = left.len().max(right.len());
    (0..length)
        .map(|i| {
            left.get(i).copied().unwrap_or_else(Frac::zero)
                + right.get(i).copied().unwrap_or_else(Frac::zero)
        })
        .collect()
}

fn poly_sub(left: &[Frac], right: &[Frac]) -> Vec<Frac> {
    let neg_right: Vec<Frac> = right.iter().map(|entry| -*entry).collect();
    poly_add(left, &neg_right)
}

fn poly_mul(left: &[Frac], right: &[Frac]) -> Vec<Frac> {
    if left.is_empty() || right.is_empty() {
        return vec![Frac::zero()];
    }
    let mut result = vec![Frac::zero(); left.len() + right.len() - 1];
    for (i, a) in left.iter().enumerate() {
        for (j, b) in right.iter().enumerate() {
            result[i + j] = result[i + j] + (*a * *b);
        }
    }
    result
}

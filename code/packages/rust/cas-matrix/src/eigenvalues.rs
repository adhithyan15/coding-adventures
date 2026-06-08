//! Characteristic polynomial helpers for exact-rational matrices.

use cas_solve::frac::Frac as SolveFrac;
use cas_solve::{solve_linear, solve_quadratic, SolveResult};
use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, POW};

use crate::matrix::{num_cols, num_rows, MatrixError, MatrixResult};
use crate::rowreduce::{matrix_to_fracs, Frac};

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

/// Return `List(List(lambda, multiplicity), ...)` for 1x1/2x2 exact matrices.
pub fn eigenvalues(m: &IRNode) -> MatrixResult<IRNode> {
    let n = num_rows(m)?;
    let ncols = num_cols(m)?;
    if n != ncols {
        return Err(MatrixError(format!(
            "eigenvalues: matrix must be square, got {n}x{ncols}"
        )));
    }
    if n > 2 {
        return Err(MatrixError(
            "eigenvalues: only 1x1 and 2x2 matrices are supported in this Rust port".into(),
        ));
    }

    let coeffs: Vec<SolveFrac> = char_poly_coeff_fracs(m)?
        .into_iter()
        .map(to_solve_frac)
        .collect::<MatrixResult<Vec<SolveFrac>>>()?;
    let result = if n == 1 {
        solve_linear(coeffs[1], coeffs[0])
    } else {
        solve_quadratic(coeffs[2], coeffs[1], coeffs[0])
    };

    let SolveResult::Solutions(roots) = result else {
        return Ok(apply(sym("Eigenvalues"), vec![m.clone()]));
    };
    let multiplicity = if n == 2 && roots.len() == 1 { 2 } else { 1 };
    Ok(apply(
        sym("List"),
        roots
            .into_iter()
            .map(|root| apply(sym("List"), vec![root, int(multiplicity)]))
            .collect(),
    ))
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

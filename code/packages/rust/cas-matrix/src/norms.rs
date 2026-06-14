//! Exact vector and Frobenius norms for rational matrices.
//!
//! `norm` computes the Euclidean norm for a row or column vector.  Use
//! `frobenius_norm` for a general matrix.

use symbolic_ir::{apply, sym, IRNode, SQRT};

use crate::matrix::{num_cols, num_rows, MatrixError, MatrixResult};
use crate::rowreduce::{matrix_to_fracs, Frac};

/// Compute the Euclidean norm of a row or column vector.
///
/// Numeric integer/rational entries are evaluated exactly.  Perfect-square
/// sums return an integer/rational IR node; otherwise the result is
/// `Sqrt(sum_of_squares)`.
pub fn norm(m: &IRNode) -> MatrixResult<IRNode> {
    let nr = num_rows(m)?;
    let nc = num_cols(m)?;
    if nr != 1 && nc != 1 {
        return Err(MatrixError(format!(
            "norm: Euclidean norm requires a column or row vector (got {nr}×{nc}); use frobenius_norm for matrices"
        )));
    }
    norm_from_entries(m)
}

/// Compute the Frobenius norm of any numeric matrix.
pub fn frobenius_norm(m: &IRNode) -> MatrixResult<IRNode> {
    norm_from_entries(m)
}

fn norm_from_entries(m: &IRNode) -> MatrixResult<IRNode> {
    let total = matrix_to_fracs(m)?
        .into_iter()
        .flatten()
        .fold(Frac::zero(), |acc, entry| acc + entry * entry);
    sqrt_frac(total)
}

fn sqrt_frac(total: Frac) -> MatrixResult<IRNode> {
    if let Some(exact) = isqrt_frac(total) {
        return exact.to_irnode();
    }
    Ok(apply(sym(SQRT), vec![total.to_irnode()?]))
}

fn isqrt_frac(value: Frac) -> Option<Frac> {
    if value.numer < 0 || value.denom < 0 {
        return None;
    }
    let numer = u128::try_from(value.numer).ok()?;
    let denom = u128::try_from(value.denom).ok()?;
    let sqrt_numer = isqrt(numer);
    let sqrt_denom = isqrt(denom);
    if sqrt_numer * sqrt_numer == numer && sqrt_denom * sqrt_denom == denom {
        Some(Frac::new(sqrt_numer as i128, sqrt_denom as i128))
    } else {
        None
    }
}

fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = (x + n / x) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

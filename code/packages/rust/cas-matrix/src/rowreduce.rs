//! Exact rational row reduction and rank.
//!
//! `row_reduce` and `rank` intentionally accept only integer/rational matrix
//! entries.  Symbolic entries return [`MatrixError`] so callers can simplify
//! first or fall through to a symbolic backend later.

use std::ops::{Add, Div, Mul, Neg, Sub};

use symbolic_ir::{int, rat, IRNode};

use crate::matrix::{matrix, rows_of, MatrixError, MatrixResult};

/// Return the reduced row echelon form (RREF) of a numeric matrix.
///
/// The algorithm is Gauss-Jordan elimination over exact rational arithmetic.
pub fn row_reduce(m: &IRNode) -> MatrixResult<IRNode> {
    let mut rows = matrix_to_fracs(m)?;
    let nr = rows.len();
    let nc = rows.first().map_or(0, |row| row.len());

    let mut pivot_row = 0;
    for col in 0..nc {
        let pivot_pos = (pivot_row..nr).find(|&row| !rows[row][col].is_zero());
        let Some(pivot_pos) = pivot_pos else {
            continue;
        };

        if pivot_pos != pivot_row {
            rows.swap(pivot_row, pivot_pos);
        }

        let pivot = rows[pivot_row][col];
        rows[pivot_row] = rows[pivot_row].iter().map(|entry| *entry / pivot).collect();

        for row in 0..nr {
            if row == pivot_row {
                continue;
            }
            let factor = rows[row][col];
            if factor.is_zero() {
                continue;
            }
            rows[row] = (0..nc)
                .map(|c| rows[row][c] - factor * rows[pivot_row][c])
                .collect();
        }

        pivot_row += 1;
    }

    fracs_to_matrix(rows)
}

/// Return the rank of a numeric matrix as `IRNode::Integer`.
///
/// This uses forward elimination only; the rank is the number of pivot rows.
pub fn rank(m: &IRNode) -> MatrixResult<IRNode> {
    let mut rows = matrix_to_fracs(m)?;
    let nr = rows.len();
    let nc = rows.first().map_or(0, |row| row.len());

    let mut pivot_row = 0;
    for col in 0..nc {
        let pivot_pos = (pivot_row..nr).find(|&row| !rows[row][col].is_zero());
        let Some(pivot_pos) = pivot_pos else {
            continue;
        };

        if pivot_pos != pivot_row {
            rows.swap(pivot_row, pivot_pos);
        }

        let pivot = rows[pivot_row][col];
        rows[pivot_row] = rows[pivot_row].iter().map(|entry| *entry / pivot).collect();

        for row in (pivot_row + 1)..nr {
            let factor = rows[row][col];
            if factor.is_zero() {
                continue;
            }
            rows[row] = (0..nc)
                .map(|c| rows[row][c] - factor * rows[pivot_row][c])
                .collect();
        }

        pivot_row += 1;
    }

    let rank = rows
        .iter()
        .filter(|row| row.iter().any(|entry| !entry.is_zero()))
        .count();
    Ok(int(rank as i64))
}

pub(crate) fn matrix_to_fracs(m: &IRNode) -> MatrixResult<Vec<Vec<Frac>>> {
    rows_of(m)?
        .into_iter()
        .map(|row| row.into_iter().map(entry_to_frac).collect())
        .collect()
}

pub(crate) fn entry_to_frac(entry: IRNode) -> MatrixResult<Frac> {
    match entry {
        IRNode::Integer(value) => Ok(Frac::from_i64(value)),
        IRNode::Rational(numer, denom) => Ok(Frac::new(numer as i128, denom as i128)),
        other => Err(MatrixError(format!(
            "row_reduce/rank: symbolic entry not supported: {other:?}"
        ))),
    }
}

pub(crate) fn fracs_to_matrix(rows: Vec<Vec<Frac>>) -> MatrixResult<IRNode> {
    rows.into_iter()
        .map(|row| row.into_iter().map(Frac::to_irnode).collect())
        .collect::<MatrixResult<Vec<Vec<IRNode>>>>()
        .and_then(matrix)
}

/// Exact rational number in reduced form.
///
/// `denom > 0`; the sign is always carried by `numer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Frac {
    pub(crate) numer: i128,
    pub(crate) denom: i128,
}

impl Frac {
    pub(crate) fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    pub(crate) fn new(numer: i128, denom: i128) -> Self {
        assert!(denom != 0, "Frac denominator must not be zero");
        if numer == 0 {
            return Self::zero();
        }
        let (numer, denom) = if denom < 0 {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let divisor = gcd(numer.unsigned_abs(), denom.unsigned_abs()) as i128;
        Self {
            numer: numer / divisor,
            denom: denom / divisor,
        }
    }

    pub(crate) fn from_i64(value: i64) -> Self {
        Self {
            numer: value as i128,
            denom: 1,
        }
    }

    pub(crate) fn is_zero(self) -> bool {
        self.numer == 0
    }

    pub(crate) fn abs(self) -> Self {
        Self {
            numer: self.numer.abs(),
            denom: self.denom,
        }
    }

    pub(crate) fn to_irnode(self) -> MatrixResult<IRNode> {
        let numer = i64::try_from(self.numer).map_err(|_| {
            MatrixError(format!(
                "row_reduce/rank: numerator overflow: {}",
                self.numer
            ))
        })?;
        let denom = i64::try_from(self.denom).map_err(|_| {
            MatrixError(format!(
                "row_reduce/rank: denominator overflow: {}",
                self.denom
            ))
        })?;

        if denom == 1 {
            Ok(int(numer))
        } else {
            Ok(rat(numer, denom))
        }
    }
}

impl Add for Frac {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numer * rhs.denom + rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }
}

impl Sub for Frac {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Mul for Frac {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }
}

impl Div for Frac {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        assert!(!rhs.is_zero(), "Frac division by zero");
        Self::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }
}

impl Neg for Frac {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            numer: -self.numer,
            denom: self.denom,
        }
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

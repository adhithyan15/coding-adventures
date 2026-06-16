//! The MATLAB value model.
//!
//! MATLAB's universe is the array, so the workhorse value is an
//! [`array_runtime::Array`] (a dense, column-major `f64` matrix — a scalar is
//! `1×1`, a row vector `1×n`). Char arrays (`'abc'` / `"abc"`) are a thin string
//! layer on top. Logicals are ordinary `0.0`/`1.0` numeric arrays, exactly as in
//! MATLAB where `true`/`false` are `1`/`0`.

use array_runtime::Array;
use std::fmt;

/// A MATLAB value.
#[derive(Clone, Debug)]
pub enum MatValue {
    /// A numeric (or logical) array — the common case.
    Num(Array),
    /// A char array / string.
    Char(String),
}

impl MatValue {
    /// A `1×1` numeric scalar.
    pub fn scalar(x: f64) -> MatValue {
        MatValue::Num(Array::scalar(x))
    }

    /// Borrow the underlying numeric array, or error with `ctx` if this is a
    /// non-numeric value (a char array). Most operators are numeric-only.
    pub fn as_num(&self, ctx: &str) -> Result<&Array, String> {
        match self {
            MatValue::Num(a) => Ok(a),
            MatValue::Char(_) => Err(format!("{ctx}: operation is not defined for char arrays")),
        }
    }

    /// MATLAB truthiness for `if`/`while`: a numeric array is true iff it is
    /// non-empty and *every* element is non-zero. A char array is true iff
    /// non-empty.
    pub fn is_true(&self) -> bool {
        match self {
            MatValue::Num(a) => !a.is_empty() && a.data().iter().all(|&x| x != 0.0),
            MatValue::Char(s) => !s.is_empty(),
        }
    }
}

impl fmt::Display for MatValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatValue::Num(a) => write!(f, "{a}"),
            MatValue::Char(s) => write!(f, "{s}"),
        }
    }
}

/// Render a value the way the MATLAB prompt echoes an unsuppressed result:
/// `name = <value>`. A scalar prints on the same line; a matrix on the lines
/// below. `ans` is the name for an unassigned expression result.
pub fn echo(name: &str, value: &MatValue) -> String {
    match value {
        MatValue::Num(a) if a.is_scalar() => format!("{name} = {a}"),
        MatValue::Char(s) => format!("{name} = {s}"),
        MatValue::Num(a) => format!("{name} =\n\n{a}\n"),
    }
}

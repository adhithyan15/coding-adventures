//! The N-D array value model — dense, rectangular, **column-major** `f64`.
//!
//! Column-major (Fortran/MATLAB) storage is deliberate: the first numeric-array
//! frontend on this substrate is MATLAB, whose `reshape`, linear indexing, and
//! `[a; b]` literal semantics all assume column-major order. Element `(r, c)` of
//! an `[nrows, ncols]` matrix lives at flat index `c * nrows + r`.

use std::fmt;

/// A dense N-dimensional `f64` array. `shape == []` is a scalar, `[n]` a vector,
/// `[r, c]` a matrix; higher ranks are stored but only rank ≤ 2 ops are defined
/// in MA-1.
#[derive(Clone, Debug, PartialEq)]
pub struct Array {
    data: Vec<f64>,
    shape: Vec<usize>,
}

impl Array {
    /// A length-1 (scalar) array.
    pub fn scalar(value: f64) -> Array {
        Array {
            data: vec![value],
            shape: vec![],
        }
    }

    /// A 1-D array (a column vector's worth of values, shape `[n]`).
    pub fn from_vec(data: Vec<f64>) -> Array {
        let n = data.len();
        Array {
            data,
            shape: vec![n],
        }
    }

    /// Build a matrix from rows. All rows must be the same length; the data is
    /// transposed into column-major order on the way in.
    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Array, String> {
        let nrows = rows.len();
        if nrows == 0 {
            return Ok(Array {
                data: vec![],
                shape: vec![0, 0],
            });
        }
        let ncols = rows[0].len();
        if rows.iter().any(|r| r.len() != ncols) {
            return Err("from_rows: ragged rows".into());
        }
        let n = nrows
            .checked_mul(ncols)
            .ok_or_else(|| "from_rows: nrows * ncols overflows usize".to_string())?;
        let mut data = vec![0.0; n];
        for (r, row) in rows.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                data[c * nrows + r] = v; // column-major store
            }
        }
        Ok(Array {
            data,
            shape: vec![nrows, ncols],
        })
    }

    /// Wrap raw column-major data with an explicit shape (the product of the
    /// dims must equal the data length).
    pub fn from_shape(data: Vec<f64>, shape: Vec<usize>) -> Result<Array, String> {
        // Element count = product of the dims (an empty shape `[]` is a scalar →
        // exactly one element). Use *checked* multiplication: a crafted shape
        // whose product overflows `usize` must not wrap to a small count that
        // spuriously passes the length check below and leaves `shape`
        // disagreeing with `data.len()` — an invariant the indexing and
        // `Display` code rely on (they compute offsets from `nrows()/ncols()`,
        // not from `data.len()`).
        let n: usize = if shape.is_empty() {
            1
        } else {
            shape
                .iter()
                .try_fold(1usize, |acc, &d| acc.checked_mul(d))
                .ok_or_else(|| {
                    format!("from_shape: shape {shape:?} element count overflows usize")
                })?
        };
        if n != data.len() {
            return Err(format!(
                "from_shape: shape {shape:?} implies {n} elements, got {}",
                data.len()
            ));
        }
        Ok(Array { data, shape })
    }

    /// An `[rows, cols]` array of zeros / ones / a constant.
    pub fn filled(rows: usize, cols: usize, value: f64) -> Array {
        // `checked_mul` turns an absurd `rows * cols` into a clean panic rather
        // than a release-mode wrap that would under-allocate `data` and let
        // `eye` (and later index math) write out of bounds.
        let n = rows
            .checked_mul(cols)
            .expect("Array::filled: rows * cols overflows usize");
        Array {
            data: vec![value; n],
            shape: vec![rows, cols],
        }
    }
    pub fn zeros(rows: usize, cols: usize) -> Array {
        Array::filled(rows, cols, 0.0)
    }
    pub fn ones(rows: usize, cols: usize) -> Array {
        Array::filled(rows, cols, 1.0)
    }

    /// The `n × n` identity matrix.
    pub fn eye(n: usize) -> Array {
        let mut a = Array::zeros(n, n);
        for i in 0..n {
            a.data[i * n + i] = 1.0;
        }
        a
    }

    // --- accessors ------------------------------------------------------

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }
    pub fn data(&self) -> &[f64] {
        &self.data
    }
    pub fn ndims(&self) -> usize {
        self.shape.len()
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn is_scalar(&self) -> bool {
        self.data.len() == 1
    }

    /// Rows / columns, treating a scalar as 1×1 and a vector `[n]` as `n×1`.
    pub fn nrows(&self) -> usize {
        match self.shape.as_slice() {
            [] => 1,
            [n] => *n,
            [r, _, ..] => *r,
        }
    }
    pub fn ncols(&self) -> usize {
        match self.shape.as_slice() {
            [] => 1,
            [_] => 1,
            [_, c, ..] => *c,
        }
    }

    /// Element `(r, c)` (column-major), or `None` if out of bounds.
    pub fn get(&self, r: usize, c: usize) -> Option<f64> {
        if r < self.nrows() && c < self.ncols() {
            self.data.get(c * self.nrows() + r).copied()
        } else {
            None
        }
    }
}

impl fmt::Display for Array {
    /// MATLAB-ish display: a scalar prints bare; a matrix prints right-aligned
    /// rows.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_scalar() {
            return write!(f, "{}", fmt_num(self.data[0]));
        }
        let (rows, cols) = (self.nrows(), self.ncols());
        let cells: Vec<String> = self.data.iter().map(|&x| fmt_num(x)).collect();
        let width = cells.iter().map(|s| s.len()).max().unwrap_or(1);
        for r in 0..rows {
            for c in 0..cols {
                let cell = &cells[c * rows + r];
                write!(f, "{}{cell:>width$}", if c == 0 { "" } else { "  " })?;
            }
            if r + 1 < rows {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

/// Integer-valued doubles print without a decimal point (`3`, not `3.0`).
fn fmt_num(x: f64) -> String {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_is_rank0_length1() {
        let s = Array::scalar(42.0);
        assert!(s.is_scalar());
        assert_eq!(s.ndims(), 0);
        assert_eq!(s.shape(), &[] as &[usize]);
        assert_eq!(s.nrows(), 1);
        assert_eq!(s.ncols(), 1);
        assert_eq!(s.get(0, 0), Some(42.0));
    }

    #[test]
    fn vector_shape_and_dims() {
        let v = Array::from_vec(vec![1.0, 2.0, 3.0]);
        assert_eq!(v.shape(), &[3]);
        assert_eq!(v.ndims(), 1);
        assert_eq!(v.nrows(), 3);
        assert_eq!(v.ncols(), 1); // a vector is a column
        assert!(!v.is_scalar());
        assert!(!v.is_empty());
    }

    #[test]
    fn from_rows_stores_column_major() {
        // Row-major rows [[1,2,3],[4,5,6]] become column-major [1,4,2,5,3,6].
        let a = Array::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]).unwrap();
        assert_eq!(a.shape(), &[2, 3]);
        assert_eq!(a.data(), &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
        assert_eq!(a.get(0, 0), Some(1.0));
        assert_eq!(a.get(1, 2), Some(6.0));
        assert_eq!(a.get(0, 2), Some(3.0));
    }

    #[test]
    fn from_rows_empty_and_ragged() {
        let empty = Array::from_rows(vec![]).unwrap();
        assert_eq!(empty.shape(), &[0, 0]);
        assert!(empty.is_empty());
        assert!(Array::from_rows(vec![vec![1.0, 2.0], vec![3.0]]).is_err());
    }

    #[test]
    fn from_shape_validates_element_count() {
        assert!(Array::from_shape(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]).is_ok());
        assert!(Array::from_shape(vec![1.0, 2.0, 3.0], vec![2, 2]).is_err());
        // A scalar shape [] implies exactly one element.
        assert!(Array::from_shape(vec![7.0], vec![]).is_ok());
        assert!(Array::from_shape(vec![7.0, 8.0], vec![]).is_err());
    }

    #[test]
    fn from_shape_rejects_overflowing_product() {
        // A shape whose element count overflows usize must be an error, not a
        // wrapped-small count that spuriously matches a short data vector.
        let huge = vec![1.0]; // pretend a 1-element buffer
        let shape = vec![usize::MAX, 2]; // product wraps
        assert!(Array::from_shape(huge, shape).is_err());
    }

    #[test]
    #[should_panic(expected = "overflows usize")]
    fn filled_panics_on_overflowing_dims() {
        // Deterministic panic (not a silent release-mode wrap that would
        // under-allocate and corrupt later index math).
        let _ = Array::filled(usize::MAX, 2, 0.0);
    }

    #[test]
    fn constructors_zeros_ones_filled_eye() {
        assert_eq!(Array::zeros(2, 2).data(), &[0.0; 4]);
        assert_eq!(Array::ones(1, 3).data(), &[1.0; 3]);
        assert_eq!(Array::filled(2, 1, 9.0).data(), &[9.0, 9.0]);
        let i = Array::eye(3);
        assert_eq!(i.shape(), &[3, 3]);
        // Identity has 1s on the diagonal, 0 elsewhere.
        for r in 0..3 {
            for c in 0..3 {
                assert_eq!(i.get(r, c), Some(if r == c { 1.0 } else { 0.0 }));
            }
        }
    }

    #[test]
    fn get_out_of_bounds_is_none() {
        let a = Array::from_rows(vec![vec![1.0, 2.0]]).unwrap(); // 1x2
        assert_eq!(a.get(0, 1), Some(2.0));
        assert_eq!(a.get(1, 0), None); // row OOB
        assert_eq!(a.get(0, 2), None); // col OOB
    }

    #[test]
    fn display_scalar_and_integer_valued() {
        assert_eq!(format!("{}", Array::scalar(3.0)), "3"); // no trailing .0
        assert_eq!(format!("{}", Array::scalar(2.5)), "2.5");
    }

    #[test]
    fn display_matrix_is_row_aligned() {
        let a = Array::from_rows(vec![vec![1.0, 20.0], vec![300.0, 4.0]]).unwrap();
        // Right-aligned to the widest cell ("300"), rows separated by newlines.
        assert_eq!(format!("{a}"), "  1   20\n300    4");
    }
}

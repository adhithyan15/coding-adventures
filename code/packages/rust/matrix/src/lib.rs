/// # Matrix — A Pure Rust Matrix Library
///
/// This module provides a 2D matrix type with arithmetic, reductions,
/// element-wise math, shape manipulation, and comparison operations.
///
/// ## Design Principles
///
/// 1. **Immutable by default.** Methods return a *new* Matrix; `&self`
///    borrows are never modified. Rust's ownership model makes this
///    natural — the caller decides whether to keep or drop the original.
///
/// 2. **No external dependencies.** Only `f64` methods from Rust's
///    standard library (sqrt, abs, powf).
///
/// 3. **Result types for fallible operations.** Dimension mismatches and
///    out-of-bounds accesses return `Err` rather than panicking, so the
///    caller can handle errors gracefully.
///
/// ## Internal Representation
///
/// ```text
///   data: Vec<Vec<f64>>   -- 2D vector, outer = rows, inner = columns
///   rows: usize           -- cached row count
///   cols: usize           -- cached column count
/// ```

#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub data: Vec<Vec<f64>>,
    pub rows: usize,
    pub cols: usize,
}

impl Matrix {
    // ─── Constructors ────────────────────────────────────────────────

    /// Create a matrix from a 2D vector.
    ///
    /// ```
    /// let m = matrix::Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    /// assert_eq!(m.rows, 2);
    /// assert_eq!(m.cols, 2);
    /// ```
    pub fn new_2d(data: Vec<Vec<f64>>) -> Self {
        let rows = data.len();
        let cols = if rows > 0 { data[0].len() } else { 0 };
        Self { data, rows, cols }
    }

    /// Create a 1-row matrix from a 1D vector.
    pub fn new_1d(data: Vec<f64>) -> Self {
        let cols = data.len();
        Self { data: vec![data], rows: 1, cols }
    }

    /// Create a 1x1 matrix from a single value.
    pub fn new_scalar(val: f64) -> Self {
        Self { data: vec![vec![val]], rows: 1, cols: 1 }
    }

    /// Create an rows x cols matrix filled with zeros.
    ///
    /// This is the workhorse factory — used internally by `dot`,
    /// `transpose`, and many other methods that need a blank canvas.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self { data: vec![vec![0.0; cols]; rows], rows, cols }
    }

    // ─── Factory Methods ─────────────────────────────────────────────

    /// Create an n x n identity matrix.
    ///
    /// The identity matrix has 1.0 on the main diagonal and 0.0 elsewhere.
    /// It is the multiplicative identity for matrix dot products:
    ///
    ///   identity(n).dot(&M) == M   (for any n x m matrix M)
    ///
    /// This is analogous to multiplying a number by 1.
    pub fn identity(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.data[i][i] = 1.0;
        }
        m
    }

    /// Create a diagonal matrix from a slice of values.
    ///
    /// The resulting matrix is n x n where n = values.len().
    /// Only the main diagonal is populated; off-diagonal entries are 0.
    ///
    /// ```text
    /// from_diagonal(&[2.0, 3.0]) -> [[2, 0],
    ///                                 [0, 3]]
    /// ```
    pub fn from_diagonal(values: &[f64]) -> Self {
        let n = values.len();
        let mut m = Self::zeros(n, n);
        for i in 0..n {
            m.data[i][i] = values[i];
        }
        m
    }

    // ─── Basic Arithmetic ────────────────────────────────────────────

    /// Element-wise matrix addition. Both matrices must have the same shape.
    pub fn add(&self, other: &Matrix) -> Result<Self, &'static str> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err("Matrix addition dimensions rigorously mismatch");
        }
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] + other.data[i][j];
            }
        }
        Ok(c)
    }

    /// Add a scalar to every element (broadcast addition).
    pub fn add_scalar(&self, scalar: f64) -> Self {
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] + scalar;
            }
        }
        c
    }

    /// Element-wise matrix subtraction.
    pub fn subtract(&self, other: &Matrix) -> Result<Self, &'static str> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err("Matrix subtraction dimensions rigorously mismatch");
        }
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] - other.data[i][j];
            }
        }
        Ok(c)
    }

    /// Multiply every element by a scalar.
    pub fn scale(&self, scalar: f64) -> Self {
        let mut c = Self::zeros(self.rows, self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[i][j] = self.data[i][j] * scalar;
            }
        }
        c
    }

    /// Transpose: swap rows and columns. M^T[j][i] = M[i][j].
    pub fn transpose(&self) -> Self {
        if self.rows == 0 { return Self::zeros(0, 0); }
        let mut c = Self::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                c.data[j][i] = self.data[i][j];
            }
        }
        c
    }

    /// Matrix multiplication (dot product).
    ///
    /// For an m x k matrix A and a k x n matrix B, the result is m x n
    /// where C[i][j] = sum over k of A[i][k] * B[k][j].
    pub fn dot(&self, other: &Matrix) -> Result<Self, &'static str> {
        if self.cols != other.rows {
            return Err("Matrix dot mapping inner dimensions strictly contradict");
        }
        let mut c = Self::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            for j in 0..other.cols {
                for k in 0..self.cols {
                    c.data[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }
        Ok(c)
    }

    // ─── Element Access ──────────────────────────────────────────────

    /// Get the element at (row, col). Zero-based indices.
    ///
    /// Returns Err if the index is out of bounds.
    pub fn get(&self, row: usize, col: usize) -> Result<f64, String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Index ({}, {}) out of bounds for {}x{} matrix",
                row, col, self.rows, self.cols
            ));
        }
        Ok(self.data[row][col])
    }

    /// Return a new matrix with the element at (row, col) replaced.
    ///
    /// The original matrix is not modified — immutability is key.
    pub fn set(&self, row: usize, col: usize, value: f64) -> Result<Self, String> {
        if row >= self.rows || col >= self.cols {
            return Err(format!(
                "Index ({}, {}) out of bounds for {}x{} matrix",
                row, col, self.rows, self.cols
            ));
        }
        let mut c = self.clone();
        c.data[row][col] = value;
        Ok(c)
    }

    // ─── Reductions ──────────────────────────────────────────────────

    /// Sum of all elements.
    ///
    /// For [[1,2],[3,4]]: 1 + 2 + 3 + 4 = 10.0
    ///
    /// This is a "full reduction" — the entire matrix collapses to one scalar.
    pub fn sum(&self) -> f64 {
        let mut total = 0.0;
        for i in 0..self.rows {
            for j in 0..self.cols {
                total += self.data[i][j];
            }
        }
        total
    }

    /// Sum each row, returning an n x 1 column vector.
    ///
    /// For [[1,2],[3,4]]: rows -> [[3],[7]]
    pub fn sum_rows(&self) -> Self {
        let data: Vec<Vec<f64>> = self.data.iter()
            .map(|row| vec![row.iter().sum()])
            .collect();
        Self::new_2d(data)
    }

    /// Sum each column, returning a 1 x m row vector.
    ///
    /// For [[1,2],[3,4]]: cols -> [[4,6]]
    pub fn sum_cols(&self) -> Self {
        let mut sums = vec![0.0; self.cols];
        for i in 0..self.rows {
            for j in 0..self.cols {
                sums[j] += self.data[i][j];
            }
        }
        Self::new_1d(sums)
    }

    /// Arithmetic mean of all elements: sum / count.
    pub fn mean(&self) -> f64 {
        self.sum() / (self.rows * self.cols) as f64
    }

    /// Minimum element value.
    pub fn min_val(&self) -> f64 {
        let mut min = f64::INFINITY;
        for i in 0..self.rows {
            for j in 0..self.cols {
                if self.data[i][j] < min { min = self.data[i][j]; }
            }
        }
        min
    }

    /// Maximum element value.
    pub fn max_val(&self) -> f64 {
        let mut max = f64::NEG_INFINITY;
        for i in 0..self.rows {
            for j in 0..self.cols {
                if self.data[i][j] > max { max = self.data[i][j]; }
            }
        }
        max
    }

    /// (row, col) of the minimum element. First occurrence in row-major order.
    pub fn argmin(&self) -> (usize, usize) {
        let mut min = f64::INFINITY;
        let mut pos = (0, 0);
        for i in 0..self.rows {
            for j in 0..self.cols {
                if self.data[i][j] < min {
                    min = self.data[i][j];
                    pos = (i, j);
                }
            }
        }
        pos
    }

    /// (row, col) of the maximum element. First occurrence in row-major order.
    ///
    /// ```text
    /// [[1,2],[3,4]].argmax() -> (1, 1)   (element 4 at row 1, col 1)
    /// ```
    pub fn argmax(&self) -> (usize, usize) {
        let mut max = f64::NEG_INFINITY;
        let mut pos = (0, 0);
        for i in 0..self.rows {
            for j in 0..self.cols {
                if self.data[i][j] > max {
                    max = self.data[i][j];
                    pos = (i, j);
                }
            }
        }
        pos
    }

    // ─── Element-wise Math ───────────────────────────────────────────

    /// Apply a function to every element, returning a new matrix.
    ///
    /// This is the most general element-wise operation. `sqrt`, `abs_val`,
    /// and `pow_val` are all special cases of `map`.
    pub fn map<F: Fn(f64) -> f64>(&self, f: F) -> Self {
        let data: Vec<Vec<f64>> = self.data.iter()
            .map(|row| row.iter().map(|&v| f(v)).collect())
            .collect();
        Self::new_2d(data)
    }

    /// Element-wise square root.
    pub fn sqrt(&self) -> Self {
        self.map(f64::sqrt)
    }

    /// Element-wise absolute value.
    pub fn abs_val(&self) -> Self {
        self.map(f64::abs)
    }

    /// Element-wise exponentiation: each element raised to `exp`.
    pub fn pow_val(&self, exp: f64) -> Self {
        self.map(|v| v.powf(exp))
    }

    // ─── Shape Operations ────────────────────────────────────────────

    /// Flatten into a 1 x n row vector (n = rows * cols).
    ///
    /// Elements are read in row-major order.
    pub fn flatten(&self) -> Self {
        let mut flat = Vec::with_capacity(self.rows * self.cols);
        for i in 0..self.rows {
            for j in 0..self.cols {
                flat.push(self.data[i][j]);
            }
        }
        Self::new_1d(flat)
    }

    /// Reshape into a matrix with the given dimensions.
    ///
    /// rows * cols must equal self.rows * self.cols.
    pub fn reshape(&self, rows: usize, cols: usize) -> Result<Self, String> {
        if rows * cols != self.rows * self.cols {
            return Err(format!(
                "Cannot reshape {}x{} to {}x{}",
                self.rows, self.cols, rows, cols
            ));
        }
        let flat = self.flatten();
        let mut data = Vec::with_capacity(rows);
        for i in 0..rows {
            data.push(flat.data[0][i * cols..(i + 1) * cols].to_vec());
        }
        Ok(Self::new_2d(data))
    }

    /// Extract row i as a 1 x cols matrix.
    pub fn row(&self, i: usize) -> Result<Self, String> {
        if i >= self.rows {
            return Err(format!("Row index {} out of bounds for {} rows", i, self.rows));
        }
        Ok(Self::new_1d(self.data[i].clone()))
    }

    /// Extract column j as a rows x 1 matrix.
    pub fn col(&self, j: usize) -> Result<Self, String> {
        if j >= self.cols {
            return Err(format!("Column index {} out of bounds for {} cols", j, self.cols));
        }
        let data: Vec<Vec<f64>> = self.data.iter().map(|row| vec![row[j]]).collect();
        Ok(Self::new_2d(data))
    }

    /// Extract a sub-matrix from rows [r0..r1) and columns [c0..c1).
    ///
    /// The range is half-open: r1 and c1 are exclusive.
    pub fn slice(&self, r0: usize, r1: usize, c0: usize, c1: usize) -> Result<Self, String> {
        if r0 >= r1 || c0 >= c1 || r1 > self.rows || c1 > self.cols {
            return Err(format!(
                "Invalid slice [{}:{}, {}:{}] for {}x{} matrix",
                r0, r1, c0, c1, self.rows, self.cols
            ));
        }
        let data: Vec<Vec<f64>> = (r0..r1)
            .map(|i| self.data[i][c0..c1].to_vec())
            .collect();
        Ok(Self::new_2d(data))
    }

    // ─── Equality and Comparison ─────────────────────────────────────

    /// Exact element-wise equality.
    pub fn equals(&self, other: &Matrix) -> bool {
        if self.rows != other.rows || self.cols != other.cols { return false; }
        for i in 0..self.rows {
            for j in 0..self.cols {
                if self.data[i][j] != other.data[i][j] { return false; }
            }
        }
        true
    }

    /// Approximate equality within a tolerance.
    ///
    /// Returns true iff |a - b| <= tolerance for every element pair.
    pub fn close(&self, other: &Matrix, tolerance: f64) -> bool {
        if self.rows != other.rows || self.cols != other.cols { return false; }
        for i in 0..self.rows {
            for j in 0..self.cols {
                if (self.data[i][j] - other.data[i][j]).abs() > tolerance { return false; }
            }
        }
        true
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Base operations ──────────────────────────────────────────────

    #[test]
    fn test_zeros() {
        let z = Matrix::zeros(2, 3);
        assert_eq!(z.rows, 2);
        assert_eq!(z.cols, 3);
        assert_eq!(z.data[1][2], 0.0);
    }

    #[test]
    fn test_add_subtract() {
        let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = Matrix::new_2d(vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
        let c = a.add(&b).unwrap();
        assert_eq!(c.data, vec![vec![6.0, 8.0], vec![10.0, 12.0]]);
        let d = b.subtract(&a).unwrap();
        assert_eq!(d.data, vec![vec![4.0, 4.0], vec![4.0, 4.0]]);
    }

    #[test]
    fn test_dot() {
        let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = Matrix::new_2d(vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
        let c = a.dot(&b).unwrap();
        assert_eq!(c.data, vec![vec![19.0, 22.0], vec![43.0, 50.0]]);
    }

    // ── Factory methods ──────────────────────────────────────────────

    #[test]
    fn test_identity() {
        let i3 = Matrix::identity(3);
        assert_eq!(i3.rows, 3);
        assert_eq!(i3.cols, 3);
        assert_eq!(i3.data[0], vec![1.0, 0.0, 0.0]);
        assert_eq!(i3.data[1], vec![0.0, 1.0, 0.0]);
        assert_eq!(i3.data[2], vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_identity_dot_m_equals_m() {
        let m = Matrix::new_2d(vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ]);
        let i3 = Matrix::identity(3);
        let result = i3.dot(&m).unwrap();
        assert!(result.equals(&m));
    }

    #[test]
    fn test_from_diagonal() {
        let d = Matrix::from_diagonal(&[2.0, 3.0]);
        assert_eq!(d.data, vec![vec![2.0, 0.0], vec![0.0, 3.0]]);
    }

    // ── Element access ───────────────────────────────────────────────

    #[test]
    fn test_get() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.get(0, 0).unwrap(), 1.0);
        assert_eq!(m.get(1, 1).unwrap(), 4.0);
        assert!(m.get(2, 0).is_err());
    }

    #[test]
    fn test_set() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let m2 = m.set(0, 0, 99.0).unwrap();
        assert_eq!(m2.get(0, 0).unwrap(), 99.0);
        assert_eq!(m.get(0, 0).unwrap(), 1.0); // original unchanged
        assert!(m.set(5, 0, 1.0).is_err());
    }

    // ── Reductions ───────────────────────────────────────────────────

    #[test]
    fn test_sum() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.sum(), 10.0);
    }

    #[test]
    fn test_mean() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.mean(), 2.5);
    }

    #[test]
    fn test_sum_rows() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let sr = m.sum_rows();
        assert_eq!(sr.data, vec![vec![3.0], vec![7.0]]);
    }

    #[test]
    fn test_sum_cols() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let sc = m.sum_cols();
        assert_eq!(sc.data, vec![vec![4.0, 6.0]]);
    }

    #[test]
    fn test_min_max() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.min_val(), 1.0);
        assert_eq!(m.max_val(), 4.0);
    }

    #[test]
    fn test_argmin_argmax() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.argmin(), (0, 0));
        assert_eq!(m.argmax(), (1, 1));
    }

    #[test]
    fn test_argmin_argmax_ties() {
        let t = Matrix::new_2d(vec![vec![5.0, 5.0], vec![5.0, 5.0]]);
        assert_eq!(t.argmin(), (0, 0));
        assert_eq!(t.argmax(), (0, 0));
    }

    #[test]
    fn test_reductions_larger() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
        assert_eq!(m.sum(), 21.0);
        assert_eq!(m.mean(), 3.5);
        assert_eq!(m.sum_rows().data, vec![vec![6.0], vec![15.0]]);
        assert_eq!(m.sum_cols().data, vec![vec![5.0, 7.0, 9.0]]);
    }

    // ── Element-wise math ────────────────────────────────────────────

    #[test]
    fn test_map() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let doubled = m.map(|x| x * 2.0);
        assert_eq!(doubled.data, vec![vec![2.0, 4.0], vec![6.0, 8.0]]);
    }

    #[test]
    fn test_sqrt() {
        let m = Matrix::new_2d(vec![vec![1.0, 4.0], vec![9.0, 16.0]]);
        let s = m.sqrt();
        assert_eq!(s.data, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn test_abs() {
        let m = Matrix::new_2d(vec![vec![-1.0, 2.0], vec![-3.0, 4.0]]);
        let a = m.abs_val();
        assert_eq!(a.data, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn test_pow() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let p = m.pow_val(2.0);
        assert_eq!(p.data, vec![vec![1.0, 4.0], vec![9.0, 16.0]]);
    }

    #[test]
    fn test_close_after_sqrt_pow() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert!(m.close(&m.sqrt().pow_val(2.0), 1e-9));
    }

    // ── Shape operations ─────────────────────────────────────────────

    #[test]
    fn test_flatten() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let f = m.flatten();
        assert_eq!(f.rows, 1);
        assert_eq!(f.cols, 4);
        assert_eq!(f.data, vec![vec![1.0, 2.0, 3.0, 4.0]]);
    }

    #[test]
    fn test_flatten_reshape_roundtrip() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let roundtrip = m.flatten().reshape(m.rows, m.cols).unwrap();
        assert!(roundtrip.equals(&m));
    }

    #[test]
    fn test_reshape() {
        let flat = Matrix::new_1d(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let reshaped = flat.reshape(2, 3).unwrap();
        assert_eq!(reshaped.data, vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    }

    #[test]
    fn test_reshape_invalid() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert!(m.reshape(3, 3).is_err());
    }

    #[test]
    fn test_row() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.row(0).unwrap().data, vec![vec![1.0, 2.0]]);
        assert_eq!(m.row(1).unwrap().data, vec![vec![3.0, 4.0]]);
        assert!(m.row(2).is_err());
    }

    #[test]
    fn test_col() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.col(0).unwrap().data, vec![vec![1.0], vec![3.0]]);
        assert_eq!(m.col(1).unwrap().data, vec![vec![2.0], vec![4.0]]);
        assert!(m.col(2).is_err());
    }

    #[test]
    fn test_slice() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let s = m.slice(0, 2, 0, 1).unwrap();
        assert_eq!(s.data, vec![vec![1.0], vec![3.0]]);
    }

    #[test]
    fn test_slice_larger() {
        let m = Matrix::new_2d(vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ]);
        let s = m.slice(0, 2, 1, 3).unwrap();
        assert_eq!(s.data, vec![vec![2.0, 3.0], vec![5.0, 6.0]]);
    }

    #[test]
    fn test_slice_invalid() {
        let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert!(m.slice(0, 3, 0, 1).is_err());
        assert!(m.slice(1, 0, 0, 1).is_err());
    }

    // ── Equality ─────────────────────────────────────────────────────

    #[test]
    fn test_equals() {
        let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let c = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 5.0]]);
        assert!(a.equals(&b));
        assert!(!a.equals(&c));
    }

    #[test]
    fn test_equals_different_shapes() {
        let a = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = Matrix::new_1d(vec![1.0, 2.0, 3.0]);
        assert!(!a.equals(&b));
    }

    #[test]
    fn test_close() {
        let a = Matrix::new_scalar(1.0000000001);
        let b = Matrix::new_scalar(1.0);
        assert!(a.close(&b, 1e-9));
    }

    #[test]
    fn test_close_outside_tolerance() {
        let a = Matrix::new_scalar(1.1);
        let b = Matrix::new_scalar(1.0);
        assert!(!a.close(&b, 0.01));
    }

    #[test]
    fn test_close_different_shapes() {
        let a = Matrix::new_scalar(1.0);
        let b = Matrix::new_1d(vec![1.0, 2.0]);
        assert!(!a.close(&b, 1e-9));
    }
}

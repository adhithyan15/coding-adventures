//! CpuBlas -- pure Rust reference implementation of BLAS.
//!
//! # Why a CPU Backend?
//!
//! The CPU backend serves two critical purposes:
//!
//! 1. **Universal fallback** -- it works everywhere, on any machine, with no
//!    GPU drivers or hardware requirements. If everything else fails, CPU works.
//!
//! 2. **Reference implementation** -- every other backend (CUDA, Metal, etc.)
//!    is tested against the CPU backend's results. If CudaBlas and CpuBlas
//!    disagree, the bug is in CudaBlas.
//!
//! # How It Works
//!
//! Every BLAS operation is implemented with explicit Rust loops. No SIMD
//! intrinsics, no unsafe code, no tricks -- just `for` loops and arithmetic.
//! This makes every operation completely transparent:
//!
//! ```text
//! SAXPY:  for i in 0..n { result[i] = alpha * x[i] + y[i] }
//! GEMM:   for i, for j, for k { C[i][j] += A[i][k] * B[k][j] }
//! DOT:    sum(x[i] * y[i] for i in 0..n)
//! ```
//!
//! # Performance
//!
//! The CPU backend is SLOW. O(n^3) for GEMM with loop overhead on every
//! element. A 1000x1000 matrix multiply takes seconds. But that's fine --
//! the CPU backend optimizes for **clarity**, not speed. The GPU backends
//! optimize for speed.

use crate::traits::{BlasBackend, MlBlasBackend};
use crate::types::{Matrix, Side, Transpose, Vector};

// =========================================================================
// Helper: access a matrix element respecting transpose
// =========================================================================

/// Access matrix element, respecting the transpose flag.
///
/// # Virtual Transpose -- No Copy Needed
///
/// Instead of physically transposing a matrix (allocating new memory and
/// rearranging elements), we just swap the row/col indices:
///
/// ```text
/// NoTrans: A[row][col] = data[row * cols + col]
/// Trans:   A[row][col] = data[col * cols + row]
///          (swap row and col, keep the original cols stride)
/// ```
///
/// This is how real BLAS libraries handle transpose -- the data stays in
/// place, only the access pattern changes.
fn get_element(m: &Matrix, row: usize, col: usize, trans: Transpose) -> f32 {
    match trans {
        Transpose::Trans => {
            // Transposed: logical (row, col) maps to physical (col, row)
            m.data()[col * m.cols() + row]
        }
        Transpose::NoTrans => {
            // Not transposed: direct access
            m.data()[row * m.cols() + col]
        }
    }
}

/// Get the effective (rows, cols) after applying the transpose flag.
///
/// A 2x3 matrix transposed becomes 3x2:
///   NoTrans: (2, 3) -> (2, 3)
///   Trans:   (2, 3) -> (3, 2)
fn effective_shape(m: &Matrix, trans: Transpose) -> (usize, usize) {
    match trans {
        Transpose::Trans => (m.cols(), m.rows()),
        Transpose::NoTrans => (m.rows(), m.cols()),
    }
}

// =========================================================================
// CpuBlas -- the reference implementation
// =========================================================================

/// Pure Rust BLAS implementation -- the reference backend.
///
/// # CPU BLAS -- The Reference Implementation
///
/// This struct implements both [`BlasBackend`] and [`MlBlasBackend`] traits
/// using nothing but Rust loops and the standard library's math functions.
///
/// Every other backend's correctness is measured against this one. If
/// `CudaBlas::sgemm()` and `CpuBlas::sgemm()` disagree on the result,
/// the bug is in CudaBlas, not CpuBlas.
///
/// # Example
///
/// ```
/// use blas_library::{CpuBlas, Vector, Transpose, Matrix};
/// use blas_library::traits::BlasBackend;
///
/// let blas = CpuBlas;
/// let x = Vector::new(vec![1.0, 2.0, 3.0]);
/// let y = Vector::new(vec![4.0, 5.0, 6.0]);
/// let result = blas.saxpy(2.0, &x, &y).unwrap();
/// assert_eq!(result.data(), &[6.0, 9.0, 12.0]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CpuBlas;

// =================================================================
// BlasBackend implementation
// =================================================================

impl BlasBackend for CpuBlas {
    fn name(&self) -> &str {
        "cpu"
    }

    fn device_name(&self) -> String {
        "CPU (pure Rust)".to_string()
    }

    // =================================================================
    // LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
    // =================================================================

    fn saxpy(&self, alpha: f32, x: &Vector, y: &Vector) -> Result<Vector, String> {
        // SAXPY: result = alpha * x + y
        //
        // # SAXPY -- The Hello World of BLAS
        //
        // S = Single precision, A = Alpha, X = vector X, P = Plus, Y = vector Y
        //
        // This is the simplest BLAS operation and our running example since
        // Layer 11 (logic gates). Each element:
        //
        //     result[i] = alpha * x[i] + y[i]
        //
        // Time complexity: O(n) -- one pass through both vectors.
        if x.size() != y.size() {
            return Err(format!(
                "SAXPY dimension mismatch: x.size={} != y.size={}",
                x.size(),
                y.size()
            ));
        }
        let data: Vec<f32> = x
            .data()
            .iter()
            .zip(y.data().iter())
            .map(|(&xi, &yi)| alpha * xi + yi)
            .collect();
        Ok(Vector::new(data))
    }

    fn sdot(&self, x: &Vector, y: &Vector) -> Result<f32, String> {
        // DOT product: result = sum(x[i] * y[i])
        //
        // # Dot Product -- Foundation of Similarity
        //
        // The dot product measures how "aligned" two vectors are:
        // - Parallel vectors: large positive dot product
        // - Perpendicular vectors: dot product = 0
        // - Anti-parallel: large negative dot product
        //
        // It's also the building block of matrix multiply (GEMM is just
        // a grid of dot products).
        //
        // Time complexity: O(n)
        if x.size() != y.size() {
            return Err(format!(
                "DOT dimension mismatch: x.size={} != y.size={}",
                x.size(),
                y.size()
            ));
        }
        Ok(x.data()
            .iter()
            .zip(y.data().iter())
            .map(|(&xi, &yi)| xi * yi)
            .sum())
    }

    fn snrm2(&self, x: &Vector) -> f32 {
        // Euclidean norm: result = sqrt(sum(x[i]^2))
        //
        // # Euclidean Norm (L2 Norm)
        //
        // The "length" of a vector in Euclidean space. Used for:
        // - Normalizing vectors (dividing by the norm to get unit vectors)
        // - Convergence checks (is the gradient small enough?)
        // - Regularization (keeping weights small)
        //
        // Time complexity: O(n)
        x.data().iter().map(|&xi| xi * xi).sum::<f32>().sqrt()
    }

    fn sscal(&self, alpha: f32, x: &Vector) -> Vector {
        // Scale: result = alpha * x
        //
        // Multiply every element by the scalar alpha.
        // Time complexity: O(n)
        Vector::new(x.data().iter().map(|&xi| alpha * xi).collect())
    }

    fn sasum(&self, x: &Vector) -> f32 {
        // Absolute sum (L1 norm): result = sum(|x[i]|)
        //
        // Also called the Manhattan distance or taxicab norm. Used in
        // L1 regularization (LASSO) which encourages sparsity.
        //
        // Time complexity: O(n)
        x.data().iter().map(|xi| xi.abs()).sum()
    }

    fn isamax(&self, x: &Vector) -> usize {
        // Index of maximum absolute value: argmax(|x[i]|)
        //
        // Returns the 0-based index of the element with the largest absolute
        // value. Used in partial pivoting for LU decomposition to improve
        // numerical stability.
        //
        // Time complexity: O(n)
        if x.size() == 0 {
            return 0;
        }
        let mut max_idx = 0;
        let mut max_val = x.data()[0].abs();
        for (i, &v) in x.data().iter().enumerate().skip(1) {
            let av = v.abs();
            if av > max_val {
                max_val = av;
                max_idx = i;
            }
        }
        max_idx
    }

    fn scopy(&self, x: &Vector) -> Vector {
        // Copy: result = x (deep copy)
        //
        // Creates a completely independent copy. Modifying the result does
        // not affect the original.
        //
        // Time complexity: O(n)
        Vector::new(x.data().to_vec())
    }

    fn sswap(&self, x: &Vector, y: &Vector) -> Result<(Vector, Vector), String> {
        // Swap: exchange the contents of x and y.
        //
        // Returns (new_x, new_y) where new_x has y's data and new_y has
        // x's data. The originals are not modified.
        //
        // Time complexity: O(n)
        if x.size() != y.size() {
            return Err(format!(
                "SWAP dimension mismatch: x.size={} != y.size={}",
                x.size(),
                y.size()
            ));
        }
        Ok((
            Vector::new(y.data().to_vec()),
            Vector::new(x.data().to_vec()),
        ))
    }

    // =================================================================
    // LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
    // =================================================================

    fn sgemv(
        &self,
        trans: Transpose,
        alpha: f32,
        a: &Matrix,
        x: &Vector,
        beta: f32,
        y: &Vector,
    ) -> Result<Vector, String> {
        // General Matrix-Vector multiply: y = alpha * op(A) * x + beta * y
        //
        // # GEMV -- Matrix Times Vector
        //
        // op(A) is the matrix A, optionally transposed:
        //   NoTrans: op(A) = A        (M x N)
        //   Trans:   op(A) = A^T      (N x M)
        //
        // After applying the transpose:
        //   op(A) has shape (m x n)
        //   x must have size n
        //   y must have size m
        //   result has size m
        //
        // Time complexity: O(M * N)
        let (m, n) = effective_shape(a, trans);

        if x.size() != n {
            return Err(format!(
                "GEMV dimension mismatch: op(A) is {}x{} but x.size={}",
                m,
                n,
                x.size()
            ));
        }
        if y.size() != m {
            return Err(format!(
                "GEMV dimension mismatch: op(A) is {}x{} but y.size={}",
                m,
                n,
                y.size()
            ));
        }

        let mut result = vec![0.0_f32; m];
        for i in 0..m {
            let mut s = 0.0_f32;
            for k in 0..n {
                s += get_element(a, i, k, trans) * x.data()[k];
            }
            result[i] = alpha * s + beta * y.data()[i];
        }

        Ok(Vector::new(result))
    }

    fn sger(
        &self,
        alpha: f32,
        x: &Vector,
        y: &Vector,
        a: &Matrix,
    ) -> Result<Matrix, String> {
        // Outer product (rank-1 update): A = alpha * x * y^T + A
        //
        // # GER -- Outer Product
        //
        // The outer product of two vectors creates a matrix:
        //
        // ```text
        // x = [a, b]     y = [c, d, e]
        //
        // x * y^T = [ a*c  a*d  a*e ]
        //           [ b*c  b*d  b*e ]
        // ```
        //
        // Then we scale by alpha and add to the existing matrix A.
        // Each element: result[i][j] = alpha * x[i] * y[j] + A[i][j]
        //
        // Time complexity: O(M * N)
        if a.rows() != x.size() {
            return Err(format!(
                "GER dimension mismatch: A.rows={} != x.size={}",
                a.rows(),
                x.size()
            ));
        }
        if a.cols() != y.size() {
            return Err(format!(
                "GER dimension mismatch: A.cols={} != y.size={}",
                a.cols(),
                y.size()
            ));
        }

        let mut result: Vec<f32> = a.data().to_vec();
        for i in 0..a.rows() {
            for j in 0..a.cols() {
                result[i * a.cols() + j] += alpha * x.data()[i] * y.data()[j];
            }
        }

        Ok(Matrix::with_order(result, a.rows(), a.cols(), a.order()))
    }

    // =================================================================
    // LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
    // =================================================================

    fn sgemm(
        &self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: f32,
        a: &Matrix,
        b: &Matrix,
        beta: f32,
        c: &Matrix,
    ) -> Result<Matrix, String> {
        // General Matrix Multiply: C = alpha * op(A) * op(B) + beta * C
        //
        // # GEMM -- The Most Important Function in All of Computing
        //
        // This is the function that NVIDIA employs entire teams to optimize.
        // 70-90% of ML training time is spent here.
        //
        // C = alpha * op(A) * op(B) + beta * C
        //
        // where:
        //   op(A) has shape (M x K)
        //   op(B) has shape (K x N)
        //   C     has shape (M x N)
        //
        // Time complexity: O(M * N * K)
        let (m, k_a) = effective_shape(a, trans_a);
        let (k_b, n) = effective_shape(b, trans_b);

        if k_a != k_b {
            return Err(format!(
                "GEMM dimension mismatch: op(A) is {}x{}, op(B) is {}x{}. Inner dimensions {} != {}",
                m, k_a, k_b, n, k_a, k_b
            ));
        }
        let k = k_a;

        if c.rows() != m || c.cols() != n {
            return Err(format!(
                "GEMM dimension mismatch: result should be {}x{} but C is {}x{}",
                m,
                n,
                c.rows(),
                c.cols()
            ));
        }

        // The triple nested loop -- the heart of linear algebra
        let mut result = vec![0.0_f32; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0_f32;
                for kk in 0..k {
                    s += get_element(a, i, kk, trans_a) * get_element(b, kk, j, trans_b);
                }
                result[i * n + j] = alpha * s + beta * c.data()[i * c.cols() + j];
            }
        }

        Ok(Matrix::with_order(result, m, n, c.order()))
    }

    fn ssymm(
        &self,
        side: Side,
        alpha: f32,
        a: &Matrix,
        b: &Matrix,
        beta: f32,
        c: &Matrix,
    ) -> Result<Matrix, String> {
        // Symmetric Matrix Multiply.
        //
        // # SYMM -- Symmetric Matrix Multiply
        //
        // Like GEMM, but exploits the fact that A is symmetric (A = A^T).
        // The backend only needs to read half of A.
        //
        // LEFT:  C = alpha * A * B + beta * C
        // RIGHT: C = alpha * B * A + beta * C
        //
        // A must be square (rows == cols).
        if a.rows() != a.cols() {
            return Err(format!(
                "SSYMM: A must be square but is {}x{}",
                a.rows(),
                a.cols()
            ));
        }

        match side {
            Side::Left => {
                let m = a.rows();
                if b.rows() != m {
                    return Err(format!(
                        "SSYMM LEFT: A is {}x{} but B.rows={}",
                        m,
                        m,
                        b.rows()
                    ));
                }
                let n = b.cols();
                if c.rows() != m || c.cols() != n {
                    return Err(format!(
                        "SSYMM: C should be {}x{} but is {}x{}",
                        m,
                        n,
                        c.rows(),
                        c.cols()
                    ));
                }
                // A is symmetric so A = A^T
                self.sgemm(Transpose::NoTrans, Transpose::NoTrans, alpha, a, b, beta, c)
            }
            Side::Right => {
                let n = a.rows();
                if b.cols() != n {
                    return Err(format!(
                        "SSYMM RIGHT: A is {}x{} but B.cols={}",
                        n,
                        n,
                        b.cols()
                    ));
                }
                let m = b.rows();
                if c.rows() != m || c.cols() != n {
                    return Err(format!(
                        "SSYMM: C should be {}x{} but is {}x{}",
                        m,
                        n,
                        c.rows(),
                        c.cols()
                    ));
                }
                self.sgemm(Transpose::NoTrans, Transpose::NoTrans, alpha, b, a, beta, c)
            }
        }
    }

    fn sgemm_batched(
        &self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: f32,
        a_list: &[Matrix],
        b_list: &[Matrix],
        beta: f32,
        c_list: &[Matrix],
    ) -> Result<Vec<Matrix>, String> {
        // Batched GEMM: multiple independent GEMMs.
        //
        // # Batched GEMM -- Many Matrix Multiplies at Once
        //
        // Used for multi-head attention (each head is a separate GEMM),
        // batched inference (each sample is a separate GEMM), and more.
        //
        // On a GPU, all GEMMs can run in parallel. On CPU, we just loop.
        if a_list.len() != b_list.len() || b_list.len() != c_list.len() {
            return Err(format!(
                "Batched GEMM: batch sizes don't match: A={}, B={}, C={}",
                a_list.len(),
                b_list.len(),
                c_list.len()
            ));
        }
        let mut results = Vec::with_capacity(a_list.len());
        for i in 0..a_list.len() {
            results.push(self.sgemm(trans_a, trans_b, alpha, &a_list[i], &b_list[i], beta, &c_list[i])?);
        }
        Ok(results)
    }
}

// =================================================================
// ML Extensions implementation
// =================================================================

impl MlBlasBackend for CpuBlas {
    fn relu(&self, x: &Matrix) -> Matrix {
        // ReLU activation: max(0, x)
        //
        // # ReLU -- Rectified Linear Unit
        //
        // The most common activation function in deep learning:
        //     relu(x) = max(0, x)
        //
        // Truth table for a single element:
        //     x < 0  -> 0.0    (negative inputs are zeroed)
        //     x >= 0 -> x      (positive inputs pass through)
        //
        // ReLU is popular because:
        // 1. It's extremely fast to compute (just a comparison)
        // 2. It doesn't saturate for positive values (no vanishing gradient)
        // 3. It produces sparse activations (many zeros)
        Matrix::with_order(
            x.data().iter().map(|&v| v.max(0.0)).collect(),
            x.rows(),
            x.cols(),
            x.order(),
        )
    }

    fn gelu(&self, x: &Matrix) -> Matrix {
        // GELU activation: x * Phi(x) where Phi is the CDF of N(0,1).
        //
        // # GELU -- Gaussian Error Linear Unit
        //
        // Used in GPT, BERT, and modern Transformers. Unlike ReLU which has
        // a hard cutoff at 0, GELU smoothly transitions:
        //
        //     gelu(x) = x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
        //
        // This approximation (from Hendrycks & Gimpel, 2016) is what
        // PyTorch and TensorFlow use.
        let sqrt_2_over_pi = (2.0_f32 / std::f32::consts::PI).sqrt();
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| {
                let inner = sqrt_2_over_pi * (v + 0.044715 * v * v * v);
                0.5 * v * (1.0 + inner.tanh())
            })
            .collect();
        Matrix::with_order(data, x.rows(), x.cols(), x.order())
    }

    fn sigmoid(&self, x: &Matrix) -> Matrix {
        // Sigmoid activation: 1 / (1 + exp(-x))
        //
        // # Sigmoid -- The Logistic Function
        //
        // Maps any real number to the range (0, 1):
        //     sigmoid(-inf) -> 0
        //     sigmoid(0)    -> 0.5
        //     sigmoid(+inf) -> 1
        //
        // Numerically stable: for x >= 0, compute as 1/(1+exp(-x));
        // for x < 0, compute as exp(x)/(1+exp(x)).
        let data: Vec<f32> = x
            .data()
            .iter()
            .map(|&v| {
                if v >= 0.0 {
                    1.0 / (1.0 + (-v).exp())
                } else {
                    let ev = v.exp();
                    ev / (1.0 + ev)
                }
            })
            .collect();
        Matrix::with_order(data, x.rows(), x.cols(), x.order())
    }

    fn tanh_activation(&self, x: &Matrix) -> Matrix {
        // Tanh activation: tanh(x)
        //
        // Maps any real number to (-1, 1). Used in RNNs and as an activation
        // function. Related to sigmoid: tanh(x) = 2*sigmoid(2x) - 1.
        Matrix::with_order(
            x.data().iter().map(|&v| v.tanh()).collect(),
            x.rows(),
            x.cols(),
            x.order(),
        )
    }

    fn softmax(&self, x: &Matrix, axis: i32) -> Matrix {
        // Numerically stable softmax along an axis.
        //
        // # Softmax -- Probability Distribution Over a Vector
        //
        // Converts a vector of real numbers into a probability distribution:
        //     softmax(x)[i] = exp(x[i]) / sum(exp(x[j]))
        //
        // The STABLE version subtracts the max first:
        //     softmax(x)[i] = exp(x[i] - max(x)) / sum(exp(x[j] - max(x)))
        //
        // axis=-1 means "along the last dimension" (columns for 2D).
        let actual_axis = if axis == -1 { 1 } else { axis };

        let mut result = vec![0.0_f32; x.rows() * x.cols()];

        if actual_axis == 1 {
            // Softmax along each row
            for i in 0..x.rows() {
                let row_start = i * x.cols();
                let row = &x.data()[row_start..row_start + x.cols()];
                let max_val = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let exps: Vec<f32> = row.iter().map(|&v| (v - max_val).exp()).collect();
                let total: f32 = exps.iter().sum();
                for j in 0..x.cols() {
                    result[row_start + j] = exps[j] / total;
                }
            }
        } else {
            // axis == 0: softmax along each column
            result.copy_from_slice(x.data());
            for j in 0..x.cols() {
                let col: Vec<f32> = (0..x.rows())
                    .map(|i| x.data()[i * x.cols() + j])
                    .collect();
                let max_val = col.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let exps: Vec<f32> = col.iter().map(|&v| (v - max_val).exp()).collect();
                let total: f32 = exps.iter().sum();
                for i in 0..x.rows() {
                    result[i * x.cols() + j] = exps[i] / total;
                }
            }
        }

        Matrix::with_order(result, x.rows(), x.cols(), x.order())
    }

    fn layer_norm(
        &self,
        x: &Matrix,
        gamma: &Vector,
        beta: &Vector,
        eps: f32,
    ) -> Result<Matrix, String> {
        // Layer Normalization (Ba et al., 2016).
        //
        // # Layer Norm -- Normalize Each Sample Independently
        //
        // For each row (sample) in the matrix:
        //   1. Compute mean: mu = sum(x) / n
        //   2. Compute variance: var = sum((x - mu)^2) / n
        //   3. Normalize: x_hat = (x - mu) / sqrt(var + eps)
        //   4. Scale and shift: result = gamma * x_hat + beta
        //
        // Used in: Transformers, GPT, BERT
        if gamma.size() != x.cols() {
            return Err(format!(
                "LayerNorm: gamma.size={} != x.cols={}",
                gamma.size(),
                x.cols()
            ));
        }
        if beta.size() != x.cols() {
            return Err(format!(
                "LayerNorm: beta.size={} != x.cols={}",
                beta.size(),
                x.cols()
            ));
        }

        let n = x.cols();
        let mut result = vec![0.0_f32; x.rows() * x.cols()];

        for i in 0..x.rows() {
            let row = &x.data()[i * n..(i + 1) * n];

            // Step 1: mean
            let mean: f32 = row.iter().sum::<f32>() / n as f32;

            // Step 2: variance
            let var: f32 = row.iter().map(|&v| (v - mean) * (v - mean)).sum::<f32>() / n as f32;

            // Step 3 & 4: normalize, scale, shift
            let inv_std = 1.0 / (var + eps).sqrt();
            for j in 0..n {
                let x_hat = (row[j] - mean) * inv_std;
                result[i * n + j] = gamma.data()[j] * x_hat + beta.data()[j];
            }
        }

        Ok(Matrix::with_order(result, x.rows(), x.cols(), x.order()))
    }

    fn batch_norm(
        &self,
        x: &Matrix,
        gamma: &Vector,
        beta: &Vector,
        running_mean: &Vector,
        running_var: &Vector,
        eps: f32,
        training: bool,
    ) -> Result<Matrix, String> {
        // Batch Normalization (Ioffe & Szegedy, 2015).
        //
        // # Batch Norm -- Normalize Each Feature Across the Batch
        //
        // Unlike layer norm (which normalizes each sample), batch norm
        // normalizes each FEATURE across all samples in the batch.
        if gamma.size() != x.cols() {
            return Err(format!(
                "BatchNorm: gamma.size={} != x.cols={}",
                gamma.size(),
                x.cols()
            ));
        }
        if beta.size() != x.cols() {
            return Err(format!(
                "BatchNorm: beta.size={} != x.cols={}",
                beta.size(),
                x.cols()
            ));
        }

        let batch_size = x.rows();
        let n_features = x.cols();
        let mut result = vec![0.0_f32; batch_size * n_features];

        if training {
            for j in 0..n_features {
                let col: Vec<f32> = (0..batch_size)
                    .map(|i| x.data()[i * n_features + j])
                    .collect();
                let mean: f32 = col.iter().sum::<f32>() / batch_size as f32;
                let var: f32 =
                    col.iter().map(|&v| (v - mean) * (v - mean)).sum::<f32>() / batch_size as f32;
                let inv_std = 1.0 / (var + eps).sqrt();
                for i in 0..batch_size {
                    let x_hat = (col[i] - mean) * inv_std;
                    result[i * n_features + j] = gamma.data()[j] * x_hat + beta.data()[j];
                }
            }
        } else {
            for j in 0..n_features {
                let mean = running_mean.data()[j];
                let var = running_var.data()[j];
                let inv_std = 1.0 / (var + eps).sqrt();
                for i in 0..batch_size {
                    let x_hat = (x.data()[i * n_features + j] - mean) * inv_std;
                    result[i * n_features + j] = gamma.data()[j] * x_hat + beta.data()[j];
                }
            }
        }

        Ok(Matrix::with_order(result, x.rows(), x.cols(), x.order()))
    }

    fn conv2d(
        &self,
        input: &Matrix,
        weight: &Matrix,
        bias: Option<&Vector>,
        stride: usize,
        padding: usize,
    ) -> Result<Matrix, String> {
        // 2D Convolution via im2col + GEMM.
        //
        // # Conv2D -- Simplified 2D Convolution
        //
        // We treat input as a 2D spatial feature map (height x width) and
        // weight as a 2D filter (kH x kW). This is a simplified
        // single-channel convolution for demonstration.
        let h_in = input.rows();
        let w_in = input.cols();
        let k_h = weight.rows();
        let k_w = weight.cols();

        let out_h = (h_in + 2 * padding - k_h) / stride + 1;
        let out_w = (w_in + 2 * padding - k_w) / stride + 1;

        if out_h == 0 || out_w == 0 {
            return Err(format!(
                "Conv2d: output dimensions are non-positive: {}x{}",
                out_h, out_w
            ));
        }

        // Create padded input if needed
        let (padded, padded_w) = if padding > 0 {
            let padded_h = h_in + 2 * padding;
            let padded_w = w_in + 2 * padding;
            let mut padded = vec![0.0_f32; padded_h * padded_w];
            for i in 0..h_in {
                for j in 0..w_in {
                    padded[(i + padding) * padded_w + (j + padding)] =
                        input.data()[i * w_in + j];
                }
            }
            (padded, padded_w)
        } else {
            (input.data().to_vec(), w_in)
        };

        // Compute convolution
        let mut result = vec![0.0_f32; out_h * out_w];

        for oh in 0..out_h {
            for ow in 0..out_w {
                let mut s = 0.0_f32;
                for kh in 0..k_h {
                    for kw in 0..k_w {
                        let ih = oh * stride + kh;
                        let iw = ow * stride + kw;
                        s += padded[ih * padded_w + iw] * weight.data()[kh * k_w + kw];
                    }
                }
                if let Some(b) = bias {
                    if b.size() > 0 {
                        s += b.data()[0];
                    }
                }
                result[oh * out_w + ow] = s;
            }
        }

        Ok(Matrix::new(result, out_h, out_w))
    }

    fn attention(
        &self,
        q: &Matrix,
        k: &Matrix,
        v: &Matrix,
        mask: Option<&Matrix>,
        scale: Option<f32>,
    ) -> Result<Matrix, String> {
        // Scaled Dot-Product Attention (Vaswani et al., 2017).
        //
        // # Attention -- The Core of Transformers
        //
        // Attention(Q, K, V) = softmax(Q * K^T / sqrt(d_k)) * V
        //
        // Steps:
        // 1. scores = Q * K^T                     (SGEMM, Level 3)
        // 2. scores = scores / scale               (element-wise)
        // 3. if mask: scores = scores + mask        (element-wise)
        // 4. weights = softmax(scores, axis=-1)    (ML extension)
        // 5. output = weights * V                  (SGEMM, Level 3)
        let d_k = q.cols();
        let scale_val = scale.unwrap_or_else(|| (d_k as f32).sqrt());

        // Step 1: scores = Q * K^T
        let seq_len = q.rows();
        let scores_c = Matrix::zeros(seq_len, k.rows());
        let scores = self.sgemm(
            Transpose::NoTrans,
            Transpose::Trans,
            1.0,
            q,
            k,
            0.0,
            &scores_c,
        )?;

        // Step 2: scale
        let mut scaled_data: Vec<f32> = scores.data().iter().map(|&v| v / scale_val).collect();

        // Step 3: apply mask
        if let Some(m) = mask {
            for i in 0..scaled_data.len() {
                scaled_data[i] += m.data()[i];
            }
        }

        let scores_matrix = Matrix::new(scaled_data, scores.rows(), scores.cols());

        // Step 4: softmax along the last dimension
        let weights = self.softmax(&scores_matrix, -1);

        // Step 5: output = weights * V
        let output_c = Matrix::zeros(weights.rows(), v.cols());
        self.sgemm(
            Transpose::NoTrans,
            Transpose::NoTrans,
            1.0,
            &weights,
            v,
            0.0,
            &output_c,
        )
    }
}

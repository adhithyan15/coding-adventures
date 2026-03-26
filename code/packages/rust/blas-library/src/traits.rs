//! BLAS Backend Traits -- the contracts every backend must fulfill.
//!
//! # What is a Trait?
//!
//! A trait in Rust defines a set of methods that a type must implement to be
//! considered "compatible." It's the Rust equivalent of Python's Protocol or
//! Java's interface:
//!
//! ```text
//! trait BlasBackend {
//!     fn saxpy(&self, alpha: f32, x: &Vector, y: &Vector) -> Result<Vector, String>;
//! }
//!
//! struct MyCoolBackend;
//!
//! impl BlasBackend for MyCoolBackend {
//!     fn saxpy(&self, alpha: f32, x: &Vector, y: &Vector) -> Result<Vector, String> {
//!         // ... your implementation
//!     }
//! }
//! ```
//!
//! # Two Traits
//!
//! 1. [`BlasBackend`] -- the core BLAS operations (Levels 1, 2, 3).
//!    Every backend MUST implement this.
//!
//! 2. [`MlBlasBackend`] -- extends BlasBackend with ML operations
//!    (activations, softmax, normalization, conv2d, attention).
//!    This is OPTIONAL. The CPU backend implements it as a reference.

use crate::types::{Matrix, Side, Transpose, Vector};

// =========================================================================
// BlasBackend -- the core trait
// =========================================================================

/// The BLAS backend trait -- the contract every backend must fulfill.
///
/// # The BLAS Backend Trait
///
/// This is the contract every backend must fulfill. Whether you're running
/// on an NVIDIA GPU, an Apple M4, or a Raspberry Pi CPU, if you implement
/// this trait, you're a valid BLAS backend.
///
/// All operations return NEW Matrix/Vector objects. They do not mutate
/// inputs. This is cleaner for testing and avoids aliasing bugs. Real BLAS
/// mutates in-place for performance, but we optimize for clarity.
pub trait BlasBackend {
    /// Backend identifier: "cpu", "cuda", "metal", etc.
    fn name(&self) -> &str;

    /// Human-readable device name: "NVIDIA H100", "Apple M4", "CPU", etc.
    fn device_name(&self) -> String;

    // ==========================================================
    // LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
    // ==========================================================

    /// SAXPY: y = alpha * x + y
    ///
    /// The most famous BLAS operation. Each element:
    ///     result[i] = alpha * x[i] + y[i]
    ///
    /// Requires: x.size() == y.size()
    /// Returns: new Vector of same size
    fn saxpy(&self, alpha: f32, x: &Vector, y: &Vector) -> Result<Vector, String>;

    /// DOT product: result = x . y = sum(x_i * y_i)
    ///
    /// Requires: x.size() == y.size()
    /// Returns: scalar float
    fn sdot(&self, x: &Vector, y: &Vector) -> Result<f32, String>;

    /// Euclidean norm: result = ||x||_2 = sqrt(sum(x_i^2))
    ///
    /// Returns: scalar float >= 0
    fn snrm2(&self, x: &Vector) -> f32;

    /// Scale: result = alpha * x
    ///
    /// Returns: new Vector of same size
    fn sscal(&self, alpha: f32, x: &Vector) -> Vector;

    /// Absolute sum: result = sum(|x_i|)
    ///
    /// Returns: scalar float >= 0
    fn sasum(&self, x: &Vector) -> f32;

    /// Index of max absolute value: result = argmax(|x_i|)
    ///
    /// Returns: integer index (0-based)
    fn isamax(&self, x: &Vector) -> usize;

    /// Copy: result = x (deep copy)
    ///
    /// Returns: new Vector with same data
    fn scopy(&self, x: &Vector) -> Vector;

    /// Swap: x <-> y
    ///
    /// Returns: (new_x with y's data, new_y with x's data)
    /// Requires: x.size() == y.size()
    fn sswap(&self, x: &Vector, y: &Vector) -> Result<(Vector, Vector), String>;

    // ==========================================================
    // LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
    // ==========================================================

    /// General Matrix-Vector multiply: y = alpha * op(A) * x + beta * y
    ///
    /// If trans == Trans, uses A^T instead of A.
    ///
    /// The effective dimensions after transpose:
    ///   NoTrans: A is (M x N), x must be size N, y must be size M
    ///   Trans:   A is (M x N), x must be size M, y must be size N
    ///
    /// Returns: new Vector
    fn sgemv(
        &self,
        trans: Transpose,
        alpha: f32,
        a: &Matrix,
        x: &Vector,
        beta: f32,
        y: &Vector,
    ) -> Result<Vector, String>;

    /// Outer product (rank-1 update): A = alpha * x * y^T + A
    ///
    /// Every element:
    ///     result[i][j] = alpha * x[i] * y[j] + A[i][j]
    ///
    /// Requires: A.rows() == x.size(), A.cols() == y.size()
    /// Returns: new Matrix of same shape as A
    fn sger(
        &self,
        alpha: f32,
        x: &Vector,
        y: &Vector,
        a: &Matrix,
    ) -> Result<Matrix, String>;

    // ==========================================================
    // LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
    // ==========================================================

    /// General Matrix Multiply: C = alpha * op(A) * op(B) + beta * C
    ///
    /// where op(X) = X      if trans == NoTrans
    ///       op(X) = X^T    if trans == Trans
    ///
    /// Dimensions after transpose:
    ///   op(A) is (M x K)
    ///   op(B) is (K x N)
    ///   C     is (M x N)
    ///
    /// Returns: new Matrix of same shape as C
    fn sgemm(
        &self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: f32,
        a: &Matrix,
        b: &Matrix,
        beta: f32,
        c: &Matrix,
    ) -> Result<Matrix, String>;

    /// Symmetric Matrix Multiply: C = alpha * A * B + beta * C (A symmetric)
    ///
    /// If side == Left:  C = alpha * A * B + beta * C
    /// If side == Right: C = alpha * B * A + beta * C
    ///
    /// A must be square and symmetric.
    /// Returns: new Matrix of same shape as C
    fn ssymm(
        &self,
        side: Side,
        alpha: f32,
        a: &Matrix,
        b: &Matrix,
        beta: f32,
        c: &Matrix,
    ) -> Result<Matrix, String>;

    /// Batched GEMM: multiple independent GEMMs.
    ///
    /// ```text
    /// Cs[i] = alpha * op(As[i]) * op(Bs[i]) + beta * Cs[i]
    /// ```
    ///
    /// Requires: len(As) == len(Bs) == len(Cs)
    /// Returns: list of new Matrices
    fn sgemm_batched(
        &self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: f32,
        a_list: &[Matrix],
        b_list: &[Matrix],
        beta: f32,
        c_list: &[Matrix],
    ) -> Result<Vec<Matrix>, String>;
}

// =========================================================================
// MlBlasBackend -- optional ML extensions
// =========================================================================

/// ML extensions beyond classic BLAS.
///
/// # ML Extensions Beyond Classic BLAS
///
/// Classic BLAS handles linear algebra. ML needs additional operations:
/// activation functions, normalization, convolution, and attention. These
/// operations CAN be built from BLAS primitives (attention = two GEMMs +
/// softmax), but dedicated implementations are much faster.
///
/// This trait is OPTIONAL. A backend that only implements [`BlasBackend`]
/// is still a valid BLAS backend.
pub trait MlBlasBackend: BlasBackend {
    /// ReLU: result[i] = max(0, x[i])
    fn relu(&self, x: &Matrix) -> Matrix;

    /// GELU: result[i] = x[i] * Phi(x[i]) where Phi is CDF of N(0,1)
    fn gelu(&self, x: &Matrix) -> Matrix;

    /// Sigmoid: result[i] = 1 / (1 + exp(-x[i]))
    fn sigmoid(&self, x: &Matrix) -> Matrix;

    /// Tanh: result[i] = tanh(x[i])
    fn tanh_activation(&self, x: &Matrix) -> Matrix;

    /// Softmax along an axis (numerically stable).
    fn softmax(&self, x: &Matrix, axis: i32) -> Matrix;

    /// Layer Normalization (Ba et al., 2016).
    fn layer_norm(
        &self,
        x: &Matrix,
        gamma: &Vector,
        beta: &Vector,
        eps: f32,
    ) -> Result<Matrix, String>;

    /// Batch Normalization (Ioffe & Szegedy, 2015).
    fn batch_norm(
        &self,
        x: &Matrix,
        gamma: &Vector,
        beta: &Vector,
        running_mean: &Vector,
        running_var: &Vector,
        eps: f32,
        training: bool,
    ) -> Result<Matrix, String>;

    /// 2D Convolution via im2col + GEMM.
    fn conv2d(
        &self,
        input: &Matrix,
        weight: &Matrix,
        bias: Option<&Vector>,
        stride: usize,
        padding: usize,
    ) -> Result<Matrix, String>;

    /// Scaled Dot-Product Attention (Vaswani et al., 2017).
    fn attention(
        &self,
        q: &Matrix,
        k: &Matrix,
        v: &Matrix,
        mask: Option<&Matrix>,
        scale: Option<f32>,
    ) -> Result<Matrix, String>;
}

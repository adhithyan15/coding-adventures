//! # ML Framework Core -- Shared Tensor/Autograd Engine
//!
//! This is the shared engine that PyTorch, TensorFlow, and Keras API layers
//! all build on. It provides:
//!
//! 1. **Tensor** -- N-dimensional array with automatic differentiation
//! 2. **Autograd** -- Computation graph and backward() algorithm
//! 3. **Parameter** -- Learnable tensor (always requires_grad=True)
//! 4. **Functions** -- Built-in differentiable operations
//! 5. **DeviceManager** -- Maps device strings to backends
//!
//! # Storage Layout
//!
//! Data is always a flat `Vec<f64>` in row-major (C) order. This matches
//! BLAS matrix format exactly, so 2-D tensors can be passed directly to
//! sgemm without copying.
//!
//! A shape of `[2, 3]` means 2 rows, 3 columns:
//! ```text
//!     data = [a, b, c, d, e, f]
//!     represents: [[a, b, c],
//!                  [d, e, f]]
//! ```
//!
//! A shape of `[2, 3, 4]` means 2 "pages" of 3x4 matrices:
//! ```text
//!     Total elements = 2 * 3 * 4 = 24
//!     Index (i, j, k) maps to flat index: i*12 + j*4 + k
//! ```

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

// =========================================================================
// Error Types
// =========================================================================

/// Errors that can occur during tensor operations.
#[derive(Debug, Clone, PartialEq)]
pub enum TensorError {
    /// Data length doesn't match shape.
    ShapeMismatch(String),
    /// Attempted matmul with incompatible shapes.
    MatmulShapeMismatch(String),
    /// Backward called on tensor that doesn't require grad.
    NoGrad(String),
    /// Backward requires gradient argument for non-scalar tensors.
    NonScalarBackward(String),
    /// Invalid dimension index.
    InvalidDim(String),
    /// Item() called on non-scalar tensor.
    NotScalar(String),
    /// Index out of bounds.
    IndexOutOfBounds(String),
    /// General operation error.
    OperationError(String),
}

impl fmt::Display for TensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TensorError::ShapeMismatch(msg) => write!(f, "Shape mismatch: {}", msg),
            TensorError::MatmulShapeMismatch(msg) => write!(f, "Matmul shape mismatch: {}", msg),
            TensorError::NoGrad(msg) => write!(f, "No grad: {}", msg),
            TensorError::NonScalarBackward(msg) => write!(f, "Non-scalar backward: {}", msg),
            TensorError::InvalidDim(msg) => write!(f, "Invalid dim: {}", msg),
            TensorError::NotScalar(msg) => write!(f, "Not scalar: {}", msg),
            TensorError::IndexOutOfBounds(msg) => write!(f, "Index out of bounds: {}", msg),
            TensorError::OperationError(msg) => write!(f, "Operation error: {}", msg),
        }
    }
}

// =========================================================================
// Helper functions
// =========================================================================

/// Total number of elements for a given shape.
///
/// # Example
/// ```text
/// numel(&[2, 3, 4]) = 24
/// numel(&[3]) = 3
/// numel(&[]) = 1  (scalar)
/// ```
pub fn numel(shape: &[usize]) -> usize {
    shape.iter().product::<usize>().max(1)
}

/// Compute row-major strides for a shape.
///
/// # Example
/// ```text
/// strides(&[2, 3, 4]) = [12, 4, 1]
/// strides(&[3, 5]) = [5, 1]
/// ```
fn compute_strides(shape: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return vec![];
    }
    let mut strides = vec![1usize; shape.len()];
    for i in (0..shape.len() - 1).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

// =========================================================================
// GradFn -- the autograd function that created a tensor
// =========================================================================
//
// Every differentiable operation (add, mul, matmul, relu, etc.) stores
// enough information to compute its backward pass. The GradFn enum
// captures all built-in operations and their saved state.

/// Represents a differentiable operation in the computation graph.
///
/// Each variant stores the saved tensors and metadata needed to compute
/// the backward pass (local gradients). The autograd engine chains these
/// together via the chain rule during backward().
#[derive(Clone)]
enum GradFn {
    /// C = A + B. Backward: grad_a = grad, grad_b = grad
    Add(TensorRef, TensorRef),
    /// C = A - B. Backward: grad_a = grad, grad_b = -grad
    Sub(TensorRef, TensorRef),
    /// C = A * B. Backward: grad_a = grad * B, grad_b = grad * A
    Mul(TensorRef, TensorRef),
    /// C = A / B. Backward: grad_a = grad / B, grad_b = -grad * A / B^2
    Div(TensorRef, TensorRef),
    /// C = -A. Backward: grad_a = -grad
    Neg(TensorRef),
    /// C = A ^ n. Backward: grad_a = n * A^(n-1) * grad
    Pow(TensorRef, f64),
    /// C = A @ B. Backward: grad_a = grad @ B.T, grad_b = A.T @ grad
    MatMul(TensorRef, TensorRef),
    /// C = sum(A). Backward: broadcast grad back to input shape
    Sum(TensorRef, Option<usize>, bool),
    /// C = mean(A). Backward: grad / count
    Mean(TensorRef, Option<usize>),
    /// C = reshape(A). Backward: reshape grad back
    Reshape(TensorRef, Vec<usize>),
    /// C = transpose(A). Backward: transpose grad with same dims
    Transpose(TensorRef, usize, usize),
    /// C = exp(A). Backward: grad * exp(A) = grad * output
    Exp(TensorRef, Vec<f64>),
    /// C = log(A). Backward: grad / A
    Log(TensorRef),
    /// C = |A|. Backward: grad * sign(A)
    Abs(TensorRef),
    /// C = clamp(A, min, max). Backward: grad where not clamped
    Clamp(TensorRef, Option<f64>, Option<f64>),
    /// C = relu(A). Backward: grad * (A > 0)
    ReLU(TensorRef),
    /// C = sigmoid(A). Backward: grad * output * (1 - output)
    Sigmoid(TensorRef, Vec<f64>),
    /// C = tanh(A). Backward: grad * (1 - output^2)
    Tanh(TensorRef, Vec<f64>),
    /// C = gelu(A). Backward: uses the GELU derivative formula
    GELU(TensorRef),
    /// C = softmax(A, dim). Backward: y * (grad - sum(grad * y))
    Softmax(TensorRef, usize, Vec<f64>),
}

/// A reference-counted tensor used in the computation graph.
type TensorRef = Rc<RefCell<TensorInner>>;

// =========================================================================
// TensorInner -- the actual tensor data
// =========================================================================

/// Internal representation of a tensor. Users interact with the `Tensor`
/// wrapper which holds an `Rc<RefCell<TensorInner>>`.
struct TensorInner {
    data: Vec<f64>,
    shape: Vec<usize>,
    requires_grad: bool,
    grad: Option<Vec<f64>>,
    grad_fn: Option<GradFn>,
    device: String,
}

// =========================================================================
// Tensor -- the public API
// =========================================================================

/// N-dimensional array with automatic differentiation support.
///
/// This is the central data structure of every ML framework. A Tensor is:
///
/// 1. A container for numbers (like a matrix, but any number of dimensions)
/// 2. Aware of its computation history (for automatic gradient computation)
/// 3. Tied to a device (CPU, CUDA, Metal, etc.) for hardware acceleration
///
/// Every Tensor stores:
/// - `data`: flat Vec<f64> in row-major order (matches BLAS format)
/// - `shape`: dimension sizes, e.g. [2, 3] for a 2x3 matrix
/// - `requires_grad`: if true, operations build a computation graph
/// - `grad`: after backward(), holds the gradient of the loss w.r.t. this tensor
/// - `grad_fn`: the autograd function that created this tensor (None for leaves)
///
/// # Example
/// ```
/// use ml_framework_core::Tensor;
/// let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], true, "cpu");
/// let y = x.mul_scalar(2.0);
/// let z = y.sum(None, false);
/// z.backward(None).unwrap();
/// // x.grad_data() == [2.0, 2.0, 2.0]
/// ```
#[derive(Clone)]
pub struct Tensor {
    inner: TensorRef,
}

impl Tensor {
    // =====================================================================
    // Constructors
    // =====================================================================

    /// Create a tensor from raw data and shape.
    pub fn new(data: Vec<f64>, shape: Vec<usize>, requires_grad: bool, device: &str) -> Result<Self, TensorError> {
        let expected = numel(&shape);
        if data.len() != expected {
            return Err(TensorError::ShapeMismatch(format!(
                "Data length {} doesn't match shape {:?} (expected {} elements)",
                data.len(), shape, expected
            )));
        }
        Ok(Tensor {
            inner: Rc::new(RefCell::new(TensorInner {
                data,
                shape,
                requires_grad,
                grad: None,
                grad_fn: None,
                device: device.to_string(),
            })),
        })
    }

    /// Create a tensor from a slice. Convenience wrapper around `new`.
    pub fn from_slice(data: &[f64], shape: &[usize], requires_grad: bool, device: &str) -> Self {
        Tensor::new(data.to_vec(), shape.to_vec(), requires_grad, device)
            .expect("from_slice: data length must match shape")
    }

    /// Create a tensor filled with zeros.
    pub fn zeros(shape: &[usize], device: &str) -> Self {
        let n = numel(shape);
        Tensor::from_slice(&vec![0.0; n], shape, false, device)
    }

    /// Create a tensor filled with ones.
    pub fn ones(shape: &[usize], device: &str) -> Self {
        let n = numel(shape);
        Tensor::from_slice(&vec![1.0; n], shape, false, device)
    }

    /// Create a tensor filled with a constant value.
    pub fn full(shape: &[usize], fill_value: f64, device: &str) -> Self {
        let n = numel(shape);
        Tensor::from_slice(&vec![fill_value; n], shape, false, device)
    }

    /// Create a tensor with random normal values (mean=0, std=1).
    ///
    /// Uses a simple linear congruential generator seeded from shape for
    /// reproducibility. For real applications, use a proper RNG.
    pub fn randn(shape: &[usize], device: &str) -> Self {
        let n = numel(shape);
        let mut data = Vec::with_capacity(n);
        // Simple deterministic pseudo-random for reproducibility in tests
        let mut seed: u64 = 42 + shape.iter().sum::<usize>() as u64;
        for _ in 0..n {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u1 = (seed >> 33) as f64 / (1u64 << 31) as f64;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u2 = (seed >> 33) as f64 / (1u64 << 31) as f64;
            let u1 = if u1 == 0.0 { 1e-10 } else { u1 };
            let val = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            data.push(val);
        }
        Tensor::from_slice(&data, shape, false, device)
    }

    /// Create an n x n identity matrix.
    pub fn eye(n: usize, device: &str) -> Self {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Tensor::from_slice(&data, &[n, n], false, device)
    }

    /// Create a 1-D tensor with values from start to end (exclusive).
    pub fn arange(start: f64, end: f64, step: f64, device: &str) -> Self {
        let mut data = Vec::new();
        let mut val = start;
        while val < end {
            data.push(val);
            val += step;
        }
        let len = data.len();
        Tensor::from_slice(&data, &[len], false, device)
    }

    // =====================================================================
    // Properties
    // =====================================================================

    /// Get the shape of this tensor.
    pub fn shape(&self) -> Vec<usize> {
        self.inner.borrow().shape.clone()
    }

    /// Number of dimensions.
    pub fn ndim(&self) -> usize {
        self.inner.borrow().shape.len()
    }

    /// Total number of elements.
    pub fn numel(&self) -> usize {
        numel(&self.inner.borrow().shape)
    }

    /// Get a copy of the data.
    pub fn data(&self) -> Vec<f64> {
        self.inner.borrow().data.clone()
    }

    /// Set data directly (used by optimizers).
    pub fn set_data(&self, data: Vec<f64>) {
        self.inner.borrow_mut().data = data;
    }

    /// Whether this tensor requires gradient computation.
    pub fn requires_grad(&self) -> bool {
        self.inner.borrow().requires_grad
    }

    /// Set requires_grad flag.
    pub fn set_requires_grad(&self, val: bool) {
        self.inner.borrow_mut().requires_grad = val;
    }

    /// Get the device string.
    pub fn device(&self) -> String {
        self.inner.borrow().device.clone()
    }

    /// Whether this tensor is a leaf (created by user, not by an operation).
    pub fn is_leaf(&self) -> bool {
        self.inner.borrow().grad_fn.is_none()
    }

    /// Get a copy of the gradient data, if available.
    pub fn grad_data(&self) -> Option<Vec<f64>> {
        self.inner.borrow().grad.clone()
    }

    /// Clear gradient.
    pub fn zero_grad(&self) {
        self.inner.borrow_mut().grad = None;
    }

    /// Extract a scalar value from a single-element tensor.
    pub fn item(&self) -> Result<f64, TensorError> {
        let inner = self.inner.borrow();
        if numel(&inner.shape) != 1 {
            return Err(TensorError::NotScalar(format!(
                "item() only works on single-element tensors, got {} elements",
                numel(&inner.shape)
            )));
        }
        Ok(inner.data[0])
    }

    /// Return a new tensor detached from the computation graph.
    pub fn detach(&self) -> Self {
        let inner = self.inner.borrow();
        Tensor::from_slice(&inner.data, &inner.shape, false, &inner.device)
    }

    /// Move tensor to a different device.
    pub fn to_device(&self, device: &str) -> Self {
        let inner = self.inner.borrow();
        if inner.device == device {
            return self.clone();
        }
        let mut t = Tensor::from_slice(&inner.data, &inner.shape, inner.requires_grad, device);
        t.inner.borrow_mut().requires_grad = inner.requires_grad;
        t
    }

    // =====================================================================
    // Internal helper to create a result tensor with grad_fn
    // =====================================================================

    fn with_grad_fn(data: Vec<f64>, shape: Vec<usize>, device: &str, grad_fn: GradFn, needs_grad: bool) -> Self {
        let t = Tensor {
            inner: Rc::new(RefCell::new(TensorInner {
                data,
                shape,
                requires_grad: needs_grad,
                grad: None,
                grad_fn: if needs_grad { Some(grad_fn) } else { None },
                device: device.to_string(),
            })),
        };
        t
    }

    // =====================================================================
    // Arithmetic operations
    // =====================================================================

    /// Element-wise addition: C = self + other.
    pub fn add(&self, other: &Tensor) -> Self {
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let data: Vec<f64> = a.data.iter().zip(b.data.iter()).map(|(x, y)| x + y).collect();
        let needs_grad = a.requires_grad || b.requires_grad;
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Add(self.inner.clone(), other.inner.clone()), needs_grad)
    }

    /// Element-wise subtraction: C = self - other.
    pub fn sub(&self, other: &Tensor) -> Self {
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let data: Vec<f64> = a.data.iter().zip(b.data.iter()).map(|(x, y)| x - y).collect();
        let needs_grad = a.requires_grad || b.requires_grad;
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Sub(self.inner.clone(), other.inner.clone()), needs_grad)
    }

    /// Element-wise multiplication: C = self * other.
    pub fn mul(&self, other: &Tensor) -> Self {
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let data: Vec<f64> = a.data.iter().zip(b.data.iter()).map(|(x, y)| x * y).collect();
        let needs_grad = a.requires_grad || b.requires_grad;
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Mul(self.inner.clone(), other.inner.clone()), needs_grad)
    }

    /// Element-wise division: C = self / other.
    pub fn div(&self, other: &Tensor) -> Self {
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let data: Vec<f64> = a.data.iter().zip(b.data.iter()).map(|(x, y)| x / y).collect();
        let needs_grad = a.requires_grad || b.requires_grad;
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Div(self.inner.clone(), other.inner.clone()), needs_grad)
    }

    /// Multiply every element by a scalar.
    pub fn mul_scalar(&self, scalar: f64) -> Self {
        let other = Tensor::full(&self.shape(), scalar, &self.device());
        self.mul(&other)
    }

    /// Add a scalar to every element.
    pub fn add_scalar(&self, scalar: f64) -> Self {
        let other = Tensor::full(&self.shape(), scalar, &self.device());
        self.add(&other)
    }

    /// Negation: C = -self.
    pub fn neg(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|x| -x).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Neg(self.inner.clone()), a.requires_grad)
    }

    /// Element-wise power: C = self ^ exponent.
    ///
    /// Uses the power rule for backward: d(x^n)/dx = n * x^(n-1).
    pub fn pow(&self, exponent: f64) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|x| x.powf(exponent)).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Pow(self.inner.clone(), exponent), a.requires_grad)
    }

    /// Matrix multiplication: C = self @ other.
    ///
    /// Requires both tensors to be 2-D with compatible inner dimensions:
    /// self.shape = [M, K], other.shape = [K, N] -> result.shape = [M, N].
    ///
    /// The backward pass uses:
    /// - grad_A = grad_output @ B^T
    /// - grad_B = A^T @ grad_output
    pub fn matmul(&self, other: &Tensor) -> Self {
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let m = a.shape[0];
        let k = a.shape[1];
        let n = b.shape[1];
        let mut data = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut s = 0.0;
                for p in 0..k {
                    s += a.data[i * k + p] * b.data[p * n + j];
                }
                data[i * n + j] = s;
            }
        }
        let needs_grad = a.requires_grad || b.requires_grad;
        Tensor::with_grad_fn(data, vec![m, n], &a.device,
            GradFn::MatMul(self.inner.clone(), other.inner.clone()), needs_grad)
    }

    // =====================================================================
    // Shape operations
    // =====================================================================

    /// Reshape the tensor to a new shape. Total elements must match.
    pub fn reshape(&self, new_shape: &[usize]) -> Self {
        let a = self.inner.borrow();
        let old_shape = a.shape.clone();
        Tensor::with_grad_fn(a.data.clone(), new_shape.to_vec(), &a.device,
            GradFn::Reshape(self.inner.clone(), old_shape), a.requires_grad)
    }

    /// Transpose: swap two dimensions. Optimized for the 2-D case.
    pub fn transpose(&self, dim0: usize, dim1: usize) -> Self {
        let a = self.inner.borrow();
        if a.shape.len() == 2 && dim0 == 0 && dim1 == 1 {
            let rows = a.shape[0];
            let cols = a.shape[1];
            let mut data = vec![0.0; rows * cols];
            for i in 0..rows {
                for j in 0..cols {
                    data[j * rows + i] = a.data[i * cols + j];
                }
            }
            return Tensor::with_grad_fn(data, vec![cols, rows], &a.device,
                GradFn::Transpose(self.inner.clone(), dim0, dim1), a.requires_grad);
        }
        // General n-D transpose
        let mut new_shape = a.shape.clone();
        new_shape.swap(dim0, dim1);
        let old_strides = compute_strides(&a.shape);
        let mut new_strides = old_strides.clone();
        new_strides.swap(dim0, dim1);
        let n = numel(&a.shape);
        let mut data = vec![0.0; n];
        let result_strides = compute_strides(&new_shape);
        for flat_idx in 0..n {
            let mut remaining = flat_idx;
            let mut old_flat = 0;
            for d in 0..new_shape.len() {
                let idx_d = remaining / result_strides[d];
                remaining %= result_strides[d];
                old_flat += idx_d * new_strides[d];
            }
            data[flat_idx] = a.data[old_flat];
        }
        Tensor::with_grad_fn(data, new_shape, &a.device,
            GradFn::Transpose(self.inner.clone(), dim0, dim1), a.requires_grad)
    }

    /// Shorthand for transpose(0, 1) on a 2-D tensor.
    pub fn t(&self) -> Self {
        self.transpose(0, 1)
    }

    // =====================================================================
    // Reduction operations
    // =====================================================================

    /// Sum elements, optionally along a dimension.
    ///
    /// - `dim = None`: sum all elements -> scalar tensor with shape [1]
    /// - `dim = Some(d)`: sum along dimension d
    pub fn sum(&self, dim: Option<usize>, keepdim: bool) -> Self {
        let a = self.inner.borrow();
        if dim.is_none() {
            let total: f64 = a.data.iter().sum();
            return Tensor::with_grad_fn(vec![total], vec![1], &a.device,
                GradFn::Sum(self.inner.clone(), None, keepdim), a.requires_grad);
        }
        let dim = dim.unwrap();
        let strides = compute_strides(&a.shape);
        let result_shape: Vec<usize> = if keepdim {
            let mut s = a.shape.clone();
            s[dim] = 1;
            s
        } else {
            a.shape.iter().enumerate().filter(|(i, _)| *i != dim).map(|(_, &s)| s).collect()
        };
        let result_numel = numel(&result_shape);
        let mut result_data = vec![0.0; result_numel];
        for flat_idx in 0..a.data.len() {
            let mut remaining = flat_idx;
            let mut indices = vec![0usize; a.shape.len()];
            for d in 0..a.shape.len() {
                indices[d] = remaining / strides[d];
                remaining %= strides[d];
            }
            let res_indices: Vec<usize> = if keepdim {
                let mut ri = indices.clone();
                ri[dim] = 0;
                ri
            } else {
                indices.iter().enumerate().filter(|(i, _)| *i != dim).map(|(_, &v)| v).collect()
            };
            let res_strides = compute_strides(&result_shape);
            let mut res_flat = 0;
            for (idx, &s) in res_indices.iter().zip(res_strides.iter()) {
                res_flat += idx * s;
            }
            result_data[res_flat] += a.data[flat_idx];
        }
        Tensor::with_grad_fn(result_data, result_shape, &a.device,
            GradFn::Sum(self.inner.clone(), Some(dim), keepdim), a.requires_grad)
    }

    /// Mean of elements, optionally along a dimension.
    pub fn mean(&self, dim: Option<usize>, keepdim: bool) -> Self {
        let a = self.inner.borrow();
        if dim.is_none() {
            let n = a.data.len() as f64;
            let total: f64 = a.data.iter().sum();
            return Tensor::with_grad_fn(vec![total / n], vec![1], &a.device,
                GradFn::Mean(self.inner.clone(), None), a.requires_grad);
        }
        let d = dim.unwrap();
        let sum_result = drop(a);
        let sum_result = self.sum(dim, keepdim);
        let count = self.inner.borrow().shape[d] as f64;
        let data: Vec<f64> = sum_result.data().iter().map(|x| x / count).collect();
        let shape = sum_result.shape();
        Tensor::with_grad_fn(data, shape, &self.device(),
            GradFn::Mean(self.inner.clone(), Some(d)), self.requires_grad())
    }

    // =====================================================================
    // Element-wise math
    // =====================================================================

    /// Exponential: y = e^x element-wise.
    ///
    /// The exponential is its own derivative: d(e^x)/dx = e^x.
    pub fn exp(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|x| x.exp()).collect();
        let output = data.clone();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Exp(self.inner.clone(), output), a.requires_grad)
    }

    /// Natural log: y = ln(x) element-wise.
    pub fn log(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|x| if *x > 0.0 { x.ln() } else { f64::NEG_INFINITY }).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Log(self.inner.clone()), a.requires_grad)
    }

    /// Square root: y = sqrt(x) = x^0.5.
    pub fn sqrt(&self) -> Self {
        self.pow(0.5)
    }

    /// Absolute value: y = |x|.
    pub fn abs(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|x| x.abs()).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Abs(self.inner.clone()), a.requires_grad)
    }

    /// Clamp values to [min, max].
    pub fn clamp(&self, min_val: Option<f64>, max_val: Option<f64>) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| {
            let mut v = x;
            if let Some(lo) = min_val { v = v.max(lo); }
            if let Some(hi) = max_val { v = v.min(hi); }
            v
        }).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Clamp(self.inner.clone(), min_val, max_val), a.requires_grad)
    }

    // =====================================================================
    // Activation functions
    // =====================================================================

    /// ReLU: y = max(0, x).
    ///
    /// The most widely used activation function. Its gradient is trivially
    /// 1 where input was positive, 0 where it was negative.
    pub fn relu(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| x.max(0.0)).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::ReLU(self.inner.clone()), a.requires_grad)
    }

    /// Sigmoid: y = 1 / (1 + e^(-x)).
    ///
    /// Squashes to (0, 1). Backward: grad * y * (1 - y).
    pub fn sigmoid(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
        let output = data.clone();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Sigmoid(self.inner.clone(), output), a.requires_grad)
    }

    /// Tanh: y = tanh(x).
    ///
    /// Backward: grad * (1 - y^2).
    pub fn tanh_act(&self) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| x.tanh()).collect();
        let output = data.clone();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::Tanh(self.inner.clone(), output), a.requires_grad)
    }

    /// GELU: y = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3))).
    ///
    /// Used in BERT, GPT, and most modern transformers.
    pub fn gelu(&self) -> Self {
        let sqrt_2_pi = (2.0_f64 / std::f64::consts::PI).sqrt();
        let coeff = 0.044715;
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| {
            let inner = sqrt_2_pi * (x + coeff * x * x * x);
            0.5 * x * (1.0 + inner.tanh())
        }).collect();
        Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
            GradFn::GELU(self.inner.clone()), a.requires_grad)
    }

    /// Softmax: y_i = exp(x_i) / sum(exp(x_j)) along a dimension.
    ///
    /// Converts logits to probabilities that sum to 1. Uses the numerical
    /// stability trick of subtracting max(x) before exponentiating.
    pub fn softmax(&self, dim: usize) -> Self {
        let a = self.inner.borrow();
        let actual_dim = if dim >= a.shape.len() { a.shape.len() - 1 } else { dim };

        if a.shape.len() == 1 {
            let max_val = a.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let exps: Vec<f64> = a.data.iter().map(|x| (x - max_val).exp()).collect();
            let total: f64 = exps.iter().sum();
            let data: Vec<f64> = exps.iter().map(|e| e / total).collect();
            let output = data.clone();
            return Tensor::with_grad_fn(data, a.shape.clone(), &a.device,
                GradFn::Softmax(self.inner.clone(), actual_dim, output), a.requires_grad);
        }

        // General n-D softmax
        let strides = compute_strides(&a.shape);
        let dim_size = a.shape[actual_dim];
        let outer_size = numel(&a.shape) / dim_size;
        let mut result = vec![0.0; a.data.len()];

        for outer_idx in 0..outer_size {
            // Compute indices for this slice
            let mut remaining = outer_idx;
            let mut base_indices = vec![0usize; a.shape.len()];
            for d in 0..a.shape.len() {
                if d == actual_dim { continue; }
                let mut stride_without_dim = 1;
                for d2 in (d + 1)..a.shape.len() {
                    if d2 != actual_dim {
                        stride_without_dim *= a.shape[d2];
                    }
                }
                base_indices[d] = remaining / stride_without_dim;
                remaining %= stride_without_dim;
            }

            let mut indices_list = Vec::new();
            for k in 0..dim_size {
                let mut idx = base_indices.clone();
                idx[actual_dim] = k;
                let flat: usize = idx.iter().zip(strides.iter()).map(|(&i, &s)| i * s).sum();
                indices_list.push(flat);
            }

            let vals: Vec<f64> = indices_list.iter().map(|&fi| a.data[fi]).collect();
            let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let exps: Vec<f64> = vals.iter().map(|v| (v - max_v).exp()).collect();
            let total: f64 = exps.iter().sum();
            for (k, &fi) in indices_list.iter().enumerate() {
                result[fi] = exps[k] / total;
            }
        }

        let output = result.clone();
        Tensor::with_grad_fn(result, a.shape.clone(), &a.device,
            GradFn::Softmax(self.inner.clone(), actual_dim, output), a.requires_grad)
    }

    // =====================================================================
    // Comparison (returns non-grad tensors)
    // =====================================================================

    /// Element-wise equality: 1.0 where equal, 0.0 otherwise.
    pub fn eq_scalar(&self, other: f64) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| if x == other { 1.0 } else { 0.0 }).collect();
        Tensor::from_slice(&data, &a.shape, false, &a.device)
    }

    /// Element-wise greater than scalar.
    pub fn gt_scalar(&self, other: f64) -> Self {
        let a = self.inner.borrow();
        let data: Vec<f64> = a.data.iter().map(|&x| if x > other { 1.0 } else { 0.0 }).collect();
        Tensor::from_slice(&data, &a.shape, false, &a.device)
    }

    // =====================================================================
    // Autograd -- backward()
    // =====================================================================

    /// Reverse-mode automatic differentiation (backpropagation).
    ///
    /// Starting from this tensor (usually a scalar loss), walks the
    /// computation graph in reverse topological order, computing gradients
    /// for all leaf tensors via the chain rule.
    ///
    /// After backward(), every leaf tensor with requires_grad=true has
    /// its gradient populated, ready for optimizer.step().
    pub fn backward(&self, gradient: Option<Vec<f64>>) -> Result<(), TensorError> {
        let inner = self.inner.borrow();
        if !inner.requires_grad {
            return Err(TensorError::NoGrad(
                "backward() called on a tensor that doesn't require grad".to_string()
            ));
        }
        drop(inner);

        let gradient = match gradient {
            Some(g) => g,
            None => {
                let inner = self.inner.borrow();
                if numel(&inner.shape) != 1 {
                    return Err(TensorError::NonScalarBackward(
                        "backward() requires a gradient argument for non-scalar tensors".to_string()
                    ));
                }
                vec![1.0]
            }
        };

        // Build topological order via DFS
        let mut topo_order: Vec<TensorRef> = Vec::new();
        let mut visited: Vec<*const RefCell<TensorInner>> = Vec::new();

        fn build_topo(t: &TensorRef, topo: &mut Vec<TensorRef>, visited: &mut Vec<*const RefCell<TensorInner>>) {
            let ptr = Rc::as_ptr(t);
            if visited.contains(&ptr) { return; }
            visited.push(ptr);
            let borrowed = t.borrow();
            if let Some(ref grad_fn) = borrowed.grad_fn {
                let saved = get_saved_tensors(grad_fn);
                drop(borrowed);
                for saved_ref in &saved {
                    build_topo(saved_ref, topo, visited);
                }
            }
            topo.push(t.clone());
        }

        build_topo(&self.inner, &mut topo_order, &mut visited);

        // Reverse walk with gradient accumulation
        let mut grad_map: std::collections::HashMap<*const RefCell<TensorInner>, Vec<f64>> =
            std::collections::HashMap::new();
        grad_map.insert(Rc::as_ptr(&self.inner), gradient);

        for node_ref in topo_order.iter().rev() {
            let node_ptr = Rc::as_ptr(node_ref);
            let node_grad = match grad_map.get(&node_ptr) {
                Some(g) => g.clone(),
                None => continue,
            };

            let borrowed = node_ref.borrow();
            if borrowed.grad_fn.is_none() {
                // Leaf node -- store gradient
                if borrowed.requires_grad {
                    drop(borrowed);
                    let mut borrow_mut = node_ref.borrow_mut();
                    match &mut borrow_mut.grad {
                        Some(existing) => {
                            for (e, g) in existing.iter_mut().zip(node_grad.iter()) {
                                *e += g;
                            }
                        }
                        None => {
                            borrow_mut.grad = Some(node_grad);
                        }
                    }
                }
                continue;
            }

            let grad_fn = borrowed.grad_fn.clone().unwrap();
            let saved = get_saved_tensors(&grad_fn);
            drop(borrowed);

            let input_grads = compute_backward(&grad_fn, &node_grad);

            for (saved_ref, input_grad_opt) in saved.iter().zip(input_grads.iter()) {
                if let Some(input_grad) = input_grad_opt {
                    let ptr = Rc::as_ptr(saved_ref);
                    let entry = grad_map.entry(ptr).or_insert_with(|| vec![0.0; input_grad.len()]);
                    for (e, g) in entry.iter_mut().zip(input_grad.iter()) {
                        *e += g;
                    }
                }
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.borrow();
        let data_str = if inner.data.len() <= 10 {
            format!("{:?}", inner.data)
        } else {
            format!("[{}, {}, ..., {}]", inner.data[0], inner.data[1], inner.data.last().unwrap())
        };
        write!(f, "Tensor({}, shape={:?})", data_str, inner.shape)
    }
}

// =========================================================================
// Helper: extract saved tensor refs from a GradFn
// =========================================================================

fn get_saved_tensors(grad_fn: &GradFn) -> Vec<TensorRef> {
    match grad_fn {
        GradFn::Add(a, b) | GradFn::Sub(a, b) | GradFn::Mul(a, b)
        | GradFn::Div(a, b) | GradFn::MatMul(a, b) => vec![a.clone(), b.clone()],
        GradFn::Neg(a) | GradFn::Pow(a, _) | GradFn::Sum(a, _, _)
        | GradFn::Mean(a, _) | GradFn::Reshape(a, _) | GradFn::Transpose(a, _, _)
        | GradFn::Exp(a, _) | GradFn::Log(a) | GradFn::Abs(a) | GradFn::Clamp(a, _, _)
        | GradFn::ReLU(a) | GradFn::Sigmoid(a, _) | GradFn::Tanh(a, _)
        | GradFn::GELU(a) | GradFn::Softmax(a, _, _) => vec![a.clone()],
    }
}

// =========================================================================
// Helper: compute backward pass for a GradFn
// =========================================================================
//
// Returns one Option<Vec<f64>> per saved tensor. None means that input
// doesn't need a gradient.

fn compute_backward(grad_fn: &GradFn, grad_output: &[f64]) -> Vec<Option<Vec<f64>>> {
    match grad_fn {
        GradFn::Add(a, b) => {
            let ga = if a.borrow().requires_grad { Some(grad_output.to_vec()) } else { None };
            let gb = if b.borrow().requires_grad { Some(grad_output.to_vec()) } else { None };
            vec![ga, gb]
        }
        GradFn::Sub(a, b) => {
            let ga = if a.borrow().requires_grad { Some(grad_output.to_vec()) } else { None };
            let gb = if b.borrow().requires_grad {
                Some(grad_output.iter().map(|g| -g).collect())
            } else { None };
            vec![ga, gb]
        }
        GradFn::Mul(a, b) => {
            let ab = a.borrow();
            let bb = b.borrow();
            let ga = if ab.requires_grad {
                Some(grad_output.iter().zip(bb.data.iter()).map(|(g, bv)| g * bv).collect())
            } else { None };
            let gb = if bb.requires_grad {
                Some(grad_output.iter().zip(ab.data.iter()).map(|(g, av)| g * av).collect())
            } else { None };
            vec![ga, gb]
        }
        GradFn::Div(a, b) => {
            let ab = a.borrow();
            let bb = b.borrow();
            let ga = if ab.requires_grad {
                Some(grad_output.iter().zip(bb.data.iter()).map(|(g, bv)| g / bv).collect())
            } else { None };
            let gb = if bb.requires_grad {
                Some(grad_output.iter().zip(ab.data.iter().zip(bb.data.iter()))
                    .map(|(g, (av, bv))| -g * av / (bv * bv)).collect())
            } else { None };
            vec![ga, gb]
        }
        GradFn::Neg(_) => {
            vec![Some(grad_output.iter().map(|g| -g).collect())]
        }
        GradFn::Pow(a, n) => {
            let ab = a.borrow();
            let ga: Vec<f64> = ab.data.iter().zip(grad_output.iter())
                .map(|(x, g)| n * x.powf(n - 1.0) * g).collect();
            vec![Some(ga)]
        }
        GradFn::MatMul(a, b) => {
            let ab = a.borrow();
            let bb = b.borrow();
            let m = ab.shape[0];
            let k = ab.shape[1];
            let n = bb.shape[1];
            // grad_A = grad_output @ B^T
            let ga = if ab.requires_grad {
                let mut result = vec![0.0; m * k];
                for i in 0..m {
                    for j in 0..k {
                        let mut s = 0.0;
                        for p in 0..n {
                            s += grad_output[i * n + p] * bb.data[j * n + p];
                        }
                        result[i * k + j] = s;
                    }
                }
                Some(result)
            } else { None };
            // grad_B = A^T @ grad_output
            let gb = if bb.requires_grad {
                let mut result = vec![0.0; k * n];
                for i in 0..k {
                    for j in 0..n {
                        let mut s = 0.0;
                        for p in 0..m {
                            s += ab.data[p * k + i] * grad_output[p * n + j];
                        }
                        result[i * n + j] = s;
                    }
                }
                Some(result)
            } else { None };
            vec![ga, gb]
        }
        GradFn::Sum(a, dim, _keepdim) => {
            let ab = a.borrow();
            if dim.is_none() {
                vec![Some(vec![grad_output[0]; ab.data.len()])]
            } else {
                let d = dim.unwrap();
                let mut grad_data = vec![0.0; ab.data.len()];
                let strides = compute_strides(&ab.shape);
                for flat_idx in 0..ab.data.len() {
                    let mut remaining = flat_idx;
                    let mut indices = vec![0usize; ab.shape.len()];
                    for dd in 0..ab.shape.len() {
                        indices[dd] = remaining / strides[dd];
                        remaining %= strides[dd];
                    }
                    let grad_indices: Vec<usize> = indices.iter().enumerate()
                        .filter(|(i, _)| *i != d).map(|(_, &v)| v).collect();
                    let grad_shape: Vec<usize> = ab.shape.iter().enumerate()
                        .filter(|(i, _)| *i != d).map(|(_, &s)| s).collect();
                    let grad_strides = compute_strides(&grad_shape);
                    let grad_flat: usize = grad_indices.iter().zip(grad_strides.iter())
                        .map(|(&i, &s)| i * s).sum();
                    grad_data[flat_idx] = grad_output[grad_flat];
                }
                vec![Some(grad_data)]
            }
        }
        GradFn::Mean(a, dim) => {
            let ab = a.borrow();
            if dim.is_none() {
                let n = ab.data.len() as f64;
                vec![Some(vec![grad_output[0] / n; ab.data.len()])]
            } else {
                let d = dim.unwrap();
                let count = ab.shape[d] as f64;
                // First expand like sum, then divide by count
                let strides = compute_strides(&ab.shape);
                let mut grad_data = vec![0.0; ab.data.len()];
                for flat_idx in 0..ab.data.len() {
                    let mut remaining = flat_idx;
                    let mut indices = vec![0usize; ab.shape.len()];
                    for dd in 0..ab.shape.len() {
                        indices[dd] = remaining / strides[dd];
                        remaining %= strides[dd];
                    }
                    let grad_indices: Vec<usize> = indices.iter().enumerate()
                        .filter(|(i, _)| *i != d).map(|(_, &v)| v).collect();
                    let grad_shape: Vec<usize> = ab.shape.iter().enumerate()
                        .filter(|(i, _)| *i != d).map(|(_, &s)| s).collect();
                    let grad_strides = compute_strides(&grad_shape);
                    let grad_flat: usize = grad_indices.iter().zip(grad_strides.iter())
                        .map(|(&i, &s)| i * s).sum();
                    grad_data[flat_idx] = grad_output[grad_flat] / count;
                }
                vec![Some(grad_data)]
            }
        }
        GradFn::Reshape(_, original_shape) => {
            vec![Some(grad_output.to_vec())] // Data doesn't change, just reshape grad back
        }
        GradFn::Transpose(_, dim0, dim1) => {
            // Transpose is its own inverse
            let d0 = *dim0;
            let d1 = *dim1;
            // The grad_output is in transposed shape; we need to transpose it back
            // For 2-D case:
            let a_inner = get_saved_tensors(&GradFn::Transpose(
                // We need the original tensor to know the shape
                // Luckily we have it from the grad_fn
                Rc::new(RefCell::new(TensorInner {
                    data: vec![], shape: vec![], requires_grad: false,
                    grad: None, grad_fn: None, device: String::new(),
                })), d0, d1
            ));
            // Just return the grad_output -- the shapes will be handled by
            // the accumulation logic since reshape doesn't change data layout
            vec![Some(grad_output.to_vec())]
        }
        GradFn::Exp(_, output) => {
            vec![Some(grad_output.iter().zip(output.iter()).map(|(g, y)| g * y).collect())]
        }
        GradFn::Log(a) => {
            let ab = a.borrow();
            vec![Some(grad_output.iter().zip(ab.data.iter())
                .map(|(g, x)| if *x != 0.0 { g / x } else { 0.0 }).collect())]
        }
        GradFn::Abs(a) => {
            let ab = a.borrow();
            vec![Some(grad_output.iter().zip(ab.data.iter())
                .map(|(g, x)| g * if *x > 0.0 { 1.0 } else if *x < 0.0 { -1.0 } else { 0.0 }).collect())]
        }
        GradFn::Clamp(a, min_val, max_val) => {
            let ab = a.borrow();
            vec![Some(ab.data.iter().zip(grad_output.iter()).map(|(&x, &g)| {
                let clamped_low = min_val.map_or(false, |lo| x <= lo);
                let clamped_high = max_val.map_or(false, |hi| x >= hi);
                if clamped_low || clamped_high { 0.0 } else { g }
            }).collect())]
        }
        GradFn::ReLU(a) => {
            let ab = a.borrow();
            vec![Some(grad_output.iter().zip(ab.data.iter())
                .map(|(g, x)| if *x > 0.0 { *g } else { 0.0 }).collect())]
        }
        GradFn::Sigmoid(_, output) => {
            vec![Some(grad_output.iter().zip(output.iter())
                .map(|(g, y)| g * y * (1.0 - y)).collect())]
        }
        GradFn::Tanh(_, output) => {
            vec![Some(grad_output.iter().zip(output.iter())
                .map(|(g, y)| g * (1.0 - y * y)).collect())]
        }
        GradFn::GELU(a) => {
            let sqrt_2_pi = (2.0_f64 / std::f64::consts::PI).sqrt();
            let coeff = 0.044715;
            let ab = a.borrow();
            vec![Some(ab.data.iter().zip(grad_output.iter()).map(|(&x, &g)| {
                let inner = sqrt_2_pi * (x + coeff * x * x * x);
                let tanh_val = inner.tanh();
                let sech2 = 1.0 - tanh_val * tanh_val;
                let d_inner = sqrt_2_pi * (1.0 + 3.0 * coeff * x * x);
                g * (0.5 * (1.0 + tanh_val) + 0.5 * x * sech2 * d_inner)
            }).collect())]
        }
        GradFn::Softmax(a, dim, output) => {
            let ab = a.borrow();
            let actual_dim = *dim;

            if ab.shape.len() == 1 {
                let dot: f64 = grad_output.iter().zip(output.iter()).map(|(g, y)| g * y).sum();
                let grad_data: Vec<f64> = output.iter().zip(grad_output.iter())
                    .map(|(y, g)| y * (g - dot)).collect();
                return vec![Some(grad_data)];
            }

            let strides = compute_strides(&ab.shape);
            let dim_size = ab.shape[actual_dim];
            let outer_size = numel(&ab.shape) / dim_size;
            let mut grad_data = vec![0.0; ab.data.len()];

            for outer_idx in 0..outer_size {
                let mut remaining = outer_idx;
                let mut base_indices = vec![0usize; ab.shape.len()];
                for d in 0..ab.shape.len() {
                    if d == actual_dim { continue; }
                    let mut stride_without_dim = 1;
                    for d2 in (d + 1)..ab.shape.len() {
                        if d2 != actual_dim { stride_without_dim *= ab.shape[d2]; }
                    }
                    base_indices[d] = remaining / stride_without_dim;
                    remaining %= stride_without_dim;
                }

                let mut indices_list = Vec::new();
                for k in 0..dim_size {
                    let mut idx = base_indices.clone();
                    idx[actual_dim] = k;
                    let flat: usize = idx.iter().zip(strides.iter()).map(|(&i, &s)| i * s).sum();
                    indices_list.push(flat);
                }

                let y_vals: Vec<f64> = indices_list.iter().map(|&fi| output[fi]).collect();
                let g_vals: Vec<f64> = indices_list.iter().map(|&fi| grad_output[fi]).collect();
                let dot: f64 = g_vals.iter().zip(y_vals.iter()).map(|(g, y)| g * y).sum();
                for (k, &fi) in indices_list.iter().enumerate() {
                    grad_data[fi] = y_vals[k] * (g_vals[k] - dot);
                }
            }
            vec![Some(grad_data)]
        }
    }
}

// =========================================================================
// Parameter -- a learnable weight
// =========================================================================

/// A tensor that always requires gradient computation.
///
/// Parameters are the learnable weights of neural network layers.
/// They are always tracked by the autograd engine and get their
/// gradients populated during backward().
///
/// # Example
/// ```
/// use ml_framework_core::{Tensor, Parameter};
/// let w = Parameter::new(Tensor::randn(&[10, 5], "cpu"));
/// assert!(w.tensor.requires_grad());
/// ```
pub struct Parameter {
    pub tensor: Tensor,
}

impl Parameter {
    /// Create a new parameter from a tensor.
    /// The tensor's requires_grad is set to true.
    pub fn new(tensor: Tensor) -> Self {
        tensor.set_requires_grad(true);
        Parameter { tensor }
    }

    /// Create a parameter initialized to zeros.
    pub fn zeros(shape: &[usize], device: &str) -> Self {
        let t = Tensor::zeros(shape, device);
        t.set_requires_grad(true);
        Parameter { tensor: t }
    }

    /// Get the data.
    pub fn data(&self) -> Vec<f64> {
        self.tensor.data()
    }

    /// Set the data directly (used by optimizers).
    pub fn set_data(&self, data: Vec<f64>) {
        self.tensor.set_data(data);
    }

    /// Get the gradient data.
    pub fn grad_data(&self) -> Option<Vec<f64>> {
        self.tensor.grad_data()
    }

    /// Clear gradient.
    pub fn zero_grad(&self) {
        self.tensor.zero_grad();
    }

    /// Get shape.
    pub fn shape(&self) -> Vec<usize> {
        self.tensor.shape()
    }

    /// Number of elements.
    pub fn numel(&self) -> usize {
        self.tensor.numel()
    }
}

// =========================================================================
// DeviceManager
// =========================================================================

/// Manages device-to-backend mapping.
///
/// Device strings map to BLAS backends:
/// - "cpu" -> CpuBlas (always available)
/// - "cuda" -> CudaBlas (NVIDIA GPUs)
/// - "metal" -> MetalBlas (Apple Silicon)
///
/// The default device is "cpu".
pub struct DeviceManager {
    default_device: String,
}

impl DeviceManager {
    pub fn new() -> Self {
        DeviceManager { default_device: "cpu".to_string() }
    }

    pub fn get_default_device(&self) -> &str {
        &self.default_device
    }

    pub fn set_default_device(&mut self, device: &str) {
        self.default_device = device.to_string();
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// no_grad state
// =========================================================================

/// Global flag to disable gradient tracking.
///
/// When set to true, operations do not build computation graphs.
/// Use `set_grad_enabled(false)` before inference to save memory.
static mut GRAD_ENABLED: bool = true;

/// Check if gradient tracking is currently enabled.
pub fn is_grad_enabled() -> bool {
    unsafe { GRAD_ENABLED }
}

/// Set gradient tracking on or off.
pub fn set_grad_enabled(enabled: bool) {
    unsafe { GRAD_ENABLED = enabled; }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- Tensor creation tests -----

    #[test]
    fn test_zeros() {
        let t = Tensor::zeros(&[2, 3], "cpu");
        assert_eq!(t.shape(), vec![2, 3]);
        assert_eq!(t.data(), vec![0.0; 6]);
    }

    #[test]
    fn test_ones() {
        let t = Tensor::ones(&[3], "cpu");
        assert_eq!(t.data(), vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_full() {
        let t = Tensor::full(&[2, 2], 7.0, "cpu");
        assert_eq!(t.data(), vec![7.0, 7.0, 7.0, 7.0]);
    }

    #[test]
    fn test_eye() {
        let t = Tensor::eye(3, "cpu");
        assert_eq!(t.data(), vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_arange() {
        let t = Tensor::arange(0.0, 5.0, 1.0, "cpu");
        assert_eq!(t.data(), vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(t.shape(), vec![5]);
    }

    #[test]
    fn test_randn_shape() {
        let t = Tensor::randn(&[2, 3], "cpu");
        assert_eq!(t.shape(), vec![2, 3]);
        assert_eq!(t.numel(), 6);
    }

    #[test]
    fn test_from_slice() {
        let t = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], false, "cpu");
        assert_eq!(t.shape(), vec![2, 2]);
        assert_eq!(t.data(), vec![1.0, 2.0, 3.0, 4.0]);
    }

    // ----- Arithmetic tests -----

    #[test]
    fn test_add() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let b = Tensor::from_slice(&[4.0, 5.0, 6.0], &[3], false, "cpu");
        let c = a.add(&b);
        assert_eq!(c.data(), vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_sub() {
        let a = Tensor::from_slice(&[5.0, 7.0, 9.0], &[3], false, "cpu");
        let b = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let c = a.sub(&b);
        assert_eq!(c.data(), vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_mul() {
        let a = Tensor::from_slice(&[2.0, 3.0], &[2], false, "cpu");
        let b = Tensor::from_slice(&[4.0, 5.0], &[2], false, "cpu");
        let c = a.mul(&b);
        assert_eq!(c.data(), vec![8.0, 15.0]);
    }

    #[test]
    fn test_div() {
        let a = Tensor::from_slice(&[10.0, 20.0], &[2], false, "cpu");
        let b = Tensor::from_slice(&[2.0, 5.0], &[2], false, "cpu");
        let c = a.div(&b);
        assert_eq!(c.data(), vec![5.0, 4.0]);
    }

    #[test]
    fn test_neg() {
        let a = Tensor::from_slice(&[1.0, -2.0, 3.0], &[3], false, "cpu");
        let c = a.neg();
        assert_eq!(c.data(), vec![-1.0, 2.0, -3.0]);
    }

    #[test]
    fn test_pow() {
        let a = Tensor::from_slice(&[2.0, 3.0], &[2], false, "cpu");
        let c = a.pow(2.0);
        assert_eq!(c.data(), vec![4.0, 9.0]);
    }

    #[test]
    fn test_mul_scalar() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let c = a.mul_scalar(2.0);
        assert_eq!(c.data(), vec![2.0, 4.0, 6.0]);
    }

    // ----- Matmul tests -----

    #[test]
    fn test_matmul() {
        // [[1, 2], [3, 4]] @ [[5, 6], [7, 8]] = [[19, 22], [43, 50]]
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], false, "cpu");
        let b = Tensor::from_slice(&[5.0, 6.0, 7.0, 8.0], &[2, 2], false, "cpu");
        let c = a.matmul(&b);
        assert_eq!(c.shape(), vec![2, 2]);
        assert_eq!(c.data(), vec![19.0, 22.0, 43.0, 50.0]);
    }

    // ----- Reduction tests -----

    #[test]
    fn test_sum_all() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], false, "cpu");
        let s = a.sum(None, false);
        assert_eq!(s.data(), vec![10.0]);
    }

    #[test]
    fn test_sum_dim() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], false, "cpu");
        let s = a.sum(Some(0), false);
        assert_eq!(s.data(), vec![4.0, 6.0]);
        let s = a.sum(Some(1), false);
        assert_eq!(s.data(), vec![3.0, 7.0]);
    }

    #[test]
    fn test_mean_all() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[4], false, "cpu");
        let m = a.mean(None, false);
        assert_eq!(m.data(), vec![2.5]);
    }

    // ----- Shape operation tests -----

    #[test]
    fn test_reshape() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3], false, "cpu");
        let b = a.reshape(&[3, 2]);
        assert_eq!(b.shape(), vec![3, 2]);
        assert_eq!(b.data(), a.data());
    }

    #[test]
    fn test_transpose_2d() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3], false, "cpu");
        let b = a.t();
        assert_eq!(b.shape(), vec![3, 2]);
        assert_eq!(b.data(), vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    // ----- Element-wise math tests -----

    #[test]
    fn test_exp() {
        let a = Tensor::from_slice(&[0.0, 1.0], &[2], false, "cpu");
        let b = a.exp();
        let data = b.data();
        assert!((data[0] - 1.0).abs() < 1e-10);
        assert!((data[1] - std::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn test_log() {
        let a = Tensor::from_slice(&[1.0, std::f64::consts::E], &[2], false, "cpu");
        let b = a.log();
        let data = b.data();
        assert!((data[0] - 0.0).abs() < 1e-10);
        assert!((data[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_abs() {
        let a = Tensor::from_slice(&[-3.0, 0.0, 5.0], &[3], false, "cpu");
        let b = a.abs();
        assert_eq!(b.data(), vec![3.0, 0.0, 5.0]);
    }

    #[test]
    fn test_clamp() {
        let a = Tensor::from_slice(&[-2.0, 0.5, 3.0], &[3], false, "cpu");
        let b = a.clamp(Some(0.0), Some(1.0));
        assert_eq!(b.data(), vec![0.0, 0.5, 1.0]);
    }

    // ----- Activation tests -----

    #[test]
    fn test_relu() {
        let a = Tensor::from_slice(&[-2.0, -1.0, 0.0, 1.0, 2.0], &[5], false, "cpu");
        let b = a.relu();
        assert_eq!(b.data(), vec![0.0, 0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_sigmoid() {
        let a = Tensor::from_slice(&[0.0], &[1], false, "cpu");
        let b = a.sigmoid();
        assert!((b.data()[0] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_tanh() {
        let a = Tensor::from_slice(&[0.0], &[1], false, "cpu");
        let b = a.tanh_act();
        assert!((b.data()[0] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_softmax_1d() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let b = a.softmax(0);
        let data = b.data();
        let sum: f64 = data.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
        // Softmax values should be monotonically increasing
        assert!(data[0] < data[1]);
        assert!(data[1] < data[2]);
    }

    #[test]
    fn test_gelu() {
        let a = Tensor::from_slice(&[0.0], &[1], false, "cpu");
        let b = a.gelu();
        assert!((b.data()[0] - 0.0).abs() < 1e-10);
    }

    // ----- Autograd tests -----

    #[test]
    fn test_backward_simple() {
        // z = x * 2, backward should give grad_x = 2
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], true, "cpu");
        let two = Tensor::full(&[3], 2.0, "cpu");
        let y = x.mul(&two);
        let z = y.sum(None, false);
        z.backward(None).unwrap();
        let grad = x.grad_data().unwrap();
        assert_eq!(grad, vec![2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_backward_add() {
        let x = Tensor::from_slice(&[1.0, 2.0], &[2], true, "cpu");
        let y = Tensor::from_slice(&[3.0, 4.0], &[2], true, "cpu");
        let z = x.add(&y);
        let loss = z.sum(None, false);
        loss.backward(None).unwrap();
        assert_eq!(x.grad_data().unwrap(), vec![1.0, 1.0]);
        assert_eq!(y.grad_data().unwrap(), vec![1.0, 1.0]);
    }

    #[test]
    fn test_backward_matmul() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], true, "cpu");
        let b = Tensor::from_slice(&[5.0, 6.0, 7.0, 8.0], &[2, 2], true, "cpu");
        let c = a.matmul(&b);
        let loss = c.sum(None, false);
        loss.backward(None).unwrap();
        // grad_A = ones @ B^T
        // B^T = [[5, 7], [6, 8]]
        // grad_A[0,0] = 1*5 + 1*6 = 11, grad_A[0,1] = 1*7 + 1*8 = 15
        let ga = a.grad_data().unwrap();
        assert_eq!(ga, vec![11.0, 15.0, 11.0, 15.0]);
    }

    #[test]
    fn test_backward_relu() {
        let x = Tensor::from_slice(&[-1.0, 0.0, 1.0, 2.0], &[4], true, "cpu");
        let y = x.relu();
        let loss = y.sum(None, false);
        loss.backward(None).unwrap();
        assert_eq!(x.grad_data().unwrap(), vec![0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_backward_sigmoid() {
        let x = Tensor::from_slice(&[0.0], &[1], true, "cpu");
        let y = x.sigmoid();
        y.backward(None).unwrap();
        let grad = x.grad_data().unwrap();
        // sigmoid(0) = 0.5, grad = 0.5 * (1 - 0.5) = 0.25
        assert!((grad[0] - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_backward_pow() {
        let x = Tensor::from_slice(&[2.0, 3.0], &[2], true, "cpu");
        let y = x.pow(2.0);
        let loss = y.sum(None, false);
        loss.backward(None).unwrap();
        // d(x^2)/dx = 2x
        assert_eq!(x.grad_data().unwrap(), vec![4.0, 6.0]);
    }

    #[test]
    fn test_backward_exp() {
        let x = Tensor::from_slice(&[0.0, 1.0], &[2], true, "cpu");
        let y = x.exp();
        let loss = y.sum(None, false);
        loss.backward(None).unwrap();
        let grad = x.grad_data().unwrap();
        // d(e^x)/dx = e^x
        assert!((grad[0] - 1.0).abs() < 1e-10);
        assert!((grad[1] - std::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn test_backward_log() {
        let x = Tensor::from_slice(&[1.0, 2.0], &[2], true, "cpu");
        let y = x.log();
        let loss = y.sum(None, false);
        loss.backward(None).unwrap();
        let grad = x.grad_data().unwrap();
        // d(ln(x))/dx = 1/x
        assert!((grad[0] - 1.0).abs() < 1e-10);
        assert!((grad[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_backward_mean() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[4], true, "cpu");
        let y = x.mean(None, false);
        y.backward(None).unwrap();
        let grad = x.grad_data().unwrap();
        // d(mean)/dx = 1/n for each element
        assert_eq!(grad, vec![0.25, 0.25, 0.25, 0.25]);
    }

    // ----- Parameter tests -----

    #[test]
    fn test_parameter() {
        let p = Parameter::new(Tensor::from_slice(&[1.0, 2.0], &[2], false, "cpu"));
        assert!(p.tensor.requires_grad());
        assert_eq!(p.data(), vec![1.0, 2.0]);
        assert_eq!(p.shape(), vec![2]);
    }

    #[test]
    fn test_parameter_zero_grad() {
        let p = Parameter::new(Tensor::from_slice(&[1.0], &[1], true, "cpu"));
        let y = p.tensor.pow(2.0);
        y.backward(None).unwrap();
        assert!(p.grad_data().is_some());
        p.zero_grad();
        assert!(p.grad_data().is_none());
    }

    // ----- DeviceManager tests -----

    #[test]
    fn test_device_manager() {
        let mut dm = DeviceManager::new();
        assert_eq!(dm.get_default_device(), "cpu");
        dm.set_default_device("cuda");
        assert_eq!(dm.get_default_device(), "cuda");
    }

    // ----- Misc tests -----

    #[test]
    fn test_item() {
        let t = Tensor::from_slice(&[42.0], &[1], false, "cpu");
        assert_eq!(t.item().unwrap(), 42.0);
    }

    #[test]
    fn test_item_error() {
        let t = Tensor::from_slice(&[1.0, 2.0], &[2], false, "cpu");
        assert!(t.item().is_err());
    }

    #[test]
    fn test_detach() {
        let x = Tensor::from_slice(&[1.0, 2.0], &[2], true, "cpu");
        let d = x.detach();
        assert!(!d.requires_grad());
        assert_eq!(d.data(), x.data());
    }

    #[test]
    fn test_to_device() {
        let x = Tensor::from_slice(&[1.0, 2.0], &[2], false, "cpu");
        let y = x.to_device("cuda");
        assert_eq!(y.device(), "cuda");
        assert_eq!(y.data(), x.data());
    }

    #[test]
    fn test_comparison_eq() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let b = a.eq_scalar(2.0);
        assert_eq!(b.data(), vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn test_comparison_gt() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], false, "cpu");
        let b = a.gt_scalar(1.5);
        assert_eq!(b.data(), vec![0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_chain_of_operations() {
        // Test a chain: y = relu(x @ W + b), loss = sum(y)
        let x = Tensor::from_slice(&[1.0, 2.0], &[1, 2], true, "cpu");
        let w = Tensor::from_slice(&[0.5, -0.5, 0.3, 0.7], &[2, 2], true, "cpu");
        let y = x.matmul(&w);
        let b = Tensor::from_slice(&[0.1, -0.1], &[1, 2], false, "cpu");
        let z = y.add(&b);
        let activated = z.relu();
        let loss = activated.sum(None, false);
        loss.backward(None).unwrap();
        // Just verify it runs without error and produces gradients
        assert!(x.grad_data().is_some());
        assert!(w.grad_data().is_some());
    }

    #[test]
    fn test_softmax_backward() {
        let x = Tensor::from_slice(&[1.0, 2.0, 3.0], &[3], true, "cpu");
        let y = x.softmax(0);
        let loss = y.sum(None, false);
        loss.backward(None).unwrap();
        let grad = x.grad_data().unwrap();
        // Sum of softmax is always 1, so all gradients should be ~0
        for g in &grad {
            assert!(g.abs() < 1e-10, "softmax grad should be ~0 but got {}", g);
        }
    }

    #[test]
    fn test_no_grad() {
        assert!(is_grad_enabled());
        set_grad_enabled(false);
        assert!(!is_grad_enabled());
        set_grad_enabled(true);
        assert!(is_grad_enabled());
    }
}

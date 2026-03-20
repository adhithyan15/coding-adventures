//! BLAS Data Types -- Matrix, Vector, and enumeration types.
//!
//! # What Lives Here
//!
//! This module defines the core data types used throughout the BLAS library:
//!
//! 1. [`StorageOrder`]  -- how matrix elements are laid out in memory
//! 2. [`Transpose`]     -- whether to logically transpose a matrix
//! 3. [`Side`]          -- which side the special matrix is on (for SYMM)
//! 4. [`Vector`]        -- a 1-D array of floats
//! 5. [`Matrix`]        -- a 2-D array of floats stored as a flat Vec
//!
//! # Why Flat Storage?
//!
//! GPUs need contiguous memory. A `Vec<Vec<f32>>` (nested vectors) has each
//! row allocated separately in memory. A flat `Vec<f32>` is one contiguous
//! block -- when we upload it to GPU memory, it's a single memcpy.
//!
//! ```text
//! Nested (like Vec<Vec<f32>>):
//!     data = [[1, 2, 3],
//!             [4, 5, 6]]
//!     // Each inner Vec is a separate heap allocation
//!
//! Flat (BLAS library):
//!     data = [1, 2, 3, 4, 5, 6]
//!     // One contiguous allocation. A[i][j] = data[i * cols + j]
//! ```

// =========================================================================
// Enumerations -- small types that control BLAS operation behavior
// =========================================================================

/// How matrix elements are laid out in memory.
///
/// # How Matrices Are Stored in Memory
///
/// A 2x3 matrix:
/// ```text
///     [ 1  2  3 ]
///     [ 4  5  6 ]
///
/// Row-major (C convention):    [1, 2, 3, 4, 5, 6]
///     A[i][j] = data[i * cols + j]
///
/// Column-major (Fortran/BLAS): [1, 4, 2, 5, 3, 6]
///     A[i][j] = data[j * rows + i]
/// ```
///
/// We default to row-major because Rust, C, and most ML frameworks use
/// row-major. Traditional BLAS uses column-major (Fortran heritage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageOrder {
    RowMajor,
    ColumnMajor,
}

impl Default for StorageOrder {
    fn default() -> Self {
        StorageOrder::RowMajor
    }
}

/// Transpose flags for GEMM and GEMV.
///
/// # Transpose Flags for GEMM and GEMV
///
/// When computing C = alpha * A * B + beta * C, you often want to use A^T
/// or B^T without physically transposing the matrix. The Transpose flag
/// tells the backend to "pretend" the matrix is transposed.
///
/// This is a classic BLAS optimization: instead of allocating a new matrix
/// and copying transposed data, you just change the access pattern. For a
/// row-major matrix with shape (M, N):
///
/// - `NoTrans`: access as (M, N), stride = N
/// - `Trans`:   access as (N, M), stride = M
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transpose {
    NoTrans,
    Trans,
}

/// Which side the special matrix is on (for SYMM, TRMM).
///
/// # Which Side the Special Matrix is On
///
/// SYMM computes C = alpha * A * B + beta * C where A is symmetric.
///
/// - `Left`:  A is on the left  -> C = alpha * (A) * B + beta * C
/// - `Right`: A is on the right -> C = alpha * B * (A) + beta * C
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

// =========================================================================
// Vector -- a 1-D array of single-precision floats
// =========================================================================

/// A 1-D array of single-precision floats.
///
/// # A 1-D Array of Single-Precision Floats
///
/// This is the simplest possible vector type. It holds:
/// - `data`: a flat Vec of f32 values
/// - `size`: how many elements
///
/// It is NOT a tensor. It is NOT a GPU buffer. It lives on the host (CPU).
/// Each backend copies it to the device when needed and copies results back.
/// This keeps the interface dead simple.
///
/// # Example
///
/// ```
/// use blas_library::Vector;
/// let v = Vector::new(vec![1.0, 2.0, 3.0]);
/// assert_eq!(v.size(), 3);
/// assert_eq!(v.data()[0], 1.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Vector {
    data: Vec<f32>,
}

impl Vector {
    /// Create a new Vector from a Vec of f32 values.
    ///
    /// The size is automatically determined from the data length.
    pub fn new(data: Vec<f32>) -> Self {
        Self { data }
    }

    /// Create a zero vector of the given size.
    pub fn zeros(size: usize) -> Self {
        Self {
            data: vec![0.0; size],
        }
    }

    /// Get a reference to the underlying data.
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get the number of elements.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

// =========================================================================
// Matrix -- a 2-D array of single-precision floats (flat storage)
// =========================================================================

/// A 2-D array of single-precision floats stored as a flat Vec.
///
/// # A 2-D Array of Single-Precision Floats
///
/// Stored as a flat Vec in row-major order by default:
///
/// ```text
/// Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3)
///
/// represents:  [ 1  2  3 ]
///              [ 4  5  6 ]
///
/// data[i * cols + j] = element at row i, column j
/// ```
///
/// The Matrix type is deliberately simple -- it's a container for moving
/// data between the caller and the BLAS backend. The backend handles
/// device memory management internally.
#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    data: Vec<f32>,
    rows: usize,
    cols: usize,
    order: StorageOrder,
}

impl Matrix {
    /// Create a new Matrix with validation.
    ///
    /// # Panics
    ///
    /// Panics if `data.len() != rows * cols`.
    pub fn new(data: Vec<f32>, rows: usize, cols: usize) -> Self {
        assert_eq!(
            data.len(),
            rows * cols,
            "Matrix data has {} elements but shape is {}x{} = {}",
            data.len(),
            rows,
            cols,
            rows * cols,
        );
        Self {
            data,
            rows,
            cols,
            order: StorageOrder::RowMajor,
        }
    }

    /// Create a new Matrix with a specified storage order.
    pub fn with_order(data: Vec<f32>, rows: usize, cols: usize, order: StorageOrder) -> Self {
        assert_eq!(
            data.len(),
            rows * cols,
            "Matrix data has {} elements but shape is {}x{} = {}",
            data.len(),
            rows,
            cols,
            rows * cols,
        );
        Self {
            data,
            rows,
            cols,
            order,
        }
    }

    /// Create a zero matrix of the given dimensions.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![0.0; rows * cols],
            rows,
            cols,
            order: StorageOrder::RowMajor,
        }
    }

    /// Get a reference to the underlying flat data.
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get the number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get the number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get the storage order.
    pub fn order(&self) -> StorageOrder {
        self.order
    }
}

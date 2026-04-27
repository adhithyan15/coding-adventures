//! # cas-matrix
//!
//! Matrix operations over symbolic IR — the Rust port of the Python
//! `cas-matrix` package.
//!
//! Matrices are represented as:
//! ```text
//! Apply(Symbol("Matrix"),
//!   [ Apply(Symbol("List"), [cell, cell, ...]),   ← row 0
//!     Apply(Symbol("List"), [cell, cell, ...]),   ← row 1
//!     ... ])
//! ```
//!
//! where every `cell` is an arbitrary `IRNode`.  Arithmetic operations produce
//! un-simplified IR: each entry is an `IRApply(ADD/SUB/MUL/DIV/NEG, …)`.
//! Pass results through `cas_simplify::simplify` to reduce numeric entries.
//!
//! ## Quick start
//!
//! ```rust
//! use cas_matrix::{matrix, transpose, determinant};
//! use symbolic_ir::{int, IRNode};
//!
//! // 2×2 matrix [[1, 2], [3, 4]]
//! let m = matrix(vec![
//!     vec![int(1), int(2)],
//!     vec![int(3), int(4)],
//! ]).unwrap();
//!
//! let t = transpose(&m).unwrap();
//! // t == [[1, 3], [2, 4]]
//!
//! let d = determinant(&m).unwrap();
//! // d = Sub(Mul(1, 4), Mul(2, 3))  (un-simplified; run through simplify to get -2)
//! ```
//!
//! ## Determinant and inverse
//!
//! Both use cofactor expansion, which is O(n!).  Practical up to about n=6.
//!
//! ## Stack position
//!
//! ```text
//! symbolic-ir  ←  cas-matrix
//! ```

pub mod arithmetic;
pub mod determinant;
pub mod matrix;

pub use arithmetic::{
    add_matrices, dot, identity_matrix, scalar_multiply, sub_matrices, trace, transpose,
    zero_matrix,
};
pub use determinant::{determinant, inverse};
pub use matrix::{
    dimensions, get_entry, is_matrix, matrix, num_cols, num_rows, rows_of, MatrixError,
    MatrixResult, MATRIX,
};

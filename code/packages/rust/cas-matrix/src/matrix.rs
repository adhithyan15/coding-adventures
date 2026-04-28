//! Matrix construction, shape inspection, and accessor functions.
//!
//! ## Representation
//!
//! A matrix is stored as an `IRNode` of the form:
//! ```text
//! Apply(Symbol("Matrix"),
//!   [ Apply(Symbol("List"), [cell₀₀, cell₀₁, …]),   ← row 0
//!     Apply(Symbol("List"), [cell₁₀, cell₁₁, …]),   ← row 1
//!     … ])
//! ```
//! Each row is a `List(...)` node and each cell is an arbitrary `IRNode`.
//!
//! ## Invariants
//!
//! - Every row has the same length (width).
//! - At least one row exists (empty matrices require passing an empty `Vec`
//!   but that is rejected by `matrix()`).
//!
//! ## Indexing
//!
//! All public accessors use 1-based (row, col) indexing to match the MACSYMA
//! / Mathematica convention.

use std::fmt;

use symbolic_ir::{apply, int, sym, IRNode, LIST};

/// Head name for the `Matrix(...)` IR node.
///
/// Stored in a `&'static str` so it can be used as a HashMap key or compared
/// directly against symbol names.
pub const MATRIX: &str = "Matrix";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error raised on shape mismatch, malformed matrix, or out-of-range access.
///
/// Mirrors Python's `MatrixError(ValueError)`.
#[derive(Debug, Clone, PartialEq)]
pub struct MatrixError(pub String);

impl fmt::Display for MatrixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MatrixError: {}", self.0)
    }
}

impl std::error::Error for MatrixError {}

/// Shorthand result type for matrix operations.
pub type MatrixResult<T> = Result<T, MatrixError>;

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Build a `Matrix(List(row₀), List(row₁), …)` IR node.
///
/// Every row must have the same length.  At least one row is required.
///
/// ```rust
/// use cas_matrix::matrix;
/// use symbolic_ir::int;
///
/// let m = matrix(vec![
///     vec![int(1), int(2)],
///     vec![int(3), int(4)],
/// ]).unwrap();
///
/// assert!(cas_matrix::is_matrix(&m));
/// ```
pub fn matrix(rows: Vec<Vec<IRNode>>) -> MatrixResult<IRNode> {
    if rows.is_empty() {
        return Err(MatrixError("matrix() requires at least one row".into()));
    }
    let width = rows[0].len();
    for (i, row) in rows.iter().enumerate() {
        if row.len() != width {
            return Err(MatrixError(format!(
                "matrix row {i} has {} entries, expected {width}",
                row.len()
            )));
        }
    }
    let ir_rows: Vec<IRNode> = rows
        .into_iter()
        .map(|row| apply(sym(LIST), row))
        .collect();
    Ok(apply(sym(MATRIX), ir_rows))
}

// ---------------------------------------------------------------------------
// Structural helpers
// ---------------------------------------------------------------------------

/// Returns `true` if `node` is a `Matrix(...)` IR application.
///
/// ```rust
/// use cas_matrix::{matrix, is_matrix};
/// use symbolic_ir::{int, sym};
///
/// let m = matrix(vec![vec![int(1)]]).unwrap();
/// assert!(is_matrix(&m));
/// assert!(!is_matrix(&int(5)));
/// assert!(!is_matrix(&sym("x")));
/// ```
pub fn is_matrix(node: &IRNode) -> bool {
    if let IRNode::Apply(a) = node {
        if let IRNode::Symbol(s) = &a.head {
            return s == MATRIX;
        }
    }
    false
}

/// Extract the rows of a `Matrix` as a `Vec<Vec<IRNode>>` (cloned).
///
/// Returns an error if `m` is not a `Matrix`, or if any row is not a `List`.
///
/// This is the primary internal helper used by all arithmetic and determinant
/// functions.
pub fn rows_of(m: &IRNode) -> MatrixResult<Vec<Vec<IRNode>>> {
    if !is_matrix(m) {
        return Err(MatrixError(format!("expected a Matrix, got {m:?}")));
    }
    let IRNode::Apply(outer) = m else {
        unreachable!()
    };
    outer.args.iter().map(row_args).collect()
}

/// Extract the cells from a single `List(...)` row node.
fn row_args(row: &IRNode) -> MatrixResult<Vec<IRNode>> {
    if let IRNode::Apply(a) = row {
        if a.head == sym(LIST) {
            return Ok(a.args.clone());
        }
    }
    Err(MatrixError(format!(
        "matrix row must be a List, got {row:?}"
    )))
}

// ---------------------------------------------------------------------------
// Shape
// ---------------------------------------------------------------------------

/// Return `List(IRInteger(nrows), IRInteger(ncols))`.
///
/// ```rust
/// use cas_matrix::{matrix, dimensions};
/// use symbolic_ir::{apply, int, sym, LIST};
///
/// let m = matrix(vec![vec![int(1), int(2)], vec![int(3), int(4)]]).unwrap();
/// assert_eq!(dimensions(&m).unwrap(), apply(sym(LIST), vec![int(2), int(2)]));
/// ```
pub fn dimensions(m: &IRNode) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |r| r.len());
    Ok(apply(sym(LIST), vec![int(nrows as i64), int(ncols as i64)]))
}

/// Number of rows.
///
/// ```rust
/// use cas_matrix::{matrix, num_rows};
/// use symbolic_ir::int;
///
/// let m = matrix(vec![vec![int(1)], vec![int(2)], vec![int(3)]]).unwrap();
/// assert_eq!(num_rows(&m).unwrap(), 3);
/// ```
pub fn num_rows(m: &IRNode) -> MatrixResult<usize> {
    Ok(rows_of(m)?.len())
}

/// Number of columns.
///
/// ```rust
/// use cas_matrix::{matrix, num_cols};
/// use symbolic_ir::int;
///
/// let m = matrix(vec![vec![int(1), int(2), int(3)]]).unwrap();
/// assert_eq!(num_cols(&m).unwrap(), 3);
/// ```
pub fn num_cols(m: &IRNode) -> MatrixResult<usize> {
    let rows = rows_of(m)?;
    Ok(rows.first().map_or(0, |r| r.len()))
}

/// 1-based (row, col) element access.
///
/// ```rust
/// use cas_matrix::{matrix, get_entry};
/// use symbolic_ir::int;
///
/// let m = matrix(vec![
///     vec![int(1), int(2)],
///     vec![int(3), int(4)],
/// ]).unwrap();
/// assert_eq!(get_entry(&m, 2, 1).unwrap(), int(3));
/// ```
pub fn get_entry(m: &IRNode, row: usize, col: usize) -> MatrixResult<IRNode> {
    let rows = rows_of(m)?;
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |r| r.len());
    if row < 1 || row > nrows || col < 1 || col > ncols {
        return Err(MatrixError(format!(
            "index ({row}, {col}) out of range for {nrows}×{ncols} matrix"
        )));
    }
    Ok(rows[row - 1][col - 1].clone())
}

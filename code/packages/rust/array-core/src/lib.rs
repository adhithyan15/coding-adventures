//! Array Core
//!
//! Phase 1 implementation of `code/specs/backend-crate-catalog.md`'s
//! `array-core` row — Excel 365 dynamic-array helpers usable from any
//! spreadsheet frontend (VisiCalc faithful, modern reconstruction, or a
//! Numbers/Sheets-style host).
//!
//! # Design overview
//!
//! ## Why a dedicated `Array2D<T>`?
//!
//! Excel-365 dynamic-array functions are inherently 2-D: `SEQUENCE(3, 4)` is a
//! 3-row, 4-column block, `HSTACK` glues blocks side by side, `SORT` reorders
//! rows or columns. `r-vector::Double` only models a flat 1-D atomic vector
//! plus an optional `dim` attribute. Threading a 2-D abstraction through
//! callers of every function in this crate via that attribute would obscure
//! intent and duplicate validation in every call site. So we wrap the storage
//! in a small in-crate type:
//!
//! ```text
//!   Array2D { rows, cols, data }   // row-major Vec<T>
//! ```
//!
//! `data` is row-major: `data[r * cols + c]` is the cell at `(r, c)`. This is
//! the same convention as `r-vector::Double`'s eventual matrix view and what
//! most C/Rust callers expect. (R's column-major convention is a presentation
//! detail; we convert at the language-binding edge, not here.)
//!
//! For numeric arrays we use `Array2D<f64>` and reuse `r-vector::na_real()`'s
//! NA bit-pattern, so missing values flow through arithmetic and through every
//! function in this crate without an additional `Option` layer. `Array2D<T>`
//! is generic so future Phase-2 work (mixed text + numeric arrays) can layer
//! on without rewriting the storage.
//!
//! ## 1-based indexing at the public surface
//!
//! Excel cell references and array-function arguments are 1-based. To keep
//! the API readable from a spreadsheet engineer's perspective, all
//! user-facing indices (`CHOOSEROWS`, `CHOOSECOLS`, `SORT`'s `sort_index`)
//! are 1-based. Negative values count from the end, matching Excel:
//! `CHOOSEROWS(arr, -1)` is the last row. We convert to 0-based right at the
//! function entry; internal helpers stay 0-based.
//!
//! ## NA propagation rules (per-function notes are inline in each module)
//!
//! * SORT keeps NA values — they sort to the end in ascending order (matching
//!   Excel's behavior of placing errors at the end).
//! * UNIQUE treats two NA positions as equal — duplicates of NA collapse.
//! * FILTER's boolean mask: NA in the mask is treated as `FALSE` (do not
//!   include). This matches Excel where `=FILTER(A:A, B:B)` with an empty
//!   cell in `B` excludes the corresponding row.
//! * EXPAND / HSTACK / VSTACK / WRAPROWS / WRAPCOLS pad with NA (representing
//!   Excel's `#N/A` error sentinel for "no value here") when shapes differ.

pub mod filter;
pub mod generate;
pub mod pick;
pub mod reshape;
pub mod shape;
pub mod sort;
pub mod stack;
pub mod unique;

pub use r_vector::{is_na_real, na_real};

/// Errors returned by `array-core` operations.
///
/// We deliberately keep this small and serializable-friendly (no boxed
/// `dyn Error`). Each variant captures enough context for a frontend to
/// translate into Excel error sentinels (`#VALUE!`, `#CALC!`, `#REF!`).
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayError {
    /// Two arrays whose shapes did not match for the requested operation.
    ShapeMismatch {
        expected: String,
        found: String,
    },
    /// A function produced no rows / cols (FILTER with everything excluded
    /// and no `if_empty`, for example). Maps to Excel `#CALC!`.
    EmptyResult {
        function: &'static str,
    },
    /// A caller-supplied parameter was outside its allowed range.
    BadParameter {
        name: &'static str,
        value: String,
    },
    /// A 1-based index (possibly negative) resolved outside the array bounds.
    /// `index` is the original 1-based value; `max` is the maximum absolute
    /// 1-based value that would have been valid.
    OutOfRange {
        function: &'static str,
        index: i64,
        max: usize,
    },
}

impl std::fmt::Display for ArrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrayError::ShapeMismatch { expected, found } => {
                write!(f, "shape mismatch: expected {expected}, found {found}")
            }
            ArrayError::EmptyResult { function } => {
                write!(f, "{function}: empty result")
            }
            ArrayError::BadParameter { name, value } => {
                write!(f, "bad parameter {name}={value}")
            }
            ArrayError::OutOfRange {
                function,
                index,
                max,
            } => write!(
                f,
                "{function}: index {index} out of range (|index| must be <= {max})"
            ),
        }
    }
}

impl std::error::Error for ArrayError {}

/// 2-D row-major array used by every function in this crate.
///
/// # Invariants
///
/// * `data.len() == rows * cols`
/// * If `rows == 0` or `cols == 0`, the array is considered empty and most
///   functions short-circuit (returning empty, propagating NA, or erroring as
///   the underlying Excel function does).
///
/// # Example
/// ```text
///   Array2D::new(2, 3, vec![1.0, 2.0, 3.0,    // row 0
///                            4.0, 5.0, 6.0])  // row 1
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Array2D<T: Clone> {
    pub rows: usize,
    pub cols: usize,
    /// Row-major storage. `data[r * cols + c]` is the cell at `(r, c)`.
    pub data: Vec<T>,
}

impl<T: Clone> Array2D<T> {
    /// Construct from explicit dimensions and a row-major buffer.
    ///
    /// Returns `ShapeMismatch` if `data.len() != rows * cols`. We accept the
    /// `0 x 0` empty array (with an empty `data` vec) since several functions
    /// can legitimately produce an empty result (`DROP` past the end, for
    /// example).
    pub fn new(rows: usize, cols: usize, data: Vec<T>) -> Result<Self, ArrayError> {
        if data.len() != rows.saturating_mul(cols) {
            return Err(ArrayError::ShapeMismatch {
                expected: format!("{} elements ({rows}x{cols})", rows.saturating_mul(cols)),
                found: format!("{} elements", data.len()),
            });
        }
        Ok(Self { rows, cols, data })
    }

    /// Wrap a 1-D vector as a single-column 2-D array `(n, 1)`.
    ///
    /// This is the canonical "lift" from `r-vector` shape to `Array2D` for
    /// callers that receive a `Double` and want to pipe it through an
    /// `array-core` helper. Length-zero produces a `(0, 1)` array (still
    /// width 1; Excel treats a single-column empty range as 1 column wide).
    pub fn from_vector(data: Vec<T>) -> Self {
        let rows = data.len();
        Self {
            rows,
            cols: 1,
            data,
        }
    }

    /// Construct an `Array2D` filled by repeating `value`.
    pub fn filled(rows: usize, cols: usize, value: T) -> Self {
        Self {
            rows,
            cols,
            data: vec![value; rows * cols],
        }
    }

    /// Borrow a cell by `(row, col)` (0-based). Panics if out of bounds — the
    /// public-facing functions in this crate are responsible for bounds
    /// checking before calling `get`.
    pub fn get(&self, r: usize, c: usize) -> &T {
        debug_assert!(r < self.rows && c < self.cols, "Array2D::get out of bounds");
        &self.data[r * self.cols + c]
    }

    /// Set a cell. Intended only for in-place builders inside this crate.
    pub fn set(&mut self, r: usize, c: usize, value: T) {
        debug_assert!(r < self.rows && c < self.cols, "Array2D::set out of bounds");
        self.data[r * self.cols + c] = value;
    }

    /// Materialize a row as a fresh `Vec<T>`.
    pub fn row(&self, r: usize) -> Vec<T> {
        debug_assert!(r < self.rows, "Array2D::row out of bounds");
        let start = r * self.cols;
        self.data[start..start + self.cols].to_vec()
    }

    /// Materialize a column as a fresh `Vec<T>`. (Row-major storage makes
    /// this an O(rows) gather, which is fine for v1; callers operating on
    /// many cols can use `data` directly.)
    pub fn col(&self, c: usize) -> Vec<T> {
        debug_assert!(c < self.cols, "Array2D::col out of bounds");
        (0..self.rows).map(|r| self.data[r * self.cols + c].clone()).collect()
    }

    /// `true` iff the array has zero cells.
    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.cols == 0
    }
}

impl Array2D<f64> {
    /// `true` iff the cell holds the canonical NA bit pattern from
    /// `r-vector`. (This is a no-cost wrapper that exists so callers do not
    /// have to import `r_vector::is_na_real` for typical scans.)
    pub fn is_na(&self, r: usize, c: usize) -> bool {
        is_na_real(*self.get(r, c))
    }
}

/// Resolve a 1-based, possibly-negative Excel-style index against a length.
///
/// Returns the 0-based offset, or `ArrayError::OutOfRange` if `index` is
/// `0` (Excel rejects zero indices) or `|index| > max`.
///
/// # Examples
/// * `resolve_index("CHOOSEROWS",  1, 5)` -> Ok(0) — first item.
/// * `resolve_index("CHOOSEROWS", -1, 5)` -> Ok(4) — last item.
/// * `resolve_index("CHOOSEROWS",  0, 5)` -> Err(OutOfRange)
/// * `resolve_index("CHOOSEROWS",  6, 5)` -> Err(OutOfRange)
pub(crate) fn resolve_index(
    function: &'static str,
    index: i64,
    max: usize,
) -> Result<usize, ArrayError> {
    if index == 0 || (index.unsigned_abs() as usize) > max {
        return Err(ArrayError::OutOfRange {
            function,
            index,
            max,
        });
    }
    if index > 0 {
        Ok((index - 1) as usize)
    } else {
        // Negative: -1 means last, -2 second-to-last, etc.
        Ok(max - (index.unsigned_abs() as usize))
    }
}

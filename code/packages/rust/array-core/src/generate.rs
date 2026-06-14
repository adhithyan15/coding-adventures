//! Array generators.
//!
//! Phase 1 covers SEQUENCE. RANDARRAY is intentionally deferred — it requires
//! a pluggable RNG, which the catalog routes through a separate crate; see
//! `code/specs/backend-crate-catalog.md`.

use crate::{Array2D, ArrayError};

/// `SEQUENCE(rows, [cols], [start], [step])` — Excel 365.
///
/// Produces a `(rows, cols)` array filled row-major from `start`, stepping
/// by `step`. Defaults match Excel: `cols = 1`, `start = 1`, `step = 1`.
///
/// # Truth table for tiny inputs
///
/// | Call                              | Result                            |
/// |-----------------------------------|-----------------------------------|
/// | `sequence(3, None, None, None)`   | `[[1], [2], [3]]`                 |
/// | `sequence(2, Some(3), None, None)`| `[[1, 2, 3], [4, 5, 6]]`          |
/// | `sequence(3, None, Some(0.), Some(0.5))` | `[[0.0], [0.5], [1.0]]`    |
/// | `sequence(3, None, Some(5.), Some(-1.))` | `[[5.0], [4.0], [3.0]]`    |
///
/// # Errors
///
/// `rows == 0` returns `BadParameter("rows", "0")` — Excel reports `#VALUE!`
/// for non-positive row counts. Same for `cols == 0`.
pub fn sequence(
    rows: usize,
    cols: Option<usize>,
    start: Option<f64>,
    step: Option<f64>,
) -> Result<Array2D<f64>, ArrayError> {
    if rows == 0 {
        return Err(ArrayError::BadParameter {
            name: "rows",
            value: "0".into(),
        });
    }
    let cols = cols.unwrap_or(1);
    if cols == 0 {
        return Err(ArrayError::BadParameter {
            name: "cols",
            value: "0".into(),
        });
    }
    let start = start.unwrap_or(1.0);
    let step = step.unwrap_or(1.0);

    let n = rows * cols;
    let mut data = Vec::with_capacity(n);
    // We compute `start + i * step` (rather than accumulating) to avoid
    // accumulating floating-point error across long sequences.
    for i in 0..n {
        data.push(start + (i as f64) * step);
    }
    Array2D::new(rows, cols, data)
}

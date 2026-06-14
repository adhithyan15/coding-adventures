//! ROW / COLUMN / ROWS / COLUMNS — dimension introspection.
//!
//! # Simplification vs Excel
//!
//! In Excel, `ROW(A3)` returns the absolute worksheet row number (`3`).
//! That requires a workbook reference type, which we don't have at Layer 1.
//! Instead, given an *array shape*, `row`/`column` return the **1-based
//! index sequence within that shape**:
//!
//! - `row(Vector(n))` → `[1, 2, ..., n]`
//! - `column(Vector(n))` → `[1]`   (a column-vector has one column)
//! - `row(Matrix(r×c))` → `[1, 2, ..., r]`
//! - `column(Matrix(r×c))` → `[1, 2, ..., c]`
//!
//! `ROWS` / `COLUMNS` return scalar counts.
//!
//! When `spreadsheet-core` introduces real references, a thin adapter can
//! pick which API to call.

/// Shape passed to the position helpers — either a 1-D vector or a 2-D
/// row-major matrix.  This lets a caller introspect either kind of array
/// without forcing them to inflate a vector into a 1×n matrix.
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Vector(usize),
    Matrix { rows: usize, cols: usize },
}

/// `ROW(array)`.  See module docs for the simplified semantics.
pub fn row(shape: Shape) -> Vec<i64> {
    match shape {
        Shape::Vector(n) => (1..=(n as i64)).collect(),
        Shape::Matrix { rows, .. } => (1..=(rows as i64)).collect(),
    }
}

/// `COLUMN(array)`.
pub fn column(shape: Shape) -> Vec<i64> {
    match shape {
        Shape::Vector(_) => vec![1],
        Shape::Matrix { cols, .. } => (1..=(cols as i64)).collect(),
    }
}

/// `ROWS(array)`.
pub fn rows(shape: Shape) -> usize {
    match shape {
        Shape::Vector(n) => n,
        Shape::Matrix { rows, .. } => rows,
    }
}

/// `COLUMNS(array)`.
pub fn columns(shape: Shape) -> usize {
    match shape {
        Shape::Vector(_) => 1,
        Shape::Matrix { cols, .. } => cols,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_on_vector_is_one_to_n() {
        assert_eq!(row(Shape::Vector(4)), vec![1, 2, 3, 4]);
    }

    #[test]
    fn column_on_vector_is_just_one() {
        assert_eq!(column(Shape::Vector(4)), vec![1]);
    }

    #[test]
    fn row_on_matrix_uses_row_count() {
        assert_eq!(
            row(Shape::Matrix { rows: 3, cols: 5 }),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn column_on_matrix_uses_col_count() {
        assert_eq!(
            column(Shape::Matrix { rows: 3, cols: 5 }),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn rows_columns_scalars() {
        assert_eq!(rows(Shape::Vector(7)), 7);
        assert_eq!(columns(Shape::Vector(7)), 1);
        assert_eq!(rows(Shape::Matrix { rows: 3, cols: 5 }), 3);
        assert_eq!(columns(Shape::Matrix { rows: 3, cols: 5 }), 5);
    }

    #[test]
    fn rows_columns_on_empty_vector() {
        assert_eq!(rows(Shape::Vector(0)), 0);
        assert_eq!(columns(Shape::Vector(0)), 1);
        assert_eq!(row(Shape::Vector(0)), Vec::<i64>::new());
    }
}

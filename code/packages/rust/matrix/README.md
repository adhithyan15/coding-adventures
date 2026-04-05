# Matrix (Rust)

A pure Rust 2D matrix library with no external dependencies. Part of the coding-adventures monorepo.

## Usage

```rust
use matrix::Matrix;

// Construction
let m = Matrix::new_2d(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
let z = Matrix::zeros(3, 3);
let i = Matrix::identity(3);
let d = Matrix::from_diagonal(&[2.0, 3.0]);

// Arithmetic (Result types for dimension-checked operations)
m.add(&other)?;        // element-wise addition
m.subtract(&other)?;   // element-wise subtraction
m.add_scalar(2.0);     // broadcast scalar addition
m.scale(2.0);          // scalar multiplication
m.dot(&other)?;        // matrix multiplication

// Element access (Result types for bounds-checked operations)
m.get(0, 0)?;          // read element at (row, col)
m.set(0, 0, 99.0)?;   // new matrix with element replaced

// Reductions
m.sum();               // sum of all elements
m.sum_rows();          // n x 1 column vector of row sums
m.sum_cols();          // 1 x m row vector of column sums
m.mean();              // arithmetic mean
m.min_val();           // minimum element
m.max_val();           // maximum element
m.argmin();            // (row, col) of minimum
m.argmax();            // (row, col) of maximum

// Element-wise math
m.map(|x| x * 2.0);   // apply closure to every element
m.sqrt();              // element-wise square root
m.abs_val();           // element-wise absolute value
m.pow_val(2.0);        // element-wise exponentiation

// Shape operations
m.flatten();           // 1 x n row vector
m.reshape(3, 2)?;      // reshape (total elements must match)
m.row(0)?;             // extract row as 1 x cols matrix
m.col(0)?;             // extract column as rows x 1 matrix
m.slice(0, 2, 1, 3)?;  // sub-matrix [r0..r1), [c0..c1)
m.transpose();         // swap rows and columns

// Equality
m.equals(&other);      // exact element-wise comparison
m.close(&other, 1e-9); // approximate comparison within tolerance
```

## Design Principles

1. **Immutable by default.** Methods take `&self` and return a new Matrix.
2. **No external dependencies.** Only `f64` methods from Rust's standard library.
3. **Result types.** Fallible operations return `Result` rather than panicking.
4. **Literate programming.** Source code includes inline doc comments with examples.

## Running Tests

```bash
cargo test --verbose
```

## Package Structure

- `src/lib.rs` -- Matrix struct with all operations (35 unit tests inline)
- `tests/matrix_tests.rs` -- Integration tests (5 tests)

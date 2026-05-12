# Changelog — cas-matrix (Rust)

## Unreleased

### Added

- `rowreduce` module with exact rational Gauss-Jordan `row_reduce(m)` and
  forward-elimination `rank(m)` for integer/rational matrices.
- Row-reduction tests covering identity, zero, full-rank, singular, rational
  dependent rows, wide/tall rank, and symbolic-entry errors.
- Exact rational `norm(m)` and `frobenius_norm(m)` APIs, returning exact
  integer/rational results or `Sqrt(...)` IR when needed.
- Exact rational `lu_decompose(m)` with partial pivoting, returning
  `List(L, U, P)`.
- Exact rational `nullspace(m)`, `columnspace(m)`, and `rowspace(m)` APIs,
  each returning basis matrices inside `List(...)` IR nodes.

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-matrix` package.
- `matrix` module: `MatrixError` error type, `MatrixResult<T>` alias,
  `MATRIX = "Matrix"` head constant.
  - `matrix(rows: Vec<Vec<IRNode>>) -> MatrixResult<IRNode>` — construct
    `Matrix(List(row₀), List(row₁), ...)` IR; validates uniform row width.
  - `is_matrix(node)` — predicate.
  - `rows_of(m)` — extract `Vec<Vec<IRNode>>` (public for arithmetic module).
  - `dimensions(m)` — returns `List(nrows, ncols)`.
  - `num_rows(m)` / `num_cols(m)` — shape accessors.
  - `get_entry(m, row, col)` — 1-based element access.
- `arithmetic` module: elementwise and structural operations.
  - `identity_matrix(n)` — n×n identity (integer 0/1 entries).
  - `zero_matrix(nrows, ncols)` — all-zero matrix.
  - `transpose(m)` — transpose.
  - `add_matrices(a, b)` — elementwise `Add(aᵢⱼ, bᵢⱼ)`.
  - `sub_matrices(a, b)` — elementwise `Sub(aᵢⱼ, bᵢⱼ)`.
  - `scalar_multiply(scalar, m)` — elementwise `Mul(scalar, mᵢⱼ)`.
  - `trace(m)` — sum of diagonal; returns single element for 1×1.
  - `dot(a, b)` — matrix product; each entry is `Add(Mul(...), ...)`.
- `determinant` module: cofactor expansion (O(n!)).
  - `determinant(m)` — symbolic determinant; base cases for 0×0, 1×1, 2×2.
  - `inverse(m)` — symbolic inverse via adjugate/determinant.
- 31 integration tests; all passing.

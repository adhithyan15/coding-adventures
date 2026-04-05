# Changelog

All notable changes to the Rust matrix package will be documented here.

## [0.2.0] - 2026-04-04

### Added
- **Element access:** `get(row, col)`, `set(row, col, value)` with Result error handling
- **Reductions:** `sum()`, `sum_rows()`, `sum_cols()`, `mean()`, `min_val()`, `max_val()`, `argmin()`, `argmax()`
- **Element-wise math:** `map(fn)`, `sqrt()`, `abs_val()`, `pow_val(exp)` for applying functions to every element
- **Shape operations:** `flatten()`, `reshape(rows, cols)`, `row(i)`, `col(j)`, `slice(r0, r1, c0, c1)` with Result types
- **Equality:** `equals(other)` for exact comparison, `close(other, tolerance)` for approximate comparison
- **Factory methods:** `Matrix::identity(n)`, `Matrix::from_diagonal(values)`
- 35 unit tests plus 1 doc-test covering all new and existing operations

## [0.1.0] - 2026-04-03

### Added
- Initial implementation with `zeros`, `new_2d`, `new_1d`, `new_scalar`
- Arithmetic: `add`, `add_scalar`, `subtract`, `scale`
- `transpose` and `dot` with Result-based error handling

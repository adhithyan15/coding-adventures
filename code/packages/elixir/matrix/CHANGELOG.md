# Changelog

All notable changes to the Elixir matrix package will be documented here.

## [0.2.0] - 2026-04-04

### Added
- **Element access:** `get(m, row, col)`, `set(m, row, col, value)` for reading and immutably updating elements
- **Reductions:** `sum(m)`, `sum_rows(m)`, `sum_cols(m)`, `mean(m)`, `min_val(m)`, `max_val(m)`, `argmin(m)`, `argmax(m)`
- **Element-wise math:** `map_elements(m, fn)`, `matrix_sqrt(m)`, `matrix_abs(m)`, `matrix_pow(m, exp)`
- **Shape operations:** `flatten(m)`, `reshape(m, rows, cols)`, `get_row(m, i)`, `get_col(m, j)`, `matrix_slice(m, r0, r1, c0, c1)`
- **Equality:** `equals(a, b)` for exact comparison, `close(a, b, tolerance)` for approximate comparison
- **Factory methods:** `Matrix.identity(n)`, `Matrix.from_diagonal(values)`
- 42 tests covering all new and existing operations

## [0.1.0] - 2026-04-03

### Added
- Initial implementation with `zeros`, `new`, `add`, `add_scalar`, `subtract`, `scale`, `transpose`, `dot`

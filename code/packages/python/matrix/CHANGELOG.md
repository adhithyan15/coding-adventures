# Changelog

All notable changes to the Python matrix package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-04-04

### Added

- **Element access:** `get(row, col)` and `set(row, col, value)` for fine-grained
  cell reads and immutable updates.
- **Reductions:** `sum()`, `sum_rows()`, `sum_cols()`, `mean()`, `min()`, `max()`,
  `argmin()`, `argmax()` for aggregate computations.
- **Element-wise math:** `map(fn)`, `sqrt()`, `abs()`, `pow(exp)` for applying
  functions to every element independently.
- **Shape operations:** `flatten()`, `reshape(rows, cols)`, `row(i)`, `col(j)`,
  `slice(r0, r1, c0, c1)` for rearranging elements.
- **Equality:** `equals(other)` for exact comparison, `close(other, tolerance)`
  for floating-point-safe comparison.
- **Factory methods:** `Matrix.identity(n)` and `Matrix.from_diagonal(values)`.
- Comprehensive test suite (48 tests) including cross-language parity vectors.

## [0.1.0] - 2026-03-20

### Added

- Initial implementation of the Python matrix package.
- `Matrix` class with `data`, `rows`, `cols` attributes.
- Constructors: scalar, 1D list, 2D list, `zeros(rows, cols)`.
- Arithmetic: `__add__`, `__sub__`, `__mul__` (scalar), `dot`.
- `transpose` and `__eq__`.

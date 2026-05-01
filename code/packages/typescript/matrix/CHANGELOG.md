# Changelog

All notable changes to the TypeScript matrix package will be documented here.

## Unreleased

### Changed
- Exposed `src/matrix.ts` as the package module entry so Vite/browser builds
  can bundle the Matrix implementation as ESM.

## [1.1.0] - 2026-04-04

### Added
- **Element access:** `get(row, col)`, `set(row, col, value)` for reading and immutably updating individual elements
- **Reductions:** `sum()`, `sumRows()`, `sumCols()`, `mean()`, `min()`, `max()`, `argmin()`, `argmax()`
- **Element-wise math:** `map(fn)`, `sqrt()`, `abs()`, `pow(exp)` for applying functions to every element
- **Shape operations:** `flatten()`, `reshape(rows, cols)`, `row(i)`, `col(j)`, `slice(r0, r1, c0, c1)`
- **Equality:** `equals(other)` for exact comparison, `close(other, tolerance)` for approximate comparison
- **Factory methods:** `Matrix.identity(n)`, `Matrix.fromDiagonal(values)`
- Comprehensive test suite with 47 tests covering all new and existing operations

## [1.0.0] - 2026-04-03

### Added
- Initial implementation with `zeros`, `add`, `subtract`, `scale`, `transpose`, `dot`
- Support for scalar, 1D array, and 2D array construction

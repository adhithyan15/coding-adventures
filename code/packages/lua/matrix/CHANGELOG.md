# Changelog — coding-adventures-matrix (Lua)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.2.0] — 2026-04-04

### Added (ML03 Matrix Extensions)

- **Reductions:** `sum(mat)`, `sum_rows(mat)`, `sum_cols(mat)`, `mean(mat)`, `min(mat)`, `max(mat)`, `argmin(mat)`, `argmax(mat)`
- **Element-wise math:** `map(mat, fn)`, `sqrt(mat)`, `abs(mat)`, `pow(mat, exp)`
- **Shape operations:** `flatten(mat)`, `reshape(mat, rows, cols)`, `row(mat, i)`, `col(mat, j)`, `slice(mat, r0, r1, c0, c1)`
- **Equality and comparison:** `equals(A, B)`, `close(A, B, tol)`
- **Factory methods:** `identity(n)`, `from_diagonal(values)`

### Changed

- `set(mat, i, j, val)` now returns a **new** matrix instead of mutating in place, following the ML03 immutability principle
- `get(mat, i, j)` now validates bounds and raises on out-of-range indices

---

## [0.1.0] — 2026-03-29

### Added

- `zeros(rows, cols)` — allocate an all-zero m×n matrix
- `new_2d(data)` — construct a matrix from a nested Lua table (deep copy)
- `new_1d(data)` — construct a 1×n row vector from a flat table
- `new_scalar(val)` — construct a 1×1 matrix
- `get(mat, i, j)` / `set(mat, i, j, val)` — element access (1-based)
- `add(A, B)` — element-wise addition with dimension check
- `add_scalar(A, s)` — add a scalar to every element
- `subtract(A, B)` — element-wise subtraction with dimension check
- `scale(A, s)` — multiply every element by a scalar
- `transpose(A)` — flip rows and columns
- `dot(A, B)` — matrix multiplication with inner-dimension check
- `VERSION = "0.1.0"`
- Rockspec `coding-adventures-matrix-0.1.0-1.rockspec`
- Comprehensive busted test suite including property-based checks

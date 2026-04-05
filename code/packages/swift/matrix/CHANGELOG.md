# Changelog — Matrix (Swift)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-04

### Added

**Base operations (ML03):**
- `Matrix.zeros(rows:cols:)` — allocate an all-zero m x n matrix
- `Matrix(from2D:)` — construct from a nested array (deep copy)
- `Matrix(from1D:)` — construct a 1 x n row vector
- `Matrix(scalar:)` — construct a 1 x 1 matrix
- Subscript `matrix[row, col]` — element access (0-based)
- `get(row:col:)` / `set(row:col:value:)` — named element access (throws on bounds)
- `add(_:)` — element-wise addition with dimension check
- `addScalar(_:)` — add a scalar to every element
- `subtract(_:)` — element-wise subtraction with dimension check
- `scale(_:)` — multiply every element by a scalar
- `transpose()` — flip rows and columns
- `dot(_:)` — matrix multiplication with inner-dimension check
- Operator overloads: `+`, `-`, `*` (scalar)

**Extension operations (ML03):**
- `sum()`, `sumRows()`, `sumCols()`, `mean()` — reductions
- `min()`, `max()`, `argmin()`, `argmax()` — extremum finders
- `map(_:)`, `sqrt()`, `abs()`, `pow(_:)` — element-wise math
- `flatten()`, `reshape(rows:cols:)`, `row(_:)`, `col(_:)`, `slice(r0:r1:c0:c1:)` — shape ops
- `close(_:tolerance:)` — approximate equality
- `Matrix.identity(n:)`, `Matrix.fromDiagonal(_:)` — factory methods

**Package infrastructure:**
- Swift Package Manager manifest (swift-tools-version:6.0)
- `Matrix` struct with `Equatable`, `Sendable`, `CustomStringConvertible`
- `MatrixError` enum for descriptive error handling
- 73 unit tests including parity test vectors
- BUILD / BUILD_windows scripts
- README.md, CHANGELOG.md, required_capabilities.json

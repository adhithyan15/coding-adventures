# Changelog — matrix (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of Swift matrix package.
- `Matrix` struct with scalar, 1D array, and 2D array initializers.
- `Matrix.zeros(rows:cols:)` factory.
- Element-wise `add`, `subtract` (matrix and scalar variants).
- `scale` for scalar multiplication.
- `transpose` to swap rows and columns.
- `dot` for true matrix multiplication.
- Subscript access `matrix[row, col]`.
- `Equatable` conformance.

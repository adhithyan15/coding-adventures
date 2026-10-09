# Changelog

All notable changes to the TypeScript matrix package will be documented here.

## Unreleased

### Security

- Move the test toolchain from `jest` 29 to `jest` ^30.5.2, `@types/jest` to ^30.0.0, and `ts-jest` to ^29.4.14, which accepts jest 30. jest 29 reaches `braces` 3.0.3 through `micromatch`. `braces` has a high-severity stack-exhaustion advisory (GHSA-vfj7-8cjw-p6xm) and no patched release, so the only fix is the jest 30 tree, which no longer depends on it. A moderate `sprintf-js` advisory (GHSA-hp3w-g68c-fv3c) remains through `babel-plugin-istanbul` → `@istanbuljs/load-nyc-config` → `js-yaml` 3 → `argparse` 1. It has no upstream fix and is tracked in `code/specs/EXTERNAL-DEPENDENCY-ALERT-BACKLOG.md`.

### Added
- Added a `MatrixBackend` contract plus pure-JS `CpuMatrixBackend` with
  `getMatrixBackend`, `setMatrixBackend`, and `resetMatrixBackend`, giving
  browser and host callers a shared dispatch point for dense numeric matrix
  operations.

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

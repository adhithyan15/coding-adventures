# Changelog

All notable changes to the Ruby matrix package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

- Package existing 0.2.0 pure-Ruby arithmetic as `coding_adventures_matrix`,
  with explicit gem files/license and no runtime dependencies. Retain matrix_ml
  and its public Matrix API rather than substituting a backend.
- Add both BUILD fronts, package-local Bundler/Rake/Standard/SimpleCov setup,
  a 95% coverage floor and actual isolated gem build/install/dot-product smoke.
- Extend boundary and metadata/build-front tests, retain constructor aliasing
  explicitly, and apply Standard formatting without arithmetic changes.
- Separate this NN44 prerequisite from native neural training/conformance.

## [0.2.0] - 2026-04-04

### Added

- **Element access:** `get(row, col)` and `set(row, col, value)` for fine-grained
  cell reads and immutable updates.
- **Reductions:** `sum`, `sum_rows`, `sum_cols`, `mean`, `min`, `max`,
  `argmin`, `argmax` for aggregate computations.
- **Element-wise math:** `map_elements(&block)`, `sqrt`, `abs`, `pow(exp)` for
  applying functions to every element independently.
- **Shape operations:** `flatten`, `reshape(rows, cols)`, `row(i)`, `col(j)`,
  `slice(r0, r1, c0, c1)` for rearranging elements.
- **Equality:** `equals(other)` for exact comparison, `close(other, tolerance)`
  for floating-point-safe comparison.
- **Factory methods:** `Matrix.identity(n)` and `Matrix.from_diagonal(values)`.
- Comprehensive test suite (47 tests) including cross-language parity vectors.

## [0.1.0] - 2026-03-20

### Added

- Initial implementation of the Ruby matrix package.
- `Matrix` class with `data`, `rows`, `cols` attributes.
- Constructors: scalar, 1D array, 2D array, `zeros(rows, cols)`.
- Arithmetic: `+`, `-`, `*` (scalar), `dot`.
- `transpose` method.

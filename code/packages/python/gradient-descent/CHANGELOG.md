# Changelog

All notable changes to the Python gradient-descent package will be documented
in this file.

## [1.1.0] - 2026-10-25

### Added

- Cross-platform repository BUILD fronts with Ruff and production-only coverage
  validation.
- ML02 parity, formula, immutability, empty-input, and shape-mismatch tests.
- Package documentation and complete publishing and development metadata.

### Changed

- Moved pytest out of runtime dependencies and into the development extra.
- Modernized type annotations without changing the public `sgd` API or update
  behavior.

## [1.0.0] - 2026-03-20

### Added

- Initial pure stochastic-gradient-descent update.

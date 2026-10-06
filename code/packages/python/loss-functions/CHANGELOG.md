# Changelog

All notable changes to the Python loss-functions package will be documented in
this file.

## [1.0.0] - 2026-09-30

### Added

- Exact ML01 parity vectors, epsilon-boundary coverage, and derivative contract
  tests.
- Cross-platform repository BUILD fronts with coverage and Ruff validation.
- Package documentation and publishable Hatch metadata.

### Fixed

- Assert the exact canonical `1e-7` BCE and CCE clamp values and derivatives at
  zero and one instead of checking only that boundary results are finite.

## [0.1.0] - 2026-03-20

### Added

- Initial loss functions and derivatives.

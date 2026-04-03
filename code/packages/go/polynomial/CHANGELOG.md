# Changelog — go/polynomial

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of polynomial arithmetic over float64.
- `Normalize`, `Degree`, `Zero`, `One`.
- `Add`, `Subtract`, `Multiply`.
- `Divmod` (panics for zero divisor), `Divide`, `Mod`.
- `Evaluate` using Horner's method.
- `GCD` using Euclidean algorithm.
- Comprehensive test suite covering all functions and edge cases.

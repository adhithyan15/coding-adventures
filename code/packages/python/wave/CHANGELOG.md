# Changelog

## Unreleased

### Changed
- Enforce finite parameters, angular-frequency overflow, and finite evaluation
  time across the complete PHY01 binary64 range.
- Reduce time and phase before local trig evaluation and handle infinite
  represented periods for positive subnormal frequencies.
- Add exact-zero and amplitude-bound regression coverage.

## [0.1.0] - 2026-03-23

### Added
- `Wave` class with amplitude, frequency, and phase
- `evaluate(t)` method computing wave value at time t
- `period()` and `angular_frequency()` derived quantities
- Input validation for amplitude and frequency

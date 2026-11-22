# Changelog

## Unreleased

### Changed
- Add VER15 CWE-20/190/400/674 validation hardening (`_safe_repr`, `_finite_float`,
  `_validate_state`, negative-zero normalization) to `Wave`.
- Add `wave.warrant`, `tests/test_hostile_inputs.py`, `tests/test_pbt.py`
  (SplitMix64), `ruff` lint config, and `>=95%` coverage threshold in `pyproject.toml`.
- Align `BUILD` and `BUILD_windows` with `--no-project`, `ruff check`, and `pytest-cov`.
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

# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed
- Enforce `#![forbid(unsafe_code)]`, `#![warn(rust_2018_idioms)]`, negative-zero
  normalization, public `validate()`, and non-panicking `try_period`,
  `try_angular_frequency`, and `try_evaluate` methods (VER15 CWE-20/190/400/674).
- Add `wave.warrant` specification, `tests/test_hostile_inputs.rs`, and
  `tests/test_pbt.rs` (SplitMix64 property-based tests).
- Enforce finite construction, angular-frequency overflow, and finite-time
  validation even after mutation of the public `Wave` fields.
- Reduce time and phase before local trig evaluation and cover exact-zero,
  maximum-finite, and minimum-subnormal boundaries.

## [0.1.0] - 2026-03-22

### Added
- `Wave` struct with amplitude, frequency, and phase fields
- `Wave::new()` constructor with validation (amplitude >= 0, frequency > 0)
- `period()` method — computes 1/frequency
- `angular_frequency()` method — computes 2 * PI * frequency
- `evaluate(t)` method — computes amplitude * sin(2 * PI * frequency * t + phase)
- Comprehensive integration tests covering periodicity, phase shifts, edge cases, and validation
- Literate programming style with inline physics explanations

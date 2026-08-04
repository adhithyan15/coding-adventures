# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Changed

- Enforce the full PHY01 finite-input and angular-frequency-overflow contract.
- Reduce time and phase before local first-principles trig evaluation, including
  infinite represented periods for positive subnormal frequencies.
- Add shared-corpus boundary coverage and a direct local `trig` dependency.

## [0.1.0] - 2026-04-26

### Added

- Pure C# immutable sinusoidal wave model with amplitude, frequency, and phase
- Period, angular-frequency, and time-domain evaluation helpers
- Validation for non-negative amplitudes and positive frequencies
- xUnit coverage for construction, phase, extrema, periodicity, invalid input, and high-frequency behavior
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI

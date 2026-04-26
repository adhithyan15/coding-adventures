# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-26

### Added

- Pure F# immutable sinusoidal wave model with amplitude, frequency, and phase
- Period, angular-frequency, and time-domain evaluation helpers
- Validation for non-negative amplitudes and positive frequencies
- xUnit coverage for construction, phase, extrema, periodicity, invalid input, and high-frequency behavior
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI

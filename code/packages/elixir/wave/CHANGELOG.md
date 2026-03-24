# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- `Wave` struct with amplitude, frequency, and phase fields
- `Wave.new/2,3` constructor with validation (amplitude >= 0, frequency > 0)
- `Wave.evaluate/2` computes displacement at time t using sinusoidal equation
- `Wave.period/1` returns the period (1/frequency) in seconds
- `Wave.angular_frequency/1` returns omega (2*pi*f) in rad/s
- Comprehensive ExUnit test suite covering:
  - Constructor validation (valid params, negative amplitude, zero frequency)
  - Basic wave evaluation at key time points (0, T/4, T/2, 3T/4, T)
  - Periodicity verification across multiple cycles
  - Phase shift behavior (pi/2 starts at peak, pi inverts, -pi/2 at trough)
  - Amplitude scaling and zero-amplitude flat line
  - Different frequencies (2 Hz, 0.5 Hz)
  - Negative time evaluation
- Literate programming style with inline explanations and truth table
- Depends on `trig` package (Maclaurin series sine, no `:math` usage)

# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `Wave` struct with amplitude, frequency, and phase fields
- `Wave::new()` constructor with validation (amplitude >= 0, frequency > 0)
- `period()` method — computes 1/frequency
- `angular_frequency()` method — computes 2 * PI * frequency
- `evaluate(t)` method — computes amplitude * sin(2 * PI * frequency * t + phase)
- Comprehensive integration tests covering periodicity, phase shifts, edge cases, and validation
- Literate programming style with inline physics explanations

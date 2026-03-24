# Changelog

All notable changes to the Wave package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- `Wave` class with amplitude, frequency, and phase parameters
- `evaluate(t)` method computing `A * sin(2 * PI * f * t + phase)` using the trig package
- `period` method returning `1.0 / frequency`
- `angular_frequency` method returning `2 * PI * frequency`
- Input validation: `ArgumentError` for negative amplitude or non-positive frequency
- Comprehensive Minitest test suite (14 tests)
- Literate programming style with physics explanations throughout

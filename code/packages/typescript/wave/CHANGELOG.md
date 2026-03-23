# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- `Wave` class with constructor validation (amplitude >= 0, frequency > 0)
- `evaluate(t)` method computing `A · sin(2πft + φ)` using first-principles trig
- `period()` method returning `1/f`
- `angularFrequency()` method returning `2πf`
- Readonly `amplitude`, `frequency`, and `phase` properties
- Comprehensive test suite covering evaluation, periodicity, phase shifting, derived quantities, validation, and higher frequencies
- Literate programming style with inline explanations of wave physics

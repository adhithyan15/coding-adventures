# Changelog

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All public functions and methods (`New`,
  `Period`, `AngularFrequency`, `Evaluate`) are now wrapped with `StartNew[T]`
  from the package's Operations infrastructure. Each call gains automatic
  timing, structured logging, and panic recovery.

## [0.1.0] - 2026-03-22

### Added
- `Wave` struct with Amplitude, Frequency, and Phase
- `New()` constructor with validation
- `Evaluate(t)` method computing wave value at time t
- `Period()` and `AngularFrequency()` derived properties

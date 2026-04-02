# Changelog

All notable changes to the Go matrix package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All public functions and methods (`Zeros`,
  `New2D`, `New1D`, `NewScalar`, `Add`, `AddScalar`, `Subtract`, `Scale`,
  `Transpose`, `Dot`) are now wrapped with `StartNew[T]` from the package's
  Operations infrastructure. Each call gains automatic timing, structured
  logging, and panic recovery.

## [0.1.0] - 2026-03-20

### Added

- Initial implementation of the Go matrix package.
- `Matrix` struct with `Data [][]float64`, `Rows`, and `Cols` fields.
- Constructors: `Zeros(rows, cols)`, `New2D(data)`, `New1D(data)`, `NewScalar(val)`.
- Arithmetic methods: `Add`, `Subtract`, `Scale`, `AddScalar`.
- Linear algebra methods: `Transpose`, `Dot`.
- Comprehensive test suite with error cases for dimension mismatches.
- Literate programming documentation explaining matrix fundamentals.

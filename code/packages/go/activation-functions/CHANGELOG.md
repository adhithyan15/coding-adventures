# Changelog

All notable changes to the Go activation-functions package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All six public functions (`Sigmoid`, `ReLU`,
  `LeakyReLU`, `Tanh`, `Softmax`, `Linear`) are now wrapped with `StartNew[T]`
  from the package's Operations infrastructure. Each call gains automatic timing,
  structured logging, and panic recovery at zero cost to callers.

## [0.1.0] - 2026-03-20

### Added

- Initial implementation of six activation functions: `Sigmoid`, `ReLU`,
  `LeakyReLU`, `Tanh`, `Softmax`, `Linear`
- Knuth-style literate documentation with mathematical derivations and ASCII
  diagrams for each function
- Comprehensive unit tests covering standard inputs, edge cases, and numerical
  stability checks

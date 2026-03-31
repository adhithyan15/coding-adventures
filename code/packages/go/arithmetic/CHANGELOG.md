# Changelog

All notable changes to the arithmetic package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-31

### Changed

- **Operations system integration**: All public functions and methods (`HalfAdder`,
  `FullAdder`, `RippleCarryAdder`, `NewALU`, `ALU.Execute`) are now wrapped with
  `StartNew[T]` from the package's Operations infrastructure. Multi-return functions
  use inline helper structs to package values into a single generic type parameter.
  Each call gains automatic timing, structured logging, and panic recovery.

## [0.1.0] - 2026-03-20

### Added
- Initial package scaffolding with `go.mod`
- `HalfAdder`, `FullAdder` and `RippleCarryAdder` implemented from logic gates
- `ALU` implemented with `ADD`, `SUB`, `AND`, `OR`, `XOR`, `NOT` operations
- Comprehensive test suite matching Python specifications
- Extensive literate-programming documentation

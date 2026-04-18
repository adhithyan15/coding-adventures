# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# MA02 Reed-Solomon implementation over the local `.NET` `gf256` package
- Generator construction, systematic encoding, syndrome computation, and Berlekamp-Massey decoding helpers
- Chien-search and Forney-based correction with explicit invalid-input and too-many-errors exceptions
- xUnit coverage for structural vectors, correction-at-capacity cases, invalid inputs, and unrecoverable codewords

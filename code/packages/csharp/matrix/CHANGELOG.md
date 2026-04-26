# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-26

### Added

- Pure C# immutable matrix type for double-precision values
- Factory helpers for scalars, row vectors, and zero-filled matrices
- Element-wise add/subtract, scalar add/subtract/scale, transpose, dot product, indexing, deep-copy access, equality, hashing, and dimension validation
- xUnit coverage for construction, arithmetic, immutability, equality, and invalid dimensions
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI

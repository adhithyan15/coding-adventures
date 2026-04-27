# Changelog - coding-adventures-nib-compiler

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-14

### Added

- End-to-end Nib source to Intel HEX compiler package
- `NibCompiler` facade with `compile_source()` and `write_hex_file()`
- `PackageResult` carrying AST, IR, assembly, binary, and HEX artifacts
- `PackageError` with stage-aware diagnostics

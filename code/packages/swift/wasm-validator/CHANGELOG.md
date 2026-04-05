# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added

- WasmValidator namespace struct for module compatibility
- Comprehensive test suite (16 tests) covering all validation rules:
  type index validation, function index validation, memory/table limits,
  duplicate export detection, export index range, start function signature,
  function-code count mismatch, imported function type validation

## [0.1.0] - 2026-04-05

### Added

- Initial structural validator with all WASM 1.0 validation rules
- ValidatedModule type with resolved function type array

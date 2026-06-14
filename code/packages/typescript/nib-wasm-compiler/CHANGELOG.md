# Changelog

## [0.1.1] - 2026-06-03

### Changed

- Added tests for `optimize: false` constructor option, `writeWasmFile` error wrapping
  (covers the `tryStage` catch block), and `extractSignatures` helper function branches
  (empty program, missing decl, non-fn_decl top-level, missing NAME token, nested NAME
  resolution). These tests bring branch coverage above the 70% threshold after the
  vitest 3→4 upgrade exposed gaps.

## [0.1.0] - 2026-04-18

### Added

- End-to-end TypeScript Nib-to-Wasm orchestration package.
- Tests for artifact capture, file output, stage errors, and runtime execution.

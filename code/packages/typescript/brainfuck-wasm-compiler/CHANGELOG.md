# Changelog

## 0.1.1

### Changed

- Added tests for `PackageError` constructor, `optimize: false` constructor option, and
  `writeWasmFile` error wrapping (covers the `tryStage` catch block). These tests bring
  line coverage above the 80% threshold after the vitest 3→4 upgrade exposed gaps.

## 0.1.0

- Initial TypeScript Brainfuck-to-WASM pipeline.

# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Fixed

- Declare the virtual-machine and Wasm-opcodes packages used by the test-only
  parser/validator path so clean full-build metadata validation matches the
  local `PERL5LIB` closure.

## [0.1.0] - 2026-04-18

### Added

- Encodes plain Perl Wasm module hashrefs into `.wasm` bytes.
- Supports the key aliases already used in the local Wasm stack.
- Round-trip coverage through the local parser and validator.

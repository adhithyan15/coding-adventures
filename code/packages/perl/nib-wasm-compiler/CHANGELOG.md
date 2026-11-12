# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.1.0] - 2026-04-18

### Added

- End-to-end Nib to Wasm orchestration for Perl.
- Structured package errors for parse, type-check, lowering, encoding, and
  write stages.
- Runtime-backed coverage for function calls, loops, and user-facing errors.

# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.1.0] - 2026-04-18

### Added

- Typed-AST to compiler-IR lowering for Nib functions, returns, arithmetic,
  loops, and calls.
- Release and debug config helpers matching the rest of the compiler stack.
- Coverage for entry-point lowering and loop control-flow generation.

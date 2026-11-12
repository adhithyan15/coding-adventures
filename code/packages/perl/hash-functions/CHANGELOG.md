# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.1.0

- Initial Perl package with binary-safe FNV-1a, DJB2, polynomial rolling,
  MurmurHash3 x86_32, and deterministic avalanche and distribution helpers.
- Preserve exact 64-bit results with core `Math::BigInt` values.

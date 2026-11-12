# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.1.0 - 2026-07-15

- Add a pure Perl HyperLogLog cardinality estimator.
- Support deterministic hashing, bounded precision, merging, clearing, and
  estimator metadata.
- Add core-only tests, CPAN metadata, build scripts, and a capability
  declaration.

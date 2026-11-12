# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.1.0 - 2026-07-15

- Add a pure Perl span-augmented probabilistic skip list.
- Support ordered map mutation, lookup, rank, selection, ranges, iteration,
  custom comparators, and deterministic topology generation.
- Add invariant-heavy tests, CPAN metadata, build scripts, and capability
  declarations.

# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0 - 2026-07-16

- Add a persistent DT19 hash set backed by the Perl DT18 hash map.
- Add complete set algebra, relation predicates, option preservation, and
  reference-identity support.
- Add MakeMaker metadata, capability declaration, and Test::More coverage.

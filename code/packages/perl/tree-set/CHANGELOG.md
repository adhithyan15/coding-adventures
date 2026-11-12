# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries, with `avl-tree` as a dependency) instead of a backslash `-I` recipe that failed on Windows.

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0 - 2026-07-15

- Add a pure-Perl AVL-backed ordered set with mutation, rank, selection,
  ranges, custom comparison, and set algebra.

# Changelog

## [Unreleased]

### Fixed

- `BUILD_windows` passes dependency library paths with `-I` instead of `set PERL5LIB=...`, which MSYS perl on CI splits on `:` and so never found.

## 0.1.0 - 2026-07-15

- Add a pure-Perl AVL-backed ordered set with mutation, rank, selection,
  ranges, custom comparison, and set algebra.

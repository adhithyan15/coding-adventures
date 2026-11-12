# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.1.0

- Initial Perl trie package with insertion, exact lookup, deletion, prefix
  scans, longest-prefix matching, sorted enumeration, and validation helpers.

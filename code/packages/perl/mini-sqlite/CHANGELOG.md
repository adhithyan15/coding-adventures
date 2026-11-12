# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01

- Add a Level 0 in-memory mini-sqlite facade.
- Support DB-API-inspired connections, cursors, qmark binding, and transaction snapshots.

# Changelog

## Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

- Added graph, node, and edge property bags with synchronized `weight` edge metadata.

## 0.01

- Add the DT00 undirected weighted graph package for Perl.

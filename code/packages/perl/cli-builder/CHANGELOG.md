# Changelog — cli-builder (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-03-31

Initial release.

- `TokenClassifier` — argv token classification
- `SpecLoader` — spec validation and normalization
- `HelpGenerator` — help text generation
- `FlagValidator` — flag constraint validation
- `Parser` — three-phase parsing pipeline
- `parse_hashref` — main public API

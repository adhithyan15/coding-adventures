# Changelog — upc-a (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-04-13

Initial release.

- `compute_upc_a_check_digit` — checksum generation
- `normalize_upc_a` — payload validation and completion
- `encode_upc_a` — left/right digit encoding
- `draw_upc_a` — PaintScene generation

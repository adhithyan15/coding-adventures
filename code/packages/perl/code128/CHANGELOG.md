# Changelog — code128 (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-04-13

Initial release.

- `normalize_code128_b` — printable ASCII validation
- `compute_code128_checksum` — weighted checksum generation
- `encode_code128_b` — start/data/check/stop sequence generation
- `draw_code128` — PaintScene generation

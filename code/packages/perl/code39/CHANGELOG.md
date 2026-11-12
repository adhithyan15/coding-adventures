# Changelog — code39 (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-03-31

Initial release.

- `normalize_code39` — uppercase conversion and validation
- `encode_code39_char` — single character to N/W pattern
- `encode_code39` — full string encoding with start/stop markers
- `expand_code39_runs` — bar/space run expansion
- `draw_code39` — SVG rendering
- `compute_checksum` — optional mod-43 checksum

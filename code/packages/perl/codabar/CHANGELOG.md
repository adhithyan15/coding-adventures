# Changelog — codabar (Perl)

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## 0.01 — 2026-04-13

Initial release.

- `normalize_codabar` — guard validation and normalization
- `encode_codabar` — full payload encoding with start/stop markers
- `expand_codabar_runs` — bar/space run expansion with quiet gaps
- `draw_codabar` — PaintScene generation

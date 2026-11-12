# Changelog — itf (Perl)

## Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

- Add shared fixture conformance, Unicode-scalar preflight, and stable error IDs.

## 0.01 — 2026-04-13

Initial release.

- `normalize_itf` — even-length digit validation
- `encode_itf` — digit-pair interleaving
- `expand_itf_runs` — start/data/stop run emission
- `draw_itf` — PaintScene generation

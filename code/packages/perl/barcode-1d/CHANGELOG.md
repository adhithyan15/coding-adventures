# Changelog — code39 (Perl)

## 0.01 — 2026-03-31

Initial release.

- `normalize_code39` — uppercase conversion and validation
- `encode_code39_char` — single character to N/W pattern
- `encode_code39` — full string encoding with start/stop markers
- `expand_code39_runs` — bar/space run expansion
- `draw_code39` — SVG rendering
- `compute_checksum` — optional mod-43 checksum

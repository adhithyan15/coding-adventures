# Changelog — code39 (Lua)

## 0.1.0 — 2026-03-31

Initial release.

- `PATTERNS` — complete 44-character Code 39 encoding table
- `normalize_code39` — uppercase conversion and validation
- `encode_code39_char` — single character to N/W pattern
- `encode_code39` — full string encoding with start/stop markers
- `expand_code39_runs` — bar/space run expansion with inter-character gaps
- `draw_code39` — SVG rendering with configurable dimensions
- `compute_checksum` — optional mod-43 check character

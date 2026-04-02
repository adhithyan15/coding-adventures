# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `load(data)` — parse raw font bytes, validate magic numbers, collect table offsets
- `font_metrics(font)` — global metrics from head/hhea/maxp/name/OS2 tables
- `glyph_id(font, codepoint)` — cmap Format 4 lookup for Unicode BMP codepoints
- `glyph_metrics(font, glyph_id)` — per-glyph advance width and left side bearing
- `kerning(font, left, right)` — kern Format 0 binary search for glyph pair adjustments
- `FontMetrics` frozen dataclass with all typographic metrics
- `GlyphMetrics` frozen dataclass with advance_width and left_side_bearing
- `FontError` exception class with `kind` attribute for programmatic handling
- Zero dependencies — pure Python, uses only `struct` from stdlib
- Accepts `bytes`, `bytearray`, and `memoryview`
- 30+ unit tests; synthetic font builder for kern/cmap logic testing

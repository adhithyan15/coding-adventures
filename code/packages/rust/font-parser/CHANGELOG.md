# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `load(bytes)` — parse raw font bytes, validate magic numbers, collect table offsets
- `font_metrics(font)` — extract global metrics from head/hhea/maxp/name/OS2 tables
- `glyph_id(font, codepoint)` — map Unicode BMP codepoints to glyph IDs via cmap Format 4
- `glyph_metrics(font, glyph_id)` — per-glyph advance width and left side bearing from hmtx
- `kerning(font, left, right)` — kern Format 0 binary search for glyph pair adjustments
- `FontMetrics` struct with units_per_em, ascender, descender, line_gap, x_height, cap_height, num_glyphs, family_name, subfamily_name
- `GlyphMetrics` struct with advance_width and left_side_bearing
- `FontError` enum: InvalidMagic, InvalidHeadMagic, TableNotFound, BufferTooShort, UnsupportedCmapFormat
- Zero dependencies — pure Rust, no `unsafe`, WASM-bake-able
- 26 unit tests covering all public functions and error paths
- Tested against Inter Regular v4.0 (SIL OFL): units_per_em=2048, A+V kern negative

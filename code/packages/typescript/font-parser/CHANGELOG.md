# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `load(bytes)` — parse raw font Uint8Array, validate magic numbers, collect table offsets
- `fontMetrics(font)` — global metrics from head/hhea/maxp/name/OS2 tables
- `glyphId(font, codepoint)` — cmap Format 4 lookup for Unicode BMP codepoints
- `glyphMetrics(font, glyphId)` — per-glyph advance width and left side bearing
- `kerning(font, left, right)` — kern Format 0 binary search for glyph pair adjustments
- `FontMetrics` interface with all typographic metrics
- `GlyphMetrics` interface with advance width and left side bearing
- `FontError` class with `kind` discriminant for specific failure modes
- Zero dependencies — pure TypeScript, no DOM, WASM-bake-able
- TextDecoder fast path with manual UTF-16 BE fallback for bare WASM environments
- 30 unit tests covering all public API and error paths
- Synthetic font builder utility for testing kern/cmap logic without external files

# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `font-parser` crate: a metrics-only OpenType/
  TrueType font parser that reads head/hhea/maxp/cmap/hmtx/kern/name/OS-2 to
  measure text without touching the OS font stack.
- `font_load` (owned `FontFile`) / `font_free`; `font_metrics` (unitsPerEm,
  ascender/descender/lineGap with OS/2 typo preference, x/cap height, numGlyphs,
  UTF-16 BE → UTF-8 family/subfamily names); `font_glyph_id` (cmap Format 4
  binary search); `font_glyph_metrics` (hmtx, incl. shared-advance section);
  `font_kerning` (kern Format 0 binary search); and the bounds-checked
  big-endian helpers `font_read_u16`/`_i16`/`_u32`.
- Every multi-byte read is bounds-checked with overflow-safe arithmetic; a
  truncated or corrupt file yields an error or a 0/false return, never an
  out-of-bounds access. Verified clean under ASan + UBSan, the macOS `leaks`
  tool (0 leaks), and a truncation fuzz over every prefix of a valid font.
- Documented divergences: `FONT_ERR_TABLE_NOT_FOUND` drops the missing table's
  name the Rust carries; name strings decode into fixed 128-byte buffers.
- 46 checks run under every ISO C compiler via the shared `iso-harness`,
  exercising the full parser against a self-contained synthetic in-memory font
  (no external `.ttf` fixture): metrics, cmap glyph lookup, glyph metrics
  (direct + shared-advance), kerning, error cases, OTTO magic, and the fuzz.

# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `Load(data)` — parse raw font bytes, return `*FontFile` or `*FontError`
- `GetFontMetrics(font)` — global metrics from head/hhea/maxp/name/OS2 tables
- `GlyphID(font, rune)` — cmap Format 4 lookup for Unicode BMP codepoints
- `GetGlyphMetrics(font, glyph_id)` — per-glyph advance width and left side bearing
- `Kerning(font, left, right)` — kern Format 0 binary search for glyph pair adjustments
- `Metrics` struct with all typographic fields (XHeight/CapHeight as *int16)
- `GlyphMetrics` struct with AdvanceWidth and LeftSideBearing
- `FontError` with `Kind` field and `Is()` for errors.As compatibility
- `FontErrorKind` constants: ErrInvalidMagic, ErrInvalidHeadMagic, ErrTableNotFound, ErrBufferTooShort
- Uses `encoding/binary.BigEndian` for all reads — no manual bit shifts
- Uses `unicode/utf16.Decode` for name table UTF-16 BE decoding
- Zero external dependencies
- 31 tests, 83.8% coverage, go vet clean

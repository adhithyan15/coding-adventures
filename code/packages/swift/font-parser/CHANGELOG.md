# Changelog — FontParser (Swift)

## [0.1.0] — 2026-04-01

### Added

- Initial release — metrics-only OpenType/TrueType font parser with zero
  external dependencies (only Swift stdlib + Foundation for `Data` and
  `String.Encoding.utf16BigEndian`).

- **`load(_:) throws -> FontFile`** — Parses a `Data` binary. Throws
  `FontError` on failure: `.bufferTooShort`, `.invalidMagic`,
  `.tableNotFound(String)`, `.parseError(String)`.

- **`fontMetrics(_:) -> FontMetrics`** — Returns a `FontMetrics` struct.

- **`glyphId(_:codepoint:) -> UInt16?`** — Returns `nil` for out-of-BMP or
  unmapped codepoints.

- **`glyphMetrics(_:glyphId:) -> GlyphMetrics?`** — Returns `nil` for
  out-of-range glyph IDs.

- **`kerning(_:left:right:) -> Int16`** — Returns `0` when absent.

### Implementation notes

- Manual big-endian reads via bit shifts (`UInt16(data[offset]) << 8 | ...`)
  using `data.startIndex + offset` to support `Data` slices.
- cmap Format 4 idRangeOffset: `iroAbs + idRangeOffset + (cp - startCode) * 2`
- kern Format 0 coverage: format in HIGH byte (`coverage >> 8`).
- OS/2 sxHeight at offset 86, sCapHeight at offset 88.
- Inter v4.0 uses GPOS not kern table; kern logic verified via an in-memory
  synthetic font builder in the test suite.
- Test fixture resolved at runtime using `#filePath` — no SPM resource bundle
  needed.
- 30 tests, 0 failures.

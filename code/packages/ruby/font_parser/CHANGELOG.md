# Changelog — coding_adventures_font_parser (Ruby)

## [0.1.0] — 2026-04-01

### Added

- Initial release of `CodingAdventures::FontParser` — a metrics-only OpenType/TrueType
  font parser with zero runtime dependencies.

- **`load(bytes)`** — Parses a font binary (String in binary encoding or any object
  that responds to `bytesize` / `getbyte`). Returns an opaque `FontFile` struct or
  raises `FontError` with a `kind` attribute for programmatic error handling.
  Recognised kinds: `BufferTooShort`, `InvalidMagic`, `TableNotFound`, `ParseError`.

- **`font_metrics(font)`** — Returns a `FontMetrics` struct with:
  - `units_per_em` (Integer) — design-space units per em square (typically 1000 or 2048)
  - `ascender`, `descender`, `line_gap` (Integer, signed) — vertical extents in font units
  - `x_height`, `cap_height` (Integer or nil) — optional OS/2 v2+ metrics
  - `num_glyphs` (Integer) — total glyph count from `maxp`
  - `family_name`, `subfamily_name` (String) — from `name` table, UTF-8; falls back to
    `"(unknown)"` when the name record is absent

- **`glyph_id(font, codepoint)`** — Maps a Unicode codepoint (Integer) to a glyph index
  via the `cmap` Format 4 BMP subtable (platform 3, encoding 1). Returns `nil` for
  codepoints outside the BMP or not present in the font.

- **`glyph_metrics(font, glyph_id)`** — Returns a `GlyphMetrics` struct with:
  - `advance_width` (Integer) — horizontal advance in font units
  - `left_side_bearing` (Integer, signed) — left-side bearing
  Returns `nil` for out-of-range or negative glyph IDs.

- **`kerning(font, left_glyph_id, right_glyph_id)`** — Returns the kern adjustment
  (Integer, signed, in font units) for the given glyph pair from the `kern` Format 0
  subtable. Returns `0` if no kern table is present or the pair is not listed.

### Tables parsed

`head`, `hhea`, `maxp`, `cmap` (Format 4), `hmtx`, `kern` (Format 0),
`name` (UTF-16 BE), `OS/2` (version ≥ 2 for x\_height/cap\_height).

### Notes

- Inter v4.0 (the test fixture) uses GPOS for kerning rather than the legacy `kern`
  table, so `kerning()` returns 0 for all Inter glyph pairs. Kern logic is verified
  via a synthetic font builder in the test suite.
- No glyph outline parsing, shaping, or rasterisation — metrics only.
- Requires Ruby ≥ 3.3.

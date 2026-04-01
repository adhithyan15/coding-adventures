# Changelog — coding_adventures_font_parser (Elixir)

## [0.1.0] — 2026-04-01

### Added

- Initial release of `CodingAdventures.FontParser` — a metrics-only
  OpenType/TrueType font parser with zero runtime dependencies.

- **`load/1`** — Parses a font binary (Elixir `binary`). Returns an opaque
  `FontFile` struct or raises `FontError` with a `kind` field for programmatic
  error handling. Recognised kinds: `"BufferTooShort"`, `"InvalidMagic"`,
  `"TableNotFound"`, `"ParseError"`.

- **`font_metrics/1`** — Returns a `FontMetrics` struct with:
  - `units_per_em` — design-space units per em (u16)
  - `ascender`, `descender`, `line_gap` — vertical extents (signed integer)
  - `x_height`, `cap_height` — optional OS/2 v2+ metrics (integer | nil)
  - `num_glyphs` — total glyph count from `maxp`
  - `family_name`, `subfamily_name` — UTF-8 strings from `name` table;
    fallback to `"(unknown)"`

- **`glyph_id/2`** — Maps a Unicode codepoint (integer) to a glyph index via
  the `cmap` Format 4 BMP subtable. Returns `nil` for out-of-BMP codepoints,
  negative values, or unmapped codepoints.

- **`glyph_metrics/2`** — Returns a `GlyphMetrics` struct with `advance_width`
  and `left_side_bearing`. Returns `nil` for out-of-range glyph IDs.

- **`kerning/3`** — Returns the kern adjustment (signed integer, font units)
  for the given glyph pair from `kern` Format 0. Returns `0` if absent.

### Tables parsed

`head`, `hhea`, `maxp`, `cmap` (Format 4), `hmtx`, `kern` (Format 0),
`name` (UTF-16 BE via `:unicode`), `OS/2` (v2+ for x\_height/cap\_height).

### Implementation notes

- Uses Elixir binary pattern matching (`<<v::unsigned-big-16>>`) for all
  big-endian reads — no external parsing library needed.
- All ranges use explicit step `//1` to avoid the default decreasing step
  when the count is zero (e.g., empty kern pair list).
- cmap Format 4 idRangeOffset uses the absolute byte address formula
  `iro_abs + iro + (cp - start_code) * 2`.
- kern coverage format bits are in the HIGH byte (bits 8-15); the
  `coverage >>> 8` shift extracts the subtable format number.
- UTF-16 BE decoding delegates to `:unicode.characters_to_binary/3`.
- 32 tests, 90.97% coverage.

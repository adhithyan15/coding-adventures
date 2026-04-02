# Changelog — coding-adventures-font-parser (Lua)

## [0.1.0] — 2026-04-01

### Added

- Initial release of `coding_adventures.font_parser` — a metrics-only
  OpenType/TrueType font parser with zero runtime dependencies.

- **`load(data)`** — Parses a font binary string. Returns an opaque
  `FontFile` table on success, or raises `{kind=..., message=...}` on failure.
  Recognised `kind` values: `"BufferTooShort"`, `"InvalidMagic"`,
  `"TableNotFound"`, `"ParseError"`.

- **`font_metrics(font)`** — Returns a table with:
  - `units_per_em`, `ascender`, `descender`, `line_gap` (integers)
  - `x_height`, `cap_height` (integer or nil — OS/2 v2+)
  - `num_glyphs` (integer)
  - `family_name`, `subfamily_name` (strings, fallback `"(unknown)"`)

- **`glyph_id(font, codepoint)`** — Maps a Unicode BMP codepoint to a glyph
  index. Returns `nil` for out-of-range or unmapped codepoints.

- **`glyph_metrics(font, glyph_id)`** — Returns `{advance_width, left_side_bearing}`
  or `nil` for out-of-range glyph IDs.

- **`kerning(font, left, right)`** — Returns kern value (integer) or `0`.

### Implementation notes

- `string.pack(">I2", v)` / `string.unpack(">I2", data, off+1)` handle all
  big-endian reads — no manual bit arithmetic needed.
- cmap Format 4 idRangeOffset absolute byte address:
  `iro_abs + id_range_offset + (cp - start_code) * 2`
- kern Format 0 coverage: format is in the HIGH byte (`coverage >> 8`).
- UTF-16 BE decoded manually for family/subfamily names; handles BMP correctly
  with U+FFFD replacement for surrogates.
- 30 tests, 0 failures.

# Changelog — font-parser-ruby (Rust)

## [0.1.0] — 2026-04-01

### Added

- Initial release — Ruby C extension wrapping the Rust `font-parser` core.

- **`CodingAdventures::FontParserNative.load(data) → FontFile`** — Parses a
  font from a binary String. Returns an opaque Data object. Raises
  `RuntimeError` on parse failure.

- **`CodingAdventures::FontParserNative.font_metrics(font) → Hash`** — Returns
  a Hash with Symbol keys: `:units_per_em`, `:ascender`, `:descender`,
  `:line_gap`, `:x_height` (Integer | nil), `:cap_height` (Integer | nil),
  `:num_glyphs`, `:family_name`, `:subfamily_name`.

- **`CodingAdventures::FontParserNative.glyph_id(font, cp) → Integer | nil`**
  — Maps a Unicode codepoint to a glyph ID. Returns `nil` if unmapped.

- **`CodingAdventures::FontParserNative.glyph_metrics(font, gid) → Hash | nil`**
  — Returns a Hash with `:advance_width` and `:left_side_bearing`. Returns
  `nil` for out-of-range glyph IDs.

- **`CodingAdventures::FontParserNative.kerning(font, left, right) → Integer`**
  — Returns the kern value for a pair of glyph IDs; 0 when not found.

### Implementation notes

- Uses `rb_data_object_wrap` (via ruby-bridge) to store `Box<FontFile>` in a
  Ruby Data object. GC-safe: destructor calls `Box::from_raw` to drop Rust mem.
- Data pointer stored/read at RData offset 4 (word 4 = `data` field in RData).
- Hash keys are Ruby Symbols via `rb_intern` + `rb_id2sym`.
- `crate-type = ["cdylib"]`, lib name `font_parser_native` — matches the
  `Init_font_parser_native` entry point and the `.so`/`.bundle` file name.

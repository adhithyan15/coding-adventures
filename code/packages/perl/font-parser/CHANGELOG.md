# Changelog — CodingAdventures::FontParser (Perl)

## [0.1.0] — 2026-04-01

### Added

- Initial release — metrics-only OpenType/TrueType font parser, zero CPAN
  dependencies (only core `Encode` module used for UTF-16 BE decoding).

- **`load($data)`** — Parses a binary font string. Returns a hashref on
  success (FontFile). Dies with a `CodingAdventures::FontParser::FontError`
  object on failure; check `$err->{kind}`: `BufferTooShort`, `InvalidMagic`,
  `TableNotFound`, `ParseError`.

- **`font_metrics($font)`** — Returns a hashref with `units_per_em`,
  `ascender`, `descender`, `line_gap`, `x_height` (undef if absent),
  `cap_height` (undef if absent), `num_glyphs`, `family_name`,
  `subfamily_name`.

- **`glyph_id($font, $codepoint)`** — Returns glyph index or `undef`.

- **`glyph_metrics($font, $glyph_id)`** — Returns `{advance_width,
  left_side_bearing}` or `undef`.

- **`kerning($font, $left, $right)`** — Returns kern value or `0`.

### Implementation notes

- `unpack('n', ...)` / `unpack('N', ...)` / `unpack('s>', ...)` for
  unsigned u16 / u32 / signed i16 big-endian reads with 0-based offsets.
- cmap Format 4 idRangeOffset: `iro_abs + iro + (cp - start_code) * 2`
- kern Format 0 coverage: format in HIGH byte (`coverage >> 8`).
- UTF-16 BE decoded via `Encode::decode('UTF-16BE', $raw)`.
- OS/2 sxHeight at offset 86, sCapHeight at offset 88.
- Test paths resolved via `dirname(abs_path(__FILE__))` so `prove` and
  direct invocation both find the Inter fixture.
- 30 tests, 0 failures.

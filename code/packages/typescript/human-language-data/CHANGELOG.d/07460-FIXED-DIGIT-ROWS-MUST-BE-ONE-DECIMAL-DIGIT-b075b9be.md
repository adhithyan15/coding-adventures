### Fixed — digit rows must be one decimal digit

- `validate()` now checks every script's `digits` rows. Since the Bengali
  inventory landed, a digit row's glyph counts as covered in headwords, so a
  malformed row (a letter, a two-digit string, a letter-number such as Ⅶ, or
  an empty glyph) could have hidden a real glyph gap. Each row must now be
  exactly one code point of Unicode category Nd; anything else is an error
  with code `invalid-digit-row` that names the script and the glyph.
- `uncoveredGlyphs` adds a digit row to the covered set only when it passes
  the same check, so a malformed row covers nothing even before the error is
  read.
- New exported helper `isSingleDecimalDigit` (counts code points, so an
  astral Nd digit is one). Tests cover good rows, four kinds of bad row, the
  no-cover rule and the helper. HL01 §"The round-trip validator" item 4 says
  so. The Bengali, Kannada, Malayalam and Telugu digit rows already pass.

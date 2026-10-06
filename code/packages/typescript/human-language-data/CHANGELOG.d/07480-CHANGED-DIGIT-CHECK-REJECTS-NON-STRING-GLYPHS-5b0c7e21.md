### Changed — the digit check rejects non-string glyphs

- `isSingleDecimalDigit` takes `unknown` and returns false for anything that is
  not a string, so a malformed digit row (a number or a missing glyph in hand-
  edited JSON) is reported as `invalid-digit-row` instead of crashing the
  validator with a TypeError.

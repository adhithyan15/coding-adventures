# minify_two_statements — two consecutive statements round-trip

Pins that two consecutive top-level statements emit on a single
line with no inserted whitespace between them, and exactly one
trailing newline at end-of-file. Catches:

- Statement separator regressions (e.g. emitter inserting `\n`
  or space between statements under WHITESPACE_ONLY).
- Trailing-newline drift (multiple trailing newlines, missing
  trailing newline, BOM).
- Null-literal round-trip (`null` not folded to `0` or similar).

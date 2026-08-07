# Changelog

## [0.1.0] - 2026-07-11

### Added

- Initial grammar-driven Rust APL tokenizer (MA05 §6, task MA-4c).
- Statically linked compiled token grammar (`code/grammars/apl/apl.tokens`),
  covering the historical-core subset fixed by MA-4a: dense numeric arrays,
  the primitive function glyphs, the three operators (reduce/scan/outer
  product), assignment, parenthesised grouping, and `⍝` line comments.
- No pre/post-tokenize hooks needed — every APL primitive in this subset is
  a single dedicated Unicode code point, so there is no character-overloading
  disambiguation to do at the lexer level (unlike MATLAB's `'` or Wolfram's
  bracketed-newline handling).

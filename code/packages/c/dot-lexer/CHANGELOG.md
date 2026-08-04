# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `dot-lexer` crate: a tokeniser for the
  Graphviz DOT language.
- `dot_tokenise(source)` returning a malloc'd `DotLexResult` (tokens + recoverable
  errors) freed with `dot_lex_result_free`.
- Byte-oriented scanner: whitespace/line/block-comment skipping (unterminated
  comment reported), case-insensitive keywords, punctuation, `->`/`--` edge
  operators, numerals, quoted strings (`\"` `\\` `\n` `\t` escapes), and
  balanced HTML strings; unexpected characters recover and continue.
- Overflow-guarded growable value buffer and token/error arrays; 1-based
  line/column tracking. ASCII bytes are stored verbatim (matching the Rust
  behaviour byte-for-byte).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): the crate's doc example
  plus keywords, punctuation, numerals, quotes, HTML, comments, and error
  recovery.

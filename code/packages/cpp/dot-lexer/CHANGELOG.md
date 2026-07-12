# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `dot-lexer` crate, in
  namespace `ca::dot`: a tokeniser for the Graphviz DOT language.
- `tokenise(source)` returning a `LexResult` (`std::vector<Token>` +
  `std::vector<LexError>`), recovering after errors.
- Byte-oriented scanner: whitespace/comment skipping, case-insensitive keywords,
  punctuation, `->`/`--`, numerals, quoted strings with escapes, and balanced
  HTML strings; unexpected characters recover and continue. 1-based line/column.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): the crate's doc example
  plus keywords, punctuation, numerals, quotes, HTML, comments, and error
  recovery.

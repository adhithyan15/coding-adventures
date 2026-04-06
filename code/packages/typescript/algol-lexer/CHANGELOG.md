# Changelog — @coding-adventures/algol-lexer

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-06

### Added

- Initial release of the ALGOL 60 lexer.
- `tokenizeAlgol(source: string): Token[]` — tokenizes ALGOL 60 source text using the grammar-driven `grammarTokenize` engine loaded with `algol.tokens`.
- Supports all ALGOL 60 token types: `INTEGER_LIT`, `REAL_LIT`, `STRING_LIT`, `IDENT`, all operators and delimiters, and 28 reserved keywords.
- Case-insensitive keyword recognition: `BEGIN`, `Begin`, `begin` all produce the token type `begin`.
- Comment skipping: `comment <text>;` is silently consumed; no token is emitted.
- Operator disambiguation: `:=` before `:`, `**` before `*`, `<=` before `<`, `>=` before `>`, `!=` standalone.
- Maximum-munch identifier matching: `beginning` is `IDENT`, not `begin + ning`.
- Position tracking: every token carries `line` and `column` (1-indexed).
- Comprehensive test suite with 50+ tests covering all token types, edge cases, and position tracking.

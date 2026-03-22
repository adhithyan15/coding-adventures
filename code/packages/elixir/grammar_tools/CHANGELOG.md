# Changelog

## [0.2.0] - 2026-03-21

### Added
- Pattern group support: `group NAME:` sections in `.tokens` files for context-sensitive lexing.
- `groups` field on `TokenGrammar` struct — a map from group name to pattern group (with `name` and `definitions`).
- `effective_token_names/1` function — returns token names as the parser sees them (aliases replace original names).
- `token_names/1` now includes names from all pattern groups and handles aliases.
- Group name validation: must match `[a-z_][a-z0-9_]*`, rejects reserved names (`default`, `skip`, `keywords`, `reserved`, `errors`), rejects duplicates.
- Group definition parsing: same `NAME = /pattern/` or `NAME = "literal"` format as other sections, with `-> ALIAS` support.
- Comprehensive test coverage for pattern groups (parsing, aliases, error cases).

## [0.1.0] - 2026-03-20

### Added
- Initial release — port of the Python grammar-tools package to Elixir.
- `TokenGrammar` module: parses `.tokens` files into structured data.
- `ParserGrammar` module: parses `.grammar` files (EBNF notation).
- `CrossValidator` module: validates token/grammar cross-references.
- Full extended format support: skip, aliases, reserved, mode directives.

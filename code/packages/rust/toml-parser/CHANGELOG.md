# Changelog

All notable changes to this project will be documented in this file.

## [0.1.2] - 2026-08-03

### Added

- Add `try_create_toml_parser` and `try_parse_toml` so malformed TOML returns typed lexical or syntax errors instead of panicking.
- Keep fallible error diagnostics payload-blind: source text and token values are never retained or displayed.

## [0.1.1] - 2026-07-18

### Fixed
- **Security hardening**: `create_toml_parser` never called `GrammarParser::with_max_depth`, leaving every caller (including this crate's own `parse_toml`) exposed to a native-stack-overflow DoS from adversarial deeply-nested input. Added a `MAX_RULE_DEPTH = 165` cap, derived from independently measuring `toml.grammar`'s two distinct self-referential recursion shapes (nested arrays, nested inline tables) — binary search over candidate `with_max_depth` values against a 5000-deep adversarial input per shape. Both shapes land on the identical floor (236 safe / 237 crash). Cap sits ~30% below that. 3 new depth-guard regression tests.

## [0.1.0] - 2026-03-21

### Added

- Initial release of the TOML parser crate.
- `create_toml_parser()` factory function returning a `GrammarParser` configured for TOML.
- `parse_toml()` convenience function returning a `GrammarASTNode` directly.
- Loads the `toml.grammar` file at runtime from the shared `grammars/` directory.
- Depends on `coding-adventures-toml-lexer` for tokenization.
- Supports all TOML grammar rules: document, expression, keyval, key, simple_key, table_header, array_table_header, value, array, array_values, inline_table.
- 21 unit tests covering: simple key-value pairs, integer/boolean/float/datetime values, table headers, array-of-tables headers, dotted keys, inline tables, arrays (single-line and multi-line), multiple key-value pairs, empty documents, comment-only documents, nested table headers, literal string values, quoted table headers, and a full multi-section document integration test.

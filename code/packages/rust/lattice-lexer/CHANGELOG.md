# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of the Lattice CSS superset tokenizer
- `tokenize_lattice(source: &str) -> Vec<Token>` — tokenize Lattice source to a token list
- `create_lattice_lexer(source: &str) -> GrammarLexer` — create a reusable lexer instance
- Grammar-driven tokenization via `lattice.tokens` grammar file (read at compile time using `env!("CARGO_MANIFEST_DIR")`)
- Support for all Lattice-specific tokens: `VARIABLE` (`$name`), `AT_KEYWORD` (`@mixin`, `@if`, etc.), `FUNCTION` (`name(`), `IDENT`, `HASH`, `DIMENSION`, `PERCENTAGE`, `CUSTOM_PROPERTY`, `COLON_COLON`, `URL_TOKEN`
- Support for Lattice comparison operators: `EQUALS_EQUALS` (`==`), `NOT_EQUALS` (`!=`), `GREATER_EQUALS` (`>=`), `LESS_EQUALS` (`<=`)
- 20 unit tests covering all token types and edge cases

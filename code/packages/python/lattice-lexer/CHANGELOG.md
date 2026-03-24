# Changelog

## [0.1.0] - 2026-03-22

### Added

- Thin wrapper around `GrammarLexer` for Lattice tokenization
- Loads `lattice.tokens` grammar extending CSS with 5 new token types
- `VARIABLE` token for `$name` references
- `EQUALS_EQUALS`, `NOT_EQUALS`, `GREATER_EQUALS`, `LESS_EQUALS` comparison operators
- `LINE_COMMENT` skip pattern for `//` single-line comments
- `tokenize_lattice()` convenience function returning `list[Token]`
- `create_lattice_lexer()` for lower-level access to the `GrammarLexer` instance
- All CSS token types preserved unchanged from `css.tokens`

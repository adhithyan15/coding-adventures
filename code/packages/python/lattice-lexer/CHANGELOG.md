# Changelog

## [0.1.1] - 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_lattice_lexer` now imports a pre-compiled `_grammar` module instead of reading and parsing the `lattice.tokens` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/lattice_lexer/_grammar.py` predated a `grammar-tools` compiler update (missing the `ModeTransition`/`TransitionAction` fields and the `# ruff: noqa` header the compiler now emits) and was never imported by `tokenizer.py`, which always read `lattice.tokens` directly from disk. It has been regenerated fresh from the current grammar file and wired in.

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

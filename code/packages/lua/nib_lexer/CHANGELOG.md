# Changelog — coding-adventures-nib-lexer (Lua)

All notable changes to this package are documented here.

## Unreleased

### Fixed

- Eliminated runtime grammar loading: `tokenize` now requires a
  pre-compiled `_grammar` module instead of reading and parsing the
  `nib.tokens` file from `code/grammars/` on every call. The old code
  walked out of the installed package's own directory to a
  monorepo-relative path that a published LuaRocks package does not ship.
- Corrected this file, which previously contained an unrelated package's
  (`coding-adventures-starlark-lexer`) changelog content verbatim — a
  pre-existing copy-paste error unrelated to this fix.

## [0.1.0]

### Added

- Initial implementation of `coding_adventures.nib_lexer`.
- `tokenize(source)` — tokenizes a Nib string using the shared
  `nib.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` — returns the cached `TokenGrammar` for direct use.

# Changelog

All notable changes to the TOML lexer package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_toml_lexer` now imports a
  pre-compiled `_grammar` module instead of reading and parsing the
  `.tokens` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative
  path that a published PyPI package does not ship, so `pip install` +
  first use would raise `FileNotFoundError`.

## [0.1.0] - 2026-03-21

### Added
- Initial release of the TOML lexer thin wrapper.
- `tokenize_toml()` function for one-step tokenization of TOML text.
- `create_toml_lexer()` factory for creating configured `GrammarLexer` instances.
- Full TOML v1.0.0 token support: 4 string types, integers (decimal/hex/octal/binary),
  floats (decimal/scientific/inf/nan), booleans, 4 date/time types, bare keys,
  and all structural delimiters (=, ., ,, [, ], {, }).
- Newline sensitivity — NEWLINE tokens emitted (unlike JSON where whitespace is skipped).
- Comments skipped via skip pattern (hash to end of line).
- Comprehensive test suite targeting 95%+ coverage.

# Changelog

All notable changes to the Ruby Lexer package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_ruby_lexer` now imports a pre-compiled `_grammar` module instead of reading and parsing the `ruby.tokens` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/ruby_lexer/_grammar.py` predated a `grammar-tools` compiler update (missing the `ModeTransition`/`TransitionAction` fields and the `# ruff: noqa` header the compiler now emits) and was never imported by `tokenizer.py`, which always read `ruby.tokens` directly from disk. It has been regenerated fresh from the current grammar file and wired in. Its token set was unaffected — only the compiler's output format had drifted.

## [0.1.0] - 2026-03-18

### Added
- Initial release of the Ruby lexer package.
- `tokenize_ruby()` function that tokenizes Ruby source code using the grammar-driven lexer.
- `create_ruby_lexer()` factory function for creating a `GrammarLexer` configured for Ruby.
- Ruby token grammar file (`ruby.tokens`) with support for:
  - Ruby keywords: `def`, `end`, `if`, `else`, `elsif`, `puts`, `true`, `false`, `nil`, etc.
  - Ruby-specific operators: `..` (range), `=>` (hash rocket), `!=`, `<=`, `>=`
  - Standard operators: `+`, `-`, `*`, `/`, `=`, `==`
  - Literals: names, numbers, double-quoted strings with escape sequences
  - Delimiters: parentheses, commas, colons
- Comprehensive test suite with 80%+ coverage.

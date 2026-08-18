# Changelog

All notable changes to the JavaScript Lexer package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_excel_lexer` now imports a pre-compiled `_grammar` module instead of reading and parsing the `excel.tokens` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/excel_lexer/_grammar.py` predated a `grammar-tools` compiler update (missing the `ModeTransition`/`TransitionAction` fields and the `# ruff: noqa` header the compiler now emits) and was never imported by `tokenizer.py`, which always read `excel.tokens` directly from disk. It has been regenerated fresh from the current grammar file and wired in.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the JavaScript lexer package.
- `tokenize_excel()` function that tokenizes JavaScript source code using the grammar-driven lexer.
- `create_excel_lexer()` factory function for creating a `GrammarLexer` configured for JavaScript.
- JavaScript token grammar file (`excel.tokens`) with support for:
  - JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`, etc.
  - JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`
  - Delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
  - `$` in identifiers
- Comprehensive test suite with 80%+ coverage.

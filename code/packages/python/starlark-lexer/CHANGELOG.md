# Changelog

All notable changes to the Starlark Lexer package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_starlark_lexer` now imports a pre-compiled `_grammar` module instead of reading and parsing the `starlark.tokens` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- The pre-existing checked-in `src/starlark_lexer/_grammar.py` predated a `grammar-tools` compiler update (it was missing the `ModeTransition`/`TransitionAction` fields and the `# ruff: noqa` header the compiler now emits) and was never imported by `tokenizer.py`, which always read `starlark.tokens` directly from disk. It has been regenerated fresh from the current grammar file and wired in.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Starlark lexer package.
- `tokenize_starlark()` function that tokenizes Starlark source code using the grammar-driven lexer.
- `create_starlark_lexer()` factory function for creating a `GrammarLexer` configured for Starlark.
- Starlark token grammar file (`starlark.tokens`) with support for:
  - Indentation mode: automatic INDENT/DEDENT/NEWLINE token generation
  - Bracket suppression: INDENT/DEDENT/NEWLINE suppressed inside (), [], {}
  - Starlark keywords: `def`, `return`, `if`, `elif`, `else`, `for`, `in`, `pass`, `break`, `continue`, `load`, `and`, `or`, `not`, `lambda`, `True`, `False`, `None`
  - Reserved keywords that cause lex errors: `class`, `import`, `while`, `try`, `except`, `raise`, `with`, `as`, `assert`, `async`, `await`, `del`, `finally`, `from`, `global`, `is`, `nonlocal`, `yield`
  - Multi-character operators: `**`, `//`, `==`, `!=`, `<=`, `>=`, `<<`, `>>`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `**=`, `//=`, `<<=`, `>>=`
  - Single-character operators: `+`, `-`, `*`, `/`, `%`, `=`, `<`, `>`, `&`, `|`, `^`, `~`
  - Delimiters: `(`, `)`, `[`, `]`, `{`, `}`, `,`, `:`, `;`, `.`
  - String literals: double-quoted, single-quoted, triple-quoted, with optional r/b prefixes, all aliased to STRING
  - Numeric literals: integers (decimal, hex, octal), floats
  - Comment skipping
- Comprehensive test suite with 80%+ coverage.

# Changelog

All notable changes to the Starlark Lexer package will be documented in this file.

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

# Changelog

All notable changes to `coding_adventures_starlark_lexer` will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::StarlarkLexer.tokenize(source)` method that tokenizes Starlark source code
- Loads `starlark.tokens` grammar file and delegates to `GrammarLexer`
- Supports Starlark keywords: `and`, `break`, `continue`, `def`, `elif`, `else`, `for`, `if`, `in`, `lambda`, `load`, `not`, `or`, `pass`, `return`, `True`, `False`, `None`
- Reserved keyword detection: `class`, `import`, `while`, `try`, `except`, etc. raise errors
- Supports operators: `+`, `-`, `*`, `/`, `%`, `=`, `==`, `!=`, `<=`, `>=`, `+=`, `-=`, `*=`, `**`, `//`, `<<`, `>>`
- Supports delimiters: `(`, `)`, `[`, `]`, `{`, `}`, `,`, `:`, `;`, `.`
- Supports string literals (single, double, triple-quoted, raw, bytes)
- Supports integer and float literals (decimal, hex, octal)
- Indentation mode with INDENT/DEDENT/NEWLINE tokens
- Full test suite with SimpleCov coverage >= 80%

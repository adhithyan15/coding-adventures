# Changelog

All notable changes to `coding_adventures_python_lexer` will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- Initial release
- `CodingAdventures::PythonLexer.tokenize(source)` method that tokenizes Python source code
- Loads `python.tokens` grammar file and delegates to `GrammarLexer`
- Supports Python keywords: `if`, `else`, `elif`, `while`, `for`, `def`, `return`, `class`, `import`, `from`, `as`, `True`, `False`, `None`
- Supports operators: `+`, `-`, `*`, `/`, `=`, `==`
- Supports delimiters: `(`, `)`, `,`, `:`
- Supports string literals, numeric literals, and identifiers
- Full test suite with SimpleCov coverage >= 80%

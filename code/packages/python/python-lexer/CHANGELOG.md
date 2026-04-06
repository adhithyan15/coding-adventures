# Changelog

All notable changes to the Python Lexer package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Python lexer package.
- `tokenize_python(source, version)` function that tokenizes Python source code using versioned grammar files.
- `create_python_lexer(source, version)` factory function for creating a `GrammarLexer` configured for a specific Python version.
- `DEFAULT_VERSION` constant (`"3.12"`) and `SUPPORTED_VERSIONS` list.
- Support for six Python versions with individual grammar files:
  - Python 2.7: `print`/`exec` as keywords, `<>` operator
  - Python 3.0: `print`/`exec` become names, `nonlocal` keyword added
  - Python 3.6: f-string prefix, underscores in numeric literals
  - Python 3.8: walrus operator `:=`
  - Python 3.10: `match`/`case` soft keywords
  - Python 3.12: `type` soft keyword
- Per-version grammar caching to avoid re-parsing on subsequent calls.
- Indentation mode support: automatic INDENT/DEDENT/NEWLINE token generation.
- Bracket suppression: INDENT/DEDENT/NEWLINE suppressed inside (), [], {}.
- Type aliases: all string quoting styles aliased to STRING.
- Version validation with clear error messages for unsupported versions.
- PEP 561 type stub marker (`py.typed`).
- Comprehensive test suite with 80%+ coverage.

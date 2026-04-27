# Changelog

All notable changes to `coding_adventures_python_lexer` will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- **Version parameter**: `tokenize` now accepts a `version:` keyword argument
  (defaults to `"3.12"`) to select which Python grammar to use
- **Supported versions**: `"2.7"`, `"3.0"`, `"3.6"`, `"3.8"`, `"3.10"`, `"3.12"`
- **Versioned grammar loading**: loads grammar files from
  `code/grammars/python/python{version}.tokens` instead of the old
  single `python.tokens`
- **Per-version grammar caching**: parsed grammars are cached so subsequent
  calls with the same version skip file I/O and parsing overhead
- `DEFAULT_VERSION` constant (`"3.12"`)
- `SUPPORTED_VERSIONS` constant listing all supported version strings
- `grammar_path(version)` helper method
- `load_grammar(version)` method with caching and error handling
- `clear_cache!` method for testing
- `ArgumentError` raised for unsupported version strings

### Changed
- `tokenize(source)` signature changed to `tokenize(source, version: "3.12")`
  -- existing callers that pass no version argument are unaffected
- Removed the old unversioned `python.tokens` fallback from the Ruby package;
  all runtime grammar loading now goes through versioned Python grammars

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

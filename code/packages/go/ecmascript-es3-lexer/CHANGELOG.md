# Changelog

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Go ECMAScript ES3 lexer package.
- `TokenizeEs3()` function that tokenizes ES3 source code into token slices.
- `CreateEs3Lexer()` factory function for incremental tokenization.
- Loads `ecmascript/es3.tokens` grammar from `code/grammars/`.
- Supports ES3-specific tokens: `===`, `!==`, `try`, `catch`, `finally`, `throw`, `instanceof`.
- Capability cage restricts file system access to only the grammar file.
- Comprehensive test suite verifying ES3-specific lexical features.

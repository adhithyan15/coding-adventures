# Changelog

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Go ECMAScript ES1 lexer package.
- `TokenizeEs1()` function that tokenizes ES1 source code into token slices.
- `CreateEs1Lexer()` factory function for incremental tokenization.
- Loads `ecmascript/es1.tokens` grammar from `code/grammars/`.
- Capability cage restricts file system access to only the grammar file.
- Comprehensive test suite verifying ES1-specific lexical features.

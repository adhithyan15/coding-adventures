# Changelog

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Go ECMAScript ES5 lexer package.
- `TokenizeEs5()` function that tokenizes ES5 source code into token slices.
- `CreateEs5Lexer()` factory function for incremental tokenization.
- Loads `ecmascript/es5.tokens` grammar from `code/grammars/`.
- `debugger` recognized as a keyword (promoted from ES3 reserved word).
- Reduced future-reserved word list compared to ES3.
- Capability cage restricts file system access to only the grammar file.
- Comprehensive test suite verifying ES5-specific lexical features.

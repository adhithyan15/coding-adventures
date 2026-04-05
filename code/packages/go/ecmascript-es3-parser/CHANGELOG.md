# Changelog

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Go ECMAScript ES3 parser package.
- `ParseEs3()` function that parses ES3 source code into AST nodes.
- `CreateEs3Parser()` factory function.
- Loads `ecmascript/es3.grammar` from `code/grammars/`.
- Supports ES3-specific grammar: try/catch/finally/throw, ===, !==, instanceof.
- Delegates tokenization to the ecmascript-es3-lexer package.
- Capability cage restricts file system access to only the grammar file.
- Comprehensive test suite covering ES3 syntactic constructs.

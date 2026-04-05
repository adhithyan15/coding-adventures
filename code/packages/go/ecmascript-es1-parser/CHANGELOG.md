# Changelog

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Go ECMAScript ES1 parser package.
- `ParseEs1()` function that parses ES1 source code into AST nodes.
- `CreateEs1Parser()` factory function.
- Loads `ecmascript/es1.grammar` from `code/grammars/`.
- Delegates tokenization to the ecmascript-es1-lexer package.
- Capability cage restricts file system access to only the grammar file.
- Comprehensive test suite covering ES1 syntactic constructs.

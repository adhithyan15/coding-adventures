# Changelog

## [0.1.0] - 2026-04-05

### Added
- Initial release of the Go ECMAScript ES5 parser package.
- `ParseEs5()` function that parses ES5 source code into AST nodes.
- `CreateEs5Parser()` factory function.
- Loads `ecmascript/es5.grammar` from `code/grammars/`.
- Supports ES5-specific grammar: debugger statement, getter/setter properties.
- Delegates tokenization to the ecmascript-es5-lexer package.
- Capability cage restricts file system access to only the grammar file.
- Comprehensive test suite covering ES5 syntactic constructs.

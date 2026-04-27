# Changelog

All notable changes to the Java Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the TypeScript Java parser package.
- `parseJava(source, version?)` function that parses Java source code into generic `ASTNode` trees. The `version` parameter selects the Java edition: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` (default: `"21"`).
- `createJavaParser(source, version?)` function returning a configured `GrammarParser` instance before parsing begins.
- Loads `java{version}.grammar` files from `code/grammars/java/`.
- Delegates tokenization to `@coding-adventures/java-lexer`.
- Supports `var_declaration`, assignments, expression statements, and operator precedence.
- Clear error thrown for unrecognised version strings.
- Comprehensive test suite with v8 coverage.

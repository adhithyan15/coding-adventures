# Changelog

## [0.1.0] - 2026-04-11

### Added
- Initial release of the Go Java parser package.
- `ParseJava()` function that parses Java source code into generic `ASTNode` trees.
- `CreateJavaParser()` factory function.
- Version support for Java 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, and 21.
- Default version is Java 21 (latest LTS) when no version is specified.
- Grammar files loaded from `code/grammars/java/java{version}.grammar`.
- `required_capabilities.json` declaring all 10 allowed grammar file paths.
- Comprehensive tests for all supported versions plus error paths.

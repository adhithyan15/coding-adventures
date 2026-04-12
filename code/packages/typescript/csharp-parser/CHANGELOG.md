# Changelog

All notable changes to the C# Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-04-11

### Added
- Initial release of the TypeScript C# parser package.
- `parseCSharp(source, version?)` function that parses C# source code using the grammar-driven parser. The `version` parameter selects the C# edition: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`, `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"` (default: `"12.0"`).
- `createCSharpParser(source, version?)` function returning a configured `GrammarParser` instance before parsing begins. Useful when you need explicit control over the parse process.
- Loads `csharp{version}.grammar` files from `code/grammars/csharp/`.
- Uses `@coding-adventures/csharp-lexer` to tokenize input before parsing; both tokens and grammar file are selected by the same version string to guarantee consistency.
- Returns an `ASTNode` with `ruleName` of `"program"` at the root.
- Supports C# grammar rules: `var_declaration`, `expression_stmt`, `expression`, `term`, `factor`, `assignment`.
- Clear error thrown for unrecognised version strings.
- Comprehensive test suite with v8 coverage.

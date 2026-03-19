# Changelog

All notable changes to the TypeScript Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript TypeScript parser package.
- `parseTypescript()` function that parses TypeScript source code into generic `ASTNode` trees.
- Loads `typescript.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/typescript-lexer`.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with v8 coverage.

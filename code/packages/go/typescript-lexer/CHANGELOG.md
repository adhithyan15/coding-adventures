# Changelog

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go TypeScript lexer package.
- `TokenizeTypescript()` function that tokenizes TypeScript source code using the grammar-driven lexer.
- `CreateTypescriptLexer()` factory function.
- Loads `typescript.tokens` from `code/grammars/`.
- Supports TypeScript-specific keywords: `interface`, `type`, `number`, `string`, `boolean`, etc.

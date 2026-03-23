# Changelog

All notable changes to the TypeScript Lexer package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript lexer package.
- `tokenize_typescript()` function that tokenizes TypeScript source code using the grammar-driven lexer.
- `create_typescript_lexer()` factory function for creating a `GrammarLexer` configured for TypeScript.
- TypeScript token grammar file (`typescript.tokens`) with support for:
  - TypeScript-specific keywords: `interface`, `type`, `enum`, `namespace`, `declare`, `readonly`, `abstract`, `number`, `string`, `boolean`, `any`, `void`, `never`, `unknown`
  - All JavaScript keywords and operators inherited
  - Delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
  - `$` in identifiers
- Comprehensive test suite with 80%+ coverage.

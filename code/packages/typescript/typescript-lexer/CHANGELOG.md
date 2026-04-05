# Changelog

All notable changes to the TypeScript Lexer (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript TypeScript lexer package.
- `tokenizeTypescript()` function that tokenizes TypeScript source code using the grammar-driven lexer.
- Loads `typescript.tokens` grammar file from `code/grammars/`.
- Supports TypeScript-specific keywords: `interface`, `type`, `enum`, `namespace`, `declare`, `readonly`, `abstract`, `implements`, `number`, `string`, `boolean`, `any`, `void`, `never`, `unknown`.
- Inherits all JavaScript keywords and operators: `let`, `const`, `var`, `function`, `===`, `!==`, `=>`, etc.
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`.
- Supports `$` in identifiers.
- Comprehensive test suite with v8 coverage.

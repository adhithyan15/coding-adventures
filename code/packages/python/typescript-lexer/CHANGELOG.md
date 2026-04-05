# Changelog

All notable changes to the TypeScript Lexer package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version` parameter added to `tokenize_typescript()` and `create_typescript_lexer()`.
  Pass `"ts1.0"` through `"ts5.8"` to load the corresponding versioned grammar file.
  Omitting `version` (or passing `None` / `""`) continues to use the generic
  `typescript.tokens` grammar — backward compatible.
- `_resolve_tokens_path(version)` private helper that maps version strings to
  grammar file paths under `code/grammars/typescript/`.
- Raises `ValueError` with a clear message for unknown version strings.
- Version-specific tests covering all six supported versions plus error handling.

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

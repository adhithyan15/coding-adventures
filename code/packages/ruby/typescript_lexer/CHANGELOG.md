# Changelog

All notable changes to `coding_adventures_typescript_lexer` will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::TypescriptLexer.tokenize(source)` method that tokenizes TypeScript source code
- Loads `typescript.tokens` grammar file and delegates to `GrammarLexer`
- Supports TypeScript-specific keywords: `interface`, `type`, `enum`, `namespace`, `declare`, `readonly`, `abstract`, `number`, `string`, `boolean`, `any`, `void`, `never`, `unknown`
- Inherits all JavaScript keywords and operators
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
- Full test suite with SimpleCov coverage >= 80%

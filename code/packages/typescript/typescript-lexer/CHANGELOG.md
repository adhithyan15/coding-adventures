# Changelog

All notable changes to the TypeScript Lexer (TypeScript) package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `tokenizeTypescript(source, version?)` — optional `version` parameter accepting
  `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, or `"ts5.8"`.
  When omitted (or empty string), the generic `typescript.tokens` grammar is used
  (backwards-compatible with v0.1.x).
- `createTypescriptLexer(source, version?)` — new function returning a configured
  `GrammarLexer` instance before tokenization begins. Useful for attaching on-token
  callbacks for context-sensitive lexing.
- Versioned grammar support loads from `code/grammars/typescript/<version>.tokens`.
- Clear error thrown for unrecognised version strings.
- Expanded test suite covering all six TS version strings, empty-string version,
  `createTypescriptLexer`, and error cases.

### Changed
- `tokenizeTypescript` signature is now `(source: string, version?: string): Token[]`
  — fully backwards-compatible; existing callers with one argument are unaffected.

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

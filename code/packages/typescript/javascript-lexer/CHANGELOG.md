# Changelog

All notable changes to the JavaScript Lexer (TypeScript) package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `tokenizeJavascript(source, version?)` — optional `version` parameter accepting
  `"es1"`, `"es3"`, `"es5"`, `"es2015"` through `"es2025"`.
  When omitted (or empty string), the generic `javascript.tokens` grammar is used
  (backwards-compatible with v0.1.x).
- `createJavascriptLexer(source, version?)` — new function returning a configured
  `GrammarLexer` instance before tokenization begins. Useful for attaching on-token
  callbacks for context-sensitive lexing.
- Versioned grammar support loads from `code/grammars/ecmascript/<version>.tokens`.
- Clear error thrown for unrecognised version strings.
- Expanded test suite covering all supported ES version strings, empty-string version,
  `createJavascriptLexer`, and error cases.

### Changed
- `tokenizeJavascript` signature is now `(source: string, version?: string): Token[]`
  — fully backwards-compatible; existing callers with one argument are unaffected.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript JavaScript lexer package.
- `tokenizeJavascript()` function that tokenizes JavaScript source code using the grammar-driven lexer.
- Loads `javascript.tokens` grammar file from `code/grammars/`.
- Supports JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`, etc.
- Supports JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`.
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`.
- Supports `$` in identifiers.
- Comprehensive test suite with v8 coverage.

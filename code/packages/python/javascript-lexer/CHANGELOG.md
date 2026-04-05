# Changelog

All notable changes to the JavaScript Lexer package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version` parameter added to `tokenize_javascript()` and `create_javascript_lexer()`.
  Pass any of the 14 valid ECMAScript version strings to load the corresponding
  versioned grammar file from `code/grammars/ecmascript/`. Omitting `version`
  (or passing `None` / `""`) loads the generic `javascript.tokens` — backward compatible.
- `_resolve_tokens_path(version)` private helper mapping version strings to paths.
- `_VALID_VERSIONS` frozenset covering `"es1"`, `"es3"`, `"es5"`, and
  `"es2015"` through `"es2025"`.
- Raises `ValueError` with a clear message for unknown version strings.
- 18 new version-specific tests (one per version + error + factory); 100% coverage.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the JavaScript lexer package.
- `tokenize_javascript()` function that tokenizes JavaScript source code using the grammar-driven lexer.
- `create_javascript_lexer()` factory function for creating a `GrammarLexer` configured for JavaScript.
- JavaScript token grammar file (`javascript.tokens`) with support for:
  - JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`, etc.
  - JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`
  - Delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
  - `$` in identifiers
- Comprehensive test suite with 80%+ coverage.

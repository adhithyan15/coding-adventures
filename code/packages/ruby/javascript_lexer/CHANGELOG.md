# Changelog

All notable changes to `coding_adventures_javascript_lexer` will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version:` keyword argument on `CodingAdventures::JavascriptLexer.tokenize(source, version: nil)`
- `VALID_VERSIONS` constant listing all supported ECMAScript version strings: `"es1"`, `"es3"`, `"es5"`, `"es2015"` through `"es2025"`
- `resolve_tokens_path(version)` class method — resolves to `javascript.tokens` (generic) or `ecmascript/<version>.tokens` (versioned)
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Tests for all version-aware paths: path resolution, file existence, tokenization with version, error cases, and backward compatibility

### Changed
- `tokenize` signature changed from `tokenize(source)` to `tokenize(source, version: nil)` — fully backward compatible; existing callers require no changes

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::JavascriptLexer.tokenize(source)` method that tokenizes JavaScript source code
- Loads `javascript.tokens` grammar file and delegates to `GrammarLexer`
- Supports JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`
- Supports JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
- Full test suite with SimpleCov coverage >= 80%

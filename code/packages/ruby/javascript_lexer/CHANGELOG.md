# Changelog

All notable changes to `coding_adventures_javascript_lexer` will be documented in this file.

## [0.2.1] - 2026-08-17

### Fixed
- Regenerated `_grammar_es2024.rb`/`_grammar_es2025.rb`, which had drifted out of sync with the current `ecmascript/es2024.tokens`/`es2025.tokens` source (the ES2025 `REGEX` token pattern was missing the `u` unicode flag). Found by a new CI drift check that regenerates every `_grammar.rb` and fails on mismatch.

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

# Changelog

All notable changes to `coding_adventures_typescript_lexer` will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version:` keyword argument on `CodingAdventures::TypescriptLexer.tokenize(source, version: nil)`
- `VALID_VERSIONS` constant listing all supported TypeScript version strings: `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`
- `resolve_tokens_path(version)` class method — resolves to `typescript.tokens` (generic) or `typescript/<version>.tokens` (versioned)
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Tests for all version-aware paths: path resolution, file existence, tokenization with version, error cases, and backward compatibility

### Changed
- `tokenize` signature changed from `tokenize(source)` to `tokenize(source, version: nil)` — fully backward compatible; existing callers require no changes

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::TypescriptLexer.tokenize(source)` method that tokenizes TypeScript source code
- Loads `typescript.tokens` grammar file and delegates to `GrammarLexer`
- Supports TypeScript-specific keywords: `interface`, `type`, `enum`, `namespace`, `declare`, `readonly`, `abstract`, `number`, `string`, `boolean`, `any`, `void`, `never`, `unknown`
- Inherits all JavaScript keywords and operators
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
- Full test suite with SimpleCov coverage >= 80%

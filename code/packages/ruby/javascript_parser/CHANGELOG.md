# Changelog

All notable changes to `coding_adventures_javascript_parser` will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version:` keyword argument on `CodingAdventures::JavascriptParser.parse(source, version: nil)`
- `VALID_VERSIONS` constant listing all supported ECMAScript version strings: `"es1"`, `"es3"`, `"es5"`, `"es2015"` through `"es2025"`
- `resolve_grammar_path(version)` class method — resolves to `javascript.grammar` (generic) or `ecmascript/<version>.grammar` (versioned)
- The `version:` argument is forwarded to `JavascriptLexer.tokenize` so both the token grammar and parser grammar match
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Tests for all version-aware paths: path resolution, file existence, parsing with version, error cases, and backward compatibility

### Changed
- `parse` signature changed from `parse(source)` to `parse(source, version: nil)` — fully backward compatible; existing callers require no changes

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::JavascriptParser.parse(source)` method that parses JavaScript source code into ASTs
- Loads `javascript.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports `var_declaration` (let/const/var), assignments, expression statements
- Full test suite with SimpleCov coverage >= 80%

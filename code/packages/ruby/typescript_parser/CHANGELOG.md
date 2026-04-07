# Changelog

All notable changes to `coding_adventures_typescript_parser` will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version:` keyword argument on `CodingAdventures::TypescriptParser.parse(source, version: nil)`
- `VALID_VERSIONS` constant listing all supported TypeScript version strings: `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`
- `resolve_grammar_path(version)` class method — resolves to `typescript.grammar` (generic) or `typescript/<version>.grammar` (versioned)
- The `version:` argument is forwarded to `TypescriptLexer.tokenize` so both the token grammar and parser grammar match
- Raises `ArgumentError` with a descriptive message when an unknown version is given
- Tests for all version-aware paths: path resolution, file existence, parsing with version, error cases, and backward compatibility

### Changed
- `parse` signature changed from `parse(source)` to `parse(source, version: nil)` — fully backward compatible; existing callers require no changes

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::TypescriptParser.parse(source)` method that parses TypeScript source code into ASTs
- Loads `typescript.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports `var_declaration` (let/const/var), assignments, expression statements
- Full test suite with SimpleCov coverage >= 80%

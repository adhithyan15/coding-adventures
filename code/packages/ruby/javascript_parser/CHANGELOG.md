# Changelog

All notable changes to `coding_adventures_javascript_parser` will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::JavascriptParser.parse(source)` method that parses JavaScript source code into ASTs
- Loads `javascript.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports `var_declaration` (let/const/var), assignments, expression statements
- Full test suite with SimpleCov coverage >= 80%

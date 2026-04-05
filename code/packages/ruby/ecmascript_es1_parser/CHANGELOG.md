# Changelog

All notable changes to `coding_adventures_ecmascript_es1_parser` will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release
- `CodingAdventures::EcmascriptEs1Parser.parse(source)` method that parses ES1 source code into ASTs
- Loads `ecmascript/es1.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports var declarations, function declarations/expressions
- Supports all 14 ES1 statement types (if, while, for, switch, etc.)
- Supports full expression precedence chain
- Full test suite with SimpleCov coverage >= 80%

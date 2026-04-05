# Changelog

All notable changes to `coding_adventures_ecmascript_es3_parser` will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release
- `CodingAdventures::EcmascriptEs3Parser.parse(source)` method that parses ES3 source code into ASTs
- Loads `ecmascript/es3.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports ES3-specific try/catch/finally/throw statements
- Supports strict equality (===, !==) in expressions
- Supports `instanceof` in relational expressions
- Supports regex literals as primary expressions
- All ES1 statement types and expressions retained
- Full test suite with SimpleCov coverage >= 80%

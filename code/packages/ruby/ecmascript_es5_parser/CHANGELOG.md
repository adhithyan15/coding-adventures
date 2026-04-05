# Changelog

All notable changes to `coding_adventures_ecmascript_es5_parser` will be documented in this file.

## [0.1.0] - 2026-04-05

### Added
- Initial release
- `CodingAdventures::EcmascriptEs5Parser.parse(source)` method that parses ES5 source code into ASTs
- Loads `ecmascript/es5.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports ES5-specific `debugger` statement
- Supports getter/setter property definitions in object literals
- All ES3 features retained (try/catch/finally/throw, strict equality)
- All ES1 statement types and expressions retained
- Full test suite with SimpleCov coverage >= 80%

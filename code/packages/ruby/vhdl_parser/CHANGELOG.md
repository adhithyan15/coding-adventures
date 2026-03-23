# Changelog

All notable changes to `coding_adventures_vhdl_parser` will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release
- `CodingAdventures::VhdlParser.parse(source)` method that parses VHDL source code into ASTs
- Loads `vhdl.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports entity declarations, architecture bodies, signal assignments, process statements, if/elsif/else, expressions
- Full test suite with SimpleCov coverage >= 80%

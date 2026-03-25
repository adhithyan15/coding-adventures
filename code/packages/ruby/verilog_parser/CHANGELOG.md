# Changelog

All notable changes to `coding_adventures_verilog_parser` will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release
- `CodingAdventures::VerilogParser.parse(source)` method that parses Verilog HDL source code into ASTs
- Loads `verilog.grammar` grammar file and delegates to `GrammarDrivenParser`
- Supports module declarations with ports, continuous assignments, always blocks, case statements, expressions
- Full test suite with SimpleCov coverage >= 80%

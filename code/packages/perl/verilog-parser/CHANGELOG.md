# Changelog — CodingAdventures::VerilogParser

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- Initial implementation of the Verilog parser (hand-written recursive descent,
  IEEE 1364-2005 synthesizable subset).
- Tokenizes with `CodingAdventures::VerilogLexer` and builds an AST.
- Supported constructs:
  - Module declarations with ports and parameters
  - Wire/reg/integer declarations with bit widths
  - Continuous assignments (assign)
  - Always and initial blocks with sensitivity lists
  - If/else, case/casex/casez statements
  - For loops
  - Module instantiation with named/positional port connections
  - Generate regions with for-generate and if-generate
  - Function and task declarations
  - Full expression grammar: ternary, logical, bitwise, shift, arithmetic,
    power, unary, primary (numbers, names, bit-selects, concatenations)
- `CodingAdventures::VerilogParser::ASTNode` — leaf and inner node class.
- `parse_verilog($source)` convenience class method.
- `t/00-load.t` and `t/01-basic.t` test suites.
- `Makefile.PL`, `cpanfile`, `BUILD`, `README.md`, `CHANGELOG.md`,
  and `required_capabilities.json`.

# Changelog — CodingAdventures::VhdlParser

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- Initial implementation of the VHDL parser (hand-written recursive descent,
  IEEE 1076-2008 synthesizable subset).
- Tokenizes with `CodingAdventures::VhdlLexer` and builds an AST.
- Supported constructs:
  - Context items: library clauses and use clauses
  - Entity declarations with generic and port clauses
  - Architecture bodies with declarative and statement regions
  - Signal, constant, variable, and type declarations
  - Enumeration, array, and record type definitions
  - Component declarations
  - Concurrent statements: signal assignments (with waveforms),
    component instantiations, process statements, generate statements
    (for-generate and if-generate)
  - Sequential statements: signal assignments, variable assignments,
    if/elsif/else, case (with choices), for loops, return, null
  - Package declarations and package bodies
  - Function and procedure declarations
  - Full expression grammar: logical (and/or/xor/nand/nor/xnor),
    relational (=/\=/</<=/>/>=), shift (sll/srl/sla/sra/rol/ror),
    adding (+/-/&), multiplying (*/////mod/rem), unary (not/abs/-),
    power (**), primary (literals, names, aggregates, function calls,
    qualified expressions, type conversions, parenthesized expressions)
- `CodingAdventures::VhdlParser::ASTNode` — leaf and inner node class.
- `parse_vhdl($source)` convenience class method.
- `t/00-load.t` and `t/01-basic.t` test suites.
- `Makefile.PL`, `cpanfile`, `BUILD`, `BUILD_windows`, `README.md`,
  `CHANGELOG.md`, and `required_capabilities.json`.

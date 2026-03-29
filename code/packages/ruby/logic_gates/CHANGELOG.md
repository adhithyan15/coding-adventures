# Changelog

All notable changes to the coding_adventures_logic_gates gem will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gates
  (not_gate, and_gate, or_gate, xor_gate, nand_gate, nor_gate, xnor_gate) now
  delegate to shared CMOS gate instances (`CMOS_INVERTER`, `CMOS_AND`, etc.)
  from `CodingAdventures::Transistors` instead of using Ruby boolean operators.
- **New runtime dependency**: `coding_adventures_transistors >= 0.1.0` added to
  gemspec. `coding_adventures_transistors` is required at the top of the entry
  point before logic_gates loads.
- **BUILD updated**: transistors bundle install runs before logic_gates tests.
- **XNOR composition**: `xnor_gate` is implemented as
  `CMOS_INVERTER.evaluate_digital(CMOS_XOR.evaluate_digital(a, b))`.

## [0.1.0] - 2026-03-18

### Added

- Initial release of the Ruby logic gates gem (port of Python logic-gates package)
- Seven fundamental logic gates: NOT, AND, OR, XOR, NAND, NOR, XNOR
- NAND-derived gates proving functional completeness: nand_not, nand_and, nand_or, nand_xor
- Multi-input gate variants: and_n, or_n
- Input validation rejecting non-Integer types and values outside {0, 1}
- RBS type signatures for all public methods
- Comprehensive Minitest test suite with 80%+ coverage
- Knuth-style literate programming comments with truth tables and circuit diagrams

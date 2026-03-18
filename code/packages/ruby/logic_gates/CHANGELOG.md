# Changelog

All notable changes to the coding_adventures_logic_gates gem will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

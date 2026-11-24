## [Unreleased]

### Changed

- All 11 basic gates validate every argument and report the first invalid
  parameter under their own name; `nand_or`/`nand_xor`/`nand_and` now validate
  up front so errors never name an inner construction wire (`nand_or 1 2`
  reports `b`).
- Tests: literal truth tables for all 11 gates, NAND-built equals direct, and
  exhaustive rejection over [-1024,1024] plus min_int/max_int extremes in every
  argument position.
- README documents input validation and compile-time rejection of non-int types.

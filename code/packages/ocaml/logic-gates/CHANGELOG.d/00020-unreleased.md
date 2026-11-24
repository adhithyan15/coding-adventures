## [Unreleased] - 2026-11-24

Ships with the first published release (0.1.0). The OCAML07 representative-chain
contract pins every representative package at 0.1.0, so no version bump is
claimed here.

### Changed

- All 11 basic gates validate every argument and report the first invalid
  parameter under their own name; `nand_or`/`nand_xor`/`nand_and` now validate
  up front so errors never name an inner construction wire (`nand_or 1 2`
  reports `b`).
- Tests: literal truth tables for all 11 gates, NAND-built equals direct, and
  exhaustive rejection over [-1024,1024] plus min_int/max_int extremes in every
  argument position.
- README documents input validation and compile-time rejection of non-int types.

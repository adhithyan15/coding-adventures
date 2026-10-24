# Changelog

## [0.1.0] — 2026-10-24

### Added

- Checked DEC VAX-11/780 integer architectural simulator.
- Complete 64 KiB little-endian state with sixteen registers, PSL N/Z/V/C,
  halt, and exact installed-program lifecycle metadata.
- All general operand addressing modes, including PC immediate, absolute, and
  relative forms.
- Byte, word, and longword integer/data families plus branches, calls,
  returns, CASE, address operations, and stack operations.
- Typed atomic instruction and lifecycle faults, transactional bounded runs,
  memory-safe scalar traces, and public encoding helpers.
- Exhaustive family/addressing/control/fault tests with greater than 95 percent
  Rust line coverage.


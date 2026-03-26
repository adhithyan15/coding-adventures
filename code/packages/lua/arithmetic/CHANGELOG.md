# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Half adder: adds two single bits, producing sum and carry outputs
- Full adder: adds two bits plus a carry-in, chaining two half adders
- Ripple carry adder: chains N full adders to add N-bit binary numbers (LSB-first / little-endian)
- ALU (Arithmetic Logic Unit) with configurable bit width and six operations:
  - ADD: binary addition via ripple carry adder
  - SUB: subtraction via two's complement negation (A + NOT(B) + 1)
  - AND: bitwise AND across all bit positions
  - OR: bitwise OR across all bit positions
  - XOR: bitwise XOR across all bit positions
  - NOT: bitwise NOT of the A bus (unary)
- ALUResult type with four condition flags: zero, carry, negative, overflow
- Comprehensive busted test suite (57 tests) covering exhaustive truth tables, multi-bit arithmetic, flag verification, input validation, and 8-bit operations
- Literate programming style with inline explanations, truth tables, circuit diagrams, and two's complement tutorial

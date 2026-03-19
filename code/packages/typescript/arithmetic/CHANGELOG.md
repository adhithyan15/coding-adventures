# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port from the Python arithmetic package.
- `halfAdder(a, b)` — adds two single bits, returns `[sum, carry]`.
- `fullAdder(a, b, carryIn)` — adds two bits plus carry-in, returns `[sum, carryOut]`.
- `rippleCarryAdder(a, b, carryIn?)` — chains full adders for N-bit addition (LSB first).
- `ALU` class with configurable bit width and six operations: ADD, SUB, AND, OR, XOR, NOT.
- `ALUResult` interface with value, zero, carry, negative, and overflow flags.
- `ALUOp` enum for operation codes.
- Full test suite for adders and ALU with >80% coverage.
- Knuth-style literate programming comments throughout all source files.

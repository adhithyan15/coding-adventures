# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-03-22

### Added

- `shiftAndAddMultiplier(a, b)` — integer multiplication using the shift-and-add algorithm, with full per-step trace data for visualization. Demonstrates that multiplication is just repeated conditional addition of shifted values.
- `MultiplierStep` interface — captures each step: which multiplier bit, the partial product (shifted multiplicand), the running total, and carry.
- `MultiplierResult` interface — full result with product (double-width), original inputs, and step trace array.
- `twosComplementNegate` — now exported (was previously internal to ALU). Useful for visualizing how subtraction reduces to addition: `A - B = A + NOT(B) + 1`.
- 28 new tests for the multiplier covering basic multiplication, edge cases (×0, ×1, powers of 2), commutativity, different bit widths, trace data verification, and error handling.

## [0.2.0] - 2026-03-22

### Added

- `rippleCarryAdderTraced(a, b, carryIn?)` — same as `rippleCarryAdder` but captures per-adder intermediate state (`FullAdderSnapshot`) for visualization.
- `FullAdderSnapshot` interface — records `a`, `b`, `cIn`, `sum`, `cOut` for each full adder stage.
- `RippleCarryResult` interface — traced adder return type with `sum`, `carryOut`, and `adders` array.

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

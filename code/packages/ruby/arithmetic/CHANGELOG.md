# Changelog

All notable changes to the `coding_adventures_arithmetic` gem will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-18

### Added

- **Half adder** (`Arithmetic.half_adder`) -- adds two single bits, returns `AdderResult` with sum and carry.
- **Full adder** (`Arithmetic.full_adder`) -- adds two bits plus carry-in, returns `AdderResult`.
- **Ripple-carry adder** (`Arithmetic.ripple_carry_adder`) -- chains N full adders for N-bit addition, returns `RippleCarryResult` with bits array and carry-out.
- **ALU class** (`Arithmetic::ALU`) -- N-bit Arithmetic Logic Unit with configurable bit width.
- **ALU operations** -- ADD, SUB (two's complement), AND, OR, XOR, NOT via `ALUOp` module constants.
- **ALU status flags** -- zero, carry, negative, overflow in `ALUResult` data object.
- **Immutable return types** using `Data.define`: `AdderResult`, `RippleCarryResult`, `ALUResult`.
- **RBS type signatures** for adder functions.
- Comprehensive test suite with >80% code coverage.
- Knuth-style literate documentation throughout all source files.

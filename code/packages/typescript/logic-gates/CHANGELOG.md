# Changelog

## 0.1.4 - 2026-04-12

### Added

- **`xorN` multi-input XOR gate**: New variadic function `xorN(...bits: Bit[]): Bit`
  that reduces a sequence of bits via XOR (parity checker). Returns 1 if an odd
  number of inputs are 1. Handles 0 inputs (returns 0) and 1 input (identity).
  Exported from `src/index.ts` alongside `andN` and `orN`.
  Used by `intel8008-gatelevel` for the Parity flag: `P = NOT(xorN(...result_bits))`.

## 0.1.3 - 2026-03-29

### Changed

- **XNOR now delegates to `CMOSXnor`**: XNOR previously composed XOR and NOT at
  the logic-gates level (`_cmosNot.evaluateDigital(_cmosXor.evaluateDigital(a, b))`).
  It now delegates directly to the dedicated `CMOSXnor` instance
  (`_cmosXnor.evaluateDigital(a, b)`), the 8-transistor CMOS XNOR gate added in
  `@coding-adventures/transistors` v0.3.0.
- Added `CMOSXnor` to the import and `_cmosXnor` module-level singleton.

## 0.1.2 - 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gate functions
  (NOT, AND, OR, XOR, NAND, NOR, XNOR) now delegate their digital evaluation to
  CMOS gate instances from `@coding-adventures/transistors`. Singletons are
  created once at module load time (`_cmosNot`, `_cmosAnd`, etc.) so the full
  transistor physics simulation is exercised on every call.
- **New dependency**: `@coding-adventures/transistors` added to `package.json`
  via a local `file:../transistors` reference.
- **XNOR composition**: implemented as
  `_cmosNot.evaluateDigital(_cmosXor.evaluateDigital(a, b))`.

## 0.1.1 - 2026-03-22

### Fixed

- Fixed TypeScript type error in `priorityEncoder` where `reduce` initial value type conflicted with `Bit[]` array type.

## 0.1.0 - 2026-03-19

- Initial TypeScript port from Python implementation
- All seven fundamental gates: NOT, AND, OR, XOR, NAND, NOR, XNOR
- NAND-derived gates proving functional completeness: nandNot, nandAnd, nandOr, nandXor, nandNor, nandXnor
- Multi-input gates: andN, orN
- Multiplexer and demultiplexer: mux, dmux
- Sequential logic: SR latch, D latch, D flip-flop, register, shift register, counter
- Comprehensive test suite with exhaustive truth table verification
- TypeScript-native Bit type alias (0 | 1) with runtime validation

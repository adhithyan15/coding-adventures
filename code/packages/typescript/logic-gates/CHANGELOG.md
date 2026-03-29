# Changelog

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

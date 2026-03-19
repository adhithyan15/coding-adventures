# Changelog

## 0.1.0 - 2026-03-19

- Initial TypeScript port from Python implementation
- All seven fundamental gates: NOT, AND, OR, XOR, NAND, NOR, XNOR
- NAND-derived gates proving functional completeness: nandNot, nandAnd, nandOr, nandXor, nandNor, nandXnor
- Multi-input gates: andN, orN
- Multiplexer and demultiplexer: mux, dmux
- Sequential logic: SR latch, D latch, D flip-flop, register, shift register, counter
- Comprehensive test suite with exhaustive truth table verification
- TypeScript-native Bit type alias (0 | 1) with runtime validation

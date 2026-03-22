# Changelog

## 0.1.0 — 2026-03-21

### Added

- Initial release: TypeScript port of the Python intel4004-gatelevel package.
- `Intel4004GateLevel` CPU class with all 46 instructions routed through real logic gates.
- `GateALU` wrapping the arithmetic package's ALU(4) for add, subtract, complement, increment, decrement, bitwise AND/OR.
- `RegisterFile` (16 x 4-bit registers), `Accumulator`, and `CarryFlag` built from D flip-flops.
- `ProgramCounter` (12-bit) with half-adder increment chain.
- `HardwareStack` (3-level x 12-bit) with circular pointer.
- `RAM` (4 banks x 4 registers x 20 nibbles) built from flip-flops.
- `decode()` instruction decoder using AND/OR/NOT gate combinational logic.
- `intToBits()`/`bitsToInt()` bit conversion helpers (LSB-first ordering).
- 66 tests covering all instructions, components, I/O, subroutines, and end-to-end programs.

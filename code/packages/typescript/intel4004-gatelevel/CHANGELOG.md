# Changelog

## 0.2.0 — 2026-03-22

### Added

- **Native ALU trace emission**: `GateALU` now captures `ALUTrace` after every arithmetic operation via `rippleCarryAdderTraced()`. The `lastTrace` getter provides operation type, input/output bits, carry, and per-adder snapshots.
- `ALUTrace` interface with operation, inputA, inputB, carryIn, adders (FullAdderSnapshot[]), result, carryOut.
- `GateALU.clearTrace()` method — called at the start of each CPU step.
- **`GateTrace` extended**: `step()` now returns `decoded` (DecodedInstruction), `aluTrace` (ALUTrace | undefined), and `memoryAccess` (MemoryAccess | undefined) natively.
- `MemoryAccess` interface tracking register reads/writes, RAM reads/writes, and port reads during instruction execution.

### Changed

- `GateTrace` is no longer a minimal type — it now includes full decoded instruction and ALU/memory trace data, eliminating the need for external replay.

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

# Changelog

## 0.1.0 - 2026-04-12

### Added

Initial release of the Intel 8008 gate-level simulator.

- **`Intel8008GateLevel` class**: Full Intel 8008 simulator where every computation
  routes through logic gate functions (AND, OR, XOR, NOT) from the `logic-gates`
  and `arithmetic` packages. Drop-in replacement for `Intel8008Simulator` with the
  same public API (`run()`, `step()`, `reset()`, `loadProgram()`, `setInputPort()`,
  `getOutputPort()`) plus `gateCount()` for gate-call metrics.

- **`ProgramCounter` class** (`pc.ts`): 14-bit program counter that increments via
  a chain of 14 half-adders. Each half-adder uses one XOR gate (for the sum bit)
  and one AND gate (for the carry). Total: 28 gate calls per PC increment.

- **`PushDownStack` class** (`stack.ts`): 8-level push-down stack matching the
  8008's on-chip hardware stack. Each entry is a 14-bit register modeled via
  D flip-flop gate simulation (`dFlipFlop()` from logic-gates). Push (CALL)
  rotates entries down; pop (RETURN) rotates entries up. Entry 0 is always the
  current program counter.

- **`RegisterFile` class** (`registers.ts`): 7-register file (A, B, C, D, E, H, L)
  where each write routes through `dFlipFlop()` to model D flip-flop rising-edge
  behavior. The flip-flop state is maintained across calls for correct master-slave
  edge-triggered simulation.

- **`FlagRegister` class** (`registers.ts`): 4-bit flag register (CY, Z, S, P) with
  per-flag D flip-flop simulation. Supports selective updates: `update()` for all
  four flags, `updateCarryOnly()` for rotates, `updateWithoutCarry()` for INR/DCR.

- **`GateALU8` class** (`alu.ts`): 8-bit ALU routing arithmetic through the
  `arithmetic` package's `ALU(8)` (ripple-carry adder chain). Operations:
  - `add(a, b, cin)`: ripple-carry addition (40 gate calls)
  - `subtract(a, b, borrowIn)`: two's complement via 8 NOT gates + add
  - `bitwiseAnd/Or/Xor`: 8 parallel gate calls each
  - `increment(a)`: A + 1 via adder
  - `decrement(a)`: A + 0xFF via adder
  - `rotateLeftCircular/RightCircular`: pure wiring (0 arithmetic gates)
  - `rotateLeftCarry/RightCarry`: 9-bit rotation through carry flag
  - `computeFlags/flagsFromResult`: zero (NOR chain), sign (wire), parity (XOR tree + NOT)

- **`decode()` function** (`decoder.ts`): Combinational gate decoder mapping an 8-bit
  opcode to `DecoderOutput` control signals using only AND, OR, NOT gates. Handles
  all 8008 instruction groups and encoding ambiguities (0x76=HLT, 0x7C=JMP,
  0x7E=CAL, 0xFF=HLT).

- **`intToBits/bitsToInt/computeParity`** (`bits.ts`): Bit conversion utilities.
  `computeParity` uses `xorN` from logic-gates for the XOR reduction tree + NOT,
  computing even parity (P=1 when even number of 1-bits).

- **`src/index.ts`**: Re-exports all public API: `Intel8008GateLevel`, `Flags`,
  `Trace`, `ProgramCounter`, `PushDownStack`, `RegisterFile`, `FlagRegister`,
  `GateALU8`, `GateFlags`, `decode`, `DecoderOutput`, `intToBits`, `bitsToInt`,
  `computeParity`.

- **Test suite** (`tests/cpu.test.ts`): 127 tests across sub-components and full
  CPU instruction set. Includes cross-validation section that runs identical programs
  through both `Intel8008GateLevel` and `Intel8008Simulator` and asserts identical
  final register/flag state. 95.77% line coverage.

- **Dependency**: `@coding-adventures/transistors` (file:../transistors) added as
  an explicit transitive dependency to ensure it is available to logic-gates during
  vitest test runs.

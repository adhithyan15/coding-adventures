# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-04-12

### Added

- `xor_n(inputs: &[u8]) -> u8` — N-input XOR gate (parity reducer). Chains
  2-input XOR gates left-to-right via `fold`, returning 1 when an odd number
  of inputs are 1 (odd parity) and 0 for even parity. Used by the Intel 8008
  gate-level simulator to compute the Parity flag: `P = NOT(xor_n(result_bits))`.

## [0.3.0] - 2026-03-29

### Changed

- `xnor_gate` now delegates to `CMOSXnor::new(None).evaluate_digital(a, b)`
  instead of composing `CMOSXor` and `CMOSInverter` inline. Behaviour is
  identical; the call site is simpler and the transistor count (8) is now
  self-documenting through `CMOSXnor::TRANSISTOR_COUNT`.

## [0.2.0] - 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gates
  (`not_gate`, `and_gate`, `or_gate`, `xor_gate`, `nand_gate`, `nor_gate`,
  `xnor_gate`) now delegate their digital evaluation to CMOS gate models from
  the `transistors` crate. Each call instantiates a default-parameter CMOS gate
  and calls `evaluate_digital(...)`, routing the computation through transistor
  physics simulation.
- **New dependency**: `transistors = { path = "../transistors" }` added to
  `Cargo.toml`. Both packages are workspace members so no registry changes are
  needed.
- **XNOR composition**: `xnor_gate` chains `CMOSXor` and `CMOSInverter`:
  `NOT(XOR(a, b))`.

## [0.1.0] - 2026-03-18

### Added
- Seven fundamental logic gates: AND, OR, NOT, XOR, NAND, NOR, XNOR
- NAND-derived gates proving functional completeness: nand_not, nand_and, nand_or, nand_xor
- Multi-input gates: and_n, or_n
- Sequential logic: SR latch, D latch, D flip-flop
- Register (N-bit parallel storage)
- Shift register (serial-to-parallel conversion, left/right)
- Counter (binary counting with overflow)
- Comprehensive doc comments with truth tables, circuit diagrams, and hardware explanations
- Inline unit tests and integration test files
- Ported from Python logic-gates package

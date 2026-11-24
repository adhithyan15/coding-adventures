# Changelog

All notable changes to `swift/logic_gates` are documented here.

## [0.1.0] — 2026-03-29

### Added

**Primitive gates** (`Gates.swift`) — seven Boolean gates, each delegating to
the CMOS transistor simulation in `swift/transistors`:

- `notGate` — CMOS inverter
- `andGate` — CMOS AND
- `orGate` — CMOS OR
- `xorGate` — CMOS XOR
- `nandGate` — CMOS NAND
- `norGate` — CMOS NOR
- `xnorGate` — CMOS XNOR

**NAND functional completeness** — proofs that NAND alone implements any
Boolean function:

- `nandNot`, `nandAnd`, `nandOr`, `nandXor`

**Multi-input gates** — variadic reductions:

- `andN(_ inputs: [Int]) throws -> Int`
- `orN(_ inputs: [Int]) throws -> Int`

**Error handling** — `LogicGateError` enum with typed, descriptive cases:

- `.invalidBit(name:got:)` — input is not 0 or 1
- `.insufficientInputs(minimum:got:)` — fewer inputs than required
- `.invalidSelectLength(expected:got:)` — MUX select bits wrong count
- `.invalidEncoderInput(_:)` — encoder input is not one-hot

**Sequential circuits** (`Sequential.swift`):

- `srLatch` — SR latch via cross-coupled NOR gates; holds state between calls
- `dLatch` — gated D latch (transparent when enable=1, opaque when enable=0)
- `dFlipFlop` — master-slave D flip-flop; captures on rising edge (CLK 0→1)
- `register` — N-bit parallel register; loads all bits on rising clock edge
- `shiftRegister` — N-bit serial-in / parallel-out shift register (default 4-bit)
- `counter` — N-bit binary up-counter with overflow flag and synchronous reset (default 4-bit)

**Combinational circuits** (`Combinational.swift`):

- `mux2` — 2-to-1 multiplexer (gate-level: AND/OR/NOT)
- `mux4` — 4-to-1 multiplexer (two-stage MUX2 tree)
- `mux8` — 8-to-1 multiplexer (delegates to `muxN`)
- `muxN` — N-to-1 multiplexer for any power-of-2 input count (recursive)
- `demux` — 1-to-2^N demultiplexer
- `decoder` — N-to-2^N binary decoder (one-hot output)
- `encoder` — 2^N-to-N binary encoder (requires exactly one active input)
- `priorityEncoder` — priority encoder; highest-index active input wins; returns `(output, valid)` tuple
- `triState` — tri-state buffer; returns `Int?` where `nil` = high-impedance (high-Z)

**Tests** — 100+ test cases across three files:

- `GatesTests.swift` — truth tables for all 7 primitives, NAND-derived match
  primitives, multi-input reductions, error descriptions, version string
- `SequentialTests.swift` — SR latch all four truth-table entries + hold +
  invalid state; D latch transparent/opaque; DFF rising-edge capture; register
  load; shift register multi-step; counter sequence, overflow, reset
- `CombinationalTests.swift` — MUX2 exhaustive 8-combo; MUX4/MUX8/MUXN each
  input reachable; DEMUX 1-to-2 and 1-to-4; decoder 1/2/3-bit exhaustive;
  encoder 4-to-2 and 8-to-3; priority encoder with all active combinations;
  tri-state enable and disable

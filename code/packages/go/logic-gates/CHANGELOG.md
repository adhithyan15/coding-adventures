# Changelog

All notable changes to the `logic-gates` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.6.0] - 2026-04-12

### Added

- `XORn(inputs ...int) int` — N-input XOR gate via left-fold over two-input XOR
  - Requires at least 2 inputs (panics on fewer)
  - Key building block for parity computation: Intel 8008 parity = NOT(XORn(b0..b7))
  - P=1 means even parity (even count of 1-bits) on the 8008

## [0.5.0] - 2026-03-31

### Changed

- **Operations system integration**: All public gate functions and combinational/
  sequential circuit functions (`AND`, `OR`, `NOT`, `XOR`, `NAND`, `NOR`, `XNOR`,
  `NAND_NOT`, `NAND_AND`, `NAND_OR`, `NAND_XOR`, `ANDn`, `ORn`, `Mux2`, `Mux4`,
  `MuxN`, `Demux`, `Decoder`, `Encoder`, `PriorityEncoder`, `TriState`, `SRLatch`,
  `DLatch`, `DFlipFlop`, `Register`, `ShiftRegister`, `Counter`) are now wrapped
  with `StartNew[T]` from the package's Operations infrastructure. Multi-return
  functions use inline helper structs. Each call gains automatic timing,
  structured logging, and panic recovery.

## [0.4.0] - 2026-03-29

### Changed

- **XNOR now delegates to `CMOSXnor`**: XNOR previously composed XOR and NOT at
  the logic-gates level (`_cmosXor.EvaluateDigital` + `_cmosNot.EvaluateDigital`).
  It now delegates directly to `transistors.NewCMOSXnor(nil).EvaluateDigital`,
  the dedicated 8-transistor CMOS XNOR gate added in transistors v0.2.0.
- Added `_cmosXnor` package-level var alongside the existing gate singletons.

## [0.3.0] - 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gate functions
  (AND, OR, NOT, XOR, NAND, NOR, XNOR) now delegate their digital evaluation to
  the `transistors` package's CMOS gate models (`CMOSAnd`, `CMOSOr`,
  `CMOSInverter`, `CMOSXor`, `CMOSNand`, `CMOSNor`). The public API is unchanged
  — inputs and outputs are still 0/1 integers — but the implementation path now
  routes through transistor-physics simulation.
- **New dependency**: `go.mod` now requires
  `github.com/adhithyan15/coding-adventures/code/packages/go/transistors` via a
  local `replace` directive.
- **XNOR composition**: XNOR is implemented as NOT(XOR(a,b)) using the
  transistors-backed XOR and NOT, since the transistors package has no dedicated
  XNOR gate.
- **BUILD updated**: `cd ../transistors && go mod download` runs before tests to
  ensure the local transistors module is available.

## [0.2.0] - 2026-03-21

### Added

- **Combinational circuits** (`combinational.go`):
  - `Mux2`: 2-to-1 multiplexer built from AND, OR, NOT gates
  - `Mux4`: 4-to-1 multiplexer built from three 2:1 MUXes
  - `MuxN`: N-to-1 recursive multiplexer (N must be power of 2)
  - `Demux`: 1-to-N demultiplexer using decoder + AND gates
  - `Decoder`: N-to-2^N binary-to-one-hot decoder
  - `Encoder`: 2^N-to-N one-hot-to-binary encoder
  - `PriorityEncoder`: priority encoder returning (binary output, valid flag)
  - `TriState`: tri-state buffer returning `*int` (nil = high-impedance)
  - Full test suite in `combinational_test.go`

## [0.1.0] - 2026-03-18

### Added

- **Combinational logic** (`gates.go`):
  - Seven fundamental gates: AND, OR, NOT, XOR, NAND, NOR, XNOR
  - NAND-derived gates proving functional completeness: NAND_NOT, NAND_AND, NAND_OR, NAND_XOR
  - Multi-input gates: ANDn, ORn (variadic, 2+ inputs)
  - Input validation with panic on invalid binary digits
  - Knuth-style literate comments with truth tables, ASCII circuit diagrams, and hardware explanations

- **Sequential logic** (`sequential.go`):
  - SRLatch: two cross-coupled NOR gates with iterative settling simulation
  - DLatch: data latch built on SR latch with data/enable interface
  - DFlipFlop: master-slave edge-triggered flip-flop
  - Register: N-bit parallel storage (N flip-flops sharing a clock)
  - ShiftRegister: serial-in/serial-out with left and right shift support
  - Counter: N-bit binary counter with ripple-carry increment and synchronous reset
  - FlipFlopState and CounterState types for state management

- **Tests** achieving 100% statement coverage:
  - `gates_test.go`: full truth tables for all 7 gates, NAND-derived equivalence proofs, multi-input tests, input validation panic tests
  - `sequential_test.go`: SR latch (set, reset, hold, invalid), D latch (transparent, hold, follow), D flip-flop (edge capture, store, nil init, data change, hold), register (store, hold, overwrite), shift register (left, right, single-bit), counter (count up, wrap, reset, hold, single-bit, empty init)

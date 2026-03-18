# Changelog

All notable changes to the `logic-gates` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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

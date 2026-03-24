# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- **Combinational gates**: AND, OR, NOT, XOR, NAND, NOR, XNOR
  - All gates validate inputs are 0 or 1
  - Use Lua 5.4 native bitwise operators (&, |, ~)
- **NAND-derived gates**: NAND_NOT, NAND_AND, NAND_OR, NAND_XOR
  - Proves functional completeness (any gate from NAND alone)
  - Gate counts: NOT=1, AND=2, OR=3, XOR=4 NAND gates
- **Multi-input gates**: ANDn, ORn
  - Variadic functions using Lua's `...` syntax
  - Require at least 2 inputs
- **Sequential logic**: SRLatch, DLatch, DFlipFlop, Register, ShiftRegister, Counter
  - SR latch with iterative convergence (NOR-based cross-coupling)
  - D latch eliminates invalid SR state
  - Master-slave D flip-flop for edge-triggered storage
  - N-bit register (parallel flip-flops)
  - Shift register with left/right direction
  - Binary counter with ripple carry and async reset
- **State constructors**: new_flip_flop_state, new_counter_state
- Comprehensive busted tests with full truth table coverage

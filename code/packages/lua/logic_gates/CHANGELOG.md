# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-03-29

### Changed

- `XNOR` now delegates directly to `_cmos_xnor:evaluate_digital` (a dedicated
  `CMOSXnor` instance) rather than composing `_cmos_xor` and `_cmos_not` inline.
  Added `_cmos_xnor` module-level singleton alongside the other CMOS instances.

## [0.2.0] - 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gates (AND, OR,
  NOT, XOR, NAND, NOR, XNOR) now delegate their digital evaluation to CMOS gate
  instances from `coding_adventures.transistors.cmos_gates`. Module-level
  singleton instances (`_cmos_not`, `_cmos_and`, etc.) are created once at load
  time to avoid per-call allocation overhead.
- **New dependency**: `coding-adventures-transistors >= 0.1.0` added to the
  rockspec `dependencies` table.
- **BUILD updated**: `luarocks make` for the transistors rockspec runs before the
  logic_gates test suite.
- **XNOR composition**: implemented as NOT of XOR using the transistors-backed
  CMOS inverter and XOR.

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

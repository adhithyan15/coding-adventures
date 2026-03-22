# Changelog

## 0.1.0 — 2026-03-21

### Added

- All seven fundamental logic gates: NOT, AND, OR, XOR, NAND, NOR, XNOR
- NAND-derived gates proving functional completeness: nand_not, nand_and, nand_or, nand_xor
- Multi-input variants: and_n, or_n
- Sequential logic elements:
  - SR latch (cross-coupled NOR gates with iterative feedback simulation)
  - D latch (controlled 1-bit memory with enable signal)
  - D flip-flop (master-slave edge-triggered design)
  - Register (N-bit parallel flip-flop array)
  - Shift register (serial-to-parallel with left/right direction)
  - Counter (ripple-carry binary counter with reset)
- Input validation: rejects booleans, non-integers, and out-of-range values
- Comprehensive test suite with truth table verification for all gates
- Literate programming style with explanations, diagrams, and analogies

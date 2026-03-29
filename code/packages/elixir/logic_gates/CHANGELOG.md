# Changelog

## 0.3.0 — 2026-03-29

### Changed

- `xnor_gate/2` now delegates to `CMOSGates.xnor_evaluate_digital/2` (the
  dedicated CMOS XNOR gate in the transistors package) instead of composing
  `inverter_evaluate_digital(xor_evaluate_digital(a, b))` inline. Behaviour
  is identical; the transistor count is now reported as XOR + Inverter (8).

## 0.2.0 — 2026-03-28

### Changed

- **Transistor-backed gate implementations**: All seven primitive gates
  (not_gate, and_gate, or_gate, xor_gate, nand_gate, nor_gate, xnor_gate) now
  delegate to `CodingAdventures.Transistors.CMOSGates` instead of using Elixir
  bitwise operators directly. The simulation now routes through CMOS transistor
  physics: NOT uses `inverter_evaluate_digital/1`, AND/OR use the 6-transistor
  compound gates, NAND/NOR use the 4-transistor natural CMOS primitives, XOR
  uses the 4-NAND construction, and XNOR is composed as NOT(XOR(a,b)).
- **New dependency**: `mix.exs` now declares
  `{:coding_adventures_transistors, path: "../transistors"}`.
- **BUILD updated**: transistors is compiled first before logic_gates tests run.

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

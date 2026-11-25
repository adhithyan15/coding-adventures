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

## 0.3.0 — 2026-03-29

### Changed

- `xnor_gate/2` now delegates to `CMOSGates.xnor_evaluate_digital/2` (the
  dedicated CMOS XNOR gate in the transistors package) instead of composing
  `inverter_evaluate_digital(xor_evaluate_digital(a, b))` inline. Behaviour
  is identical; the transistor count is now reported as XOR + Inverter (8).

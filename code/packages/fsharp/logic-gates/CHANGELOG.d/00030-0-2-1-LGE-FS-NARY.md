## 0.2.1

- LGE-FS-NARY: `andN` / `orN` / `xorN` now cap input length at the public
  `LogicGates.maxInputs = 65536`; longer lists raise `ArgumentException`
  (param `inputs`). The length walk stops after 65537 cells.
- BREAKING: a bad element is now reported as
  `ArgumentOutOfRangeException` with parameter name `inputs[i]` (first bad
  index) instead of `a` / `b` from the inner two-input gate.
- Ordering: null, too long, too short, then bit errors. Min-length rules kept
  (andN/orN >= 2, xorN >= 0). Previously, andN/orN could raise a bit error for
  an element before a later shape issue was visible; xorN had no cap.

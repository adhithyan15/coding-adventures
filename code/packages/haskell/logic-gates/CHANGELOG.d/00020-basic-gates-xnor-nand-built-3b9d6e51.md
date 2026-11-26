## 0.2.0

- Added `xnorGate` and the NAND-only constructions `nandNot`, `nandAnd`,
  `nandOr` and `nandXor` (classic 4-NAND XOR). All validate every argument.
- Literate documentation with truth tables and NAND circuit diagrams.
- Tests: full truth tables for all 11 basic gates, NAND-built equivalence
  checks, rejection of 2, -1, 255, 256, minBound and maxBound at every
  argument position, and a no-`Left`-on-valid-input check.

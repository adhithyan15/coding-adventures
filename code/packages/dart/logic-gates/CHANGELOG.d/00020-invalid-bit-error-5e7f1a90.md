## 0.2.0

- Add `InvalidBitError` (extends `ArgumentError`) carrying `name` and
  `invalidValue`; `validateBit` throws it.
- NAND-built gates (`NAND`, `NOR`, `XNOR`, `nand*`) validate their own
  arguments first, so errors name the caller's parameter, not an inner one.
- Add exhaustive tests (`gates_exhaustive_test.dart`), `dynamic` type-confusion
  tests, and a 95% branch/function coverage gate.

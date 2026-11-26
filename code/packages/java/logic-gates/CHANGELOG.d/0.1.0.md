# Changelog — logic-gates (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of all seven fundamental logic gates.
- `NOT(a)` — inverter; `AND(a,b)`, `OR(a,b)`, `XOR(a,b)` — basic gates.
- `NAND(a,b)`, `NOR(a,b)`, `XNOR(a,b)` — composite gates built from the fundamentals.
- `nandNOT(a)`, `nandAND(a,b)`, `nandOR(a,b)`, `nandXOR(a,b)` — NAND-only implementations proving functional completeness.
- `AND_N(int... inputs)` — multi-input AND, requires ≥ 2 inputs.
- `OR_N(int... inputs)` — multi-input OR, requires ≥ 2 inputs.
- `XOR_N(int... bits)` — N-input parity checker; accepts 0 or more inputs.
- Input validation: every gate rejects values outside {0, 1} with `IllegalArgumentException`.
- Literate-programming style source with inline truth tables, circuit analogies, and explanations.
- 60 unit tests covering: full truth tables for all 7 gates, NAND-derived parity with originals, multi-input variants (including parity), input validation, and De Morgan's Law cross-consistency checks.

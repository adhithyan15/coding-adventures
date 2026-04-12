# Changelog — coding-adventures-gf256 (Lua)

All notable changes to the Lua `gf256` package are documented here.

## [0.2.0] — 2026-04-11

### Added

- `new_field(polynomial)` function — parameterizable field factory that accepts any
  primitive polynomial and returns a table with the same API as the module.
  Uses Russian peasant (shift-and-XOR) multiplication; no log/antilog tables.
  - `gf.new_field(0x11B)` creates the AES GF(2^8) field.
  - `gf.new_field(0x11D)` matches the module-level functions (Reed-Solomon).
  - Returned table has: `multiply`, `divide`, `power`, `inverse`, `add`, `subtract`,
    and `polynomial` fields.
- Tests: AES sanity check (`0x53 × 0xCA = 1`), FIPS 197 Appendix B
  (`0x57 × 0x83 = 0xC1`), RS backward-compat, commutativity, error cases.

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of `coding_adventures.gf256` (MA01).
- `add(a, b)` — GF(256) addition = XOR; no tables needed; characteristic-2 field.
- `subtract(a, b)` — Identical to add in characteristic-2 (subtraction = addition = XOR).
- `multiply(a, b)` — Multiplication via log/antilog tables: O(1) two-lookup algorithm.
- `divide(a, b)` — Division via log/antilog tables; errors on division by zero.
- `power(base, exp)` — Exponentiation via log table; handles 0^0=1 and 0^n=0 by convention.
- `inverse(a)` — Multiplicative inverse via ALOG[255 - LOG[a]]; errors for a=0.
- Constants: `ZERO = 0`, `ONE = 1`, `PRIMITIVE_POLYNOMIAL = 0x11D`.
- LOG and ALOG tables built eagerly at module load time using the generator g=2
  and the primitive polynomial x^8 + x^4 + x^3 + x^2 + 1.
- Knuth-style literate comments: primitive polynomial derivation, cyclic group
  explanation, log/antilog construction algorithm with first-10-entries table.
- 44 busted unit tests covering all operations, field axioms, all-elements spot
  checks (all 255 non-zero elements have inverses; generator cycles through all 255).

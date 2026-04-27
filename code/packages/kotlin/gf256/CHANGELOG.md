# Changelog — gf256 (Kotlin)

## [0.1.0] — 2026-04-24

### Added

- `GF256` singleton object with `PRIMITIVE_POLY = 0x11D`
- Precomputed `EXP[512]` and `LOG[256]` tables built at object initialisation
- `add(a, b)` — bitwise XOR (characteristic-2 addition)
- `sub(a, b)` — bitwise XOR (identical to add in GF(2^n))
- `mul(a, b)` — multiplication via log/exp table lookup, O(1)
- `div(a, b)` — division using log tables; throws `ArithmeticException` on b=0
- `pow(base, n)` — exponentiation with modular group arithmetic; 0^0 = 1 by convention
- `inv(a)` — multiplicative inverse via `EXP[255 − LOG[a]]`; throws on a=0
- 37 unit tests covering field axioms, all operations, error cases, and spot-check vectors
- `VERSION = "0.1.0"` constant

### Notes

- Primitive polynomial `0x11D` = `x^8 + x^4 + x^3 + x^2 + 1` (Reed-Solomon standard)
- `EXP` table is 512 entries (doubled) to allow `LOG[a] + LOG[b]` without modular wrap
  in the hot-path `mul` implementation
- Matches TypeScript reference implementation `@coding-adventures/gf256` v0.1.0

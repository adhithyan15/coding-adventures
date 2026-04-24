# Changelog — gf256 (Java)

## 0.1.0 — 2026-04-24

### Added

- Initial implementation of `GF256.java` with the Reed-Solomon primitive polynomial `0x11D`.
- Precomputed `EXP_TABLE` (512 entries, doubled to avoid modular bounds check in `mul`)
  and `LOG_TABLE` (256 entries, sentinel `LOG[0] = -1`).
- Operations: `add`, `sub`, `mul`, `div`, `pow`, `inv`.
- Full JUnit Jupiter test suite (`GF256Test.java`) covering:
  - Table construction (first/last entries, doubled region, LOG/EXP inverses)
  - Field axioms (commutativity, associativity, distributivity of multiplication)
  - All non-zero elements appear exactly once in the generator cycle
  - Spec spot-check: `mul(0x53, 0x8C) = 1` under `0x11D`
  - Edge cases: zero divisor, zero base/exponent, negative exponent
- `BUILD` and `BUILD_windows` scripts for the monorepo build tool.

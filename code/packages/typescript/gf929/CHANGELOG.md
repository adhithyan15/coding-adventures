# Changelog

All notable changes to `@coding-adventures/gf929` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-24

### Added

- Initial implementation of GF(929) prime field arithmetic.
- `add(a, b)` — addition modulo 929.
- `subtract(a, b)` — subtraction modulo 929.
- `multiply(a, b)` — multiplication via log/antilog tables (O(1)).
- `divide(a, b)` — division using the multiplicative inverse.
- `inverse(a)` — multiplicative inverse via the discrete-log table.
- `power(base, exp)` — exponentiation via log tables.
- `zero()` and `one()` — convenience field-identity accessors.
- `isElement(v)` — type guard to validate field element range [0, 928].
- `EXP` — antilogarithm table: EXP[i] = 3^i mod 929 (length 929).
- `LOG` — logarithm table: LOG[v] = discrete log base 3 of v (length 929).
- Constants: `PRIME = 929`, `ORDER = 928`, `ALPHA = 3`.
- Full test suite with 70+ tests covering:
  - All field axioms (identity, commutativity, associativity, distributivity)
  - Table integrity (EXP/LOG round-trips for all 928 non-zero elements)
  - Fermat's little theorem verification
  - Known values from the PDF417 spec (α^3 = 27, α^4 = 81, α^7 = 329)
  - All edge cases (zero, one, boundary values)
  - Error conditions (division by zero, inverse of zero, invalid exponent)

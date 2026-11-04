## 0.556.0 - 2026-11-03 - Rationalized floor-root quotients

- Preserve nonzero static `sqrt` divisors inside the checked rational
  floor-root normalizer by rationalizing them into the radicand and denominator
  with checked arithmetic and exact common-factor cancellation.
- Cover positive, negative, sub-unit, dynamic, zero-divisor, and overflow
  boundaries in the compiler tests.

## 0.555.0 - 2026-11-03 - Rational floor-root products

- Preserve products of nonnegative static `sqrt` calls inside the existing
  checked rational floor-root normalizer by multiplying their radicands in
  `u128`. Rooted divisors, dynamic roots, and overflow continue to fail closed.
- Cover positive, negative, reduced-rational, dynamic, divisor, and overflow
  boundaries in the compiler tests.

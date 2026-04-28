# Changelog

## 0.1.0 — 2026-04-27

Initial release — polynomial factoring over Q[√d].

### Features

- `factor_over_extension(f_coeffs, d)` — top-level dispatcher that factors a
  polynomial over Q[√d].
- **Pattern 1**: Depressed monic quartics x⁴ + p·x² + q → two quadratic factors
  with coefficients in Q[√d], when q is a perfect rational square and
  (2s−p)/d is a non-negative perfect rational square.
- **Pattern 2**: Monic quadratics x² + bx + c → two linear factors when
  discriminant b²−4c equals d·(2β)² for rational β.
- `_is_rational_square(q)` — exact rational square root helper.
- `AlgFactor` VM handler (`alg_factor_handler`) registered under head
  `"AlgFactor"` in the symbolic VM.
- `build_alg_factor_handler_table()` — returns `{"AlgFactor": alg_factor_handler}`.
- Algebraic coefficients represented as `(rational_part, radical_part)` pairs
  of `Fraction` values.

### Examples

```
x⁴ + 1  over Q[√2]  →  (x² + √2·x + 1)(x² − √2·x + 1)
x² − 2  over Q[√2]  →  (x − √2)(x + √2)
x² − 3  over Q[√3]  →  (x − √3)(x + √3)
x² − 5  over Q[√5]  →  (x − √5)(x + √5)
x² + 1  over Q[√2]  →  irreducible (returns None)
```

### Scope

This phase covers the two most important splitting patterns.  Degree-3 and
degree-5+ polynomials over algebraic fields are deferred to a future release.

# Changelog — polynomial (Kotlin)

## [0.1.0] — 2026-04-24

### Added

- `normalize(p)` — strips trailing zero coefficients
- `degree(p)` — returns index of highest non-zero coefficient; -1 for zero polynomial
- `zero()` / `one()` — additive and multiplicative identity helpers
- `poly(vararg)` — convenience constructor with normalisation
- `add(a, b)` — coefficient-wise XOR (GF(256) addition)
- `sub(a, b)` — identical to add (subtraction = addition in characteristic-2 fields)
- `mul(a, b)` — polynomial convolution over GF(256) using `GF256.mul`
- `divmod(a, b)` — polynomial long division over GF(256); returns (quotient, remainder)
- `divide(a, b)` — quotient from divmod
- `mod(a, b)` — remainder from divmod (used by RS encoder)
- `eval(p, x)` — Horner's method evaluation at GF(256) element x (used by RS syndrome computation)
- `gcd(a, b)` — Euclidean GCD of two GF(256) polynomials
- 41 unit tests covering all operations, field axioms, RS generator verification, error cases
- `VERSION = "0.1.0"` constant

### Notes

- Coefficients are all GF(256) elements (0..255); arithmetic defers to `com.codingadventures.gf256.GF256`
- Dependency on `gf256` resolved as a local Gradle composite build via `includeBuild("../gf256")`
- Divergence from TypeScript MA00 `polynomial`: TypeScript MA00 uses real-number (f64) coefficients
  for a general polynomial library; this Kotlin package uses GF(256) coefficients to directly
  support MA02 Reed-Solomon (matching what Ruby/Python/Go implementations do)

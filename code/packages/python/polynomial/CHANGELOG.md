# Changelog — coding-adventures-polynomial

## [0.4.0] — 2026-04-19

### Added

- `resultant(a, b)` — scalar resultant of two polynomials via the
  Euclidean-PRS recurrence. Stays in the coefficient field throughout,
  which is exactly what the Q-coefficient CAS path wants. Vanishes iff
  `a` and `b` share a root. This is the primitive Rothstein–Trager
  builds on to find the log coefficients `c_i` of
  `∫ num/den dx = Σ c_i · log(v_i(x))`.
- `rational_roots(p)` — distinct rational roots of a polynomial in
  Q[z] via the Rational Roots Theorem. Returns a sorted list of
  `Fraction` values; empty when `p` is constant/zero or has no rational
  root. Used by RT to decide whether the log-part resultant has a
  closed form in Q.

See `rothstein-trager.md`.

## [0.3.0] — 2026-04-19

### Added

- `extended_gcd(a, b) → (g, s, t)` — extended Euclidean algorithm.
  Returns the GCD along with Bézout cofactors satisfying
  `s·a + t·b = g`. Over Q[x] this is the primitive Hermite reduction
  needs to invert `(U·V') mod V` when peeling a repeated factor off
  the denominator. See `hermite-reduction.md`.

## [0.2.0] — 2026-04-19

### Added

- `deriv(p)` — formal derivative `d/dx`, returns a polynomial whose
  coefficients are `(i · a_i)` for `i ≥ 1`.
- `monic(p)` — rescale so the leading coefficient is `1`. Requires a
  field (the coefficient ring must support dividing by the leading
  coefficient and staying in the ring).
- `squarefree(p)` — Yun's algorithm. Returns `[s_1, s_2, …, s_k]` such
  that `p = c · s_1 · s_2² · … · s_k^k` with every `s_i` monic,
  squarefree, and pairwise coprime. Uses only GCD + derivative — no
  irreducible factoring required, which is why it terminates quickly
  over Q[x].
- Explicit support for `fractions.Fraction` coefficients, documented
  both in the README and in a dedicated `TestFractionWorkflow` test
  class. This is the CAS path — Hermite reduction and
  Rothstein–Trager (Phase 2 of the symbolic integrator) will consume
  these functions.

### Changed

- Accumulator seeds in `multiply`, `divmod_poly`, and `evaluate`
  switched from `0.0` (float) to `0` (int). **Why:** seeding with
  `0.0` silently demotes `Fraction` to `float` (`0.0 + Fraction(1, 2)`
  yields `0.5`, not `Fraction(1, 2)`), which destroys exact arithmetic.
  Seeding with the integer `0` is the additive identity for every
  numeric type this package supports, so it preserves the caller's
  coefficient type cleanly. No behaviour change for `int` / `float`
  callers (existing gf256 and reed-solomon tests continue to pass
  unchanged).
- `evaluate`'s return type is no longer annotated as `float`; the
  Horner accumulator matches the ring of the inputs.

## [0.1.0] — 2026-04-03

### Added

- Initial implementation of polynomial arithmetic over real numbers.
- `normalize(p)` — strip trailing zero coefficients.
- `degree(p)` — highest non-zero index; -1 for zero polynomial.
- `zero()` / `one()` — additive and multiplicative identity polynomials.
- `add(a, b)` / `subtract(a, b)` — term-by-term arithmetic.
- `multiply(a, b)` — polynomial convolution.
- `divmod_poly(a, b)` — polynomial long division (named to avoid shadowing builtin).
- `divide(a, b)` / `mod(a, b)` — convenience wrappers for divmod_poly.
- `evaluate(p, x)` — Horner's method for fast evaluation.
- `gcd(a, b)` — Euclidean GCD algorithm.
- Comprehensive pytest test suite with >80% coverage.
- Literate programming docstrings with worked examples.

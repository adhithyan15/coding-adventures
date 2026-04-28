# Changelog

## 0.2.0 — 2026-04-27

**Phase 2 — Kronecker's algorithm for non-linear irreducible factors.**

Adds `cas_factor/kronecker.py` with:
- `_eval_points(n)` — generates the sequence 0, 1, −1, 2, −2, … .
- `_signed_divisors(n)` — all ±d for each positive divisor of |n|.
- `_lagrange_interpolate(xs, ys)` — exact Lagrange interpolation returning
  `list[Fraction]`; used to reconstruct candidate factor polynomials.
- `_poly_divmod_frac` / `_divides_exactly` — polynomial long division over
  Q; tests whether a candidate factor divides the target exactly with
  integer-coefficient quotient.
- `kronecker_factor(p)` — finds a non-trivial factor of degree
  1 ≤ k ≤ ⌊deg(p)/2⌋ by enumerating divisor combinations and
  Lagrange-interpolating.  Returns `(factor, cofactor)` or `None`.
  Safety cap: at most `_MAX_COMBOS = 10_000` combinations per degree trial.

Updates `factor.py`:
- `_factor_residual(residual)` — recursive work-queue that calls
  `kronecker_factor` and accumulates `(poly, multiplicity)` pairs.
- `factor_integer_polynomial` now calls `_factor_residual` instead of
  appending the residual as a single opaque factor.

Now handles cases not covered by Phase 1:
- **Sophie Germain identity**: `x^4 + 4 = (x^2+2x+2)(x^2−2x+2)`.
- **Cyclotomic identity**: `x^4+x^2+1 = (x^2+x+1)(x^2−x+1)`.
- **Repeated irreducibles**: `x^4+2x^2+1 = (x^2+1)^2`.
- **Mixed**: `(x^2+1)(x−2)` splits correctly.

Exports `kronecker_factor` from the package `__init__`.

New tests:
- `tests/test_kronecker.py` — 22 unit tests covering helpers and the
  full algorithm.
- `tests/test_factor.py` — 7 new integration tests (Sophie Germain,
  cyclotomic, repeated quadratic, mixed, `x^6−1`, irreducible regression).

## 0.1.0 — 2026-04-25

Initial release — Phase 1.

- ``content`` extraction (integer GCD of coefficients).
- Rational-root test for linear factors over Q.
- ``factor_integer_polynomial(coeffs)`` — orchestrates content +
  rational-root iteration; returns
  ``(content, [(factor_coeffs, multiplicity), ...])``.
- ``FACTOR``, ``IRREDUCIBLE`` head sentinels.

Deferred to Phase 2: Berlekamp factorization mod p, Hensel lifting,
Zassenhaus recombination.

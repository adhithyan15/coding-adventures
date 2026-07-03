# Changelog

## [0.3.0] — 2026-05-29

**Track K2 — n-variate Hensel lifting (TypeScript port).**

Ports the Python `cas_factor.hensel.try_n_variate_hensel` algorithm
(Track K1, PR #5590) to TypeScript.  Extends the bivariate Hensel
lift to n ≥ 3 variables by iterated bivariate lifting:

1. Pick a main variable `v_0`; substitute auxiliary variables with
   small integer values to reduce f to a univariate polynomial.
2. Factor the univariate image via the existing factor-uni-q chain.
3. Lift the univariate factors back one auxiliary variable at a time
   via Hensel-style expansion in powers of `(v_k − a_k)`.  Each lift
   step solves a coefficient-ring diophantine equation recursively;
   base case hits `uDiophantine` directly.
4. Verify the final product equals the input; return `null` on any
   mismatch so the caller falls through to other handlers.

Reuses the existing `tryBivariateHensel` machinery (rational types,
univariate diophantine, factor-uni-q) as building blocks.  Bounded
specialisation search (≤ 10 tuples); recursion depth bounded by `n`.

### Added

- `tryNVariateHensel(f: NPoly, numVars: number): NPoly[] | null` —
  top-level entry point for n-variate (n ≥ 2) factoring via iterated
  Hensel lifting.
- `NPoly` type — sparse n-variate polynomial as `Map<"e0,e1,…", Rational>`
  (comma-joined exponent tuple).
- `tests/n_variate_hensel.test.ts` — 13 acceptance cases mirroring the
  Python `test_n_variate_hensel.py` suite: trivariate quadratic, two
  trivariate cubics (sum-of-cubes companion, asymmetric coefficients),
  quadrivariate iterated lift, six fall-through cases, two bivariate
  regressions via the n-variate front door, and a bounded-resource
  smoke test.

## 0.2.0

**Track D2 — bivariate Hensel lifting (TypeScript port).**

Ports the Python `cas_factor.hensel` algorithm (Track D1, PR #4563) to
TypeScript.  `tryBivariateHensel(f: BiPoly) → BiPoly[] | null` factors a
bivariate polynomial in ℚ[x, y] by:

1. picking a lucky integer `y₀` so `f(x, y₀)` is squarefree with full
   x-degree,
2. univariately factoring the image via `factorIntegerPolynomial`,
3. lifting each factor back to ℚ[x, y] by solving the univariate
   diophantine equation `u·g₀ + v·h₀ = e_k` one y-layer at a time, and
4. verifying the final product reconstructs `f`.

Multi-factor inputs (image splits into r ≥ 2 univariate pieces) are
handled by iterated two-factor lift.

### Added

- New `src/hensel.ts` module:
  - `BiPoly` — sparse bivariate polynomial as `Map<"i,j", Rational>`.
  - `BiRational` (re-exported `Rational`) — exact bigint-backed rational
    with `add`/`sub`/`mul`/`div`/`pow`/`neg`/`equals`/`isZero`.
  - `tryBivariateHensel(f)` — top-level entry point.
- `tests/hensel.test.ts` — 6 acceptance cases (5 Hensel cases + 1
  univariate fall-through regression) mirroring the Python
  `test_hensel.py` suite exactly.

### Unreleased (carried)

- Bounded pure TypeScript Berlekamp-Zassenhaus/Hensel fallback for monic
  residuals that Kronecker does not split.

## 0.1.0

- Initial pure TypeScript port of `cas-factor` polynomial helpers, integer-root
  extraction, Kronecker splitting, and top-level factoring.

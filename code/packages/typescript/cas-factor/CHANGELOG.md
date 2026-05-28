# Changelog

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

# Changelog

## 0.4.0 — 2026-05-29

### Added

- **Track L2 — Lie point-symmetry handler for first-order ODEs**
  (`tryLieSymmetry` in new `src/lieSymmetry.ts`).  Port of the Python
  `cas_ode.lie_symmetry` module (Track L1, commit `d138e00f6`).
  - One generic detect-and-reduce pipeline for first-order ODEs that
    fall through every existing handler (Bernoulli, linear, separable,
    exact, homogeneous-type).  Three textbook point-symmetry groups
    are recognised numerically and reduced to a quadrature:
    1. **Translation in y**  `(x, y) → (x, y + c)` — `y' = f(x)` solved
       by direct integration `y = ∫ f dx + C`.
    2. **Translation in x**  `(x, y) → (x + c, y)` — autonomous
       `y' = g(y)` solved by the inverse quadrature
       `x = ∫ 1/g(y) dy + C`.  Catches the canonical logistic
       `y' = y(1 - y)` that separable cannot invert.
    3. **Scaling**  `(x, y) → (λx, λ^k y)` for integer `k ∈ [-3, 3] \ {0}`
       — similarity reduction `v = y / x^k` giving a separable ODE in
       `(v, x)`.  Numerical certificate confirms `f_subst / x^(k-1)`
       agrees with the extracted `G(v)` at three sample points before
       integration is attempted.
  - Detection is by *numerical invariance*: the candidate
    transformation is substituted into `f` and compared to the predicted
    transform at 3 fixed sample points × 3 sample `λ` (for scaling) ×
    7 candidate exponents = at most 63 numerical evaluations per ODE.
    No symbolic linearised determining equation is computed.
  - All iteration is bounded.  `k` search space is hard-bounded to
    `[-3, 3]` with no escape hatch.  Test points and tolerances are
    fixed constants.
  - Dispatcher hook: inserted in `solveOde` *after* every existing
    first-order family (Bernoulli, linear, separable, exact,
    homogeneous-type) and *before* the unevaluated fall-through.
    Matches the Python ordering in `cas_ode.ode.solve_ode`.
- Internal `LieOps` interface exposes the dispatcher's local helpers
  (`flattenAdd`, `isConstWrt`, `substIr`, arithmetic constructors) to
  the new module without duplicating logic.

## 0.3.0 — 2026-05-28

### Added

- **Track C2 — Frobenius / power-series ODE solver** (`tryFrobeniusSeries`):
  - Solves second-order linear ODEs `P(x)·y'' + Q(x)·y' + R(x)·y = 0` with a
    regular singular point at `x = 0` by substituting `y = x^r · Σ a_n x^n`.
  - `polyCoeffsExtract` — extract rational polynomial coefficients of `expr`
    in `x` up to arbitrary `maxDeg` (generalises the named-ODE `polynomialCoeffs`
    which is capped at degree 2).
  - `degreeOfMonomial` — recursive `[coeff, degree]` decomposition for a
    monomial `c·x^k`; handles n-ary `Mul`, `Neg`, `Pow(x, k≥0)`, rational
    literals.
  - `fracPolyMul` — truncated polynomial multiplication over `Frac`.
  - `isRegularSingular` — verify x=0 is a regular singular point and return
    the analytic Taylor series `tildeP = x·p(x)` and `tildeQ = x²·q(x)`.
    Computes `1 / P_eff(x)` (where `P_eff = P/x^m`) via the standard
    series-inverse recurrence; returns `null` for irregular singular points
    (order `m > 2`) or when the leading-zero analyticity condition fails.
  - `solveIndicial` — solve `r² + (p_0-1)r + q_0 = 0` for rational roots
    using `exactSqrt`; returns `null` on complex / irrational roots.
  - `rootsDifferByInteger` — guard for the logarithmic Frobenius case (bails).
  - `buildSeriesIr` — assemble `Mul(Pow(x, r), Add(a_0, a_1·x, …, a_N·x^N))`
    IR; signed `Frac` exponents encoded with `Neg(Rational(…))` for `r < 0`.
  - Dispatch order: inserted in `solveOde` after `tryVarCoeffNamedOde` and
    before `tryBernoulli`.  Mirrors the Python ordering: the named families
    (Bessel, Legendre, Hermite, Chebyshev) get first shot, and only un-named
    regular-singular-point ODEs reach the Frobenius helper.
  - Default truncation `N = 10`; recurrence is exact over `Frac` so the
    series matches the Python reference coefficient-for-coefficient.
- Scope (deliberate parity with Track C1):
  - Singular point at `x = 0` only.
  - Indicial roots must be rational and differ by a non-integer (the
    logarithmic / merged-root cases produce log terms and bail to `null`).
  - Irregular singular points (order of vanishing of `P` > 2) bail.

## 0.2.0 — 2026-05-16

### Added

- **Phase 21 — Variable-coefficient named ODE recognition** (`tryVarCoeffNamedOde`):
  - `collectVar2Coeffs` — extracts (P, Q, R) from `P(x)·y'' + Q(x)·y' + R(x)·y = 0`
    where coefficients are arbitrary IR expressions in x, not just rationals.
  - `splitOutFactor` — extracts coefficient K from `K·target` in Mul/Neg IR trees.
  - `evalAtXy` / `evalIrAtX` — recursive numeric IR evaluator at concrete float values.
  - `coeffMatchesFunc` — checks if an IR node ≈ expected analytic function at 4 test points.
  - `extractConstVal` — returns float value of constant (w.r.t. x) IR node.
  - `legendreNFromLambda` — finds non-negative integer n with n(n+1) = λ.
  - `nuFromRMinusXSq` — extracts rational ν from R(x) = x² − ν² (denominator ≤ 20).
  - `buildNamedSolution` — builds `Equal(y, %c1·F(n,x) + %c2·G(n,x))`.
  - `tryLegendreOde` — recognises `(1−x²)y'' − 2xy' + n(n+1)y = 0` → `LegendreP/Q(n,x)`.
  - `tryBesselOde` — recognises `x²y'' + xy' + (x²−ν²)y = 0` → `BesselJ/Y(ν,x)`.
  - `tryHermiteOde` — recognises `y'' − 2xy' + 2ny = 0` → `HermiteH/H2(n,x)`.
  - `tryChebyshevOde` — recognises `(1−x²)y'' − xy' + n²y = 0` → `ChebyshevT/U(n,x)`.
  - Dispatch order: Chebyshev → Legendre → Bessel → Hermite; inserted in `solveOde` after `tryEulerCauchy`.
- Bumped `@coding-adventures/symbolic-ir` peer to `^0.2.0` (requires new Phase 27 head symbols).

## 0.1.0

- Added the pure TypeScript symbolic ODE solver over `@coding-adventures/symbolic-ir`.
- Ported the main Python `cas-ode` families: first-order linear, separable,
  Bernoulli, exact, second-order constant-coefficient homogeneous and
  nonhomogeneous, Euler-Cauchy, and variation-of-parameters fallback.
- Added homogeneous-type `D(y,x) = f(y/x)` parity, including the degenerate
  `y = %c*x` case and symbolic implicit `Integrate(...)` fallback.

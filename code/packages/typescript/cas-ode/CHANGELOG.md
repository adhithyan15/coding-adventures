# Changelog

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

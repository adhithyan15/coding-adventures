# Changelog

## 0.1.0

- Added the pure TypeScript symbolic ODE solver over `@coding-adventures/symbolic-ir`.
- Ported the main Python `cas-ode` families: first-order linear, separable,
  Bernoulli, exact, second-order constant-coefficient homogeneous and
  nonhomogeneous, Euler-Cauchy, and variation-of-parameters fallback.
- Added homogeneous-type `D(y,x) = f(y/x)` parity, including the degenerate
  `y = %c*x` case and symbolic implicit `Integrate(...)` fallback.

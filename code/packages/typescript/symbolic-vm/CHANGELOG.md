# Changelog

## [0.3.0] — 2026-05-14

- Added EllipticE (second kind) integration recognition:
  - `∫₀^(π/2) sqrt(1-k²sin²θ) dθ` → `EllipticE(k)` (complete)
  - `∫ sqrt(1-k²sin²θ) dθ` → `EllipticE(θ, k)` (incomplete)
- Added EllipticPi (third kind) complete integration recognition:
  - `∫₀^(π/2) 1/((1+n·sin²θ)·sqrt(1-k²sin²θ)) dθ` → `EllipticPi(n, k)`
- New helper functions: `ellipticSecondKindRadicand`, `completeEllipticSecondKind`,
  `incompleteEllipticSecondKind`, `extractCharacteristicN`, `ellipticThirdKindParams`,
  `completeEllipticThirdKind`

## [0.2.0] — 2026-05-14

### Added

- Added symbolic integration recognition for the canonical elliptic first-kind
  forms, returning `EllipticF(theta, k)` and complete `EllipticK(k)` nodes.
- Added a bivariate perfect-cube factoring foothold so `Factor` recognises
  four-term binomial cube expansions: `x^3 + 3x^2y + 3xy^2 + y^3` as
  `(x+y)^3` and `x^3 - 3x^2y + 3xy^2 - y^3` as `(x-y)^3`.
- Added a canonical symbolic `Factor` handler backed by `cas-factor`, including
  a small common-symbolic-factor extraction pass for multivariate expressions
  like `x^2*y - y`.
- Extended the common multivariate factoring foothold to extract shared integer
  content as well as symbolic powers, so `2*x*y + 2*x*z` factors to
  `2*x*(y+z)`.
- Added a bivariate perfect-square factoring foothold so `Factor` recognises
  expressions like `x^2 + 2*x*y + y^2` as `(x+y)^2`.
- Added a bivariate difference-of-squares factoring foothold so `Factor`
  recognises expressions like `x^2 - y^2` as `(x-y)*(x+y)`.
- Added a bivariate cubic-identity factoring foothold so `Factor` recognises
  expressions like `x^3 - y^3` and `x^3 + y^3` as their textbook two-factor
  decompositions.
- Added a four-term bilinear grouping factoring foothold so `Factor`
  recognises expressions like `x*y + x*z + y + z` as `(x+1)*(y+z)`.
- Added a symbolic-backend-only `D` handler for pure IR differentiation,
  including arithmetic, power, elementary, hyperbolic, and inverse hyperbolic
  chain rules.
- Added reciprocal hyperbolic `Coth`, `Sech`, and `Csch` numeric handlers and
  derivative chain rules expressed via `Sinh`/`Cosh`.

## [0.1.0] - 2026-05-08

### Added

- Initial pure TypeScript symbolic VM.
- Strict and symbolic backends.
- Arithmetic, elementary numeric, comparison, logic, assignment, definition,
  list, and user-function application handlers.

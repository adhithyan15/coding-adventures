# Changelog

## [0.5.0] — 2026-05-18

### Added — Phase 28: general IBP for poly×log(Q) and poly×atan(Q)

Extends symbolic integration to handle products of a polynomial `P(x)` with
`log(Q(x))` or `atan(Q(x))` where `Q(x)` is a **non-linear** polynomial with
rational coefficients.  Uses the IBP formula:

  ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
  ∫ P·atan(Q) dx =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx

where R = ∫P (polynomial antiderivative, constant = 0).

**New functions:**

- `tryLogPolyProduct(transcendental, poly, x)` — Phase 28 log IBP handler;
  skips linear Q (deferred to Phase 3) and delegates the residual to
  `integrateRationalSimple`.
- `tryAtanPolyProduct(transcendental, poly, x)` — Phase 28 atan IBP handler;
  skips linear Q (deferred to Phase 11 if/when implemented) and delegates
  the residual to `integrateRationalSimple`.
- `integrateRationalSimple(N_ir, D_ir, x)` — targeted rational function
  integrator for Phase 28 residuals.  After polynomial long division:
  - **Case A**: remainder = c·D′ → c·log(D)
  - **Case B**: constant remainder / quadratic ax²+b with rational √(b/a)
                → r₀/(a₂·√(a₀/a₂))·atan(x/√(a₀/a₂))
- `closeRemainderOverD(R, D, D′, D_ir, x)` — attempts Cases A/B for the
  post-division remainder polynomial.
- `evalNumericNode(node)` — evaluates a closed IR numeric expression
  (handling MUL/DIV/NEG/ADD/SUB of exact rationals) to a `Numeric` value;
  used by `rpFromCoeffsMap` to extract rational coefficients from compound
  coefficient nodes produced by `toPolynomialCoeffs`.

**Rational polynomial arithmetic helpers** (used internally by Phase 28):
`rc`, `rcAdd`, `rcSub`, `rcMul`, `rcDiv`, `rcToIR`, `rpDeg`, `rpCoeff`,
`rpAdd`, `rpMul`, `rpDeriv`, `rpIntegrate`, `rpDiv`, `rpToIR`,
`rpFromCoeffsMap`, `rpProportional`, `bigIntSqrt`, `rcSqrt`, `isLinearIn`.

**Dispatch wiring:**
- MUL branch: after Phase 27, tries `tryLogPolyProduct(a,b,x)` and
  `tryAtanPolyProduct(a,b,x)` (and symmetric variants) for both-depend cases.
- Bare function path: `∫ log(Q) dx` (P=1) and `∫ atan(Q) dx` (P=1) are
  detected via head checks before the final `return undefined`.

**Examples that now evaluate:**
- `∫ log(x²+1) dx` = x·log(x²+1) − 2x + 2·atan(x)
- `∫ x·log(x²+1) dx` = (x²/2)·log(x²+1) − x²/2 + ½·log(x²+1)
- `∫ x²·log(x²+1) dx` = (x³/3)·log(x²+1) − 2x³/9 + 2x/3 − (2/3)·atan(x)
- `∫ x·atan(x²) dx` = (x²/2)·atan(x²) − ¼·log(1+x⁴)

**Fallthrough cases** (correctly left unevaluated):
- `∫ atan(x²) dx` — residual 2x²/(1+x⁴) requires irrational partial fractions
- `∫ x²·atan(x²) dx` — same reason

**Tests added:**
- `Phase 28: ∫ log(x²+1) dx` — closed-form structure and numerical check
- `Phase 28: ∫ x·log(x²+1) dx` — closed-form and numerical check
- `Phase 28: ∫ x²·log(x²+1) dx` — numerical check
- `Phase 28: ∫ atan(x²) dx fallthrough` — stays unevaluated
- `Phase 28: ∫ x·atan(x²) dx` — closed-form structure and numerical check
- `Phase 28: regression — ∫ log(x) dx still handled by Phase 3`
- `Phase 28: regression — ∫ atan(x) dx not intercepted by Phase 28`

## [0.4.0] — 2026-05-16

### Added — Phase 26: log-power integration via IBP reduction

- `polyLogPowerTerm(k, n, x)` — closed form of `∫ xᵏ · log(x)^n dx` for
  integer k ≥ 0, n ≥ 1, using the IBP reduction formula:
  `G_{k,m}(x) = x^(k+1)/(k+1) · log(x)^m − m/(k+1) · G_{k,m-1}(x)`.
- `tryLogPowerProduct(transcendental, poly, x)` — handles `∫ Q(x) · log(x)^n dx`
  for integer n ≥ 2 by decomposing Q(x) into monomials and applying
  `polyLogPowerTerm` term-by-term.
- `toPolynomialCoeffs(expr, x)` — utility that extracts a `Map<degree, coeff>`
  polynomial coefficient map from an IR expression; handles constants, `x`,
  `x^k`, `c·f`, `f·c`, ADD, SUB, NEG.
- Integration of standalone `log(x)^n` (n ≥ 2) via `polyLogPowerTerm(0, n, x)`.

### Added — Phase 27: trig-of-log integration via u = log(x) substitution

- `trigLogIntegral(trigHead, k, x)` — closed form of `∫ xᵏ · trig(log(x)) dx`
  via the identity `∫ e^((k+1)u) trig(u) du` (with u = log x):
  - `∫ xᵏ sin(log x) dx = x^(k+1)·((k+1)sin(log x)−cos(log x))/((k+1)²+1)`
  - `∫ xᵏ cos(log x) dx = x^(k+1)·((k+1)cos(log x)+sin(log x))/((k+1)²+1)`
- `tryTrigLogProduct(transcendental, poly, x)` — handles `∫ Q(x)·sin(log(x)) dx`
  and `∫ Q(x)·cos(log(x)) dx` by decomposing Q(x) and applying `trigLogIntegral`
  term-by-term.
- Integration of standalone `sin(log(x))` and `cos(log(x))` (k = 0 case):
  - `∫ sin(log x) dx = x/2·(sin(log x)−cos(log x))`
  - `∫ cos(log x) dx = x/2·(sin(log x)+cos(log x))`

## [0.3.1] — 2026-05-14

**Bug fix: elliptic modulus extraction now handles pre-evaluated numeric `k²`.**

`modulusFromSquaredFactor` previously only recognised `Pow(k, 2)` as the squared
modulus factor.  The MACSYMA compiler (and TypeScript IR evaluator) eagerly folds
`(1/2)^2` → `IRRational(1/4)` and `0.5^2` → `IRFloat(0.25)` before the
integration handler runs, so the pattern was never matched.

Extended the recogniser to extract `k` from:
- `IRFloat(v)` — returns `IRFloat(√v)`; e.g. `0.25` → `0.5`
- `IRRational(p/q)` where both numerator and denominator are perfect squares
  — returns `IRRational(√p / √q)`; e.g. `1/4` → `1/2`
- `IRInteger(n)` where `n` is a perfect square — returns `IRInteger(√n)`;
  e.g. `4` → `2`
- Non-perfect-square rationals/integers — falls back to `Sqrt(k²)` (unevaluated)

Added a new helper `bigIntIsqrt(n)` for exact integer square root over `bigint`.

Result: `integrate(sqrt(1-(1/2)^2*sin(theta)^2), theta, 0, %pi/2)` now returns
`EllipticE(1/2)` instead of falling through unevaluated.

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

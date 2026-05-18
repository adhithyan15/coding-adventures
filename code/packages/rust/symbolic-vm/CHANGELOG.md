# Changelog — symbolic-vm (Rust)

## [0.5.0] — 2026-05-18

### Added — Phase 28: general IBP for poly×log(Q) and poly×atan(Q)

Extends symbolic integration to handle products of a polynomial `P(x)` with
`log(Q(x))` or `atan(Q(x))` where `Q(x)` is a **non-linear** polynomial with
rational coefficients.  Uses the IBP formula:

  ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
  ∫ P·atan(Q) dx =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx

where R = ∫P (polynomial antiderivative, constant = 0).

**New functions:**

- `try_log_poly_product(transcendental, poly, x)` — Phase 28 log IBP handler;
  skips linear Q (deferred to Phase 3) and integrates the residual via
  `integrate_rational_simple_rp`.
- `try_atan_poly_product(transcendental, poly, x)` — Phase 28 atan IBP handler;
  skips linear Q and integrates the residual via `integrate_rational_simple_rp`.
- `integrate_rational_simple_rp(num_rp, denom_rp, denom_ir, x)` — targeted
  rational function integrator for Phase 28 residuals.  After polynomial long
  division:
  - **Case A**: remainder = c·D′ → c·log(D)
  - **Case B**: constant remainder / quadratic ax²+b with rational √(b/a)
                → r₀/(a₂·√(a₀/a₂))·atan(x/√(a₀/a₂))
- `close_remainder_over_d(r, d, d_prime, d_ir, x)` — attempts Cases A/B for
  the post-division remainder polynomial.
- `eval_numeric_node(node)` — evaluates a closed IR numeric expression
  (handling Mul/Div/Neg/Add/Sub of exact rationals) to a `RatC` value;
  used by `rp_from_poly_vec` to extract rational coefficients from compound
  coefficient nodes produced by `to_polynomial_coeffs`.
- `is_linear_in(expr, x)` — returns true iff the expression is a non-constant
  linear polynomial in `x`; used to guard the Phase 28 arms.

**Rational polynomial arithmetic helpers** (used internally by Phase 28):
`gcd128`, `rc`, `rc_neg`, `rc_add`, `rc_sub`, `rc_mul`, `rc_div`, `rc_to_ir`,
`eval_numeric_node`, `rp_from_poly_vec`, `rp_deg`, `rp_is_zero`, `rp_coeff`,
`rp_add`, `rp_sub_poly`, `rp_mul_scalar`, `rp_shift`, `rp_mul`, `rp_deriv`,
`rp_integrate`, `rp_div`, `rp_to_ir`, `rp_proportional`, `i128_sqrt`,
`rc_sqrt`.

The arithmetic layer uses `RatC = (i128, i128)` and `RatPoly = Vec<RatC>` with
`i128` to give headroom for cross-multiplications without overflow.

**Dispatch wiring:**
- MUL branch: after Phase 27, tries `try_log_poly_product(a,b,x)` and
  `try_atan_poly_product(a,b,x)` (and symmetric variants) for both-depend cases.
- Bare function path: `∫ log(Q) dx` (P=1) and `∫ atan(Q) dx` (P=1) are
  detected via new `(LOG, [q]) if …!is_linear_in` and `(ATAN, [q]) if …` arms.

**Examples that now evaluate:**
- `∫ log(x²+1) dx` = x·log(x²+1) − 2x + 2·atan(x)
- `∫ x·log(x²+1) dx` = (x²/2)·log(x²+1) − x²/2 + ½·log(x²+1)
- `∫ x²·log(x²+1) dx` = (x³/3)·log(x²+1) − 2x³/9 + 2x/3 − (2/3)·atan(x)
- `∫ x·atan(x²) dx` = (x²/2)·atan(x²) − ¼·log(1+x⁴)

**Fallthrough cases** (correctly left unevaluated):
- `∫ atan(x²) dx` — residual 2x²/(1+x⁴) requires irrational partial fractions
- `∫ atan(x) dx` — linear Q, not intercepted by Phase 28

**Tests added** (9 new tests):
- `phase28_log_x2p1_is_closed` — closed-form structure check
- `phase28_log_x2p1_numeric` — numerical correctness ∫₀¹ log(x²+1) dx
- `phase28_x_log_x2p1_is_closed` — closed-form structure check
- `phase28_x_log_x2p1_numeric` — numerical correctness
- `phase28_x2_log_x2p1_numeric` — numerical correctness
- `phase28_atan_x2_fallthrough` — stays unevaluated
- `phase28_x_atan_x2_is_closed` — closed-form structure check
- `phase28_x_atan_x2_numeric` — numerical correctness
- `phase28_regression_log_x_still_phase3` — Phase 3 regression
- `phase28_regression_atan_x_stays_unevaluated` — linear atan regression

## [0.4.0] — 2026-05-16

### Added — Phase 26: log-power integration via IBP reduction

- `is_log_of_x(node, x)` — guard helper: returns `true` when `node` is
  `Log(x)` for bare integration variable `x`.
- `to_polynomial_coeffs(expr, x)` — extracts polynomial coefficients from an
  IR expression; returns `Vec<(degree, coeff_node)>` or `None`.
  Handles constants, `x`, `x^k`, `c·f`, `f·c`, ADD, SUB, NEG.
- `poly_log_power_term(k, n, x)` — closed form of `∫ x^k · log(x)^n dx` for
  k ≥ 0, n ≥ 1, via the IBP reduction formula:
  `G_{k,m}(x) = x^(k+1)/(k+1) · log(x)^m  −  m/(k+1) · G_{k,m-1}(x)`.
- `try_log_power_product(transcendental, poly, x)` — handles
  `∫ Q(x) · log(x)^n dx` for integer n ≥ 2 by term-by-term application
  of `poly_log_power_term`.
- Standalone `∫ log(x)^n dx` (n ≥ 2) via new `(POW, [base, exp])` match arm.

### Added — Phase 27: trig-of-log integration via u = log(x) substitution

- `trig_log_integral(trig_head, k, x)` — closed form of `∫ x^k · trig(log(x)) dx`:
  - `∫ xᵏ sin(log x) dx = x^(k+1)·((k+1)sin(log x)−cos(log x))/((k+1)²+1)`
  - `∫ xᵏ cos(log x) dx = x^(k+1)·((k+1)cos(log x)+sin(log x))/((k+1)²+1)`
- `try_trig_log_product(transcendental, poly, x)` — handles
  `∫ Q(x)·sin(log(x)) dx` and `∫ Q(x)·cos(log(x)) dx`.
- Standalone `∫ sin(log(x)) dx` and `∫ cos(log(x)) dx` via new
  `(SIN|COS, [inner]) if is_log_of_x(inner, x)` match arms.

## [0.3.0] — 2026-05-14

### Added

- Added EllipticE (second kind) integration recognition:
  - `∫₀^(π/2) √(1-k²sin²θ) dθ` → `EllipticE(k)` (complete)
  - `∫ √(1-k²sin²θ) dθ` → `EllipticE(θ, k)` (incomplete)
- Added EllipticPi (third kind) complete integration recognition:
  - `∫₀^(π/2) 1/((1+n·sin²θ)·√(1-k²sin²θ)) dθ` → `EllipticPi(n, k)`
- New helper functions: `elliptic_second_kind_radicand`, `complete_elliptic_second_kind`,
  `incomplete_elliptic_second_kind`, `extract_characteristic_n`,
  `elliptic_third_kind_params`, `complete_elliptic_third_kind`

## [0.2.0] — 2026-05-14

### Added

- `Integrate` recognises canonical elliptic first-kind forms, returning
  `EllipticF(theta, k)` for the incomplete integral and `EllipticK(k)` for the
  complete `[0, %pi/2]` definite integral.
- `SymbolicBackend` now installs canonical `Factor` handling backed by
  `cas-factor`, including common-symbolic-factor extraction for additive
  multivariate expressions before univariate integer factorization.
- `Factor` extracts the greatest common integer content (GCD of all term
  coefficients) and intersection of common symbolic powers before attempting
  specific pattern matches. For example `factor(2*x + 4*y)` → `2*(x + 2*y)`,
  `factor(2*x*y + 2*x*z)` → `2*x*(y + z)`, and `factor(2*x^2*y - 2*y)` →
  `2*y*(x+1)*(x-1)` (the univariate residual is factored recursively).
- `Factor` recognises bivariate perfect-square trinomials such as
  `x^2 + 2*x*y + y^2` and rewrites them as `(x + y)^2`.
- `Factor` recognises bivariate difference-of-squares expressions such as
  `x^2 - y^2` and rewrites them as `(x - y) * (x + y)`.
- `Factor` recognises bivariate cubic identities such as `x^3 - y^3` and
  `x^3 + y^3`, rewriting them to their canonical linear/quadratic products.
- `Factor` recognises four-term bilinear grouping such as
  `x*y + x*z + y + z` and rewrites it as `(x + 1) * (y + z)`.
- `Factor` extracts shared multivariate integer content such as
  `2*x*y + 2*x*z`, including all-negative shared signs.
- `Factor` recognises four-term bivariate perfect-cube expansions such as
  `x^3 + 3*x^2*y + 3*x*y^2 + y^3` and `x^3 - 3*x^2*y + 3*x*y^2 - y^3`,
  rewriting them as `(x + y)^3` and `(x - y)^3` respectively.
- `SymbolicBackend` installs a `D` derivative handler for symbolic-only
  differentiation of arithmetic, elementary, hyperbolic, and inverse
  hyperbolic expressions; `StrictBackend` continues to reject `D` as an
  unknown head.
- Numeric and symbolic handlers for reciprocal hyperbolic heads `Coth`,
  `Sech`, and `Csch`, including `sech(0) = 1` and undefined-at-zero checks for
  `coth`/`csch`.

## [0.1.1] — 2026-04-28

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `symbolic-vm` package.
- `Backend` trait with `lookup`, `bind`, `on_unresolved`, `on_unknown_head`,
  `handler_for`, `rules`, `hold_heads`.
- `Handler` type alias: `Arc<dyn Fn(&mut VM, IRApply) -> IRNode + Send + Sync>`.
- `VM` struct with `eval(IRNode) -> IRNode` and `eval_program(Vec<IRNode>) -> Option<IRNode>`.
- `BaseBackend` — shared environment + held-heads for the two reference backends.
- `StrictBackend` — numeric-only evaluator; panics on unbound symbols or unknown heads.
- `SymbolicBackend` — Mathematica-style; unbound names stay as free variables;
  algebraic identities (`x+0→x`, `x*1→x`, `0*x→0`, `x^0→1`, etc.) are applied.
- Full handler table (34 handlers): `Add`, `Sub`, `Mul`, `Div`, `Pow`, `Neg`, `Inv`,
  `Sin`, `Cos`, `Tan`, `Exp`, `Log`, `Sqrt`, `Atan`, `Asin`, `Acos`, `Sinh`, `Cosh`,
  `Tanh`, `Asinh`, `Acosh`, `Atanh`, `Equal`, `NotEqual`, `Less`, `Greater`,
  `LessEqual`, `GreaterEqual`, `And`, `Or`, `Not`, `If`, `Assign`, `Define`, `List`.
- Exact rational arithmetic: `Numeric` enum preserving `Int(i64)`, `Rat(i64, i64)`,
  `Float(f64)` intermediate values; checked overflow falls back to `Float`.
- User-defined function support via `Define(name, List(params), body)` records,
  evaluated by substitution.
- 52 integration tests + 2 doc-tests; all passing.

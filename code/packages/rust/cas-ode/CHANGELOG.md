# Changelog — cas-ode (Rust)

## [0.4.0] — 2026-05-29

### Added

- **Track L2 — Lie point-symmetry handler for first-order ODEs**
  (`try_lie_symmetry` in new `src/lie_symmetry.rs`).  Port of the Python
  `cas_ode.lie_symmetry` module (Track L1, commit `d138e00f6`).
  - One generic detect-and-reduce pipeline for first-order ODEs that
    fall through every existing handler (Bernoulli, linear, separable,
    exact, homogeneous-type).  Three textbook point-symmetry groups
    are recognised numerically and reduced to a quadrature:
    1. **Translation in y**  `(x, y) → (x, y + c)` — `y' = f(x)` →
       direct integration `y = ∫ f dx + C`.
    2. **Translation in x**  `(x, y) → (x + c, y)` — autonomous
       `y' = g(y)` → inverse quadrature `x = ∫ 1/g(y) dy + C`.
       Catches the canonical logistic `y' = y(1 - y)` that separable
       cannot invert.
    3. **Scaling**  `(x, y) → (λx, λ^k y)` for integer
       `k ∈ [-3, 3] \ {0}` — similarity reduction `v = y / x^k`
       giving a separable ODE in `(v, x)`.  Numerical certificate
       confirms `f_subst / x^(k-1)` agrees with the extracted `G(v)`
       at three sample `(x, v)` pairs before integration is attempted.
  - Detection is *numerical*: substitute the candidate transformation
    into `f` and compare to the predicted transform at 3 fixed sample
    points × 3 sample `λ` (for scaling) × 7 candidate exponents ≤ 63
    numerical evaluations per ODE.  No symbolic linearised determining
    equation is computed.
  - All iteration is bounded.  `k` search space is hard-bounded to
    `[-3, 3]` with no escape hatch.  Test points and tolerances are
    fixed constants.
  - Dispatcher hook: inserted in `solve_ode` *after* every existing
    first-order family (Bernoulli, linear, separable, exact,
    homogeneous-type) and *before* the final fall-through.  Matches
    the Python ordering in `cas_ode.ode.solve_ode`.
  - Per Rust-cas-ode convention, the produced form keeps
    `Integrate(expr, var)` as structural IR — the package's downstream
    consumer evaluates these via the symbolic-vm.
- `pub(crate)` visibility added to internal helpers
  (`flatten_add`, `is_const_wrt`, `binary_args`, `unary_arg`,
  `unwrap_neg`, `sub`, `integrate`, `c`, `y_prime`) so the new module
  can reuse the dispatcher's local primitives without duplicating
  them.

## [0.3.0] — 2026-05-28

### Added

- **Track C2 — Frobenius / power-series ODE solver** (`try_frobenius_series`):
  - Solves second-order linear ODEs `P(x)·y'' + Q(x)·y' + R(x)·y = 0` with a
    regular singular point at `x = 0` by substituting `y = x^r · Σ a_n x^n`.
  - `poly_coeffs_extract` — extract rational polynomial coefficients of `expr`
    in `x` up to arbitrary `max_deg`.
  - `degree_of_monomial` — recursive `(coeff, degree)` decomposition for a
    monomial `c·x^k`; handles `Mul`, `Neg`, `Pow(x, k≥0)`, rational literals.
  - `frac_poly_mul` — truncated polynomial multiplication over `Frac`.
  - `is_regular_singular` — verify x=0 is a regular singular point and
    return the analytic Taylor series `tildeP = x·p(x)` and `tildeQ = x²·q(x)`.
    Computes `1 / P_eff(x)` (where `P_eff = P/x^m`) via the standard
    series-inverse recurrence; returns `None` for irregular singular points
    (order `m > 2`) or when the leading-zero analyticity condition fails.
  - `solve_indicial` — solve `r² + (p_0-1)r + q_0 = 0` for rational roots
    using `exact_sqrt_frac`; returns `None` on complex / irrational roots.
  - `roots_differ_by_integer` — guard for the logarithmic Frobenius case (bails).
  - `build_series_ir` — assemble `Mul(Pow(x, r), Add(a_0, a_1·x, …, a_N·x^N))`
    IR; signed `Frac` exponents encoded with `Neg(Rational(…))` for `r < 0`.
  - Dispatch order: inserted in `solve_ode` after `try_var_coeff_named_ode`
    and before `try_bernoulli`.  Mirrors the Python ordering: the named
    families (Bessel, Legendre, Hermite, Chebyshev) get first shot, and
    only un-named regular-singular-point ODEs reach the Frobenius helper.
  - Default truncation `N = 10`; recurrence is exact over `Frac` so the
    series matches the Python reference coefficient-for-coefficient
    (verified: `2x²y'' + 3xy' − (1+x)y = 0` produces `a_1 = 1/5`,
    `a_2 = 1/70`, `a_3 = 1/1890`, `a_4 = 1/83160`, …).
- Scope (deliberate parity with Track C1):
  - Singular point at `x = 0` only.
  - Indicial roots must be rational and differ by a non-integer.
  - Irregular singular points (order of vanishing of `P` > 2) bail.

## [0.2.0] — 2026-05-16

### Added

- **Phase 21 — Variable-coefficient named ODE recognition** (`try_var_coeff_named_ode`):
  - `collect_var2_coeffs` — extracts (P, Q, R) from `P(x)·y'' + Q(x)·y' + R(x)·y = 0`
    where the coefficients are arbitrary IR expressions in x (not just rationals).
  - `split_out_factor` — extracts coefficient K from `K·target` in Mul/Neg IR trees.
  - `eval_ir_at_xy` / `eval_ir_at_x` — recursive numeric IR evaluator at concrete f64 values.
  - `coeff_matches_func` — checks if an IR node ≈ expected analytic function at 4 test points.
  - `extract_const_val` — returns f64 value of constant (w.r.t. x) IR node.
  - `legendre_n_from_lambda` — finds non-negative integer n with n(n+1) = λ.
  - `nu_from_r_minus_xsq` — extracts rational ν from R(x) = x² − ν² (denominator ≤ 20).
  - `build_named_solution` — builds `Equal(y, %c1·F(n,x) + %c2·G(n,x))`.
  - `try_legendre_ode` — recognises `(1−x²)y'' − 2xy' + n(n+1)y = 0` → `LegendreP/Q(n,x)`.
  - `try_bessel_ode` — recognises `x²y'' + xy' + (x²−ν²)y = 0` → `BesselJ/Y(ν,x)`.
  - `try_hermite_ode` — recognises `y'' − 2xy' + 2ny = 0` → `HermiteH/H2(n,x)`.
  - `try_chebyshev_ode` — recognises `(1−x²)y'' − xy' + n²y = 0` → `ChebyshevT/U(n,x)`.
  - Dispatch order: Chebyshev → Legendre → Bessel → Hermite; inserted in `solve_ode` after
    `collect_euler_cauchy_coeffs`.
- Updated `symbolic-ir` dependency to `0.2.0` to access the Phase 27 named-ODE head constants.

## [0.1.0] — 2026-05-08

**Initial release — full Phase 18–20 symbolic ODE2 solver over symbolic IR.**

Ported from the Python `cas-ode` 0.5.0 reference implementation. All nine
ODE classes are implemented end-to-end including the Wronskian-based variation
of parameters fallback added in Python Phase 20.

### ODE classes supported

- **First-order linear** — `_collect_linear_first_order` + `solve_linear_first_order`:
  recognises `P(x)·y' + Q(x) = 0`, computes integrating factor μ = e^(∫P dx).
- **Separable** — `try_separable`: splits `g(y)·y' = h(x)`, integrates both sides.
- **Bernoulli** — `try_bernoulli`: substitution `v = y^(1-n)` reduces to linear.
- **Exact** — `try_exact`: verifies `∂M/∂y = ∂N/∂x`, constructs potential F(x,y).
- **Homogeneous-type** — `try_homogeneous_type`: detects `y' = f(y/x)`, substitutes `v = y/x`.
- **2nd-order constant-coefficient homogeneous** — `solve_second_order_const_coeff_frac`:
  solves characteristic equation `ar² + br + c = 0` for distinct real, repeated, and complex roots.
- **2nd-order constant-coefficient non-homogeneous (undetermined coefficients)** — `try_second_order_nonhom`:
  handles EPT-family forcing: constants, polynomials ≤ deg 2, exp, sin/cos, exp×sin/cos.
- **Euler-Cauchy equidimensional** — `try_euler_cauchy` + `solve_euler_cauchy_frac`:
  solves `ax²y'' + bxy' + cy = 0` via characteristic equation on `x^r`.
- **Variation of parameters (VoP) fallback** — `try_vop`:
  handles non-EPT forcing; Wronskian closed forms for distinct real, repeated, and complex roots.

### Public API

- `pub fn solve_ode(expr: IRNode, y: IRNode, x: IRNode) -> Option<IRNode>` — main dispatcher.
- `pub fn ode2_handler(expr: &IRNode) -> IRNode` — evaluates `ODE2(eqn, y, x)` IR nodes.
- `pub fn build_ode_handler_table() -> HashMap<String, Handler>` — VM integration.

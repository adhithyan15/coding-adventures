# Changelog — cas-ode (Rust)

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

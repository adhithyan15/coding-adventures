# Changelog — cas-ode (Rust)

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

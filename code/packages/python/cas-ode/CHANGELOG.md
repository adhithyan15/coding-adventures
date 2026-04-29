# Changelog — cas-ode

All notable changes to this project will be documented in this file.

## [0.2.0] — 2026-04-29

### Added

- **Bernoulli ODE solver** (`_try_bernoulli`)
  - Recognises `y' + P(x)·y = Q(x)·y^n` (n ≠ 0, 1) in zero form by
    scanning for `D(y,x)`, `y^n`, and `y` terms.
  - Applies the substitution `v = y^(1-n)` to reduce to the first-order
    linear ODE `v' + (1-n)·P·v = (1-n)·Q`, delegating to the existing
    integrating-factor solver.
  - Back-substitutes to return `Equal(y, v_sol^(1/(1-n)))`.
  - Handles integer `n` (positive or negative), arbitrary x-only P(x) and Q(x).

- **Exact ODE solver** (`_try_exact`)
  - Recognises `M(x,y) + N(x,y)·y' = 0` by extracting M (y'-free terms)
    and N (coefficient of `D(y,x)`).
  - Exactness check `∂M/∂y = ∂N/∂x` uses numerical evaluation at four
    interior test points (`_exprs_equal_numerically`) to handle structurally
    different but mathematically equal IR expressions from the VM's
    differentiation rules.
  - Computes the potential `F = ∫M dx`, then `g'(y) = N − ∂F/∂y`,
    then `g = ∫g'(y) dy`.
  - Returns the implicit solution `Equal(F + g, %c)`.
  - Runs last in the dispatch order so that explicitly solvable ODEs
    (separable, linear) return the preferred explicit `Equal(y, f(x))` form.

- **Second-order non-homogeneous solver** (`_try_second_order_nonhom`)
  - Recognises `a·y'' + b·y' + c·y = f(x)` with constant rational
    coefficients and a closed-form forcing function.
  - `_collect_second_order_nonhom` — extends the coefficient collector to
    capture the forcing term `f(x)` (x-only terms moved to the RHS).
  - `_classify_forcing` — identifies seven forcing families: constant,
    polynomial (degree ≤ 2), `e^(αx)`, `sin(βx)`, `cos(βx)`,
    `e^(αx)·sin(βx)`, `e^(αx)·cos(βx)`.
  - `_compute_particular` — undetermined-coefficients method for each
    family with full resonance handling:
    - Exponential: s = 0, 1, or 2 based on multiplicity of α as char root.
    - Trig: 2×2 linear system; falls through if det = 0 (resonance).
    - Exp×trig: exponential shift theorem to reduce to trig case.
    - Polynomial: matches from highest degree down with resonance shift.
  - Homogeneous solution from existing `solve_second_order_const_coeff`.
  - General solution: `y_h + y_p`.
  - Checked before the homogeneous solver in the dispatcher (prevents
    mis-classification of non-homogeneous ODEs as homogeneous).

- **Auxiliary helpers** (Section 9):
  - `_fold_numeric` — folds `Mul(a, Mul(b, expr))` when a,b are rationals.
  - `_eval_at_xy` — numerical evaluation of an IR tree at (x, y) = (xv, yv).
  - `_exprs_equal_numerically` — numerical equality check at four test points.
  - `_extract_linear_coeff_x` — extracts α from `α·x` patterns.
  - `_try_polynomial_forcing` — recognises polynomial IR trees up to degree 2.
  - `_char_poly_at` — evaluates `a·r² + b·r + c`.
  - `_is_pow_y` — detects `Pow(y, n)` atoms.

### Changed

- `solve_ode` dispatcher — new order:
  1. `_try_second_order_nonhom` (Phase 18)
  2. `_collect_second_order_coeffs` / `solve_second_order_const_coeff`
  3. `_try_bernoulli` (Phase 18)
  4. `_collect_linear_first_order` / `solve_linear_first_order`
  5. `_try_separable`
  6. `_try_exact` (Phase 18, last)

### Tests

- **47 new tests** in `tests/test_phase18.py` across 5 classes:
  - `TestPhase18_Bernoulli` — 10 tests: n=2,3,-1; P=1,x; fallthrough; structure
  - `TestPhase18_Exact` — 10 tests: 2xy/x², polynomial M/N; not-exact; implicit form
  - `TestPhase18_NonHomogeneous2ndOrder` — 12 tests: all forcing families;
    resonance exp; polynomial; structure checks
  - `TestPhase18_Fallthrough` — 7 tests: variable coeff, unrecognised forcing,
    trig resonance
  - `TestPhase18_Regressions` — 7 tests: all Phase 0.1.0 solver types
- Combined coverage: **82.89%** (135 tests total)

---

## [0.1.0] — 2026-04-27

### Added

- **Package foundation** — `cas-ode` 0.1.0 initial release.

- **First-order linear ODE solver** (`solve_linear_first_order`)
  - Recognises the standard form `y' + P(x)·y = Q(x)` by inspecting the
    flattened summands of the ODE expression.
  - Computes the integrating factor `μ = exp(∫ P dx)` via the VM's
    Integrate handler.
  - Returns `Equal(y, (1/μ) · (∫ μQ dx + %c))`.
  - Falls through gracefully if either integral is unevaluated (returns
    the original `ODE2(...)` node unchanged).

- **Separable ODE recogniser** (`_try_separable`)
  - Handles `y' = f(x)` (pure quadrature).
  - Handles `y' = k·y` (constant-coefficient decay/growth) by delegating
    to the linear solver.
  - Handles `y' = f(x)·k·y` (separable linear product) via the linear
    solver with `P = -k·f(x)`.
  - Handles `y' = f(x)·g(y) + Q(x)` generically by decomposing the RHS
    into a y-coefficient and a constant-with-respect-to-y term.

- **Second-order constant-coefficient solver**
  (`solve_second_order_const_coeff`)
  - Recognises `a·y'' + b·y' + c·y = 0` by pattern-matching against the
    flattened Add tree, extracting rational (Fraction) coefficients.
  - Solves the characteristic equation `a·r² + b·r + c = 0`:
    - Distinct real roots: `y = C1·exp(r1·x) + C2·exp(r2·x)`
    - Repeated root: `y = (C1 + C2·x)·exp(r·x)`
    - Complex conjugate roots `α±βi`: `y = exp(αx)·(C1·cos(βx) + C2·sin(βx))`
  - Handles irrational discriminants with symbolic `Pow(disc, 1/2)` nodes.
  - Uses `Fraction` arithmetic throughout — no floats for exact cases.

- **ODE2 VM handler** (`ode2_handler`, `build_ode_handler_table()`)
  - Accepts `ODE2(eqn, y, x)` where `eqn` may be a raw expression
    (assumed `= 0`) or an `Equal(lhs, rhs)` form.
  - Returns `Equal(y, solution)` on success; returns the unevaluated
    `ODE2(...)` node on failure (graceful fall-through).

- **Integration constants**
  - `%c` (`C_CONST`) — first-order ODE constant.
  - `%c1` (`C1`), `%c2` (`C2`) — second-order ODE constants.
  - Defined as IR symbols in `symbolic_ir/nodes.py` (version bump to 0.7.4).

- **Utility helpers** — `_flatten_add`, `_extract_coeff`,
  `_is_const_wrt`, `_isqrt_exact`, `_exact_sqrt_fraction`, and IR
  node builders (`_add`, `_mul`, `_sub`, `_div`, `_pow`, `_exp`, etc.)

- **88 tests** across 14 test classes covering all code paths.
  Coverage: 82%.

### Wired into the VM

- `symbolic_vm/cas_handlers.py` — calls `build_ode_handler_table()` and
  merges into the handler table (version bump to 0.32.5).
- `symbolic_vm/backends.py` — added `"ODE2"` to `_HELD_HEADS` so that
  `D(y, x)` inside the ODE expression is not pre-evaluated to `0`.
- `macsyma_runtime/name_table.py` — maps `"ode2"` to `ODE2` symbol
  (version bump to 1.8.0).
- `symbolic_ir` — added `ODE2`, `C_CONST`, `C1`, `C2` symbols and
  exports (version bump to 0.7.4).

### Not implemented

- **Bernoulli ODEs** (`dy/dx + P(x)·y = Q(x)·y^n`) — requires a
  `y^(1-n)` substitution and general rational-power symbolic handling.
  Deferred to a future `cas-ode` release.
- **Second-order with variable coefficients** — returns unevaluated
  (correct fall-through).
- **Non-homogeneous second-order** — method of undetermined coefficients
  or variation of parameters; deferred.
